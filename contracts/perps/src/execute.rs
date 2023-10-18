use cosmwasm_std::{
    coin, coins, to_binary, Addr, BankMsg, Coin, DepsMut, MessageInfo, Response, Storage, Uint128,
    WasmMsg,
};
use cw_utils::{may_pay, must_pay, nonpayable};
use mars_types::{
    credit_manager::{self, Action},
    math::SignedDecimal,
    oracle::ActionKind,
    perps::{Config, DenomState, PnL, Position, VaultState},
};

use crate::{
    error::{ContractError, ContractResult},
    pnl::compute_pnl,
    state::{
        decrease_deposit_shares, increase_deposit_shares, CONFIG, DENOM_STATES, OWNER, POSITIONS,
        VAULT_STATE,
    },
    vault::{amount_to_shares, shares_to_amount},
};

pub fn initialize(store: &mut dyn Storage, cfg: Config<Addr>) -> ContractResult<Response> {
    CONFIG.save(store, &cfg)?;

    // initialize vault state to zero total liquidity and zero total shares
    VAULT_STATE.save(store, &VaultState::default())?;

    Ok(Response::new().add_attribute("method", "initialize"))
}

pub fn enable_denom(
    store: &mut dyn Storage,
    sender: &Addr,
    denom: &str,
) -> ContractResult<Response> {
    OWNER.assert_owner(store, sender)?;

    DENOM_STATES.update(store, &denom, |maybe_ds| {
        // if the denom does not already exist, initialize the denom state with
        // zero total size and cost basis
        let Some(mut ds) = maybe_ds else {
            return Ok(DenomState {
                enabled: true,
                ..Default::default()
            });
        };

        // if the denom already exists, if must have not already been enabled
        if ds.enabled {
            return Err(ContractError::DenomEnabled {
                denom: denom.into(),
            });
        }

        // now we know the denom exists and is not enabled
        // flip the enabled parameter to true and return
        ds.enabled = true;

        Ok(ds)
    })?;

    Ok(Response::new().add_attribute("method", "enable_denom").add_attribute("denom", denom))
}

pub fn disable_denom(
    store: &mut dyn Storage,
    sender: &Addr,
    denom: &str,
) -> ContractResult<Response> {
    OWNER.assert_owner(store, sender)?;

    DENOM_STATES.update(store, &denom, |maybe_ds| {
        let Some(mut ds) = maybe_ds else {
            return Err(ContractError::DenomNotFound {
                denom: denom.into(),
            });
        };

        ds.enabled = false;

        Ok(ds)
    })?;

    Ok(Response::new().add_attribute("method", "disable_denom").add_attribute("denom", denom))
}

pub fn deposit(store: &mut dyn Storage, info: MessageInfo) -> ContractResult<Response> {
    let cfg = CONFIG.load(store)?;
    let mut vs = VAULT_STATE.load(store)?;

    // find the deposit amount
    let amount = must_pay(&info, &cfg.base_denom)?;

    // compute the new shares to be minted to the depositor
    let shares = amount_to_shares(&vs, amount)?;

    // increment total liquidity and deposit shares
    vs.total_liquidity = vs.total_liquidity.checked_add(amount)?;
    vs.total_shares = vs.total_shares.checked_add(shares)?;
    VAULT_STATE.save(store, &vs)?;

    // increment the user's deposit shares
    increase_deposit_shares(store, &info.sender, shares)?;

    Ok(Response::new()
        .add_attribute("method", "deposit")
        .add_attribute("amount", amount)
        .add_attribute("shares", shares))
}

pub fn withdraw(
    store: &mut dyn Storage,
    depositor: &Addr,
    shares: Uint128,
) -> ContractResult<Response> {
    let cfg = CONFIG.load(store)?;
    let mut vs = VAULT_STATE.load(store)?;

    // convert the shares to amount
    //
    // no need to check whether amount is zero; if it is then the MsgSend will
    // naturally fail
    let amount = shares_to_amount(&vs, shares)?;

    // decrement total liquidity and deposit shares
    vs.total_liquidity = vs.total_liquidity.checked_sub(amount)?;
    vs.total_shares = vs.total_shares.checked_sub(shares)?;
    VAULT_STATE.save(store, &vs)?;

    // decrement the user's deposit shares
    decrease_deposit_shares(store, depositor, shares)?;

    Ok(Response::new()
        .add_attribute("method", "withdraw")
        .add_attribute("amount", amount)
        .add_attribute("shares", shares)
        .add_message(BankMsg::Send {
            to_address: depositor.into(),
            amount: coins(amount.u128(), cfg.base_denom),
        }))
}

