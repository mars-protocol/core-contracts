use cosmwasm_std::{Coin, Deps, DepsMut, Env, Response, Uint128};
use mars_types::credit_manager::{ActionCoin, ChangeExpected};

use crate::{
    error::{ContractError, ContractResult},
    state::PERPS,
    utils::{
        assert_coin_is_whitelisted, decrement_coin_balance, get_amount_from_action_coin,
        update_balance_msg,
    },
};

pub fn deposit_to_perp_vault(
    mut deps: DepsMut,
    account_id: &str,
    coin: &ActionCoin,
    max_receivable_shares: Option<Uint128>,
) -> ContractResult<Response> {
    assert_coin_is_whitelisted(&mut deps, &coin.denom)?;

    let amount_to_deposit = Coin {
        denom: coin.denom.to_string(),
        amount: get_amount_from_action_coin(deps.as_ref(), account_id, coin)?,
    };

    decrement_coin_balance(deps.storage, account_id, &amount_to_deposit)?;

    let perps = PERPS.load(deps.storage)?;
    let msg = perps.deposit_msg(account_id, &amount_to_deposit, max_receivable_shares)?;

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "deposit_to_perp_vault")
        .add_attribute("account_id", account_id)
        .add_attribute("coin_deposited", amount_to_deposit.to_string()))
}

pub fn unlock_from_perp_vault(
    deps: Deps,
    account_id: &str,
    shares: Uint128,
) -> ContractResult<Response> {
    if shares.is_zero() {
        return Err(ContractError::NoAmount);
    }

    let perps = PERPS.load(deps.storage)?;
    let msg = perps.unlock_msg(account_id, shares)?;

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "unlock_from_perp_vault")
        .add_attribute("account_id", account_id)
        .add_attribute("shares_unlocked", shares.to_string()))
}

pub fn withdraw_from_perp_vault(
    deps: Deps,
    env: Env,
    account_id: &str,
    min_receive: Option<Uint128>,
) -> ContractResult<Response> {
    let perps = PERPS.load(deps.storage)?;
    let perp_config = perps.query_config(&deps.querier)?;

    let withdraw_from_perp_vault_msg = perps.withdraw_msg(account_id, min_receive)?;

    // Updates coin balances for account after the withdraw has taken place
    let update_coin_balance_msg = update_balance_msg(
        &deps.querier,
        &env.contract.address,
        account_id,
        &perp_config.base_denom,
        ChangeExpected::Increase,
    )?;

    Ok(Response::new()
        .add_message(withdraw_from_perp_vault_msg)
        .add_message(update_coin_balance_msg)
        .add_attribute("action", "withdraw_from_perp_vault")
        .add_attribute("account_id", account_id))
}
