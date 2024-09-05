use cosmwasm_std::{Decimal, Uint128};
use mars_types::{health::HealthResult, params::PerpParams, signed_uint::SignedUint};

use crate::Direction;

pub fn calculate_remaining_oi_value(
    long_oi_amount: Uint128,
    short_oi_amount: Uint128,
    perp_oracle_price: Decimal,
    perp_params: &PerpParams,
    direction: &Direction,
) -> HealthResult<SignedUint> {
    let long_oi_value = long_oi_amount.checked_mul_floor(perp_oracle_price)?;
    let short_oi_value = short_oi_amount.checked_mul_floor(perp_oracle_price)?;
    let total_oi_value = long_oi_value.checked_add(short_oi_value)?;

    // If we are already at the OI limits, we can't open a new position
    let net_oi_valid = total_oi_value < perp_params.max_net_oi_value;
    if !net_oi_valid {
        return Ok(SignedUint::zero());
    }

    // If we have reached the OI limits for the direction, we can't open a new position
    let direction_oi_valid = match direction {
        Direction::Long => long_oi_value < perp_params.max_long_oi_value,
        Direction::Short => short_oi_value < perp_params.max_short_oi_value,
    };

    if !direction_oi_valid {
        return Ok(SignedUint::zero());
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
) -> HealthResult<SignedUint> {
    let max_allowable_q_amount_abs = max_allowable_q_value.checked_div_floor(perp_oracle_price)?;

    Ok(match direction {
        Direction::Long => max_allowable_q_amount_abs.into(),
        Direction::Short => SignedUint {
            abs: max_allowable_q_amount_abs,
            negative: true,
        },
    })
}
