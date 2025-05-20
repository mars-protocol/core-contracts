use cosmwasm_std::{Coin, Decimal, Deps, DepsMut, Env, Int128, QuerierWrapper, Uint128};
use mars_liquidation::liquidation::{calculate_liquidation_amounts, HealthData};
use mars_types::{
    adapters::oracle::Oracle, health::HealthValuesResponse, oracle::ActionKind, traits::Stringify,
};

use crate::{
    error::{ContractError, ContractResult},
    health::query_health_values,
    repay::current_debt_for_denom,
    state::{ORACLE, PARAMS, PERPS_LB_RATIO, REWARDS_COLLECTOR},
    utils::increment_coin_balance,
};

/// Checks if the liquidatee's credit account is liquidatable.
/// If not, returns an error.
/// If liquidatable, returns the health values.
pub fn check_health(
    deps: Deps,
    env: Env,
    liquidatee_account_id: &str,
) -> ContractResult<HealthValuesResponse> {
    // Assert the liquidatee's credit account is liquidatable
    let health = query_health_values(deps, env, liquidatee_account_id, ActionKind::Liquidation)?;
    if !health.liquidatable {
        return Err(ContractError::NotLiquidatable {
            account_id: liquidatee_account_id.to_string(),
            lqdt_health_factor: health.liquidation_health_factor.to_string(),
        });
    }

    Ok(health)
}

/// Result of a liquidation calculation.
pub struct LiquidationResult {
    pub debt: Coin,
    pub liquidator_request: Coin,
    pub liquidatee_request: Coin,
    pub debt_price: Decimal,
    pub collateral_price: Decimal,
}

impl LiquidationResult {
    pub fn with_zero_amounts(
        debt_denom: &str,
        collateral_denom: &str,
        debt_price: Decimal,
        collateral_price: Decimal,
    ) -> LiquidationResult {
        LiquidationResult {
            debt: Coin {
                denom: debt_denom.to_string(),
                amount: Uint128::zero(),
            },
            liquidator_request: Coin {
                denom: collateral_denom.to_string(),
                amount: Uint128::zero(),
            },
            liquidatee_request: Coin {
                denom: collateral_denom.to_string(),
                amount: Uint128::zero(),
            },
            debt_price,
            collateral_price,
        }
    }
}

