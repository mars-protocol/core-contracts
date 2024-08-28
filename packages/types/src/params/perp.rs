use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, Uint128};
use mars_utils::helpers::{decimal_param_le_one, decimal_param_lt_one};

use super::assertions::{
    assert_lqt_gt_max_ltv, assert_max_net_oi_le_max_oi_long, assert_max_net_oi_le_max_oi_short,
    assert_max_size_gt_min, assert_skew_scale,
};
use crate::error::MarsError;

#[cw_serde]
#[derive(Default)]
pub struct PerpParams {
    /// Perp denomination
    pub denom: String,
    /// Whether the perp is enabled
    pub enabled: bool,
    /// The maximum net open interest value (in oracle uusd denomination)
    pub max_net_oi_value: Uint128,
    /// The maximum long open interest value (in oracle uusd denomination)
    pub max_long_oi_value: Uint128,
    /// The maximum short open interest value (in oracle uusd denomination)
    pub max_short_oi_value: Uint128,
    /// The fee paid by the user to close a position (as a percent)
    pub closing_fee_rate: Decimal,
    /// The fee paid by the user to open a position (as a percent)
    pub opening_fee_rate: Decimal,
    /// The minimum value of a position (in oracle uusd denomination)
    pub min_position_value: Uint128,
    /// The maximum value of a position (in oracle uusd denomination)
    pub max_position_value: Option<Uint128>,
    /// Max loan to position value for the position.
    pub max_loan_to_value: Decimal,
    /// LTV at which a position becomes liquidatable
    pub liquidation_threshold: Decimal,
    /// Determines the maximum rate at which funding can be adjusted
    pub max_funding_velocity: Decimal,
    /// Determines the funding rate for a given level of skew.
    /// The lower the skew_scale the higher the funding rate.
    pub skew_scale: Uint128,
}

impl PerpParams {
    pub fn check(&self) -> Result<PerpParams, MarsError> {
        decimal_param_le_one(self.liquidation_threshold, "liquidation_threshold")?;
        assert_lqt_gt_max_ltv(self.max_loan_to_value, self.liquidation_threshold)?;
        decimal_param_lt_one(self.opening_fee_rate, "opening_fee_rate")?;
        decimal_param_lt_one(self.closing_fee_rate, "closing_fee_rate")?;
        assert_max_net_oi_le_max_oi_long(self.max_long_oi_value, self.max_net_oi_value)?;
        assert_max_net_oi_le_max_oi_short(self.max_short_oi_value, self.max_net_oi_value)?;
        assert_max_size_gt_min(self.max_position_value, self.min_position_value)?;
        assert_skew_scale(self.skew_scale)?;

        Ok(PerpParams {
            denom: self.denom.clone(),
            enabled: self.enabled,
            max_net_oi_value: self.max_net_oi_value,
            max_long_oi_value: self.max_long_oi_value,
            max_short_oi_value: self.max_short_oi_value,
            closing_fee_rate: self.closing_fee_rate,
            opening_fee_rate: self.opening_fee_rate,
            min_position_value: self.min_position_value,
            max_position_value: self.max_position_value,
            max_loan_to_value: self.max_loan_to_value,
            liquidation_threshold: self.liquidation_threshold,
            max_funding_velocity: self.max_funding_velocity,
            skew_scale: self.skew_scale,
        })
    }
}
