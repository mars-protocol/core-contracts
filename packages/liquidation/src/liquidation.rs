use std::{
    cmp::{max, min},
    ops::Add,
};

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, Int128, StdError, Uint128};
use mars_health::health::Health;
use mars_types::{
    health::{AccountValuation, HealthValuesResponse},
    params::AssetParams,
};
#[cfg(feature = "javascript")]
use tsify::Tsify;

use crate::error::LiquidationError;

#[cw_serde]
#[cfg_attr(feature = "javascript", derive(Tsify))]
#[cfg_attr(feature = "javascript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct HealthData {
    pub liquidation_health_factor: Decimal,
    pub collateralization_ratio: Decimal,
    pub perps_pnl_loss: Uint128,
    pub account_net_value: Int128,
}

#[cw_serde]
#[cfg_attr(feature = "javascript", derive(Tsify))]
#[cfg_attr(feature = "javascript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct LiquidationAmounts {
    pub debt_amount_to_repay: Uint128,
    pub collateral_amount_to_liquidate: Uint128,
    pub collateral_amount_received_by_liquidator: Uint128,
}

/// Convert Credit Manager's Health to HealthData
impl TryFrom<HealthValuesResponse> for HealthData {
    type Error = LiquidationError;

    fn try_from(health: HealthValuesResponse) -> Result<Self, Self::Error> {
        let (liquidation_health_factor, collateralization_ratio) = prepare_hf_and_cr(
            health.liquidation_health_factor,
            health.total_collateral_value.checked_add(health.perps_pnl_profit)?,
            health.total_debt_value.checked_add(health.perps_pnl_loss)?,
        )?;

        Ok(Self {
            liquidation_health_factor,
            collateralization_ratio,
            perps_pnl_loss: health.perps_pnl_loss,
            account_net_value: health.net_value()?,
        })
    }
}

/// Convert Red Bank's Health to HealthData
impl TryFrom<Health> for HealthData {
    type Error = LiquidationError;

    fn try_from(health: Health) -> Result<Self, Self::Error> {
        let (liquidation_health_factor, collateralization_ratio) = prepare_hf_and_cr(
            health.liquidation_health_factor,
            health.total_collateral_value,
            health.total_debt_value,
        )?;

        Ok(Self {
            liquidation_health_factor,
            collateralization_ratio,
            perps_pnl_loss: Uint128::zero(),
            account_net_value: health.net_value()?,
        })
    }
}

fn prepare_hf_and_cr(
    liquidation_hf: Option<Decimal>,
    total_collateral_value: Uint128,
    total_debt_value: Uint128,
) -> Result<(Decimal, Decimal), LiquidationError> {
    // Just in case, throw an error if the health factor is not available (this shouldn’t happen, as liquidation only occurs if HF < 1)
    let liquidation_hf = liquidation_hf.ok_or_else(|| {
        LiquidationError::Std(StdError::generic_err("Liquidation health factor not available"))
    })?;

    // Just in case, throw an error if the total debt value is zero
    if total_debt_value.is_zero() {
        return Err(LiquidationError::Std(StdError::generic_err("Total debt value is zero")));
    }

    let collateralization_ratio =
        Decimal::checked_from_ratio(total_collateral_value, total_debt_value)?;

    Ok((liquidation_hf, collateralization_ratio))
}

