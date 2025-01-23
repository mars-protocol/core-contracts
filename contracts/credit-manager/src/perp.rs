use std::cmp::min;

use cosmwasm_std::{
    coin, ensure_eq, BankMsg, Coin, CosmosMsg, DepsMut, Env, Int128, MessageInfo, Response, Uint128,
};
use mars_types::{
    oracle::ActionKind,
    perps::{PnL, PnlAmounts},
};

use crate::{
    borrow,
    error::{ContractError, ContractResult},
    state::{COIN_BALANCES, PERPS, RED_BANK},
    trigger::remove_invalid_trigger_orders,
    utils::{decrement_coin_balance, increment_coin_balance},
};

/// Deducts a specified payment from the user's account. If the user's balance in the
/// specified denomination (e.g., USDC) is insufficient, the function first attempts to
/// reclaim the shortfall from the Red Bank. If the reclaimed amount is still not enough,
/// it borrows the remaining required amount from the Red Bank.
fn deduct_payment(
    deps: &mut DepsMut,
    account_id: &str,
    payment: &Coin,
    action: Option<ActionKind>,
    mut res: Response,
) -> ContractResult<Response> {
    // Determine if the payment is related to a liquidation event.
    let liquidation_related = action == Some(ActionKind::Liquidation);

    // Retrieve the user's balance for the given denomination (e.g., USDC), or default to zero if not found.
    let user_balance = COIN_BALANCES
        .may_load(deps.storage, (account_id, &payment.denom))?
        .unwrap_or(Uint128::zero());

    // Deduct the amount available in the user's account from their balance.
    let deduct_from_coin_balance = min(user_balance, payment.amount);
    decrement_coin_balance(
        deps.storage,
        account_id,
        &coin(deduct_from_coin_balance.u128(), &payment.denom),
    )?;

    // If the user’s balance covers the entire payment, return the response immediately.
    if user_balance >= payment.amount {
        return Ok(res);
    }

    // Calculate the remaining amount to be paid after deducting the user's balance.
    let mut left_amount_to_pay = payment.amount - user_balance;

    // Attempt to reclaim funds from the Red Bank if the user has lent assets.
    let red_bank = RED_BANK.load(deps.storage)?;
    let lent_amount = red_bank.query_lent(&deps.querier, account_id, &payment.denom)?;

    // If there are lent assets, reclaim as much as possible from the Red Bank.
    if !lent_amount.is_zero() {
        let reclaim_amount = min(left_amount_to_pay, lent_amount);
        let reclaim_msg = red_bank.reclaim_msg(
            &coin(reclaim_amount.u128(), &payment.denom),
            account_id,
            liquidation_related,
        )?;
        res = res.add_message(reclaim_msg);

        // If the reclaimed amount fully covers the remaining payment, return the response.
        if reclaim_amount >= left_amount_to_pay {
            return Ok(res);
        }

        // Update the remaining amount to be paid after reclaiming from the Red Bank.
        left_amount_to_pay -= reclaim_amount;
    }

    // If there is still a shortfall, borrow the remaining amount from the Red Bank.
    let (_, borrow_msg) =
        borrow::update_debt(deps, account_id, &coin(left_amount_to_pay.u128(), &payment.denom))?;

    // Add the borrow message to the response and return.
    Ok(res.add_message(borrow_msg))
}

