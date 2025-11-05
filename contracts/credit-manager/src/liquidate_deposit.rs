use cosmwasm_std::{coin, Coin, CosmosMsg, DepsMut, Env, Response, Storage};
use mars_types::{credit_manager::CallbackMsg, health::HealthValuesResponse};

use crate::{
    error::ContractResult,
    liquidate::{calculate_liquidation, increment_rewards_balance},
    state::COIN_BALANCES,
    utils::{decrement_coin_balance, increment_coin_balance},
};

pub fn liquidate_deposit(
    mut deps: DepsMut,
    env: Env,
    liquidator_account_id: &str,
    liquidatee_account_id: &str,
    debt_coin: Coin,
    request_coin_denom: &str,
    prev_health: HealthValuesResponse,
) -> ContractResult<Response> {
    let request_coin_balance = COIN_BALANCES
        .may_load(deps.storage, (liquidatee_account_id, request_coin_denom))?
        .unwrap_or_default();

    let liquidation_res = calculate_liquidation(
        &mut deps,
        env.clone(),
        liquidatee_account_id,
        &debt_coin,
        request_coin_denom,
        request_coin_balance,
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
        // Transfer the requested collateral from the liquidatee to the liquidator.
        decrement_coin_balance(
            deps.storage,
            liquidatee_account_id,
            &liquidation_res.liquidatee_request,
        )?;
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
        .add_attribute("action", "liquidate_deposit")
        .add_attribute("account_id", liquidator_account_id)
        .add_attribute("liquidatee_account_id", liquidatee_account_id)
        .add_attribute("coin_debt_repaid", liquidation_res.debt.to_string())
        .add_attribute("coin_liquidated", liquidation_res.liquidatee_request.to_string())
        .add_attribute("protocol_fee_coin", protocol_fee_coin.to_string())
        .add_attribute("debt_price", liquidation_res.debt_price.to_string())
        .add_attribute("collateral_price", liquidation_res.collateral_price.to_string()))
}

pub fn repay_debt(
    storage: &mut dyn Storage,
    env: &Env,
    liquidator_account_id: &str,
    liquidatee_account_id: &str,
    debt: &Coin,
) -> ContractResult<CosmosMsg> {
    // Transfer debt coin from liquidator's coin balance to liquidatee
    // Will be used to pay off the debt via CallbackMsg::Repay {}
    decrement_coin_balance(storage, liquidator_account_id, debt)?;
    increment_coin_balance(storage, liquidatee_account_id, debt)?;
    let msg = (CallbackMsg::Repay {
        account_id: liquidatee_account_id.to_string(),
        coin: debt.into(),
    })
    .into_cosmos_msg(&env.contract.address)?;
    Ok(msg)
}
