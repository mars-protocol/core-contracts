use std::str::FromStr;

use cosmwasm_std::{Decimal, Uint128};
use mars_types::{
    math::SignedDecimal,
    params::PerpParams,
    perps::{Funding, PerpDenomState},
};

pub struct PerpInfo {
    pub denom: String,
    pub price: Decimal,
    pub perp_params: PerpParams,
}

pub fn create_default_perp_info() -> PerpParams {
    PerpParams {
        denom: "default".to_string(),
        max_net_oi_value: Uint128::new(1200),
        max_long_oi_value: Uint128::new(800),
        max_short_oi_value: Uint128::new(800),
        closing_fee_rate: Decimal::percent(5),
        opening_fee_rate: Decimal::percent(5),
        min_position_value: Uint128::new(10),
        max_position_value: None,
        max_loan_to_value: Decimal::percent(75),
        liquidation_threshold: Decimal::percent(78),
    }
}

pub fn create_perp_denom_state(
    long_oi: Decimal,
    short_oi: Decimal,
    funding: Funding,
) -> PerpDenomState {
    PerpDenomState {
        enabled: true,
        long_oi,
        short_oi,
        funding,
        ..Default::default()
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
            max_net_oi_value: Uint128::new(1200),
            max_long_oi_value: Uint128::new(800),
            max_short_oi_value: Uint128::new(800),
            closing_fee_rate: Decimal::percent(5),
            opening_fee_rate: Decimal::percent(5),
            min_position_value: Uint128::new(10),
            max_position_value: None,
            max_loan_to_value: max_ltv,
            liquidation_threshold,
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
        skew_scale: Decimal::from_str("1000").unwrap(),
        last_funding_rate: SignedDecimal::from_str("1.0").unwrap(),
        max_funding_velocity: Decimal::percent(0),
        last_funding_accrued_per_unit_in_base_denom: SignedDecimal::from_str("300").unwrap(),
    }
}