/// Calculates precise debt, request coin amounts to liquidate, request coin transfered to liquidator and rewards-collector.
/// The debt amount will be adjusted down if:
/// - Exceeds liquidatee's total debt for denom
/// - Not enough liquidatee request coin balance to match
/// - The value of the debt repaid exceeds the Maximum Debt Repayable (MDR)
///
/// Returns -> (Debt Coin, Liquidator Request Coin, Liquidatee Request Coin)
/// Difference between Liquidator Request Coin and Liquidatee Request Coin goes to rewards-collector account as protocol fee.
pub fn calculate_liquidation(
    deps: &mut DepsMut,
    _env: Env,
    liquidatee_account_id: &str,
    debt_coin: &Coin,
    request_coin: &str,
    request_coin_balance: Uint128,
    prev_health: HealthValuesResponse,
) -> ContractResult<LiquidationResult> {
    let perps_lb_ratio = PERPS_LB_RATIO.load(deps.storage)?;

    // If the account held perps positions before liquidation started, we close those positions first.
    // The health factor may already be above 1 after closing perps, so we retain the previous value — the one that triggered the liquidation.
    let health: HealthData = prev_health.try_into()?;

    // Query the oracle for the prices of the debt and request coins
    let oracle = ORACLE.load(deps.storage)?;
    let debt_coin_price =
        oracle.query_price(&deps.querier, &debt_coin.denom, ActionKind::Liquidation)?.price;
    let request_coin_price =
        oracle.query_price(&deps.querier, request_coin, ActionKind::Liquidation)?.price;

    // If the requested coin balance is zero and the account has a negative net value (indicating bad debt),
    // we return a `LiquidationResult` with zero amounts. This outcome allows liquidation to complete smoothly
    // without transferring any assets, as there is no collateral available to cover the debt.
    if request_coin_balance.is_zero() && health.account_net_value < Int128::zero() {
        return Ok(LiquidationResult::with_zero_amounts(
            &debt_coin.denom,
            request_coin,
            debt_coin_price,
            request_coin_price,
        ));
    }

    // If the requested coin balance is zero and the account has a positive net value (indicating no bad debt),
    // we raise an error, as there should be assets available for liquidation in this case.
    if request_coin_balance.is_zero() {
        return Err(ContractError::CoinNotAvailable(request_coin.to_string()));
    }

    // Ensure debt repaid does not exceed liquidatee's total debt for denom
    let current_debt_res =
        current_debt_for_denom(deps.as_ref(), liquidatee_account_id, &debt_coin.denom);
    let total_debt_amount = match current_debt_res {
        Ok((total_debt_amount, _)) => total_debt_amount,
        // If there is no debt but there is a loss from perps, we can still liquidate
        Err(ContractError::NoDebt) if !health.perps_pnl_loss.is_zero() => Uint128::zero(),
        Err(e) => return Err(e),
    };

    let params = PARAMS.load(deps.storage)?;
    let request_coin_params = params
        .query_asset_params(&deps.querier, request_coin)?
        .ok_or(ContractError::AssetParamsNotFound(request_coin.to_string()))?;
    let debt_coin_params = params
        .query_asset_params(&deps.querier, &debt_coin.denom)?
        .ok_or(ContractError::AssetParamsNotFound(debt_coin.denom.to_string()))?;

    let (debt_amount_to_repay, request_amount_to_liquidate, request_amount_received_by_liquidator) =
        calculate_liquidation_amounts(
            request_coin_balance,
            request_coin_price,
            &request_coin_params,
            total_debt_amount,
            debt_coin.amount,
            debt_coin_price,
            &debt_coin_params,
            &health,
            perps_lb_ratio,
        )?;

    let result = LiquidationResult {
        debt: Coin {
            denom: debt_coin.denom.clone(),
            amount: debt_amount_to_repay,
        },
        liquidator_request: Coin {
            denom: request_coin.to_string(),
            amount: request_amount_received_by_liquidator,
        },
        liquidatee_request: Coin {
            denom: request_coin.to_string(),
            amount: request_amount_to_liquidate,
        },
        debt_price: debt_coin_price,
        collateral_price: request_coin_price,
    };

    assert_liquidation_profitable(&deps.querier, &oracle, &result)?;

    Ok(result)
}

/// In scenarios with small amounts or large gap between coin prices, there is a possibility
/// that the liquidation will result in loss for the liquidator. This assertion prevents this.
fn assert_liquidation_profitable(
    querier: &QuerierWrapper,
    oracle: &Oracle,
    liq_res: &LiquidationResult,
) -> ContractResult<()> {
    let debt_value = oracle.query_value(querier, &liq_res.debt, ActionKind::Liquidation)?;
    let request_value =
        oracle.query_value(querier, &liq_res.liquidator_request, ActionKind::Liquidation)?;

    if debt_value >= request_value {
        return Err(ContractError::LiquidationNotProfitable {
            debt_coin: liq_res.debt.clone(),
            request_coin: liq_res.liquidator_request.clone(),
        });
    }

    Ok(())
}

/// Guards against the case an account is trying to liquidate itself
pub fn assert_not_self_liquidation(
    liquidator_account_id: &str,
    liquidatee_account_id: &str,
) -> ContractResult<()> {
    if liquidator_account_id == liquidatee_account_id {
        return Err(ContractError::SelfLiquidation);
    }
    Ok(())
}

/// Transfer protocol fee to rewards-collector account
pub fn increment_rewards_balance(
    deps: &mut DepsMut,
    liq_req: &LiquidationResult,
) -> ContractResult<Coin> {
    let rewards_collector_account = REWARDS_COLLECTOR.load(deps.storage)?.account_id;
    let amount =
        liq_req.liquidatee_request.amount.checked_sub(liq_req.liquidator_request.amount)?;
    let protocol_fee_coin = Coin {
        denom: liq_req.liquidatee_request.denom.clone(),
        amount,
    };
    if !amount.is_zero() {
        increment_coin_balance(deps.storage, &rewards_collector_account, &protocol_fee_coin)?;
    }
    Ok(protocol_fee_coin)
}
