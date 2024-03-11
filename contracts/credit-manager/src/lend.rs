use cosmwasm_std::{Coin, DepsMut, Response};
use mars_types::credit_manager::ActionCoin;

use crate::{
    error::ContractResult,
    state::RED_BANK,
    utils::{assert_coin_is_whitelisted, decrement_coin_balance, get_amount_from_action_coin},
};

pub fn lend(mut deps: DepsMut, account_id: &str, coin: &ActionCoin) -> ContractResult<Response> {
    assert_coin_is_whitelisted(&mut deps, &coin.denom)?;

    let amount_to_lend = Coin {
        denom: coin.denom.to_string(),
        amount: get_amount_from_action_coin(deps.as_ref(), account_id, coin)?,
    };

    decrement_coin_balance(deps.storage, account_id, &amount_to_lend)?;

    let red_bank_lend_msg = RED_BANK.load(deps.storage)?.lend_msg(&amount_to_lend, account_id)?;

    Ok(Response::new()
        .add_message(red_bank_lend_msg)
        .add_attribute("action", "lend")
        .add_attribute("account_id", account_id)
        .add_attribute("coin_lent", &amount_to_lend.denom))
}