pub fn execute_perp_order(
    mut deps: DepsMut,
    account_id: &str,
    denom: &str,
    order_size: Int128,
    reduce_only: Option<bool>,
) -> ContractResult<Response> {
    let perps = PERPS.load(deps.storage)?;

    let mut response = Response::new();

    // query the perp position PnL so that we know whether funds needs to be
    // sent to the perps contract
    //
    // NOTE: This implementation is not gas efficient, because we have to query
    // the position PnL first here in the credit manager (so that it know how
    // much funds to send to the perps contract), then in the perps contract it
    // computes the PnL **again** to assert the amount is correct.
    let position =
        perps.query_position(&deps.querier, account_id, denom, Some(order_size), reduce_only)?;

    Ok(match position {
        Some(position) => {
            // Modify existing position
            let pnl = position.unrealized_pnl.to_coins(&position.base_denom).pnl;
            let pnl_string = position.unrealized_pnl.pnl.to_string();
            let (funds, response) =
                update_state_based_on_pnl(&mut deps, account_id, pnl, None, response)?;
            let funds = funds.map_or_else(Vec::new, |c| vec![c]);

            let msg =
                perps.execute_perp_order(account_id, denom, order_size, reduce_only, funds)?;

            let new_size = position.size.checked_add(order_size)?;

            // When size is 0 or positions flips, any active (order is a default or parent, or child order
            // with parent being executed) with reduce_only should be removed.
            if new_size.is_zero() || (new_size.is_negative() != position.size.is_negative()) {
                remove_invalid_trigger_orders(deps.storage, account_id, &position.denom)?;
            }

            response
                .add_message(msg)
                .add_attribute("action", "execute_perp_order")
                .add_attribute("account_id", account_id)
                .add_attribute("denom", denom)
                .add_attribute("realized_pnl", pnl_string)
                .add_attribute("reduce_only", reduce_only.unwrap_or(false).to_string())
                .add_attribute("order_size", order_size.to_string())
                .add_attribute("new_size", new_size.to_string())
        }
        None => {
            // Open new position
            let opening_fee = perps.query_opening_fee(&deps.querier, denom, order_size)?;
            let fee = opening_fee.fee;

            let funds = if !fee.amount.is_zero() {
                response = deduct_payment(&mut deps, account_id, &fee, None, response)?;
                vec![fee.clone()]
            } else {
                vec![]
            };

            let msg =
                perps.execute_perp_order(account_id, denom, order_size, reduce_only, funds)?;

            response
                .add_message(msg)
                .add_attribute("action", "open_perp_position")
                .add_attribute("account_id", account_id)
                .add_attribute("denom", denom)
                .add_attribute("reduce_only", reduce_only.unwrap_or(false).to_string())
                .add_attribute("new_size", order_size.to_string())
                .add_attribute("opening_fee", fee.to_string())
        }
    })
}

/// Check if liquidatee has any perp positions.
/// If so, close them before liquidating.
pub fn close_all_perps(
    mut deps: DepsMut,
    account_id: &str,
    action: ActionKind,
) -> ContractResult<Response> {
    let perps = PERPS.load(deps.storage)?;
    let perp_positions =
        perps.query_positions_by_account(&deps.querier, account_id, action.clone())?;
    if perp_positions.is_empty() {
        return Ok(Response::new()
            .add_attribute("action", "close_all_perps")
            .add_attribute("account_id", account_id)
            .add_attribute("number_of_positions", "0"));
    }

    let mut pnl_amounts_accumulator = PnlAmounts::default();
    for position in &perp_positions {
        pnl_amounts_accumulator.add(&position.unrealized_pnl)?;
    }

    // base denom is the same for all perp positions
    // safe to unwrap because we checked that perp_positions is not empty
    let base_denom = perp_positions.first().unwrap().base_denom.clone();

    let response = Response::new();

    let pnl = pnl_amounts_accumulator.to_coins(&base_denom).pnl;
    let (funds, response) =
        update_state_based_on_pnl(&mut deps, account_id, pnl, Some(action.clone()), response)?;
    let funds = funds.map_or_else(Vec::new, |c| vec![c]);

    // Close all perp positions at once
    let close_msg = perps.close_all_msg(account_id, funds, action)?;

    Ok(response
        .add_message(close_msg)
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
    action: Option<ActionKind>,
    res: Response,
) -> ContractResult<(Option<Coin>, Response)> {
    let res = match pnl {
        PnL::Loss(coin) => {
            let res = deduct_payment(deps, account_id, &coin, action, res)?;
            (Some(coin), res)
        }
        PnL::Profit(coin) => {
            increment_coin_balance(deps.storage, account_id, &coin)?;
            (None, res)
        }
        _ => (None, res),
    };
    Ok(res)
}

pub fn update_balance_after_deleverage(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    account_id: String,
    pnl: PnL,
    action: ActionKind,
) -> ContractResult<Response> {
    let perps = PERPS.load(deps.storage)?;

    // Only the perps contract can update the balances after deleverage
    ensure_eq!(
        &info.sender,
        perps.address(),
        ContractError::Unauthorized {
            user: info.sender.to_string(),
            action: "update balances after deleverage".to_string()
        }
    );

    let response = Response::new();

    let pnl_string = pnl.to_signed_uint()?.to_string();
    let (funds, mut response) =
        update_state_based_on_pnl(&mut deps, &account_id, pnl, Some(action), response)?;

    // Amount sent will be validated in the perps contract in reply entry point
    if let Some(f) = funds {
        if !f.amount.is_zero() {
            let send_msg = CosmosMsg::Bank(BankMsg::Send {
                to_address: perps.address().into(),
                amount: vec![f],
            });
            response = response.add_message(send_msg);
        }
    }

    Ok(response
        .add_attribute("action", "update_balance_after_deleverage")
        .add_attribute("account_id", account_id)
        .add_attribute("realized_pnl", pnl_string))
}
