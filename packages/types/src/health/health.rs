use std::fmt;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, Uint128};
#[cfg(feature = "javascript")]
use tsify::Tsify;

#[cw_serde]
pub struct Health {
    /// The sum of all debt. Does not include negative perp pnl
    pub total_debt_value: Uint128,
    /// The sum of the value of spot collateral. Does not include positive perp pnl
    pub total_collateral_value: Uint128,
    /// The sum of the value of all colletarals adjusted by their Max LTV
    pub max_ltv_adjusted_collateral: Uint128,
    /// The sum of the value of all colletarals adjusted by their Liquidation Threshold
    pub liquidation_threshold_adjusted_collateral: Uint128,
    /// The sum of the value of all collaterals multiplied by their max LTV, over the total value of debt
    pub max_ltv_health_factor: Option<Decimal>,
    /// The sum of the value of all collaterals multiplied by their liquidation threshold over the total value of debt
    pub liquidation_health_factor: Option<Decimal>,
    /// The total of winning pnl positions
    pub perps_pnl_profit: Uint128,
    /// the total of pnl losing positions
    pub perps_pnl_loss: Uint128,
    /// If the account has perps positions.
    /// `perps_pnl_profit` and `perps_pnl_loss` could be zero even with perps (`BreakEven` case).
    pub has_perps: bool,
}

impl fmt::Display for Health {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "(total_debt_value: {}, total_collateral_value: {},  max_ltv_adjusted_collateral: {}, lqdt_threshold_adjusted_collateral: {}, max_ltv_health_factor: {}, liquidation_health_factor: {}, pnl_profit : {}, pnl_losses : {}, has_perps: {})",
            self.total_debt_value,
            self.total_collateral_value,
            self.max_ltv_adjusted_collateral,
            self.liquidation_threshold_adjusted_collateral,
            self.max_ltv_health_factor.map_or("n/a".to_string(), |x| x.to_string()),
            self.liquidation_health_factor.map_or("n/a".to_string(), |x| x.to_string()),
            self.perps_pnl_profit,
            self.perps_pnl_loss,
            self.has_perps
        )
    }
}

impl Health {
    #[inline]
    pub fn is_liquidatable(&self) -> bool {
        is_below_one(&self.liquidation_health_factor)
    }

    #[inline]
    pub fn is_above_max_ltv(&self) -> bool {
        is_below_one(&self.max_ltv_health_factor)
    }
}

pub fn is_below_one(health_factor: &Option<Decimal>) -> bool {
    health_factor.map_or(false, |hf| hf < Decimal::one())
}

#[cw_serde]
#[cfg_attr(feature = "javascript", derive(Tsify))]
#[cfg_attr(feature = "javascript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct HealthValuesResponse {
    pub total_debt_value: Uint128,
    pub total_collateral_value: Uint128,
    pub max_ltv_adjusted_collateral: Uint128,
    pub liquidation_threshold_adjusted_collateral: Uint128,
    pub max_ltv_health_factor: Option<Decimal>,
    pub liquidation_health_factor: Option<Decimal>,
    pub perps_pnl_profit: Uint128,
    pub perps_pnl_loss: Uint128,
    pub liquidatable: bool,
    pub above_max_ltv: bool,
    pub has_perps: bool,
}

impl From<Health> for HealthValuesResponse {
    fn from(h: Health) -> Self {
        Self {
            total_debt_value: h.total_debt_value,
            total_collateral_value: h.total_collateral_value,
            max_ltv_adjusted_collateral: h.max_ltv_adjusted_collateral,
            liquidation_threshold_adjusted_collateral: h.liquidation_threshold_adjusted_collateral,
            max_ltv_health_factor: h.max_ltv_health_factor,
            liquidation_health_factor: h.liquidation_health_factor,
            perps_pnl_profit: h.perps_pnl_profit,
            perps_pnl_loss: h.perps_pnl_loss,
            liquidatable: h.is_liquidatable(),
            above_max_ltv: h.is_above_max_ltv(),
            has_perps: h.has_perps,
        }
    }
}

#[cw_serde]
pub enum HealthState {
    Healthy,
    Unhealthy {
        max_ltv_health_factor: Decimal,
    },
}

impl fmt::Display for HealthState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HealthState::Healthy => write!(f, "healthy"),
            HealthState::Unhealthy {
                max_ltv_health_factor,
            } => {
                write!(
                    f,
                    "unhealthy (max_ltv_health_factor: {:?}",
                    max_ltv_health_factor.to_string(),
                )
            }
        }
    }
}