/// Within this new system, the close factor (CF) will be determined dynamically using a parameter
/// known as the Target Health Factor (THF). The THF determines the ideal HF a position should be left
/// at immediately after the position has been liquidated. The CF, in turn, is a result of this parameter:
/// the maximum amount of debt that can be repaid to take the position to the THF.
/// For example, if the THF is 1.10 and a position gets liquidated at HF = 0.98, then the maximum
/// amount of debt a liquidator can repay (in other words, the CF) will be an amount such that the HF
/// after the liquidation is at maximum 1.10.
///
/// The formula to calculate the maximum debt that can be repaid by a liquidator is as follows:
/// MDR_value = (THF * total_debt_value - liq_th_collateral_value) / (THF - (requested_collateral_liq_th * (1 + LB)))
/// where:
/// MDR                         - Maximum Debt Repayable
/// THF                         - Target Health Factor
/// total_debt_value            - Value of debt before the liquidation happens
/// liq_th_collateral_value     - Value of collateral before the liquidation happens adjusted to liquidation threshold
/// requested_collateral_liq_th - Liquidation threshold of requested collateral
/// LB                          - Liquidation Bonus
///
/// PLF (Protocol Liqudiation Fee) is charged as a % of the LB.
/// For example, if we define the PLF as 10%, then the PLF would be deducted from the LB, so upon a liquidation:
/// - The liquidator receives 90% of the LB.
/// - The remaining 10% is sent to the protocol as PLF.
#[allow(clippy::too_many_arguments)]
pub fn calculate_liquidation_amounts(
    collateral_amount: Uint128,
    collateral_price: Decimal,
    collateral_params: &AssetParams,
    debt_amount: Uint128,
    debt_requested_to_repay: Uint128,
    debt_price: Decimal,
    debt_params: &AssetParams,
    health: &HealthData,
    perps_lb_ratio: Decimal,
) -> Result<LiquidationAmounts, LiquidationError> {
    let user_collateral_value = collateral_amount.checked_mul_floor(collateral_price)?;

    let liquidation_bonus = calculate_liquidation_bonus(
        health.liquidation_health_factor,
        health.collateralization_ratio,
        collateral_params,
    )?;

    // maximum debt being closed at once is restricted by the fixed close factor
    let max_debt_repayable_amount = debt_amount.checked_mul_floor(debt_params.close_factor)?;

    // calculate possible debt to repay based on available collateral
    let debt_amount_possible_to_repay = user_collateral_value
        .checked_div_floor(Decimal::one().add(liquidation_bonus))?
        .checked_div_floor(debt_price)?;

    let debt_amount_to_repay =
        *[debt_requested_to_repay, max_debt_repayable_amount, debt_amount_possible_to_repay]
            .iter()
            .min()
            .ok_or_else(|| StdError::generic_err("Minimum not found"))?;

    let debt_value_to_repay = debt_amount_to_repay.checked_mul_floor(debt_price)?;

    let mut collateral_amount_to_liquidate = debt_value_to_repay
        .checked_mul_floor(liquidation_bonus.add(Decimal::one()))?
        .checked_div_floor(collateral_price)?;

    // In some edges scenarios:
    // - if debt_amount_to_repay = 0, some liquidators could drain collaterals and all their coins
    // would be refunded, i.e.: without spending coins.
    // - if collateral_amount_to_liquidate is 0, some users could liquidate without receiving collaterals
    // in return.
    if (!collateral_amount_to_liquidate.is_zero() && debt_amount_to_repay.is_zero())
        || (collateral_amount_to_liquidate.is_zero() && !debt_amount_to_repay.is_zero())
    {
        return Err(LiquidationError::Std(StdError::generic_err(
            format!("Can't process liquidation. Invalid collateral_amount_to_liquidate ({collateral_amount_to_liquidate}) and debt_amount_to_repay ({debt_amount_to_repay})")
        )));
    }

    let mut lb_value = debt_value_to_repay.checked_mul_floor(liquidation_bonus)?;

    // If the user held perps positions with a PnL loss (perps_pnl_loss) before liquidation,
    // and a non-zero perps liquidation bonus ratio (perps_lb_ratio) is specified, calculate
    // a liquidation bonus specific to the perps PnL loss as a reward for the liquidator.
    // This bonus incentivizes liquidators to close perps in a loss, helping to improve the user's
    // Health Factor (HF) and reduce overall risk in the system.
    if !health.perps_pnl_loss.is_zero() && !perps_lb_ratio.is_zero() {
        // Calculate the adjusted perps liquidation bonus by applying perps_lb_ratio to
        // the standard liquidation bonus (liquidation_bonus). This results in a reduced
        // bonus percentage specifically for perps with PnL loss.
        let perps_lb_adjusted = perps_lb_ratio.checked_mul(liquidation_bonus)?;

        // Calculate the perps liquidation bonus value in terms of the PnL loss:
        // `perps_lb_value = perps_pnl_loss * perps_lb_adjusted`
        // This represents the raw bonus amount awarded for liquidating perps with PnL loss.
        let perps_lb_value = health.perps_pnl_loss.checked_mul_floor(perps_lb_adjusted)?;

        // Convert the perps liquidation bonus value to collateral terms, based on the current
        // collateral price, yielding the collateral amount needed to match the bonus value.
        let perps_lb_amount = perps_lb_value.checked_div_floor(collateral_price)?;

        // Add the perps PnL loss bonus (in collateral terms) to the total collateral amount
        // that needs to be liquidated.
        let prev_collateral_amount_to_liquidate = collateral_amount_to_liquidate;
        collateral_amount_to_liquidate =
            prev_collateral_amount_to_liquidate.checked_add(perps_lb_amount)?;

        // Ensure the total collateral amount to liquidate does not exceed the user's available
        // collateral amount. If it does, adjust it to the maximum collateral allowed.
        collateral_amount_to_liquidate = min(collateral_amount_to_liquidate, collateral_amount);

        // Calculate the capped perps liquidation bonus amount, based on the user's remaining
        // collateral. If no collateral is available, the capped bonus is set to 0.
        let perps_lb_amount_capped =
            collateral_amount_to_liquidate.saturating_sub(prev_collateral_amount_to_liquidate);
        let perps_lb_value_capped = perps_lb_amount_capped.checked_mul_floor(collateral_price)?;

        // Add the capped perps liquidation bonus value to the total liquidation bonus (lb_value).
        lb_value = lb_value.checked_add(perps_lb_value_capped)?;
    }

    // Use ceiling in favour of protocol
    let protocol_fee_value =
        lb_value.checked_mul_ceil(collateral_params.protocol_liquidation_fee)?;
    let protocol_fee_amount = protocol_fee_value.checked_div_floor(collateral_price)?;

    let collateral_amount_received_by_liquidator =
        collateral_amount_to_liquidate - protocol_fee_amount;

    Ok(LiquidationAmounts {
        debt_amount_to_repay,
        collateral_amount_to_liquidate,
        collateral_amount_received_by_liquidator,
    })
}

