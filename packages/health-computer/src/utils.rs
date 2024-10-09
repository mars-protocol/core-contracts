use cosmwasm_std::{Decimal, Int128, Uint128};
use mars_types::{health::HealthResult, params::PerpParams};

use crate::Direction;

pub fn calculate_remaining_oi_value(
    long_oi_amount: Uint128,
    short_oi_amount: Uint128,
    perp_oracle_price: Decimal,
    perp_params: &PerpParams,
    direction: &Direction,
) -> HealthResult<Int128> {
    let long_oi_value = long_oi_amount.checked_mul_floor(perp_oracle_price)?;
    let short_oi_value = short_oi_amount.checked_mul_floor(perp_oracle_price)?;
    let net_oi_value = long_oi_value.abs_diff(short_oi_value);

    // If we are already at the OI limits, we can't open a new position
    let net_oi_valid = net_oi_value < perp_params.max_net_oi_value;
    if !net_oi_valid {
        return Ok(Int128::zero());
    }

    // If we have reached the OI limits for the direction, we can't open a new position
    let direction_oi_valid = match direction {
        Direction::Long => long_oi_value < perp_params.max_long_oi_value,
        Direction::Short => short_oi_value < perp_params.max_short_oi_value,
    };

    if !direction_oi_valid {
        return Ok(Int128::zero());
    }

    let q_max_value = match direction {
        Direction::Long => perp_params.max_long_oi_value.checked_sub(long_oi_value)?,
        Direction::Short => perp_params.max_short_oi_value.checked_sub(short_oi_value)?,
    };

    get_max_oi_change_amount(q_max_value, perp_oracle_price, direction)
}

fn get_max_oi_change_amount(
    max_allowable_q_value: Uint128,
    perp_oracle_price: Decimal,
    direction: &Direction,
) -> HealthResult<Int128> {
    let max_allowable_q_amount_abs = max_allowable_q_value.checked_div_floor(perp_oracle_price)?;

    Ok(match direction {
        Direction::Long => max_allowable_q_amount_abs.try_into()?,
        Direction::Short => Int128::zero().checked_sub(max_allowable_q_amount_abs.try_into()?)?,
    })
}
