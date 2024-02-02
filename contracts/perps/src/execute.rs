use std::cmp::Ordering;

use cosmwasm_std::{
    coins, Addr, BankMsg, Coin, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Response, StdError,
    Storage, Uint128,
};
use cw_utils::{may_pay, must_pay};
use mars_types::{
    math::SignedDecimal,
    oracle::ActionKind,
    perps::{
        CashFlow, Config, DenomState, Funding, PnL, Position, RealizedPnlAmounts, UnlockState,
        VaultState,
    },
};

use crate::{
    accounting::CashFlowExt,
    denom::DenomStateExt,
    error::{ContractError, ContractResult},
    position::PositionExt,
    state::{
        decrease_deposit_shares, increase_deposit_shares, update_realised_pnl_for_position, CONFIG,
        DENOM_STATES, OWNER, POSITIONS, REALISED_PNL_STATES, REALIZED_PNL, TOTAL_CASH_FLOW,
        UNLOCKS, VAULT_STATE,
    },
    vault::{amount_to_shares, shares_to_amount},
};

pub fn initialize(store: &mut dyn Storage, cfg: Config<Addr>) -> ContractResult<Response> {
    CONFIG.save(store, &cfg)?;

    // initialize vault state to zero total liquidity and zero total shares
    VAULT_STATE.save(store, &VaultState::default())?;

    // initialize global cash flow to zero
    TOTAL_CASH_FLOW.save(store, &CashFlow::default())?;

    Ok(Response::new().add_attribute("method", "initialize"))
}

pub fn init_denom(
    store: &mut dyn Storage,
    env: Env,
    sender: &Addr,
    denom: &str,
    max_funding_velocity: Decimal,
    skew_scale: Decimal,
) -> ContractResult<Response> {
    OWNER.assert_owner(store, sender)?;

    if DENOM_STATES.has(store, denom) {
        return Err(ContractError::DenomAlreadyExists {
            denom: denom.into(),
        });
    }

    if skew_scale.is_zero() {
        return Err(ContractError::InvalidParam {
            reason: "skew_scale cannot be zero".to_string(),
        });
    }

    let denom_state = DenomState {
        enabled: true,
        long_oi: Decimal::zero(),
        short_oi: Decimal::zero(),
        total_entry_cost: SignedDecimal::zero(),
        total_entry_funding: SignedDecimal::zero(),
        total_squared_positions: SignedDecimal::zero(),
        total_abs_multiplied_positions: SignedDecimal::zero(),
        cash_flow: CashFlow::default(),
        funding: Funding {
            max_funding_velocity,
            skew_scale,
            last_funding_rate: SignedDecimal::zero(),
            last_funding_accrued_per_unit_in_base_denom: SignedDecimal::zero(),
        },
        last_updated: env.block.time.seconds(),
    };
    DENOM_STATES.save(store, denom, &denom_state)?;

    Ok(Response::new()
        .add_attribute("method", "init_denom")
        .add_attribute("denom", denom)
        .add_attribute("max_funding_velocity", max_funding_velocity.to_string())
        .add_attribute("skew_scale", skew_scale.to_string()))
}

