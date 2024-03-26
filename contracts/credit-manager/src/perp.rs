use cosmwasm_std::{coin, Coin, CosmosMsg, DepsMut, Response, Uint128};
use mars_types::{
    oracle::ActionKind,
    perps::{PnL, PnlAmounts},
    signed_uint::SignedUint,
};

use crate::{
    borrow,
    error::ContractResult,
    state::{COIN_BALANCES, PERPS},
    utils::{decrement_coin_balance, increment_coin_balance},
};

pub fn open_perp(
    mut deps: DepsMut,
    account_id: &str,
    denom: &str,
    size: SignedUint,
) -> ContractResult<Response> {
    let perps = PERPS.load(deps.storage)?;

    let opening_fee = perps.query_opening_fee(&deps.querier, denom, size)?;
    let fee = opening_fee.fee;

    let mut response = Response::new();

    let borrow_msg_opt = deduct_payment(&mut deps, account_id, &fee)?;
    if let Some(borrow_msg) = borrow_msg_opt {
        response = response.add_message(borrow_msg);
    }

    let msg = perps.open_msg(account_id, denom, size, vec![fee.clone()])?;

    Ok(response
        .add_message(msg)
        .add_attribute("action", "open_perp_position")
        .add_attribute("account_id", account_id)
        .add_attribute("denom", denom)
        .add_attribute("size", size.to_string())
        .add_attribute("opening_fee", fee.to_string()))
}

/// Deduct payment from the user’s account. If the user doesn’t have enough USDC, it is borrowed
fn deduct_payment(
    deps: &mut DepsMut,
    account_id: &str,
    payment: &Coin,
) -> ContractResult<Option<CosmosMsg>> {
    let coin_balance = COIN_BALANCES
        .may_load(deps.storage, (account_id, &payment.denom))?
        .unwrap_or(Uint128::zero());

    // if the user has enough USDC, it is just taken from the user’s assets
    if coin_balance >= payment.amount {
        decrement_coin_balance(deps.storage, account_id, payment)?;
        return Ok(None);
    }

    let borrow_amt = if coin_balance.is_zero() {
        // if the user doesn’t have USDC, it is all borrowed from the Red Bank
        payment.amount
    } else {
        // if the user has USDC, but not enough, all the available USDC is taken from the account
        // and the remainder is borrowed from the Red Bank
        decrement_coin_balance(
            deps.storage,
            account_id,
            &coin(coin_balance.u128(), &payment.denom),
        )?;

        payment.amount.checked_sub(coin_balance)?
    };

    let (_, borrow_msg) =
        borrow::update_debt(deps, account_id, &coin(borrow_amt.u128(), &payment.denom))?;

    Ok(Some(borrow_msg))
}

pub fn modify_perp(
    mut deps: DepsMut,
    account_id: &str,
    denom: &str,
    new_size: SignedUint,
) -> ContractResult<Response> {
    let perps = PERPS.load(deps.storage)?;

    // query the perp position PnL so that we know whether funds needs to be
    // sent to the perps contract
    //
    // NOTE: This implementation is not gas efficient, because we have to query
    // the position PnL first here in the credit manager (so that it know how
    // much funds to send to the perps contract), then in the perps contract it
    // computes the PnL **again** to assert the amount is correct.
    let position = perps.query_position(&deps.querier, account_id, denom, Some(new_size))?;
    let pnl = position.unrealised_pnl.to_coins(&position.base_denom).pnl;
    let (funds, mut msgs) = update_state_based_on_pnl(&mut deps, account_id, pnl)?;

    msgs.push(perps.modify_msg(account_id, denom, new_size, funds)?);

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "modify_perp_position")
        .add_attribute("account_id", account_id)
        .add_attribute("denom", denom)
        .add_attribute("new_size", new_size.to_string()))
}

pub fn close_perp(mut deps: DepsMut, account_id: &str, denom: &str) -> ContractResult<Response> {
    let perps = PERPS.load(deps.storage)?;

    // query the perp position PnL so that we know whether funds needs to be
    // sent to the perps contract
    //
    // NOTE: This implementation is not gas efficient, because we have to query
    // the position PnL first here in the credit manager (so that it know how
    // much funds to send to the perps contract), then in the perps contract it
    // computes the PnL **again** to assert the amount is correct.
    let position = perps.query_position(&deps.querier, account_id, denom, None)?;
    let pnl = position.unrealised_pnl.to_coins(&position.base_denom).pnl;
    let (funds, mut msgs) = update_state_based_on_pnl(&mut deps, account_id, pnl)?;

    let close_msg = perps.close_msg(account_id, denom, funds)?;
    msgs.push(close_msg);

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "close_perp_position")
        .add_attribute("account_id", account_id)
        .add_attribute("denom", denom))
}

/// Check if liquidatee has any perp positions.
/// If so, close them before liquidating.
pub fn close_all_perps(
    mut deps: DepsMut,
    account_id: &str,
    action: ActionKind,
) -> ContractResult<Response> {
    let perp_positions = PERPS.load(deps.storage)?.query_positions_by_account(
        &deps.querier,
        account_id,
        action.clone(),
    )?;
    if perp_positions.is_empty() {
        return Ok(Response::new()
            .add_attribute("action", "close_all_perps")
            .add_attribute("account_id", account_id)
            .add_attribute("number_of_positions", "0"));
    }

    let mut pnl_amounts_accumulator = PnlAmounts::default();
    for position in &perp_positions {
        pnl_amounts_accumulator.add(&position.unrealised_pnl)?;
    }

    // base denom is the same for all perp positions
    // safe to unwrap because we checked that perp_positions is not empty
    let base_denom = perp_positions.first().unwrap().base_denom.clone();

    let pnl = pnl_amounts_accumulator.to_coins(&base_denom).pnl;
    let (funds, mut msgs) = update_state_based_on_pnl(&mut deps, account_id, pnl)?;

    // Close all perp positions at once
    let perps = PERPS.load(deps.storage)?;
    let close_msg = perps.close_all_msg(account_id, funds, action)?;
    msgs.push(close_msg);

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "close_all_perps")
        .add_attribute("account_id", account_id)
        .add_attribute("number_of_positions", perp_positions.len().to_string()))
}

/// Prepare the necessary messages and funds to be sent to the perps contract based on the PnL.
/// - If PnL is negative, we need to send funds to the perps contract, and
/// decrement the internally tracked user coin balance. If no enough usdc in the user's account,
/// we need to borrow from the Red Bank.
/// - If PnL is positive, we need to increment the internally tracked user coin.
/// - Otherwise, no action is needed.
fn update_state_based_on_pnl(
    deps: &mut DepsMut,
    account_id: &str,
    pnl: PnL,
) -> ContractResult<(Vec<Coin>, Vec<CosmosMsg>)> {
    let res = match pnl {
        PnL::Loss(coin) => {
            let borrow_msg_opt = deduct_payment(deps, account_id, &coin)?;
            let mut cosmos_msgs = vec![];
            if let Some(borrow_msg) = borrow_msg_opt {
                cosmos_msgs.push(borrow_msg);
            }

            (vec![coin], cosmos_msgs)
        }
        PnL::Profit(coin) => {
            increment_coin_balance(deps.storage, account_id, &coin)?;

            (vec![], vec![])
        }
        _ => (vec![], vec![]),
    };
    Ok(res)
}