/// The LB will depend on the Health Factor and a couple other parameters as follows:
/// Liquidation Bonus = min(
///     starting_lb + (slope * (1 - HF)),
///     max(
///         min(CR - 1, max_lb),
///         min_lb
///     )
/// )
/// `CR` is the Collateralization Ratio of the position calculated as `CR = Total Assets / Total Debt`.
fn calculate_liquidation_bonus(
    liquidation_health_factor: Decimal,
    collateralization_ratio: Decimal,
    collateral_params: &AssetParams,
) -> Result<Decimal, LiquidationError> {
    // (CR - 1) can't be negative
    let collateralization_ratio_adjusted = if collateralization_ratio > Decimal::one() {
        collateralization_ratio - Decimal::one()
    } else {
        Decimal::zero()
    };

    let max_lb_adjusted = max(
        min(collateralization_ratio_adjusted, collateral_params.liquidation_bonus.max_lb),
        collateral_params.liquidation_bonus.min_lb,
    );

    let calculated_bonus = collateral_params.liquidation_bonus.starting_lb.checked_add(
        collateral_params
            .liquidation_bonus
            .slope
            .checked_mul(Decimal::one() - liquidation_health_factor)?,
    )?;

    let liquidation_bonus = min(calculated_bonus, max_lb_adjusted);

    Ok(liquidation_bonus)
}
