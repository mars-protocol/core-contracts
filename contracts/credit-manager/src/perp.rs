use cosmwasm_std::{DepsMut, Response};
use mars_types::{math::SignedDecimal, perps::PnL};

use crate::{error::ContractResult, state::PERPS, utils::decrement_coin_balance};

pub fn open_perp(
    deps: DepsMut,
    account_id: &str,
    denom: &str,
    size: SignedDecimal,
) -> ContractResult<Response> {
    let perps = PERPS.load(deps.storage)?;
    let msg = perps.open_msg(account_id, denom, size)?;

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "open_perp_position")
        .add_attribute("account_id", account_id)
        .add_attribute("denom", denom)
        .add_attribute("size", size.to_string()))
}

pub fn close_perp(deps: DepsMut, account_id: &str, denom: &str) -> ContractResult<Response> {
    let perps = PERPS.load(deps.storage)?;

    // query the perp position PnL so that we know whether funds needs to be
    // sent to the perps contract
    //
    // NOTE: This implementation is not gas efficient, because we have to query
    // the position PnL first here in the credit manager (so that it know how
    // much funds to send to the perps contract), then in the perps contract it
    // computes the PnL **again** to assert the amount is correct. A better
    // solution is the frontend provides the funds amount. Need to communicate
    // this with the FE team.
    let position = perps.query_position(&deps.querier, account_id, denom)?;

    // if PnL is negative, we need to send funds to the perps contract, and
    // decrement the internally tracked user coin balance.
    // otherwise, no action
    let funds = match position.pnl {
        PnL::Loss(coin) => {
            decrement_coin_balance(deps.storage, account_id, &coin)?;
            vec![coin]
        }
        _ => vec![],
    };

    let msg = perps.close_msg(account_id, denom, funds)?;

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "close_perp_position")
        .add_attribute("account_id", account_id)
        .add_attribute("denom", denom))
}
