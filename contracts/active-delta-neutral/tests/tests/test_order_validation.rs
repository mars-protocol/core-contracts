use std::str::FromStr;

use cosmwasm_std::{Decimal, SignedDecimal};
use mars_types::{active_delta_neutral::order_validation::{DynamicValidator, ValidationResult}, position::Direction};
use test_case::test_case;

#[test_case(
    100, // K
    "100", // perp_ex
    "99.9", // spot_ex
    "0.00075", // perp_trading_fee_rate
    "-0.10", // perp_funding_rate
    "0.05", // net_spot_yield
    Direction::Long,
    ValidationResult {
        valid: true,
        cost: SignedDecimal::from_str("-0.00025").unwrap(),
        cost_limit: SignedDecimal::from_str("0.0015").unwrap(),
    };
    "long_spot_price_lower_than_perp_price__valid"
)]
#[test_case(
    100,
    "100",
    "101.1",
    "0.00075",
    "0.10",
    "0.05",
    Direction::Long,
    ValidationResult {
        valid: false,
        cost: SignedDecimal::from_str("0.011750").unwrap(),
        cost_limit: SignedDecimal::from_str("-0.0005").unwrap(),
    };
    "long_spot_price_higher_than_perp_price__invalid"
)]
#[test_case(
    100,
    "100",
    "100",
    "0.00075",
    "-0.20",
    "-0.05",
    Direction::Long,
    ValidationResult {
        valid: true,
        cost: SignedDecimal::from_str("0.00075").unwrap(),
        cost_limit: SignedDecimal::from_str("0.0015").unwrap(),
    };
    "long_spot_price_equals_perp_price__valid"
)]
#[test_case(
    300,
    "100",
    "101",
    "0.00075",
    "-6.00",
    "1.00",
    Direction::Long,
    ValidationResult {
        valid: true,
        cost: SignedDecimal::from_str("0.01075").unwrap(),
        cost_limit: SignedDecimal::from_str("0.023333333333333333").unwrap(),
    };
    "long_spot_price_higher_than_perp_price_high_yield__valid"
)]
#[test_case(
    300,
    "100",
    "100",
    "0.00075",
    "0.30",
    "0.15",
    Direction::Long,
    ValidationResult {
        valid: false,
        cost: SignedDecimal::from_str("0.00075").unwrap(),
        cost_limit: SignedDecimal::from_str("-0.0005").unwrap(),
    };
    "long_spot_price_equals_perp_price_high_k__invalid"
)]
#[test_case(
    100,
    "123399.9999",
    "123399.9999",
    "0.00075",
    "0.10",
    "0.05",
    Direction::Long,
    ValidationResult {
        valid: false,
        cost: SignedDecimal::from_str("0.00075").unwrap(),
        cost_limit: SignedDecimal::from_str("-0.0005").unwrap(),
    };
    "long_spot_price_equals_perp_price_large_values__invalid"
)]
#[test_case(
    100, // K
    "99.9", // perp_ex
    "100", // spot_ex
    "0.00075", // perp_trading_fee_rate
    "0.10", // perp_funding_rate
    "0.05", // net_spot_yield
    Direction::Short,
    ValidationResult {
        valid: true,
        cost: SignedDecimal::from_str("0.001751001001001001").unwrap(),
        cost_limit: SignedDecimal::from_str("0.0015").unwrap(),
    };
    "short_spot_price_higher_than_perp_price__valid"
)]
#[test_case(
    100,
    "101.1",
    "100",
    "0.00075",
    "-0.10",
    "0.05",
    Direction::Short,
    ValidationResult {
        valid: false,
        cost: SignedDecimal::from_str("-0.010130316518298714").unwrap(),
        cost_limit: SignedDecimal::from_str("-0.0005").unwrap(),
    };
    "short_spot_price_lower_than_perp_price__invalid"
)]
#[test_case(
    100,
    "100",
    "100",
    "0.00075",
    "-0.20",
    "-0.05",
    Direction::Short,
    ValidationResult {
        valid: true,
        cost: SignedDecimal::from_str("0.00075").unwrap(),
        cost_limit: SignedDecimal::from_str("-0.0025").unwrap(),
    };
    "short_spot_price_equals_perp_price__valid"
)]
#[test_case(
    300,
    "101",
    "100",
    "0.00075",
    "1.00",
    "-6.00",
    Direction::Short,
    ValidationResult {
        valid: true,
        cost: SignedDecimal::from_str("-0.0091509900990099").unwrap(),
        cost_limit: SignedDecimal::from_str("-0.016666666666666666").unwrap(),
    };
    "short_spot_price_lower_than_perp_price_high_yield__valid"
)]
#[test_case(
    300,
    "100",
    "100",
    "0.00075",
    "0.15",
    "0.30",
    Direction::Short,
    ValidationResult {
        valid: false,
        cost: SignedDecimal::from_str("0.00075").unwrap(),
        cost_limit: SignedDecimal::from_str("0.0015").unwrap(),
    };
    "short_spot_price_equals_perp_price_high_k__invalid"
)]
#[test_case(
    100,
    "123399.9999",
    "123399.9999",
    "0.00075",
    "0.05",
    "0.10",
    Direction::Short,
    ValidationResult {
        valid: false,
        cost: SignedDecimal::from_str("0.00075").unwrap(),
        cost_limit: SignedDecimal::from_str("0.0015").unwrap(),
    };
    "short_spot_price_equals_perp_price_large_values__invalid"
)]
fn test_validate_order_execution(
    k: u64,
    perp_execution_price: &str,
    spot_execution_price: &str,
    perp_trading_fee_rate: &str,
    perp_funding_rate: &str,
    net_spot_yield: &str,
    direction: Direction,
    expected_result: ValidationResult,
) {
    let validator = DynamicValidator { k };

    let result = validator.validate_order_execution(
        SignedDecimal::from_str(perp_funding_rate).unwrap(),
        SignedDecimal::from_str(net_spot_yield).unwrap(),
        SignedDecimal::from_str(spot_execution_price).unwrap(),
        SignedDecimal::from_str(perp_execution_price).unwrap(),
        Decimal::from_str(perp_trading_fee_rate).unwrap(),
        direction,
    ).unwrap();

    assert_eq!(result, expected_result);
}




