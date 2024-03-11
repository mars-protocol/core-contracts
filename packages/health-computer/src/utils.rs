use cosmwasm_std::Decimal;
use mars_types::{health::HealthResult, math::SignedDecimal, params::PerpParams};

use crate::Direction;

pub fn calculate_remaining_oi_value(
    long_oi_amount: Decimal,
    short_oi_amount: Decimal,
    perp_oracle_price: Decimal,
    perp_params: &PerpParams,
    direction: &Direction,
) -> HealthResult<SignedDecimal> {
    let long_oi_value = long_oi_amount.checked_mul(perp_oracle_price)?;
    let short_oi_value = short_oi_amount.checked_mul(perp_oracle_price)?;
    let total_oi_value = long_oi_value.checked_add(short_oi_value)?;

    // Open interest limits
    let max_long_oi_value = Decimal::from_atomics(perp_params.max_long_oi_value, 0)?;
    let max_short_oi_value = Decimal::from_atomics(perp_params.max_short_oi_value, 0)?;
    let max_net_oi_value = Decimal::from_atomics(perp_params.max_net_oi_value, 0)?;

    // If we are already at the OI limits, we can't open a new position
    let net_oi_valid = total_oi_value < max_net_oi_value;

    if !net_oi_valid {
        return Ok(SignedDecimal::zero());
    }

    // If we have reached the OI limits for the direction, we can't open a new position
    let direction_oi_valid = match direction {
        Direction::Long => long_oi_value < max_long_oi_value,
        Direction::Short => short_oi_value < max_short_oi_value,
    };

    if !direction_oi_valid {
        return Ok(SignedDecimal::zero());
    }

    let q_max_value = match direction {
        Direction::Long => max_long_oi_value.checked_sub(long_oi_value)?,
        Direction::Short => max_short_oi_value.checked_sub(short_oi_value)?,
    };

    get_max_oi_change_amount(q_max_value, perp_oracle_price, direction)
}

fn get_max_oi_change_amount(
    max_allowable_q_value: Decimal,
    perp_oracle_price: Decimal,
    direction: &Direction,
) -> HealthResult<SignedDecimal> {
    let max_allowable_q_amount_abs = max_allowable_q_value.checked_div(perp_oracle_price)?;

    Ok(match *direction {
        Direction::Long => max_allowable_q_amount_abs.into(),
        Direction::Short => SignedDecimal {
            abs: max_allowable_q_amount_abs,
            negative: true,
        },
    })
}
