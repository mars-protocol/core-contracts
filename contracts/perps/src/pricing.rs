use cosmwasm_std::Decimal;
use mars_types::math::SignedDecimal;

use crate::error::ContractResult;

/// Price with market impact applied for opening a position
pub fn opening_execution_price(
    skew: SignedDecimal,
    skew_scale: Decimal,
    size: SignedDecimal,
    oracle_price: Decimal,
) -> ContractResult<SignedDecimal> {
    let initial_premium = initial_premium(skew, skew_scale)?;
    let final_premium_opening = final_premium_opening(skew, skew_scale, size)?;
    let res = execution_price(initial_premium, final_premium_opening, oracle_price)?;
    Ok(res)
}

/// Price with market impact applied for closing a position
pub fn closing_execution_price(
    skew: SignedDecimal,
    skew_scale: Decimal,
    size: SignedDecimal,
    oracle_price: Decimal,
) -> ContractResult<SignedDecimal> {
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
fn initial_premium(skew: SignedDecimal, skew_scale: Decimal) -> ContractResult<SignedDecimal> {
    Ok(skew.checked_div(skew_scale.into())?)
}

/// Calculate the final premium for a given skew before modification by opening size.
///
/// FinalPremium(t0) = FinalSkew(t0) / SkewScale
/// where:
/// FinalSkew(t0) = Skew(t0) + Size
fn final_premium_opening(
    skew: SignedDecimal,
    skew_scale: Decimal,
    size: SignedDecimal,
) -> ContractResult<SignedDecimal> {
    let final_skew = skew.checked_add(size)?;
    Ok(final_skew.checked_div(skew_scale.into())?)
}

/// Calculate the final premium for a given skew before modification by closing size.
///
/// FinalPremium(t) = FinalSkew(t) / SkewScale
/// where:
/// FinalSkew(t) = Skew(t) - Size
fn final_premium_closing(
    skew: SignedDecimal,
    skew_scale: Decimal,
    size: SignedDecimal,
) -> ContractResult<SignedDecimal> {
    let final_skew = skew.checked_sub(size)?;
    Ok(final_skew.checked_div(skew_scale.into())?)
}

/// Price with market impact applied
fn execution_price(
    initial_premium: SignedDecimal,
    final_premium: SignedDecimal,
    oracle_price: Decimal,
) -> ContractResult<SignedDecimal> {
    let avg_premium = initial_premium
        .checked_add(final_premium)?
        .checked_div(Decimal::from_atomics(2u128, 0)?.into())?;
    let res = SignedDecimal::one().checked_add(avg_premium)?.checked_mul(oracle_price.into())?;
    Ok(res)
}
