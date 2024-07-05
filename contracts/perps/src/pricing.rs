use std::{cmp::max, str::FromStr};

use cosmwasm_std::{Decimal, Uint128};
use mars_types::{math::SignedDecimal, signed_uint::SignedUint};

use crate::error::ContractResult;

/// Price with market impact applied for opening a position
pub fn opening_execution_price(
    skew: SignedUint,
    skew_scale: Uint128,
    size: SignedUint,
    oracle_price: Decimal,
) -> ContractResult<Decimal> {
    let initial_premium = initial_premium(skew, skew_scale)?;
    let final_premium_opening = final_premium_opening(skew, skew_scale, size)?;
    let res = execution_price(initial_premium, final_premium_opening, oracle_price)?;
    Ok(res)
}
/// Price with market impact applied for closing a position
pub fn closing_execution_price(
    skew: SignedUint,
    skew_scale: Uint128,
    size: SignedUint,
    oracle_price: Decimal,
) -> ContractResult<Decimal> {
    let initial_premium = initial_premium(skew, skew_scale)?;
    let final_premium_closing = final_premium_closing(skew, skew_scale, size)?;
    let res = execution_price(initial_premium, final_premium_closing, oracle_price)?;
    Ok(res)
}

/// Calculate the initial premium for a given skew before modification by opening/closing size.
///
/// InitialPremium(i) = Skew(i) / SkewScale
/// where:
/// i = t0, t
fn initial_premium(skew: SignedUint, skew_scale: Uint128) -> ContractResult<SignedDecimal> {
    Ok(SignedDecimal::checked_from_ratio(skew, skew_scale.into())?)
}

/// Calculate the final premium for a given skew before modification by opening size.
///
/// FinalPremium(t0) = FinalSkew(t0) / SkewScale
/// where:
/// FinalSkew(t0) = Skew(t0) + Size
fn final_premium_opening(
    skew: SignedUint,
    skew_scale: Uint128,
    size: SignedUint,
) -> ContractResult<SignedDecimal> {
    let final_skew = skew.checked_add(size)?;
    Ok(SignedDecimal::checked_from_ratio(final_skew, skew_scale.into())?)
}

/// Calculate the final premium for a given skew before modification by closing size.
///
/// FinalPremium(t) = FinalSkew(t) / SkewScale
/// where:
/// FinalSkew(t) = Skew(t) - Size
fn final_premium_closing(
    skew: SignedUint,
    skew_scale: Uint128,
    size: SignedUint,
) -> ContractResult<SignedDecimal> {
    let final_skew = skew.checked_sub(size)?;
    Ok(SignedDecimal::checked_from_ratio(final_skew, skew_scale.into())?)
}

/// Price with market impact applied
fn execution_price(
    initial_premium: SignedDecimal,
    final_premium: SignedDecimal,
    oracle_price: Decimal,
) -> ContractResult<Decimal> {
    let avg_premium = initial_premium
        .checked_add(final_premium)?
        .checked_div(Decimal::from_atomics(2u128, 0)?.into())?;

    // Price being negative is very unlikely scenario as we're using quite large skewScale compared to the maxSkew (risk team methodology),
    // but we add hard restriction on the market impact, just in case.
    let avg_premium_bounded = max(avg_premium, SignedDecimal::from_str("-1").unwrap());

    let res =
        SignedDecimal::one().checked_add(avg_premium_bounded)?.checked_mul(oracle_price.into())?;

    // Price won't be negative, so it is safe to return Decimal
    Ok(res.abs)
}