pub fn open_position(
    deps: DepsMut,
    info: MessageInfo,
    account_id: String,
    denom: String,
    size: SignedDecimal,
) -> ContractResult<Response> {
    let cfg = CONFIG.load(deps.storage)?;

    // no payment is expected when opening a position
    nonpayable(&info)?;

    // query the asset's price
    //
    // this will be the position's entry price, used to compute PnL when closing
    // the position
    let entry_price = cfg.oracle.query_price(&deps.querier, &denom, ActionKind::Default)?.price;

    // only the credit manager contract can open positions
    if info.sender != cfg.credit_manager {
        return Err(ContractError::SenderIsNotCreditManager);
    }

    // the denom must exists and have been enabled
    let ds = DENOM_STATES.load(deps.storage, &denom)?;
    if !ds.enabled {
        return Err(ContractError::DenomNotEnabled {
            denom,
        });
    }

    // the position's initial value cannot be too small
    let value = size.abs.checked_mul(entry_price)?.to_uint_floor();
    if value < cfg.min_position_size {
        return Err(ContractError::PositionTooSmall {
            min: cfg.min_position_size,
            found: value,
        });
    }

    // each account can only have one position for a denom at the same time
    if POSITIONS.has(deps.storage, (&account_id, &denom)) {
        return Err(ContractError::PositionExists {
            account_id,
            denom,
        });
    }

    // save the user's new position
    POSITIONS.save(
        deps.storage,
        (&account_id, &denom),
        &Position {
            size,
            entry_price,
        },
    )?;

    Ok(Response::new()
        .add_attribute("method", "open_position")
        .add_attribute("account_id", account_id)
        .add_attribute("denom", denom)
        .add_attribute("size", size.to_string())
        .add_attribute("entry_price", entry_price.to_string()))
}

pub fn close_position(
    deps: DepsMut,
    info: MessageInfo,
    account_id: String,
    denom: String,
) -> ContractResult<Response> {
    let mut res = Response::new();

    let cfg = CONFIG.load(deps.storage)?;
    let mut vs = VAULT_STATE.load(deps.storage)?;
    let position = POSITIONS.load(deps.storage, (&account_id, &denom))?;

    // when closing a position, the credit manager may send no coin (in case the
    // the position is winning or breaking even) or one coin of the base denom
    // (in case the position is losing)
    let paid_amount = may_pay(&info, &cfg.base_denom)?;

    // only the credit manager contract can close positions
    if info.sender != cfg.credit_manager {
        return Err(ContractError::SenderIsNotCreditManager);
    }

    // query the current price of the asset
    let exit_price = cfg.oracle.query_price(&deps.querier, &denom, ActionKind::Default)?.price;

    // compute the position's unrealized PnL
    let pnl = compute_pnl(&position, exit_price, &cfg.base_denom)?;

    // compute how many coins should be returned to the credit account, and
    // update global liquidity amount
    let refund_amount = match &pnl {
        PnL::Profit(Coin {
            amount,
            ..
        }) => {
            vs.total_liquidity = vs.total_liquidity.checked_sub(*amount)?;
            paid_amount.checked_add(*amount)?
        }
        PnL::Loss(Coin {
            amount,
            ..
        }) => {
            vs.total_liquidity = vs.total_liquidity.checked_add(*amount)?;
            paid_amount.checked_sub(*amount)?
        }
        PnL::BreakEven => paid_amount,
    };

    if !refund_amount.is_zero() {
        res = res.add_message(WasmMsg::Execute {
            contract_addr: cfg.credit_manager.into(),
            msg: to_binary(&credit_manager::ExecuteMsg::UpdateCreditAccount {
                account_id: account_id.clone(),
                actions: vec![Action::Deposit(coin(refund_amount.u128(), &cfg.base_denom))],
            })?,
            funds: coins(refund_amount.u128(), cfg.base_denom),
        });
    }

    // save the updated global state and delete position
    VAULT_STATE.save(deps.storage, &vs)?;
    POSITIONS.remove(deps.storage, (&account_id, &denom));

    Ok(res
        .add_attribute("method", "close_position")
        .add_attribute("account_id", account_id)
        .add_attribute("denom", denom)
        .add_attribute("size", position.size.to_string())
        .add_attribute("entry_price", position.entry_price.to_string())
        .add_attribute("exit_price", exit_price.to_string())
        .add_attribute("realized_pnl", pnl.to_string()))
}

