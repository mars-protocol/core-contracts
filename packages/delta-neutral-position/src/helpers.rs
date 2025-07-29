use cosmwasm_std::{Decimal, Int128, SignedDecimal, Uint128};
use mars_utils::helpers::{int128_to_signed_decimal, uint128_to_int128};

use crate::error::ContractResult;

/// Returns the weighted average of two prices given their respective sizes.
pub fn weighted_avg(
    old_price: Decimal,
    old_size: Uint128,
    new_price: Decimal,
    new_size: Uint128,
) -> ContractResult<Decimal> {
    if old_size.is_zero() {
        return Ok(new_price);
    }

    let total_size = old_size.checked_add(new_size)?;

    let numerator = old_price
        .checked_mul(Decimal::from_atomics(old_size, 0)?)?
        .checked_add(new_price.checked_mul(Decimal::from_atomics(new_size, 0)?)?)?;

    let result = numerator.checked_div(Decimal::from_atomics(total_size, 0)?)?;

    Ok(result)
}

pub fn prorate_i128_by_amount(
    total: Int128,
    slice: Uint128,
    total_size: Uint128,
) -> ContractResult<Int128> {
    if total_size.is_zero() {
        return Ok(Int128::zero());
    }

    let sd_slice = SignedDecimal::from_atomics(uint128_to_int128(slice)?, 0)?;
    let sd_total_size = SignedDecimal::from_atomics(uint128_to_int128(total_size)?, 0)?;
    let ratio = sd_slice.checked_div(sd_total_size)?;

    Ok(int128_to_signed_decimal(total)?
        .checked_mul(ratio)?
        .to_int_floor())
}