pub fn enable_denom(
    store: &mut dyn Storage,
    env: Env,
    sender: &Addr,
    denom: &str,
) -> ContractResult<Response> {
    OWNER.assert_owner(store, sender)?;

    DENOM_STATES.update(store, denom, |maybe_ds| {
        // if the denom does not already exist then we cannot enable it
        let Some(mut ds) = maybe_ds else {
            return Err(ContractError::DenomNotFound {
                denom: denom.into(),
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

        // When denom is disabled there is no trading activity so funding shouldn't be changed.
        // We just shift the last_updated time.
        ds.last_updated = env.block.time.seconds();

        Ok(ds)
    })?;

    Ok(Response::new().add_attribute("method", "enable_denom").add_attribute("denom", denom))
}

pub fn disable_denom(
    deps: DepsMut,
    env: Env,
    sender: &Addr,
    denom: &str,
) -> ContractResult<Response> {
    OWNER.assert_owner(deps.storage, sender)?;

    let cfg = CONFIG.load(deps.storage)?;

    DENOM_STATES.update(deps.storage, denom, |maybe_ds| {
        let Some(mut ds) = maybe_ds else {
            return Err(ContractError::DenomNotFound {
                denom: denom.into(),
            });
        };

        let current_time = env.block.time.seconds();

        let denom_price = cfg.oracle.query_price(&deps.querier, denom, ActionKind::Default)?.price;
        let base_denom_price =
            cfg.oracle.query_price(&deps.querier, &cfg.base_denom, ActionKind::Default)?.price;

        // refresh funding rate and index before disabling trading
        let current_funding = ds.current_funding(current_time, denom_price, base_denom_price)?;
        ds.funding = current_funding;

        ds.enabled = false;
        ds.last_updated = current_time;

        Ok(ds)
    })?;

    Ok(Response::new().add_attribute("method", "disable_denom").add_attribute("denom", denom))
}

pub fn deposit(deps: DepsMut, info: MessageInfo, current_time: u64) -> ContractResult<Response> {
    let cfg = CONFIG.load(deps.storage)?;
    let mut vs = VAULT_STATE.load(deps.storage)?;

    // find the deposit amount
    let amount = must_pay(&info, &cfg.base_denom)?;

    // compute the new shares to be minted to the depositor
    let shares =
        amount_to_shares(&deps.as_ref(), &vs, &cfg.oracle, current_time, &cfg.base_denom, amount)?;

    // increment total liquidity and deposit shares
    vs.total_liquidity = vs.total_liquidity.checked_add(amount)?;
    vs.total_shares = vs.total_shares.checked_add(shares)?;
    VAULT_STATE.save(deps.storage, &vs)?;

    // increment the user's deposit shares
    increase_deposit_shares(deps.storage, &info.sender, shares)?;

    Ok(Response::new()
        .add_attribute("method", "deposit")
        .add_attribute("amount", amount)
        .add_attribute("shares", shares))
}

pub fn unlock(
    deps: DepsMut,
    current_time: u64,
    depositor: &Addr,
    shares: Uint128,
) -> ContractResult<Response> {
    let cfg = CONFIG.load(deps.storage)?;
    let mut vs = VAULT_STATE.load(deps.storage)?;

    // convert the shares to amount
    let amount =
        shares_to_amount(&deps.as_ref(), &vs, &cfg.oracle, current_time, &cfg.base_denom, shares)?;

    // cannot unlock when there is zero shares
    if amount.is_zero() {
        return Err(ContractError::ZeroShares);
    }

    // decrement total liquidity and deposit shares
    vs.total_liquidity = vs.total_liquidity.checked_sub(amount)?;
    vs.total_shares = vs.total_shares.checked_sub(shares)?;
    VAULT_STATE.save(deps.storage, &vs)?;

    // decrement the user's deposit shares
    decrease_deposit_shares(deps.storage, depositor, shares)?;

    // add new unlock position
    let cooldown_end = current_time + cfg.cooldown_period;
    UNLOCKS.update(deps.storage, depositor, |maybe_unlocks| {
        let mut unlocks = maybe_unlocks.unwrap_or_default();

        unlocks.push(UnlockState {
            created_at: current_time,
            cooldown_end,
            amount,
        });

        Ok::<Vec<UnlockState>, StdError>(unlocks)
    })?;

    Ok(Response::new()
        .add_attribute("method", "unlock")
        .add_attribute("amount", amount)
        .add_attribute("shares", shares)
        .add_attribute("created_at", current_time.to_string())
        .add_attribute("cooldown_end", cooldown_end.to_string()))
}

pub fn withdraw(
    store: &mut dyn Storage,
    current_time: u64,
    depositor: &Addr,
) -> ContractResult<Response> {
    let cfg = CONFIG.load(store)?;
    let unlocks = UNLOCKS.load(store, depositor)?;

    // find all unlocked positions
    let (unlocked, unlocking): (Vec<_>, Vec<_>) =
        unlocks.into_iter().partition(|us| us.cooldown_end <= current_time);

    // cannot withdraw when there is zero unlocked positions
    if unlocked.is_empty() {
        return Err(ContractError::UnlockedPositionsNotFound {});
    }

    // clear state if no more unlocking positions
    if unlocking.is_empty() {
        UNLOCKS.remove(store, depositor);
    } else {
        UNLOCKS.save(store, depositor, &unlocking)?;
    }

    // compute the total amount to be withdrawn
    let unlocked_amt = unlocked.into_iter().map(|us| us.amount).sum::<Uint128>();

    Ok(Response::new()
        .add_attribute("method", "withdraw")
        .add_attribute("amount", unlocked_amt)
        .add_message(BankMsg::Send {
            to_address: depositor.into(),
            amount: coins(unlocked_amt.u128(), cfg.base_denom),
        }))
}

pub fn open_position(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    account_id: String,
    denom: String,
    size: SignedDecimal,
) -> ContractResult<Response> {
    let cfg = CONFIG.load(deps.storage)?;

    // only the credit manager contract can open positions
    if info.sender != cfg.credit_manager {
        return Err(ContractError::SenderIsNotCreditManager);
    }

    // the denom must exists and have been enabled
    let mut ds = DENOM_STATES.load(deps.storage, &denom)?;
    if !ds.enabled {
        return Err(ContractError::DenomNotEnabled {
            denom,
        });
    }

    // each account can only have one position for a denom at the same time
    if POSITIONS.has(deps.storage, (&account_id, &denom)) {
        return Err(ContractError::PositionExists {
            account_id,
            denom,
        });
    }

    // find the opening fee amount
    let mut opening_fee_amt = Uint128::zero();
    if !cfg.opening_fee_rate.is_zero() {
        opening_fee_amt = must_pay(&info, &cfg.base_denom)?;

        ds.cash_flow.update_opening_fees(opening_fee_amt)?;

        TOTAL_CASH_FLOW.update(deps.storage, |mut gcf| {
            gcf.update_opening_fees(opening_fee_amt)?;
            Ok::<CashFlow, ContractError>(gcf)
        })?;
    }

    // query the asset's price
    //
    // this will be the position's entry price, used to compute PnL when closing
    // the position
    let denom_price = cfg.oracle.query_price(&deps.querier, &denom, ActionKind::Default)?.price;
    let base_denom_price =
        cfg.oracle.query_price(&deps.querier, &cfg.base_denom, ActionKind::Default)?.price;

    // the position's initial value cannot be too small
    let price = denom_price.checked_div(base_denom_price)?;
    let position_in_base_denom = size.abs.checked_mul(price)?.to_uint_floor();
    if position_in_base_denom < cfg.min_position_in_base_denom {
        return Err(ContractError::PositionTooSmall {
            min: cfg.min_position_in_base_denom,
            found: position_in_base_denom,
            base_denom: cfg.base_denom,
        });
    }
    // skew _before_ modification
    let inital_skew = ds.skew()?;

    // The position's initial value cannot be too big.
    // Could be set to None if not needed.
    if let Some(max_pos_in_base_denom) = cfg.max_position_in_base_denom {
        if position_in_base_denom > max_pos_in_base_denom {
            return Err(ContractError::PositionTooBig {
                max: max_pos_in_base_denom,
                found: position_in_base_denom,
                base_denom: cfg.base_denom,
            });
        }
    }

    // validate the position's size
    let perp_params = cfg.params.query_perp_params(&deps.querier, &denom)?;
    ds.validate_position(size, &perp_params)?;

    // Update the denom's accumulators.
    // Funding rates and index is updated to the current block time (using old size).
    // TODO -  update opening_fee here
    ds.increase_position(env.block.time.seconds(), size, denom_price, base_denom_price)?;
    DENOM_STATES.save(deps.storage, &denom, &ds)?;

    // save the user's new position with updated funding index
    POSITIONS.save(
        deps.storage,
        (&account_id, &denom),
        &Position {
            size,
            entry_price: denom_price,
            entry_accrued_funding_per_unit_in_base_denom: ds
                .funding
                .last_funding_accrued_per_unit_in_base_denom,
            initial_skew: inital_skew,
            opening_fee_in_base_denom: opening_fee_amt,
        },
    )?;

    Ok(Response::new()
        .add_attribute("method", "open_position")
        .add_attribute("account_id", account_id)
        .add_attribute("denom", denom)
        .add_attribute("size", size.to_string())
        .add_attribute("entry_price", denom_price.to_string()))
}

pub fn close_position(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    account_id: String,
    denom: String,
) -> ContractResult<Response> {
    let position = POSITIONS.load(deps.storage, (&account_id, &denom))?;
    update_position_state(deps, env, info, position, account_id, denom, SignedDecimal::zero())
}

pub fn modify_position(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    account_id: String,
    denom: String,
    new_size: SignedDecimal,
) -> ContractResult<Response> {
    let position = POSITIONS.load(deps.storage, (&account_id, &denom))?;
    update_position_state(deps, env, info, position, account_id, denom, new_size)
}

fn update_position_state(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    position: Position,
    account_id: String,
    denom: String,
    new_size: SignedDecimal,
) -> ContractResult<Response> {
    let mut msgs = vec![];

    // States
    let cfg = CONFIG.load(deps.storage)?;
    let mut vs = VAULT_STATE.load(deps.storage)?;
    let mut ds = DENOM_STATES.load(deps.storage, &denom)?;

    // Only the credit manager contract can adjust positions
    if info.sender != cfg.credit_manager {
        return Err(ContractError::SenderIsNotCreditManager);
    }

    let entry_size = position.size;
    let q_change = new_size.checked_sub(position.size)?;

    // Check if we have flipped sides (e.g long -> short or vice versa).
    // To reduce complexity and contract size we reject this.
    // Users should use independent close and open actions.
    if !new_size.is_zero() && new_size.is_positive() != position.size.is_positive() {
        return Err(ContractError::IllegalPositionModification {
            reason: "Cannot flip Position. Submit independent close and open messages".to_string(),
        });
    }

    // skew _before_ modification
    let initial_skew = ds.skew()?;

    // Prices
    let entry_price = position.entry_price;
    let denom_price = cfg.oracle.query_price(&deps.querier, &denom, ActionKind::Default)?.price;
    let base_denom_price =
        cfg.oracle.query_price(&deps.querier, &cfg.base_denom, ActionKind::Default)?.price;
    let price = denom_price.checked_div(base_denom_price)?;
    let position_in_base_denom = new_size.abs.checked_mul(price)?.to_uint_floor();

    // When modifying a position, we must realise all PnL. The credit manager
    // may send no coin (in case the position is winning or breaking even) or
    // one coin of the base denom (i.e usdc) in case the position is losing
    let paid_amount = may_pay(&info, &cfg.base_denom)?;

    match new_size.abs.cmp(&position.size.abs) {
        Ordering::Less => {
            // Enforce min size if reducing
            if position_in_base_denom < cfg.min_position_in_base_denom && !new_size.is_zero() {
                return Err(ContractError::PositionTooSmall {
                    min: cfg.min_position_in_base_denom,
                    found: position_in_base_denom,
                    base_denom: cfg.base_denom,
                });
            }

            // Update the denom's accumulators.
            // Funding rates and index is updated to the current block time (using old size).
            ds.decrease_position(
                env.block.time.seconds(),
                denom_price,
                base_denom_price,
                &position,
                // the sign in this method is used to determine the position side.
                // When we are reducing a position size, q_change sign will be inverse,
                // Therefore, we need to reverse the sign.
                SignedDecimal::zero().checked_sub(q_change)?,
            )?;
        }

        // Increase position
        Ordering::Greater => {
            // When a denom is disabled it should be close only
            if !ds.enabled {
                return Err(ContractError::DenomNotEnabled {
                    denom,
                });
            }

            // Enforce position size cannot be too big when increasing.
            if let Some(max_pos_in_base_denom) = cfg.max_position_in_base_denom {
                if position_in_base_denom > max_pos_in_base_denom {
                    return Err(ContractError::PositionTooBig {
                        max: max_pos_in_base_denom,
                        found: position_in_base_denom,
                        base_denom: cfg.base_denom,
                    });
                }
            }

            // validate the position's size
            let perp_params = cfg.params.query_perp_params(&deps.querier, &denom)?;
            ds.validate_position(q_change, &perp_params)?; // q change

            // Update the denom's accumulators.
            // Funding rates and index is updated to the current block time (using old size).
            ds.increase_position(
                env.block.time.seconds(),
                q_change,
                denom_price,
                base_denom_price,
            )?;
        }

        // Means we have submitted a new size the same as the old size.
        std::cmp::Ordering::Equal => {
            return Err(ContractError::IllegalPositionModification {
                reason: "new_size is equal to old_size.".to_string(),
            })
        }
    };

    // REALISE PNL
    // ===========
    // compute the position's unrealized PnL
    let (pnl, pnl_amounts) = position.compute_pnl(
        &ds.funding,
        initial_skew,
        denom_price,
        base_denom_price,
        &cfg.base_denom,
        cfg.closing_fee_rate,
        true,
        Some(q_change),
    )?;

    // update realized PnL
    REALIZED_PNL.update(deps.storage, (&account_id, &denom), |maybe_realized_pnl| {
        let mut realized_pnl = maybe_realized_pnl.unwrap_or_default();
        realized_pnl.update(&pnl_amounts, position.opening_fee_in_base_denom)?;
        Ok::<RealizedPnlAmounts, ContractError>(realized_pnl)
    })?;

    // update the cash flow
    ds.cash_flow.update(&pnl_amounts)?;
    TOTAL_CASH_FLOW.update(deps.storage, |mut gcf| {
        gcf.update(&pnl_amounts)?;
        Ok::<CashFlow, ContractError>(gcf)
    })?;

    let (send_amount, updated_liquidity) =
        execute_payment(&cfg.base_denom, vs.total_liquidity, paid_amount, &pnl.coins.pnl)?;

    vs.total_liquidity = updated_liquidity;

    if !send_amount.is_zero() {
        // send coins to credit manager
        let send_msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: cfg.credit_manager.into(),
            amount: coins(send_amount.u128(), cfg.base_denom),
        });
        msgs.push(send_msg);
    }

    // Modify or delete position states
    let method = match new_size.is_zero() {
        // Delete the position and related state when position size modified to zero.
        true => {
            POSITIONS.remove(deps.storage, (&account_id, &denom));
            REALISED_PNL_STATES.remove(deps.storage, (&account_id, &denom));
            "close_position"
        }

        // Update position and realised pnl states
        false => {
            // Increment realised pnl state
            update_realised_pnl_for_position(deps.storage, &account_id, &denom, pnl.values)?;

            // Save updated position
            POSITIONS.save(
                deps.storage,
                (&account_id, &denom),
                &Position {
                    size: new_size,
                    entry_price: denom_price,
                    entry_accrued_funding_per_unit_in_base_denom: ds
                        .funding
                        .last_funding_accrued_per_unit_in_base_denom,
                    initial_skew,
                    opening_fee_in_base_denom: position.opening_fee_in_base_denom, // FIXME: opening fee should be updated @piobab
                },
            )?;
            "modify_position"
        }
    };

    // Save global denom and vault states
    VAULT_STATE.save(deps.storage, &vs)?;
    DENOM_STATES.save(deps.storage, &denom, &ds)?;

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("method", method)
        .add_attribute("account_id", account_id)
        .add_attribute("denom", denom)
        .add_attribute("starting_size", entry_size.to_string())
        .add_attribute("new_size", new_size.to_string())
        .add_attribute("entry_price", entry_price.to_string())
        .add_attribute("current_price", denom_price.to_string())
        .add_attribute("realised_pnl", pnl.coins.pnl.to_string()))
}

