use std::{collections::HashSet, hash::Hash};

use cosmwasm_std::{
    ensure, to_json_binary, Addr, Coin, ContractInfoResponse, CosmosMsg, Decimal, Deps, DepsMut,
    QuerierWrapper, QueryRequest, StdResult, Storage, Uint128, WasmMsg,
};

use mars_types::{
    credit_manager::{ActionCoin, CallbackMsg, ChangeExpected, ExecuteMsg},
    health::AccountKind,
};

use crate::{
    error::{ContractError, ContractResult},
    state::{
        ACCOUNT_KINDS, ACCOUNT_NFT, COIN_BALANCES, MAX_SLIPPAGE, PARAMS, PERPS, RED_BANK, TOTAL_DEBT_SHARES
    },
    update_coin_balances::query_balance,
};

/// Assert that the transaction sender is authorized to update the account.
///
/// Two actors are authorized: the NFT owner, and the perps contract.
///
/// The only case the perps contract may need to update the account is when
/// closing a perp position that is in profit, it needs to deposit the profit
/// into the account.
pub fn assert_is_authorized(deps: &DepsMut, user: &Addr, account_id: &str) -> ContractResult<()> {
    let owner = query_nft_token_owner(deps.as_ref(), account_id)?;
    if user != &owner {
        let perps = PERPS.load(deps.storage)?;
        if user != perps.address() {
            return Err(ContractError::NotTokenOwner {
                user: user.to_string(),
                account_id: account_id.to_string(),
            });
        }
    }
    Ok(())
}

/// Asserts that a vault contract is not in the blacklist.
///
/// This function performs a safety check to ensure that the specified vault contract
/// is not in the blacklist of vaults. Blacklisted vaults are considered unsafe or
/// deprecated and should not be interacted with.
///
/// # Arguments
///
/// * `deps` - A mutable reference to the dependencies, which includes storage and querier
/// * `vault` - The address of the vault contract to check
///
/// # Returns
///
/// * `ContractResult<()>` - Returns `Ok(())` if the vault is not blacklisted, or an error if:
///   - The vault is found in the blacklist
///   - The blacklist query fails
///
/// # Errors
///
/// * `ContractError::BlacklistedVault` - If the vault address is found in the blacklist
pub fn assert_is_not_blacklisted(deps: &DepsMut, vault: &Addr) -> ContractResult<()> {


    let params_addr = PARAMS.load(deps.storage)?;
    let config = params_addr.query_managed_vault_config(&deps.querier)?;
    let blacklisted_vaults = config.blacklisted_vaults;

    if blacklisted_vaults.contains(&vault.to_string()) {
        return Err(ContractError::BlacklistedVault { vault: vault.to_string() });
    }
    Ok(())
}

pub fn assert_max_slippage(max_slippage: Decimal) -> ContractResult<()> {
    if max_slippage.is_zero() || max_slippage >= Decimal::one() {
        return Err(ContractError::InvalidConfig {
            reason: "Max slippage must be greater than 0 and less than 1".to_string(),
        });
    }
    Ok(())
}

pub fn assert_slippage(storage: &dyn Storage, slippage: Decimal) -> ContractResult<()> {
    let max_slippage = MAX_SLIPPAGE.load(storage)?;
    if slippage > max_slippage {
        return Err(ContractError::SlippageExceeded {
            slippage,
            max_slippage,
        });
    }
    Ok(())
}

pub fn assert_perps_lb_ratio(perps_lb_ratio: Decimal) -> ContractResult<()> {
    if perps_lb_ratio > Decimal::one() {
        return Err(ContractError::InvalidConfig {
            reason: "Perps liquidation bonus ratio must be less than or equal to 1".to_string(),
        });
    }
    Ok(())
}

pub fn assert_withdraw_enabled(
    storage: &dyn Storage,
    querier: &QuerierWrapper,
    denom: &str,
) -> ContractResult<()> {
    let params = PARAMS.load(storage)?;
    let params_opt = params.query_asset_params(querier, denom)?;

    if let Some(params) = params_opt {
        ensure!(
            params.credit_manager.withdraw_enabled,
            ContractError::WithdrawNotEnabled {
                denom: denom.to_string(),
            }
        );
    };

    Ok(())
}

pub fn query_nft_token_owner(deps: Deps, account_id: &str) -> ContractResult<String> {
    Ok(ACCOUNT_NFT.load(deps.storage)?.query_nft_token_owner(&deps.querier, account_id)?)
}

pub fn assert_coin_is_whitelisted(deps: &mut DepsMut, denom: &str) -> ContractResult<()> {
    let params = PARAMS.load(deps.storage)?;
    match params.query_asset_params(&deps.querier, denom) {
        Ok(Some(p)) if p.credit_manager.whitelisted => Ok(()),
        _ => Err(ContractError::NotWhitelisted(denom.to_string())),
    }
}

pub fn assert_coins_are_whitelisted(deps: &mut DepsMut, denoms: Vec<&str>) -> ContractResult<()> {
    denoms.iter().try_for_each(|denom| assert_coin_is_whitelisted(deps, denom))
}

pub fn get_account_kind(storage: &dyn Storage, account_id: &str) -> ContractResult<AccountKind> {
    Ok(ACCOUNT_KINDS.may_load(storage, account_id)?.unwrap_or(AccountKind::Default))
}

