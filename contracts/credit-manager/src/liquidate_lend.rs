use cosmwasm_std::{Coin, DepsMut, Env, Response};
use mars_types::health::HealthValuesResponse;

use crate::{
    error::{ContractError::NoneLent, ContractResult},
    liquidate::calculate_liquidation,
    liquidate_deposit::repay_debt,
    state::{RED_BANK, REWARDS_COLLECTOR},
    utils::increment_coin_balance,
};

pub fn liquidate_lend(
    mut deps: DepsMut,
    env: Env,
    liquidator_account_id: &str,
    liquidatee_account_id: &str,
    debt_coin: Coin,
    request_coin_denom: &str,
    prev_health: HealthValuesResponse,
) -> ContractResult<Response> {
    let red_bank = RED_BANK.load(deps.storage)?;

    // Check how much lent coin is available for reclaim (can be withdrawn from Red Bank)
    let total_lent_amount =
        red_bank.query_lent(&deps.querier, liquidatee_account_id, request_coin_denom)?;

    if total_lent_amount.is_zero() {
        return Err(NoneLent);
    }

    let liquidation_res = calculate_liquidation(
        &mut deps,
        env.clone(),
        liquidatee_account_id,
        &debt_coin,
        request_coin_denom,
        total_lent_amount,
        prev_health,
    )?;

    // Liquidator pays down debt on behalf of liquidatee
    let repay_msg = repay_debt(
        deps.storage,
        &env,
        liquidator_account_id,
        liquidatee_account_id,
        &liquidation_res.debt,
    )?;

    // Liquidatee's lent coin reclaimed from Red Bank
    let reclaim_from_liquidatee_msg =
        red_bank.reclaim_msg(&liquidation_res.liquidatee_request, liquidatee_account_id, true)?;

    // Liquidator gets portion of reclaimed lent coin
    increment_coin_balance(
        deps.storage,
        liquidator_account_id,
        &liquidation_res.liquidator_request,
    )?;

    // Transfer protocol fee to rewards-collector account
    let rewards_collector_account = REWARDS_COLLECTOR.load(deps.storage)?.account_id;
    let protocol_fee_coin = Coin {
        denom: request_coin_denom.to_string(),
        amount: liquidation_res
            .liquidatee_request
            .amount
            .checked_sub(liquidation_res.liquidator_request.amount)?,
    };
    increment_coin_balance(deps.storage, &rewards_collector_account, &protocol_fee_coin)?;

    Ok(Response::new()
        .add_message(repay_msg)
        .add_message(reclaim_from_liquidatee_msg)
        .add_attribute("action", "liquidate_lend")
        .add_attribute("account_id", liquidator_account_id)
        .add_attribute("liquidatee_account_id", liquidatee_account_id)
        .add_attribute("coin_debt_repaid", liquidation_res.debt.to_string())
        .add_attribute("coin_liquidated", liquidation_res.liquidatee_request.to_string())
        .add_attribute("protocol_fee_coin", protocol_fee_coin.to_string())
        .add_attribute("debt_price", liquidation_res.debt_price.to_string())
        .add_attribute("collateral_price", liquidation_res.collateral_price.to_string()))
}