/// Compute how many coins should be sent to the credit account, and
/// update global liquidity amount.
/// Credit manager doesn't send more coins than required.
fn execute_payment(
    base_denom: &str,
    total_liquidity: Uint128,
    paid_amount: Uint128,
    pnl: &PnL,
) -> Result<(Uint128, Uint128), ContractError> {
    let (send_amount, updated_liquidity) = match pnl {
        PnL::Profit(Coin {
            amount,
            ..
        }) => {
            if !paid_amount.is_zero() {
                // if the position is profitable, the credit manager should not send any coins
                return Err(ContractError::InvalidPayment {
                    denom: base_denom.to_string(),
                    required: Uint128::zero(),
                    received: paid_amount,
                });
            }

            (*amount, total_liquidity.checked_sub(*amount)?)
        }
        PnL::Loss(Coin {
            amount,
            ..
        }) => {
            if paid_amount != *amount {
                // if the position is losing, the credit manager should send exactly one coin
                // of the base denom
                return Err(ContractError::InvalidPayment {
                    denom: base_denom.to_string(),
                    required: *amount,
                    received: paid_amount,
                });
            }

            (Uint128::zero(), total_liquidity.checked_add(*amount)?)
        }
        PnL::BreakEven => {
            if !paid_amount.is_zero() {
                // if the position is breaking even, the credit manager should not send any coins
                return Err(ContractError::InvalidPayment {
                    denom: base_denom.to_string(),
                    required: Uint128::zero(),
                    received: paid_amount,
                });
            }

            (Uint128::zero(), total_liquidity)
        }
    };
    Ok((send_amount, updated_liquidity))
}
