use std::str::FromStr;

use cosmwasm_std::{Decimal, SignedDecimal, Uint128};
use mars_types::{params::PerpParams, perps::Funding};

pub struct PerpInfo {
    pub denom: String,
    pub price: Decimal,
    pub perp_params: PerpParams,
}

pub fn create_default_perp_info() -> PerpParams {
    PerpParams {
        denom: "default".to_string(),
        enabled: true,
        max_net_oi_value: Uint128::new(1200),
        max_long_oi_value: Uint128::new(800),
        max_short_oi_value: Uint128::new(800),
        closing_fee_rate: Decimal::percent(5),
        opening_fee_rate: Decimal::percent(5),
        min_position_value: Uint128::new(10),
        max_position_value: None,
        max_loan_to_value: Decimal::percent(75),
        liquidation_threshold: Decimal::percent(78),
        max_funding_velocity: Decimal::from_str("36").unwrap(),
        skew_scale: Uint128::new(1_000_000_000_000_000u128),
    }
}

pub fn create_perp_info(
    denom: String,
    price: Decimal,
    max_ltv: Decimal,
    liquidation_threshold: Decimal,
) -> PerpInfo {
    PerpInfo {
        perp_params: PerpParams {
            denom: denom.clone(),
            enabled: true,
            max_net_oi_value: Uint128::new(1200),
            max_long_oi_value: Uint128::new(800),
            max_short_oi_value: Uint128::new(800),
            closing_fee_rate: Decimal::from_str("0.0002").unwrap(),
            opening_fee_rate: Decimal::percent(5),
            min_position_value: Uint128::new(10),
            max_position_value: None,
            max_loan_to_value: max_ltv,
            liquidation_threshold,
            max_funding_velocity: Decimal::from_str("36").unwrap(),
            skew_scale: Uint128::new(1_000_000_000_000_000u128),
        },
        denom,
        price,
    }
}

pub fn btcperp_info() -> PerpInfo {
    let denom: String = "btc/usd/perp".to_string();
    let price = Decimal::from_str("100").unwrap();
    let max_loan_to_value = Decimal::from_str("0.9").unwrap();
    let liquidation_threshold = Decimal::from_str("0.95").unwrap();

    create_perp_info(denom, price, max_loan_to_value, liquidation_threshold)
}

pub fn ethperp_info() -> PerpInfo {
    let denom: String = "eth/usd/perp".to_string();
    let price = Decimal::from_str("10").unwrap();
    let max_loan_to_value = Decimal::from_str("0.85").unwrap();
    let liquidation_threshold = Decimal::from_str("0.90").unwrap();

    create_perp_info(denom, price, max_loan_to_value, liquidation_threshold)
}

pub fn atomperp_info() -> PerpInfo {
    let denom: String = "atom/usd/perp".to_string();
    let price = Decimal::from_str("10").unwrap();
    let max_loan_to_value = Decimal::from_str("0.80").unwrap();
    let liquidation_threshold = Decimal::from_str("0.85").unwrap();

    create_perp_info(denom, price, max_loan_to_value, liquidation_threshold)
}

pub fn create_default_funding() -> Funding {
    Funding {
        skew_scale: Uint128::new(1_000_000_000_000_000u128),
        last_funding_rate: SignedDecimal::from_str("1.0").unwrap(),
        max_funding_velocity: Decimal::percent(0),
        last_funding_accrued_per_unit_in_base_denom: SignedDecimal::from_str("3").unwrap(),
    }
}
