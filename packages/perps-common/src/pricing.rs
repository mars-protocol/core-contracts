use std::{cmp::max, str::FromStr};

use cosmwasm_std::{Decimal, Fraction, Int128, SignedDecimal, Uint128};
use mars_types::perps::PerpsError;

/// Price with market impact applied for opening a position
pub fn opening_execution_price(
    skew: Int128,
    skew_scale: Uint128,
    size: Int128,
    oracle_price: Decimal,
) -> Result<Decimal, PerpsError> {
    let initial_premium = initial_premium(skew, skew_scale)?;
    let final_premium_opening = final_premium_opening(skew, skew_scale, size)?;
    let res = execution_price(initial_premium, final_premium_opening, oracle_price)?;
    Ok(res)
}
/// Price with market impact applied for closing a position
pub fn closing_execution_price(
    skew: Int128,
    skew_scale: Uint128,
    size: Int128,
    oracle_price: Decimal,
) -> Result<Decimal, PerpsError> {
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
fn initial_premium(skew: Int128, skew_scale: Uint128) -> Result<SignedDecimal, PerpsError> {
    Ok(SignedDecimal::checked_from_ratio(skew, Int128::try_from(skew_scale)?)?)
}

/// Calculate the final premium for a given skew before modification by opening size.
///
/// FinalPremium(t0) = FinalSkew(t0) / SkewScale
/// where:
/// FinalSkew(t0) = Skew(t0) + Size
fn final_premium_opening(
    skew: Int128,
    skew_scale: Uint128,
    size: Int128,
) -> Result<SignedDecimal, PerpsError> {
    let final_skew = skew.checked_add(size)?;
    Ok(SignedDecimal::checked_from_ratio(final_skew, Int128::try_from(skew_scale)?)?)
}

/// Calculate the final premium for a given skew before modification by closing size.
///
/// FinalPremium(t) = FinalSkew(t) / SkewScale
/// where:
/// FinalSkew(t) = Skew(t) - Size
fn final_premium_closing(
    skew: Int128,
    skew_scale: Uint128,
    size: Int128,
) -> Result<SignedDecimal, PerpsError> {
    let final_skew = skew.checked_sub(size)?;
    Ok(SignedDecimal::checked_from_ratio(final_skew, Int128::try_from(skew_scale)?)?)
}

/// Price with market impact applied
fn execution_price(
    initial_premium: SignedDecimal,
    final_premium: SignedDecimal,
    oracle_price: Decimal,
) -> Result<Decimal, PerpsError> {
    let avg_premium = initial_premium
        .checked_add(final_premium)?
        .checked_div(SignedDecimal::from_atomics(2i128, 0)?)?;

    // Price being negative is very unlikely scenario as we're using quite large skewScale compared to the maxSkew (risk team methodology),
    // but we add hard restriction on the market impact, just in case.
    let avg_premium_bounded = max(avg_premium, SignedDecimal::from_str("-1").unwrap());

    let res = SignedDecimal::one()
        .checked_add(avg_premium_bounded)?
        .checked_mul(oracle_price.try_into()?)?;

    // Price won't be negative, so it is safe to return Decimal
    let res = Decimal::from_ratio(res.numerator().unsigned_abs(), res.denominator().unsigned_abs());
    Ok(res)
}
