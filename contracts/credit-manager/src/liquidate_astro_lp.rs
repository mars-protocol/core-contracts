use cosmwasm_std::{coin, Coin, DepsMut, Env, Response};
use mars_types::health::HealthValuesResponse;

use crate::{
    error::ContractResult,
    liquidate::{calculate_liquidation, increment_rewards_balance},
    liquidate_deposit::repay_debt,
    state::INCENTIVES,
    utils::increment_coin_balance,
};

pub fn liquidate_astro_lp(
    mut deps: DepsMut,
    env: Env,
    liquidator_account_id: &str,
    liquidatee_account_id: &str,
    debt_coin: Coin,
    request_coin_denom: &str,
    prev_health: HealthValuesResponse,
) -> ContractResult<Response> {
    let incentives = INCENTIVES.load(deps.storage)?;

    // Check how much LP coins is available for withdraw (can be withdrawn from Astro)
    let lp_position = incentives.query_staked_astro_lp_position(
        &deps.querier,
        liquidatee_account_id,
        request_coin_denom,
    )?;
    let total_lp_amount = lp_position.lp_coin.amount;

    let liquidation_res = calculate_liquidation(
        &mut deps,
        env.clone(),
        liquidatee_account_id,
        &debt_coin,
        request_coin_denom,
        total_lp_amount,
        prev_health,
    )?;

    // Rewards are not accounted for in the liquidation calculation (health computer includes
    // only staked astro lps in HF calculation).
    // Rewards could increase the HF (they increase deposit balance - collateral), but the impact
    // is minimal and additional complexity is not worth it.
    // We only update liquidatee's balance with rewards.
    for reward in lp_position.rewards.iter() {
        increment_coin_balance(deps.storage, liquidatee_account_id, reward)?;
    }

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
        // Liquidatee's LP coin withdrawn from Astro.
        let withdraw_from_liquidatee_msg = incentives
            .unstake_astro_lp_msg(liquidatee_account_id, &liquidation_res.liquidatee_request)?;
        response = response.add_message(withdraw_from_liquidatee_msg);

        // Liquidator gets portion of withdrawn LP coin.
        increment_coin_balance(
            deps.storage,
            liquidator_account_id,
            &liquidation_res.liquidator_request,
        )?;

        // Apply the protocol fee to the rewards-collector account.
        increment_rewards_balance(&mut deps, &liquidation_res)?
    } else {
        // If no collateral is available, set the protocol fee to zero for this transaction.
        coin(0, request_coin_denom)
    };

    Ok(response
        .add_attribute("action", "liquidate_astro_lp")
        .add_attribute("account_id", liquidator_account_id)
        .add_attribute("liquidatee_account_id", liquidatee_account_id)
        .add_attribute("coin_debt_repaid", liquidation_res.debt.to_string())
        .add_attribute("coin_liquidated", liquidation_res.liquidatee_request.to_string())
        .add_attribute("protocol_fee_coin", protocol_fee_coin.to_string())
        .add_attribute("debt_price", liquidation_res.debt_price.to_string())
        .add_attribute("collateral_price", liquidation_res.collateral_price.to_string()))
}
