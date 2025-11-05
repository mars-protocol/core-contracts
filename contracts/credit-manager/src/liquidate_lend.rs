use cosmwasm_std::{coin, Coin, DepsMut, Env, Response};
use mars_types::health::HealthValuesResponse;

use crate::{
    error::ContractResult,
    liquidate::{calculate_liquidation, increment_rewards_balance},
    liquidate_deposit::repay_debt,
    state::RED_BANK,
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

    let liquidation_res = calculate_liquidation(
        &mut deps,
        env.clone(),
        liquidatee_account_id,
        &debt_coin,
        request_coin_denom,
        total_lent_amount,
        prev_health,
    )?;

    let mut response = Response::new();

    // If the liquidated account has outstanding debt, create a message to repay it.
    // Liquidator pays down debt on behalf of liquidatee.
    if !liquidation_res.debt.amount.is_zero() {
        let repay_msg = repay_debt(
            deps.storage,
            &env,
            liquidator_account_id,
            liquidatee_account_id,
            &liquidation_res.debt,
        )?;
        response = response.add_message(repay_msg);
    }

    // If there is collateral available for liquidation, proceed with transferring assets.
    let protocol_fee_coin = if !liquidation_res.liquidatee_request.amount.is_zero() {
        // Liquidatee's lent coin reclaimed from Red Bank.
        let reclaim_from_liquidatee_msg = red_bank.reclaim_msg(
            &liquidation_res.liquidatee_request,
            liquidatee_account_id,
            true,
        )?;
        response = response.add_message(reclaim_from_liquidatee_msg);

        // Liquidator gets portion of reclaimed lent coin.
        increment_coin_balance(
            deps.storage,
            liquidator_account_id,
            &liquidation_res.liquidator_request,
        )?;

        // Apply the protocol fee to the rewards-collector account.
        increment_rewards_balance(&mut deps, &mut response, &liquidation_res)?
    } else {
        // If no collateral is available, set the protocol fee to zero for this transaction.
        coin(0, request_coin_denom)
    };

    Ok(response
        .add_attribute("action", "liquidate_lend")
        .add_attribute("account_id", liquidator_account_id)
        .add_attribute("liquidatee_account_id", liquidatee_account_id)
        .add_attribute("coin_debt_repaid", liquidation_res.debt.to_string())
        .add_attribute("coin_liquidated", liquidation_res.liquidatee_request.to_string())
        .add_attribute("protocol_fee_coin", protocol_fee_coin.to_string())
        .add_attribute("debt_price", liquidation_res.debt_price.to_string())
        .add_attribute("collateral_price", liquidation_res.collateral_price.to_string()))
}
