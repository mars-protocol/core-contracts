use cosmwasm_std::{Coin, CosmosMsg, DepsMut, Response, Uint128};

use crate::{
    error::{ContractError, ContractResult},
    state::{DEBT_SHARES, RED_BANK, TOTAL_DEBT_SHARES},
    utils::{assert_coin_is_whitelisted, increment_coin_balance},
};

pub static DEFAULT_DEBT_SHARES_PER_COIN_BORROWED: Uint128 = Uint128::new(1_000_000);

/// calculate by how many the user's debt units should be increased
/// if total debt is zero, then we define 1 unit of coin borrowed = 1,000,000 debt unit
/// else, get debt ownership % and multiply by total existing shares
///
/// increment total debt shares, token debt shares, and asset amount
pub fn borrow(mut deps: DepsMut, account_id: &str, coin: Coin) -> ContractResult<Response> {
    if coin.amount.is_zero() {
        return Err(ContractError::NoAmount);
    }

    let (debt_shares_to_add, borrow_msg) = update_debt(deps.branch(), account_id, &coin)?;

    increment_coin_balance(deps.storage, account_id, &coin)?;

    Ok(Response::new()
        .add_message(borrow_msg)
        .add_attribute("action", "borrow")
        .add_attribute("account_id", account_id)
        .add_attribute("debt_shares_added", debt_shares_to_add)
        .add_attribute("coin_borrowed", coin.to_string()))
}

/// Update the debt state and prepare a borrow message
pub fn update_debt(
    mut deps: DepsMut,
    account_id: &str,
    coin: &Coin,
) -> ContractResult<(Uint128, CosmosMsg)> {
    assert_coin_is_whitelisted(&mut deps, &coin.denom)?;

    let red_bank = RED_BANK.load(deps.storage)?;
    let total_debt_amount = red_bank.query_debt(&deps.querier, &coin.denom)?;

    let debt_shares_to_add = if total_debt_amount.is_zero() {
        coin.amount.checked_mul(DEFAULT_DEBT_SHARES_PER_COIN_BORROWED)?
    } else {
        TOTAL_DEBT_SHARES
            .load(deps.storage, &coin.denom)?
            .checked_multiply_ratio(coin.amount, total_debt_amount)?
    };

    // It shouldn't happen but just in case
    if debt_shares_to_add.is_zero() {
        return Err(ContractError::ZeroDebtShares);
    }

    TOTAL_DEBT_SHARES.update(deps.storage, &coin.denom, |shares| {
        shares
            .unwrap_or_else(Uint128::zero)
            .checked_add(debt_shares_to_add)
            .map_err(ContractError::Overflow)
    })?;

    DEBT_SHARES.update(deps.storage, (account_id, &coin.denom), |shares| {
        shares
            .unwrap_or_else(Uint128::zero)
            .checked_add(debt_shares_to_add)
            .map_err(ContractError::Overflow)
    })?;

    Ok((debt_shares_to_add, red_bank.borrow_msg(coin)?))
}