pub fn increment_coin_balance(
    storage: &mut dyn Storage,
    account_id: &str,
    coin: &Coin,
) -> ContractResult<Uint128> {
    COIN_BALANCES.update(storage, (account_id, &coin.denom), |value_opt| {
        value_opt
            .unwrap_or_else(Uint128::zero)
            .checked_add(coin.amount)
            .map_err(ContractError::Overflow)
    })
}

pub fn decrement_coin_balance(
    storage: &mut dyn Storage,
    account_id: &str,
    coin: &Coin,
) -> ContractResult<Uint128> {
    let path = COIN_BALANCES.key((account_id, &coin.denom));
    let value_opt = path.may_load(storage)?;
    let new_value = value_opt.unwrap_or_else(Uint128::zero).checked_sub(coin.amount)?;
    if new_value.is_zero() {
        path.remove(storage);
    } else {
        path.save(storage, &new_value)?;
    }
    Ok(new_value)
}

pub fn update_balance_msg(
    querier: &QuerierWrapper,
    credit_manager_addr: &Addr,
    account_id: &str,
    denom: &str,
    change: ChangeExpected,
) -> StdResult<CosmosMsg> {
    let previous_balance = query_balance(querier, credit_manager_addr, denom)?;
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: credit_manager_addr.to_string(),
        funds: vec![],
        msg: to_json_binary(&ExecuteMsg::Callback(CallbackMsg::UpdateCoinBalance {
            account_id: account_id.to_string(),
            previous_balance,
            change,
        }))?,
    }))
}

pub fn update_balances_msgs(
    querier: &QuerierWrapper,
    credit_manager_addr: &Addr,
    account_id: &str,
    denoms: Vec<&str>,
    change: ChangeExpected,
) -> StdResult<Vec<CosmosMsg>> {
    denoms
        .iter()
        .map(|denom| {
            update_balance_msg(querier, credit_manager_addr, account_id, denom, change.clone())
        })
        .collect()
}

pub fn update_balance_after_vault_liquidation_msg(
    querier: &QuerierWrapper,
    credit_manager_addr: &Addr,
    account_id: &str,
    denom: &str,
    protocol_fee: Decimal,
) -> StdResult<CosmosMsg> {
    let previous_balance = query_balance(querier, credit_manager_addr, denom)?;
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: credit_manager_addr.to_string(),
        funds: vec![],
        msg: to_json_binary(&ExecuteMsg::Callback(
            CallbackMsg::UpdateCoinBalanceAfterVaultLiquidation {
                account_id: account_id.to_string(),
                previous_balance,
                protocol_fee,
            },
        ))?,
    }))
}

pub fn debt_shares_to_amount(deps: Deps, denom: &str, shares: Uint128) -> ContractResult<Coin> {
    // total shares of debt issued for denom
    let total_debt_shares = TOTAL_DEBT_SHARES.load(deps.storage, denom).unwrap_or(Uint128::zero());

    // total rover debt amount in Redbank for asset
    let red_bank = RED_BANK.load(deps.storage)?;
    let total_debt_amount = red_bank.query_debt(&deps.querier, denom)?;

    // Amount of debt for token's position. Rounded up to favor participants in the debt pool.
    let amount = total_debt_amount.checked_mul_ceil((shares, total_debt_shares))?;

    Ok(Coin {
        denom: denom.to_string(),
        amount,
    })
}

pub trait IntoUint128 {
    fn uint128(&self) -> Uint128;
}

impl IntoUint128 for Decimal {
    fn uint128(&self) -> Uint128 {
        *self * Uint128::new(1)
    }
}

pub fn contents_equal<T>(vec_a: &[T], vec_b: &[T]) -> bool
where
    T: Eq + Hash,
{
    let set_a: HashSet<_> = vec_a.iter().collect();
    let set_b: HashSet<_> = vec_b.iter().collect();
    set_a == set_b
}

/// Queries balance to ensure passing EXACT is not too high.
/// Also asserts the amount is greater than zero.
pub fn get_amount_from_action_coin(
    deps: Deps,
    account_id: &str,
    coin: &ActionCoin,
) -> ContractResult<Uint128> {
    let amount = if let Some(amount) = coin.amount.value() {
        amount
    } else {
        COIN_BALANCES.may_load(deps.storage, (account_id, &coin.denom))?.unwrap_or(Uint128::zero())
    };

    if amount.is_zero() {
        Err(ContractError::NoAmount)
    } else {
        Ok(amount)
    }
}

pub fn assert_allowed_managed_vault_code_ids(
    deps: &mut DepsMut<'_>,
    vault: &Addr,
) -> Result<(), ContractError> {
    let res: ContractInfoResponse =
        deps.querier.query(&QueryRequest::Wasm(cosmwasm_std::WasmQuery::ContractInfo {
            contract_addr: vault.to_string(),
        }))?;
    let code_id = res.code_id;
    let params = PARAMS.load(deps.storage)?;
    let managed_vault_config = params.query_managed_vault_config(&deps.querier)?;
    if !managed_vault_config.code_ids.contains(&code_id) {
        return Err(ContractError::InvalidVaultCodeId {});
    }
    Ok(())
}
