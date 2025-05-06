use cosmwasm_std::{ConversionOverflowError, Int128, Int256, Uint128};

/// Helper function to convert Uint128 to Int128 safely
///
/// # Arguments
///
/// * `value` - The Uint128 value to convert
///
/// # Returns
///
/// * `Int128` - The converted Int128 value
/// * `ConversionOverflowError` - If the value is too large to fit in Int128
pub fn i128_from_u128(value: Uint128) -> Result<Int128, ConversionOverflowError> {
    Int256::from(value).try_into()
}
