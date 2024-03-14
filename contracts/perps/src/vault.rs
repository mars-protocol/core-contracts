use std::cmp::max;

use cosmwasm_std::{Deps, Uint128};
use mars_types::{
    adapters::oracle::Oracle, math::SignedDecimal, oracle::ActionKind, perps::VaultState,
};

use crate::{
    denom::compute_total_accounting_data,
    error::{ContractError, ContractResult},
};

const DEFAULT_SHARES_PER_AMOUNT: u128 = 1_000_000;

/// Compute the counterparty vault's net asset value (NAV), denominated in the
/// base asset (i.e. USDC).
///
/// The NAV is defined as
///
/// ```
/// NAV := max(assets + totalWithdrawalBalance, 0)
/// ```
///
/// Here `totalWithdrawalBalance` is the amount of money available for withdrawal by LPs.
///
/// If a traders has an unrealized gain, it's a liability for the counterparty
/// vault, because if the user realizes the position it will be the vault to pay
/// for the profit.
///
/// Conversely, to realize a losing position the user must pay the vault, so
/// it's an asset for the vault.
pub fn compute_global_withdrawal_balance(
    deps: &Deps,
    vs: &VaultState,
    oracle: &Oracle,
    current_time: u64,
    base_denom: &str,
    action: ActionKind,
) -> ContractResult<Uint128> {
    let base_denom_price = oracle.query_price(&deps.querier, base_denom, action.clone())?.price;

    let global_acc_data =
        compute_total_accounting_data(deps, oracle, current_time, base_denom_price, action)?;

    let global_withdrawal_balance =
        global_acc_data.withdrawal_balance.total.checked_add(vs.total_liquidity.into())?;
    let global_withdrawal_balance = max(global_withdrawal_balance, SignedDecimal::zero());

    Ok(global_withdrawal_balance.abs.to_uint_floor())
}

/// Convert a deposit amount to shares, given the current total amount and
/// shares.
///
/// If total shares is zero, in which case a conversion rate between amount and
/// shares is undefined, we use a default conversion rate.
pub fn amount_to_shares(
    deps: &Deps,
    vs: &VaultState,
    oracle: &Oracle,
    current_time: u64,
    base_denom: &str,
    amount: Uint128,
    action: ActionKind,
) -> ContractResult<Uint128> {
    let available_liquidity =
        compute_global_withdrawal_balance(deps, vs, oracle, current_time, base_denom, action)?;

    if vs.total_shares.is_zero() || available_liquidity.is_zero() {
        return amount.checked_mul(Uint128::new(DEFAULT_SHARES_PER_AMOUNT)).map_err(Into::into);
    }

    vs.total_shares.checked_multiply_ratio(amount, available_liquidity).map_err(Into::into)
}

/// Convert a deposit shares to amount, given the current total amount and
/// shares.
///
/// If total shares is zero, in which case a conversion rate between amount and
/// shares if undefined, we throw an error.
pub fn shares_to_amount(
    deps: &Deps,
    vs: &VaultState,
    oracle: &Oracle,
    current_time: u64,
    base_denom: &str,
    shares: Uint128,
    action: ActionKind,
) -> ContractResult<Uint128> {
    // We technical don't need to check for this explicitly, because
    // checked_multiply_raio already checks for division-by-zero. However we
    // still do this to output a more descriptive error message. This consumes a
    // bit more gas but gas fee is not yet a problem on Cosmos chains anyways.
    if vs.total_shares.is_zero() {
        return Err(ContractError::ZeroTotalShares);
    }

    // We can't continue if there is zero available liquidity in the vault
    let available_liquidity =
        compute_global_withdrawal_balance(deps, vs, oracle, current_time, base_denom, action)?;
    if available_liquidity.is_zero() {
        return Err(ContractError::ZeroWithdrawalBalance);
    }

    available_liquidity.checked_multiply_ratio(shares, vs.total_shares).map_err(Into::into)
}
