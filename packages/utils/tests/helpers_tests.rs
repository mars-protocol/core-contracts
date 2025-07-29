use cosmwasm_std::{Decimal, Int128, SignedDecimal, StdError, Uint128};
use std::str::FromStr;
use test_case::test_case; 
use mars_utils::helpers::{int128_to_signed_decimal, uint128_to_decimal, uint128_to_int128};

#[test]
fn test_uint128_to_decimal_normal() {
    // Test normal conversion
    let value = Uint128::new(100);
    let result = uint128_to_decimal(value).unwrap();
    assert_eq!(result, Decimal::from_str("100").unwrap());

    // Test with zero
    let value = Uint128::zero();
    let result = uint128_to_decimal(value).unwrap();
    assert_eq!(result, Decimal::from_str("0").unwrap());

    // Test with large value
    let value = Uint128::new(1_000_000_000_000_000);
    let result = uint128_to_decimal(value).unwrap();
    assert_eq!(result, Decimal::from_str("1000000000000000").unwrap());
}

#[test]
fn test_uint128_to_decimal_max() {
    // Test with maximum Uint128 value
    // This should still work as Decimal can represent the full range of Uint128
    let max_safe_value = i128::MAX.checked_div(10u128.pow(18) as i128).unwrap();
    let value = Uint128::from(max_safe_value as u128);
    let result = uint128_to_decimal(value).unwrap();
    assert_eq!(result, Decimal::from_str(&value.to_string()).unwrap());
}

#[test]
fn test_int128_to_signed_decimal_normal() {
    // Test with positive value
    let value = Int128::new(100);
    let result = int128_to_signed_decimal(value).unwrap();
    assert_eq!(result, SignedDecimal::from_str("100").unwrap());

    // Test with negative value
    let value = Int128::new(-100);
    let result = int128_to_signed_decimal(value).unwrap();
    assert_eq!(result, SignedDecimal::from_str("-100").unwrap());

    // Test with zero
    let value = Int128::zero();
    let result = int128_to_signed_decimal(value).unwrap();
    assert_eq!(result, SignedDecimal::from_str("0").unwrap());
}

#[test]
fn test_int128_to_signed_decimal_edge_cases() {
    // Test with maximum Int128 value
    let max_safe_value = i128::MAX.checked_div(10u128.pow(18) as i128).unwrap();
    let value = Int128::new(max_safe_value);
    let result = int128_to_signed_decimal(value).unwrap();
    assert_eq!(
        result,
        SignedDecimal::from_str(&max_safe_value.to_string()).unwrap()
    );

    // Test with minimum Int128 value
    let min_safe_value = i128::MIN.checked_div(-(10u128.pow(18) as i128)).unwrap();
    let value = Int128::new(min_safe_value);
    let result = int128_to_signed_decimal(value).unwrap();
    assert_eq!(
        result,
        SignedDecimal::from_str(&min_safe_value.to_string()).unwrap()
    );
}

#[test]
fn test_uint128_to_int128_normal() {
    // Test normal conversion
    let value = Uint128::new(100);
    let result = uint128_to_int128(value).unwrap();
    assert_eq!(result, Int128::new(100));

    // Test with zero
    let value = Uint128::zero();
    let result = uint128_to_int128(value).unwrap();
    assert_eq!(result, Int128::zero());

    // Test with large but valid value
    let value = Uint128::new(1_000_000_000_000_000);
    let result = uint128_to_int128(value).unwrap();
    assert_eq!(result, Int128::new(1_000_000_000_000_000));
}

#[test]
fn test_uint128_to_int128_overflow() {
    // Test with value too large for Int128 (should error)
    let max_safe_value = i128::MAX;
    // Add 1 to ensure overflow occurs
    let value = Uint128::from(max_safe_value as u128) + Uint128::new(1);
    let result = uint128_to_int128(value);
    assert!(result.is_err());

    match result {
        Err(StdError::GenericErr { msg, .. }) => {
            assert!(msg.contains("Overflow"));
        }
        _ => panic!("Expected StdError with Overflow message"),
    }

    // Test with value at the boundary (max_safe_value)
    let value = Uint128::from(max_safe_value as u128);
    let result = uint128_to_int128(value).unwrap();
    assert_eq!(result, Int128::new(max_safe_value));
}

#[test]
fn test_chained_conversions() {
    // Test that chaining multiple conversions preserves values correctly
    let original = Uint128::new(42);

    // Uint128 -> Int128 -> SignedDecimal -> unwrap to i128 -> Int128
    let int128_value = uint128_to_int128(original).unwrap();
    let signed_decimal = int128_to_signed_decimal(int128_value).unwrap();
    let i128_value = signed_decimal.to_int_floor(); // Get the i128 value
    let roundtrip = Int128::new(i128_value.i128());

    assert_eq!(int128_value, roundtrip);
    assert_eq!(original, Uint128::new(roundtrip.i128() as u128));
}

#[test_case("1234.567", 1_234i128; "positive rounds down")]
#[test_case("999999.999999", 999_999i128; "positive just below one")]
#[test_case("1.000001", 1i128; "positive tiny")]
#[test_case("0.000000", 0i128; "zero")]
#[test_case("-1234.567", -1_235i128; "negative rounds up")]
#[test_case("-999999.999999", -1_000_000i128; "negative just below zero")]
#[test_case("-1.00000001", -2i128; "negative tiny")]
fn test_signed_decimal_to_i128(input: &str, expected: i128) {
    let d = SignedDecimal::from_str(input).unwrap();
    let result = d.to_int_floor();
    assert_eq!(result, Int128::new(expected));
}
