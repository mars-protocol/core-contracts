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
    state::{COIN_BALANCES, PERPS},
    utils::{decrement_coin_balance, increment_coin_balance},
};

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

pub fn execute_perp_order(
    mut deps: DepsMut,
    account_id: &str,
    denom: &str,
    order_size: Int128,
    reduce_only: Option<bool>,
) -> ContractResult<Response> {
    let perps = PERPS.load(deps.storage)?;

    // query the perp position PnL so that we know whether funds needs to be
    // sent to the perps contract
    //
    // NOTE: This implementation is not gas efficient, because we have to query
    // the position PnL first here in the credit manager (so that it know how
    // much funds to send to the perps contract), then in the perps contract it
    // computes the PnL **again** to assert the amount is correct.
    let position = perps.query_position(&deps.querier, account_id, denom, Some(order_size))?;
    Ok(match position {
        Some(position) => {
            // Modify existing position
            let pnl = position.unrealised_pnl.to_coins(&position.base_denom).pnl;
            let pnl_string = position.unrealised_pnl.pnl.to_string();
            let (funds, mut msgs) = update_state_based_on_pnl(&mut deps, account_id, pnl)?;
            let funds = funds.map_or_else(Vec::new, |c| vec![c]);

            msgs.push(perps.execute_perp_order(
                account_id,
                denom,
                order_size,
                reduce_only,
                funds,
            )?);

            Response::new()
                .add_messages(msgs)
                .add_attribute("action", "execute_perp_order")
                .add_attribute("account_id", account_id)
                .add_attribute("denom", denom)
                .add_attribute("realised_pnl", pnl_string)
                .add_attribute("reduce_only", reduce_only.unwrap_or(false).to_string())
                .add_attribute("order_size", order_size.to_string())
                .add_attribute("new_size", position.size.checked_add(order_size)?.to_string())
        }
        None => {
            // Open new position
            let opening_fee = perps.query_opening_fee(&deps.querier, denom, order_size)?;
            let fee = opening_fee.fee;

            let mut response = Response::new();

            let funds = if !fee.amount.is_zero() {
                let borrow_msg_opt = deduct_payment(&mut deps, account_id, &fee)?;
                if let Some(borrow_msg) = borrow_msg_opt {
                    response = response.add_message(borrow_msg);
                }
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
        pnl_amounts_accumulator.add(&position.unrealised_pnl)?;
    }

    // base denom is the same for all perp positions
    // safe to unwrap because we checked that perp_positions is not empty
    let base_denom = perp_positions.first().unwrap().base_denom.clone();

    let pnl = pnl_amounts_accumulator.to_coins(&base_denom).pnl;
    let (funds, mut msgs) = update_state_based_on_pnl(&mut deps, account_id, pnl)?;
    let funds = funds.map_or_else(Vec::new, |c| vec![c]);

    // Close all perp positions at once
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
) -> ContractResult<(Option<Coin>, Vec<CosmosMsg>)> {
    let res = match pnl {
        PnL::Loss(coin) => {
            let borrow_msg_opt = deduct_payment(deps, account_id, &coin)?;
            let mut cosmos_msgs = vec![];
            if let Some(borrow_msg) = borrow_msg_opt {
                cosmos_msgs.push(borrow_msg);
            }

            (Some(coin), cosmos_msgs)
        }
        PnL::Profit(coin) => {
            increment_coin_balance(deps.storage, account_id, &coin)?;

            (None, vec![])
        }
        _ => (None, vec![]),
    };
    Ok(res)
}

pub fn update_balance_after_deleverage(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    account_id: String,
    pnl: PnL,
    _action: ActionKind,
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

    let pnl_string = pnl.to_signed_uint()?.to_string();
    let (funds, mut msgs) = update_state_based_on_pnl(&mut deps, &account_id, pnl)?;

    // Amount sent will be validated in the perps contract in reply entry point
    if let Some(f) = funds {
        if !f.amount.is_zero() {
            let send_msg = CosmosMsg::Bank(BankMsg::Send {
                to_address: perps.address().into(),
                amount: vec![f],
            });
            msgs.push(send_msg);
        }
    }

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "update_balance_after_deleverage")
        .add_attribute("account_id", account_id)
        .add_attribute("realised_pnl", pnl_string))
}
