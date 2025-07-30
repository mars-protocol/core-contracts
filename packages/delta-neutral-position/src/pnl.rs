use std::convert::TryFrom;

use cosmwasm_std::{Decimal, Int128, Uint128};

use crate::error::{ContractError, ContractResult};

/// Computes the realized PnL for a delta-neutral position unwind.
///
/// # Formula
/// ```text
/// RealizedPnL = (SpotExitPrice - PerpExitPrice) * DecreaseAmount
///             - (TotalEntryValue / TotalPositionSize) * DecreaseAmount
///             - FeeAmount
/// ```
///
/// ## Explanation:
/// - The first term `(spot - perp) * amount` represents the value realized when unwinding the hedge.
/// - The second term is the prorated entry cost for the portion being closed.
/// - The third term subtracts trading fees.
///
/// This method assumes a 1:1 hedge ratio between spot and perp legs, and that fees are applied externally
/// (e.g., perp trading fees) and passed in as a flat value.
///
/// # Parameters:
/// - `spot_exit_price`: Spot price at time of unwind
/// - `perp_exit_price`: Perp price at time of unwind
/// - `decrease_amount`: Size of position being closed
/// - `total_entry_value`: Total entry value across the entire position
/// - `total_position_size`: Total position size at time of decrease
/// - `fee_amount`: Total fees paid for this unwind (flat, in quote asset)
/// - `net_funding_accrued`: Total funding accrued across the entire position
/// - `net_borrow_accrued`: Total borrow accrued across the entire position
///
/// # Returns:
/// - Realized PnL as `SignedDecimal`
///
/// # Errors:
/// - Returns error if decrease amount or position size is zero to avoid divide-by-zero
///
#[allow(clippy::too_many_arguments)]
pub fn compute_realized_pnl(
    spot_exit_price: Decimal,
    perp_exit_price: Decimal,
    decrease_amount: Uint128,
    total_entry_value: Int128,
    total_position_size: Uint128,
    perp_trading_fee_amount: Int128,
    net_funding_accrued: Int128,
    net_borrow_accrued: Int128,
) -> ContractResult<Int128> {
    if decrease_amount.is_zero() || total_position_size.is_zero() {
        return Err(ContractError::InvalidDecreaseOrPositionSize {});
    }

    let total_position_size_int = Int128::try_from(total_position_size)?;
    let decrease_amount_int = Int128::try_from(decrease_amount)?;

    // Calculate exit value
    let spot_exit_value = decrease_amount.checked_mul_floor(spot_exit_price)?;
    let perp_exit_value = decrease_amount.checked_mul_floor(perp_exit_price)?;

    let exit_value =
        Int128::try_from(spot_exit_value)?.checked_sub(Int128::try_from(perp_exit_value)?)?;

    // Calculate entry value portion // TODO - can we do this more accurately?
    let entry_value_position_slice =
        total_entry_value.checked_mul(decrease_amount_int)?.checked_div(total_position_size_int)?;

    // Raw PnL from price difference
    let raw_pnl = exit_value.checked_sub(entry_value_position_slice)?;

    // Calculate proportional funding and borrow
    let realized_funding =
        net_funding_accrued.checked_multiply_ratio(decrease_amount_int, total_position_size_int)?;
    let realized_borrow =
        net_borrow_accrued.checked_multiply_ratio(decrease_amount_int, total_position_size_int)?;

    // Realized borrow will always be negative
    let net_yield = realized_funding.checked_sub(realized_borrow)?;

    // Final PnL calculation
    let final_pnl = raw_pnl.checked_add(net_yield)?.checked_sub(perp_trading_fee_amount)?;

    Ok(final_pnl)
}
