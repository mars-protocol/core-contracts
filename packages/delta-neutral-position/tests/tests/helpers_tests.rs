use std::str::FromStr;

use cosmwasm_std::{Decimal, Int128, SignedDecimal, Uint128};
use mars_delta_neutral_position::helpers::{prorate_i128_by_amount, weighted_avg};
use mars_utils::helpers::{int128_to_signed_decimal, uint128_to_decimal};
use test_case::test_case;

#[test_case(1_000_000i128, "1000000.0")]
#[test_case(0i128, "0.0")]
#[test_case(-500_000i128, "-500000.0")]
fn test_decimal_from_i128(input: i128, expected: &str) {
    let result = int128_to_signed_decimal(Int128::new(input)).unwrap();
    assert_eq!(result, SignedDecimal::from_str(expected).unwrap());
}

#[test_case(1_000_000u128, "1000000.0")]
#[test_case(0u128, "0.0")]
fn test_decimal_from_u128(input: u128, expected: &str) {
    let result = uint128_to_decimal(Uint128::new(input)).unwrap();
    assert_eq!(result, Decimal::from_str(expected).unwrap());
}

#[test_case("100.0", 1_000_000u128, "200.0", 1_000_000u128, "150.0"; "equal weights")]
#[test_case("0.0", 0u128, "123.456", 1_000_000u128, "123.456"; "empty old size")]
#[test_case("50.0", 999_999u128, "150.0", 1u128, "50.0001"; "rounding test")]
fn test_weighted_avg(
    old_price: &str,
    old_amt: u128,
    new_price: &str,
    new_amt: u128,
    expected: &str,
) {
    let result = weighted_avg(
        Decimal::from_str(old_price).unwrap(),
        Uint128::new(old_amt),
        Decimal::from_str(new_price).unwrap(),
        Uint128::new(new_amt),
    )
    .unwrap();

    let expected_decimal = Decimal::from_str(expected).unwrap();
    assert!(
        result.checked_sub(expected_decimal).unwrap() < Decimal::from_ratio(1u128, 1_000_000u128)
    );
}

#[test_case(10_000_000i128, 5_000_000u128, 10_000_000u128, 5_000_000i128; "half of total")]
#[test_case(-12_345_678i128, 2_000_000u128, 8_000_000u128, -3_086_420i128; "prorate negative value")]
#[test_case(0i128, 1_000_000u128, 1_000_000u128, 0i128; "zero stays zero")]
fn test_prorate_i128_by_amount(total: i128, slice: u128, total_size: u128, expected: i128) {
    let result =
        prorate_i128_by_amount(Int128::new(total), Uint128::new(slice), Uint128::new(total_size))
            .unwrap();
    assert_eq!(result, Int128::new(expected));
}

#[test]
fn test_prorate_zero_total_size() {
    let result =
        prorate_i128_by_amount(Int128::new(100), Uint128::new(10), Uint128::zero()).unwrap();
    assert_eq!(result, Int128::zero());
}
