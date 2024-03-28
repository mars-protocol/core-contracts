use std::cmp::max;

use cosmwasm_std::{coins, BankMsg, Deps, DepsMut, MessageInfo, Response, StdError, Uint128};
use cw_utils::must_pay;
use mars_types::{
    adapters::oracle::Oracle,
    oracle::ActionKind,
    perps::{UnlockState, VaultState},
    signed_uint::SignedUint,
};

use crate::{
    denom::compute_total_accounting_data,
    error::{ContractError, ContractResult},
    state::{decrease_deposit_shares, increase_deposit_shares, CONFIG, UNLOCKS, VAULT_STATE},
    utils::create_user_id_key,
};

pub const DEFAULT_SHARES_PER_AMOUNT: u128 = 1_000_000;

pub fn deposit(
    deps: DepsMut,
    info: MessageInfo,
    current_time: u64,
    account_id: Option<String>,
) -> ContractResult<Response> {
    let cfg = CONFIG.load(deps.storage)?;

    // Don't allow users to create alternative account ids.
    // Only allow credit manager contract to create them.
    // Even if account_id contains empty string we won't allow it.
    if account_id.is_some() && info.sender != cfg.credit_manager {
        return Err(ContractError::SenderIsNotCreditManager);
    }

    let user_id_key = create_user_id_key(&info.sender, account_id)?;

    let mut vs = VAULT_STATE.load(deps.storage)?;

    // find the deposit amount
    let amount = must_pay(&info, &cfg.base_denom)?;

    // compute the new shares to be minted to the depositor
    let shares = amount_to_shares(
        &deps.as_ref(),
        &vs,
        &cfg.oracle,
        current_time,
        &cfg.base_denom,
        amount,
        ActionKind::Default,
    )?;

    // increment total liquidity and deposit shares
    vs.total_liquidity = vs.total_liquidity.checked_add(amount)?;
    vs.total_shares = vs.total_shares.checked_add(shares)?;
    VAULT_STATE.save(deps.storage, &vs)?;

    // increment the user's deposit shares
    increase_deposit_shares(deps.storage, &user_id_key, shares)?;

    Ok(Response::new()
        .add_attribute("method", "deposit")
        .add_attribute("amount", amount)
        .add_attribute("shares", shares))
}

pub fn unlock(
    deps: DepsMut,
    info: MessageInfo,
    current_time: u64,
    account_id: Option<String>,
    shares: Uint128,
) -> ContractResult<Response> {
    let cfg = CONFIG.load(deps.storage)?;

    // Don't allow users to create alternative account ids.
    // Only allow credit manager contract to create them.
    // Even if account_id contains empty string we won't allow it.
    if account_id.is_some() && info.sender != cfg.credit_manager {
        return Err(ContractError::SenderIsNotCreditManager);
    }

    let user_id_key = create_user_id_key(&info.sender, account_id)?;

    // cannot unlock zero shares
    if shares.is_zero() {
        return Err(ContractError::ZeroShares);
    }

    // decrement the user's deposit shares
    decrease_deposit_shares(deps.storage, &user_id_key, shares)?;

    // add new unlock position
    let cooldown_end = current_time + cfg.cooldown_period;
    UNLOCKS.update(deps.storage, &user_id_key, |maybe_unlocks| {
        let mut unlocks = maybe_unlocks.unwrap_or_default();

        unlocks.push(UnlockState {
            created_at: current_time,
            cooldown_end,
            shares,
        });

        Ok::<Vec<UnlockState>, StdError>(unlocks)
    })?;

    Ok(Response::new()
        .add_attribute("method", "unlock")
        .add_attribute("shares", shares)
        .add_attribute("created_at", current_time.to_string())
        .add_attribute("cooldown_end", cooldown_end.to_string()))
}

pub fn withdraw(
    deps: DepsMut,
    info: MessageInfo,
    current_time: u64,
    account_id: Option<String>,
) -> ContractResult<Response> {
    let cfg = CONFIG.load(deps.storage)?;

    // Don't allow users to create alternative account ids.
    // Only allow credit manager contract to create them.
    // Even if account_id contains empty string we won't allow it.
    if account_id.is_some() && info.sender != cfg.credit_manager {
        return Err(ContractError::SenderIsNotCreditManager);
    }

    let user_id_key = create_user_id_key(&info.sender, account_id)?;

    let unlocks = UNLOCKS.load(deps.storage, &user_id_key)?;

    // find all unlocked positions
    let (unlocked, unlocking): (Vec<_>, Vec<_>) =
        unlocks.into_iter().partition(|us| us.cooldown_end <= current_time);

    // cannot withdraw when there is zero unlocked positions
    if unlocked.is_empty() {
        return Err(ContractError::UnlockedPositionsNotFound {});
    }

    // clear state if no more unlocking positions
    if unlocking.is_empty() {
        UNLOCKS.remove(deps.storage, &user_id_key);
    } else {
        UNLOCKS.save(deps.storage, &user_id_key, &unlocking)?;
    }

    let mut vs = VAULT_STATE.load(deps.storage)?;

    // compute the total shares to be withdrawn
    let total_unlocked_shares = unlocked.into_iter().map(|us| us.shares).sum::<Uint128>();

    // convert the shares to amount
    let total_unlocked_amount = shares_to_amount(
        &deps.as_ref(),
        &vs,
        &cfg.oracle,
        current_time,
        &cfg.base_denom,
        total_unlocked_shares,
        ActionKind::Default,
    )?;

    // decrement total liquidity and deposit shares
    vs.total_liquidity = vs.total_liquidity.checked_sub(total_unlocked_amount)?;
    vs.total_shares = vs.total_shares.checked_sub(total_unlocked_shares)?;
    VAULT_STATE.save(deps.storage, &vs)?;

    Ok(Response::new()
        .add_attribute("method", "withdraw")
        .add_attribute("shares", total_unlocked_shares)
        .add_attribute("amount", total_unlocked_amount)
        .add_message(BankMsg::Send {
            to_address: info.sender.into(),
            amount: coins(total_unlocked_amount.u128(), cfg.base_denom),
        }))
}

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
    let global_withdrawal_balance = max(global_withdrawal_balance, SignedUint::zero());

    Ok(global_withdrawal_balance.abs)
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
