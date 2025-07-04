use std::str::FromStr;

use cosmwasm_std::{Decimal, Uint128};
use mars_types::params::PerpParams;

pub fn default_perp_params(denom: &str) -> PerpParams {
    PerpParams {
        denom: denom.to_string(),
        enabled: true,
        max_net_oi_value: Uint128::new(1_000_000_000),
        max_long_oi_value: Uint128::new(1_000_000_000),
        max_short_oi_value: Uint128::new(1_000_000_000),
        closing_fee_rate: Decimal::from_str("0.0").unwrap(),
        opening_fee_rate: Decimal::from_str("0.0").unwrap(),
        liquidation_threshold: Decimal::from_str("0.85").unwrap(),
        max_loan_to_value: Decimal::from_str("0.8").unwrap(),
        max_loan_to_value_usdc: None,
        liquidation_threshold_usdc: None,
        max_position_value: None,
        min_position_value: Uint128::zero(),
        max_funding_velocity: Decimal::from_str("3").unwrap(),
        skew_scale: Uint128::new(1000000u128),
    }
}
