use std::cmp::max;

use cosmwasm_std::{
    coins, ensure_eq, BankMsg, Deps, DepsMut, MessageInfo, Response, StdError, Storage, Uint128,
};
use cw_utils::must_pay;
use mars_types::{
    adapters::oracle::Oracle,
    math::SignedDecimal,
    oracle::ActionKind,
    perps::{UnlockState, VaultState},
};

use crate::{
    denom::compute_total_accounting_data,
    error::{ContractError, ContractResult},
    state::{decrease_deposit_shares, increase_deposit_shares, CONFIG, UNLOCKS, VAULT_STATE},
};

const DEFAULT_SHARES_PER_AMOUNT: u128 = 1_000_000;

pub fn deposit(
    deps: DepsMut,
    info: MessageInfo,
    current_time: u64,
    account_id: &str,
) -> ContractResult<Response> {
    let cfg = CONFIG.load(deps.storage)?;

    // only the credit manager contract can open positions
    ensure_eq!(info.sender, cfg.credit_manager, ContractError::SenderIsNotCreditManager);

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
    increase_deposit_shares(deps.storage, account_id, shares)?;

    Ok(Response::new()
        .add_attribute("method", "deposit")
        .add_attribute("amount", amount)
        .add_attribute("shares", shares))
}

pub fn unlock(
    deps: DepsMut,
    info: MessageInfo,
    current_time: u64,
    account_id: &str,
    shares: Uint128,
) -> ContractResult<Response> {
    let cfg = CONFIG.load(deps.storage)?;

    // only the credit manager contract can open positions
    ensure_eq!(info.sender, cfg.credit_manager, ContractError::SenderIsNotCreditManager);

    let mut vs = VAULT_STATE.load(deps.storage)?;

    // convert the shares to amount
    let amount = shares_to_amount(
        &deps.as_ref(),
        &vs,
        &cfg.oracle,
        current_time,
        &cfg.base_denom,
        shares,
        ActionKind::Default,
    )?;

    // cannot unlock when there is zero shares
    if amount.is_zero() {
        return Err(ContractError::ZeroShares);
    }

    // decrement total liquidity and deposit shares
    vs.total_liquidity = vs.total_liquidity.checked_sub(amount)?;
    vs.total_shares = vs.total_shares.checked_sub(shares)?;
    VAULT_STATE.save(deps.storage, &vs)?;

    // decrement the user's deposit shares
    decrease_deposit_shares(deps.storage, account_id, shares)?;

    // add new unlock position
    let cooldown_end = current_time + cfg.cooldown_period;
    UNLOCKS.update(deps.storage, account_id, |maybe_unlocks| {
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
    info: MessageInfo,
    current_time: u64,
    account_id: &str,
) -> ContractResult<Response> {
    let cfg = CONFIG.load(store)?;

    // only the credit manager contract can open positions
    ensure_eq!(info.sender, cfg.credit_manager, ContractError::SenderIsNotCreditManager);

    let unlocks = UNLOCKS.load(store, account_id)?;

    // find all unlocked positions
    let (unlocked, unlocking): (Vec<_>, Vec<_>) =
        unlocks.into_iter().partition(|us| us.cooldown_end <= current_time);

    // cannot withdraw when there is zero unlocked positions
    if unlocked.is_empty() {
        return Err(ContractError::UnlockedPositionsNotFound {});
    }

    // clear state if no more unlocking positions
    if unlocking.is_empty() {
        UNLOCKS.remove(store, account_id);
    } else {
        UNLOCKS.save(store, account_id, &unlocking)?;
    }

    // compute the total amount to be withdrawn
    let unlocked_amt = unlocked.into_iter().map(|us| us.amount).sum::<Uint128>();

    Ok(Response::new()
        .add_attribute("method", "withdraw")
        .add_attribute("amount", unlocked_amt)
        .add_message(BankMsg::Send {
            to_address: info.sender.into(),
            amount: coins(unlocked_amt.u128(), cfg.base_denom),
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