// ----------------------------------- Tests -----------------------------------

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage},
        OwnedDeps,
    };
    use cw2::ContractVersion;
    use mars_owner::{OwnerError, OwnerResponse};
    use mars_types::{adapters::oracle::Oracle, perps::DenomStateResponse};

    use super::*;
    use crate::{
        contract::{instantiate, CONTRACT_NAME, CONTRACT_VERSION},
        query,
    };

    fn mock_cfg() -> Config<Addr> {
        Config {
            credit_manager: Addr::unchecked("credit_manager"),
            oracle: Oracle::new(Addr::unchecked("oracle")),
            base_denom: "uusdc".into(),
            min_position_size: Uint128::new(5_000_000),
        }
    }

    fn setup_test() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
        let mut deps = mock_dependencies();

        instantiate(deps.as_mut(), mock_env(), mock_info("larry", &[]), mock_cfg().into()).unwrap();

        deps
    }

    #[test]
    fn proper_initialization() {
        let deps = setup_test();

        let version = cw2::get_contract_version(deps.as_ref().storage).unwrap();
        assert_eq!(
            version,
            ContractVersion {
                contract: CONTRACT_NAME.into(),
                version: CONTRACT_VERSION.into(),
            },
        );

        let owner = OWNER.query(deps.as_ref().storage).unwrap();
        assert_eq!(
            owner,
            OwnerResponse {
                owner: Some("larry".into()),
                proposed: None,
                initialized: true,
                abolished: false,
                emergency_owner: None,
            },
        );

        let cfg = CONFIG.load(deps.as_ref().storage).unwrap();
        assert_eq!(cfg, mock_cfg());
    }

    #[test]
    fn updating_denoms() {
        let mut deps = setup_test();

        // non-owner cannot enable denoms
        {
            let err = enable_denom(deps.as_mut().storage, &Addr::unchecked("pumpkin"), "uosmo")
                .unwrap_err();
            assert_eq!(err, OwnerError::NotOwner {}.into());
        }

        // owner can enable denoms
        // in this test we try listing two denoms
        {
            enable_denom(deps.as_mut().storage, &Addr::unchecked("larry"), "perp/eth/eur").unwrap();
            enable_denom(deps.as_mut().storage, &Addr::unchecked("larry"), "perp/btc/usd").unwrap();

            let dss = query::denom_states(deps.as_ref().storage, None, None).unwrap();
            assert_eq!(
                dss,
                [
                    // note: denoms are ordered alphabetically
                    DenomStateResponse {
                        denom: "perp/btc/usd".into(),
                        enabled: true,
                        ..Default::default()
                    },
                    DenomStateResponse {
                        denom: "perp/eth/eur".into(),
                        enabled: true,
                        ..Default::default()
                    },
                ],
            );
        }

        // non-owner cannot disable denoms
        {
            let err =
                disable_denom(deps.as_mut().storage, &Addr::unchecked("jake"), "perp/btc/usd")
                    .unwrap_err();
            assert_eq!(err, OwnerError::NotOwner {}.into());
        }

        // owner can disable denoms
        {
            disable_denom(deps.as_mut().storage, &Addr::unchecked("larry"), "perp/btc/usd")
                .unwrap();

            let dss = query::denom_states(deps.as_ref().storage, None, None).unwrap();
            assert_eq!(
                dss,
                [
                    DenomStateResponse {
                        denom: "perp/btc/usd".into(),
                        enabled: false,
                        ..Default::default()
                    },
                    DenomStateResponse {
                        denom: "perp/eth/eur".into(),
                        enabled: true,
                        ..Default::default()
                    }
                ]
            );
        }
    }
}
