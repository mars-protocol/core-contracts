use cosmwasm_std::{Decimal, Int128, Uint128};
use mars_types::{health::HealthResult, params::PerpParams};

use crate::Direction;

pub fn calculate_remaining_oi_amount(
    long_oi_amount: Uint128,
    short_oi_amount: Uint128,
    perp_oracle_price: Decimal,
    perp_params: &PerpParams,
    direction: &Direction,
) -> HealthResult<Int128> {
    let long_oi_value = long_oi_amount.checked_mul_floor(perp_oracle_price)?;
    let short_oi_value = short_oi_amount.checked_mul_floor(perp_oracle_price)?;
    let net_oi_value = long_oi_value.abs_diff(short_oi_value);

    // If we've already exceeded the net OI limit, we can't open a new position
    if net_oi_value >= perp_params.max_net_oi_value {
        return Ok(Int128::zero());
    }

    // Check if the direction-specific OI limits are exceeded
    let (remaining_net_oi_value, q_max_direction_value) = match direction {
        Direction::Long if long_oi_value >= perp_params.max_long_oi_value => {
            return Ok(Int128::zero())
        }
        Direction::Long => {
            // Maximum value we can add to Long without exceeding the net OI limit:
            // max_net_oi_value = (long_oi_value + remaining_net_oi_value).abs_diff(short_oi_value)
            // (long_oi_value + remaining_net_oi_value) should be >= short_oi_value
            // max_net_oi_value = (long_oi_value + remaining_net_oi_value) - short_oi_value
            // remaining_net_oi_value = max_net_oi_value + short_oi_value - long_oi_value
            let remaining_net_oi_value = perp_params
                .max_net_oi_value
                .checked_add(short_oi_value)?
                .checked_sub(long_oi_value)?;

            // Maximum allowable value for Long OI
            let remaining_long_oi_value =
                perp_params.max_long_oi_value.checked_sub(long_oi_value)?;

            (remaining_net_oi_value, remaining_long_oi_value)
        }
        Direction::Short if short_oi_value >= perp_params.max_short_oi_value => {
            return Ok(Int128::zero())
        }
        Direction::Short => {
            // Maximum value we can add to Short without exceeding the net OI limit:
            // max_net_oi_value = long_oi_value.abs_diff(short_oi_value + remaining_net_oi_value)
            // (short_oi_value + remaining_net_oi_value) should be >= long_oi_value
            // max_net_oi_value = (short_oi_value + remaining_net_oi_value) - long_oi_value
            // remaining_net_oi_value = max_net_oi_value + long_oi_value - short_oi_value
            let remaining_net_oi_value = perp_params
                .max_net_oi_value
                .checked_add(long_oi_value)?
                .checked_sub(short_oi_value)?;

            // Maximum allowable value for Short OI
            let remaining_short_oi_value =
                perp_params.max_short_oi_value.checked_sub(short_oi_value)?;

            (remaining_net_oi_value, remaining_short_oi_value)
        }
    };

    // Take the minimum of the net OI limit and the direction-specific limit (long/short)
    let q_max_value = remaining_net_oi_value.min(q_max_direction_value);

    // Calculate the maximum allowable OI change
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

#[cfg(test)]
mod tests {
    use cosmwasm_std::Uint128;
    use test_case::test_case;

    use super::*;

    #[test_case(
        Uint128::from(100u128),
        Uint128::from(40u128),
        PerpParams {
            max_net_oi_value: Uint128::from(90u128),
            max_long_oi_value: Uint128::from(100u128),
            max_short_oi_value: Uint128::from(110u128),
            ..Default::default()
        },
        Direction::Long,
        Int128::zero();
        "long position - exceeded max long oi"
    )]
    #[test_case(
        Uint128::from(90u128),
        Uint128::from(40u128),
        PerpParams {
            max_net_oi_value: Uint128::from(50u128),
            max_long_oi_value: Uint128::from(100u128),
            max_short_oi_value: Uint128::from(110u128),
            ..Default::default()
        },
        Direction::Long,
        Int128::zero();
        "long position - exceeded max net oi"
    )]
    #[test_case(
        Uint128::from(20u128),
        Uint128::from(40u128),
        PerpParams {
            max_net_oi_value: Uint128::from(90u128),
            max_long_oi_value: Uint128::from(100u128),
            max_short_oi_value: Uint128::from(110u128),
            ..Default::default()
        },
        Direction::Long,
        Int128::from(80i128);
        "long position - remaining max long oi"
    )]
    #[test_case(
        Uint128::from(20u128),
        Uint128::from(10u128),
        PerpParams {
            max_net_oi_value: Uint128::from(50u128),
            max_long_oi_value: Uint128::from(100u128),
            max_short_oi_value: Uint128::from(110u128),
            ..Default::default()
        },
        Direction::Long,
        Int128::from(40i128);
        "long position - remaining max net oi if more longs than shorts"
    )]
    #[test_case(
        Uint128::from(10u128),
        Uint128::from(20u128),
        PerpParams {
            max_net_oi_value: Uint128::from(50u128),
            max_long_oi_value: Uint128::from(100u128),
            max_short_oi_value: Uint128::from(110u128),
            ..Default::default()
        },
        Direction::Long,
        Int128::from(60i128);
        "long position - remaining max net oi if more shorts than longs"
    )]
    #[test_case(
        Uint128::from(20u128),
        Uint128::from(110u128),
        PerpParams {
            max_net_oi_value: Uint128::from(90u128),
            max_long_oi_value: Uint128::from(100u128),
            max_short_oi_value: Uint128::from(110u128),
            ..Default::default()
        },
        Direction::Short,
        Int128::zero();
        "short position - exceeded max short oi"
    )]
    #[test_case(
        Uint128::from(90u128),
        Uint128::from(40u128),
        PerpParams {
            max_net_oi_value: Uint128::from(50u128),
            max_long_oi_value: Uint128::from(100u128),
            max_short_oi_value: Uint128::from(110u128),
            ..Default::default()
        },
        Direction::Short,
        Int128::zero();
        "short position - exceeded max net oi"
    )]
    #[test_case(
        Uint128::from(20u128),
        Uint128::from(40u128),
        PerpParams {
            max_net_oi_value: Uint128::from(90u128),
            max_long_oi_value: Uint128::from(100u128),
            max_short_oi_value: Uint128::from(110u128),
            ..Default::default()
        },
        Direction::Short,
        Int128::from(-70i128);
        "short position - remaining max short oi"
    )]
    #[test_case(
        Uint128::from(20u128),
        Uint128::from(10u128),
        PerpParams {
            max_net_oi_value: Uint128::from(50u128),
            max_long_oi_value: Uint128::from(100u128),
            max_short_oi_value: Uint128::from(110u128),
            ..Default::default()
        },
        Direction::Short,
        Int128::from(-60i128);
        "short position - remaining max net oi if more longs than shorts"
    )]
    #[test_case(
        Uint128::from(10u128),
        Uint128::from(20u128),
        PerpParams {
            max_net_oi_value: Uint128::from(50u128),
            max_long_oi_value: Uint128::from(100u128),
            max_short_oi_value: Uint128::from(110u128),
            ..Default::default()
        },
        Direction::Short,
        Int128::from(-40i128);
        "short position - remaining max net oi if more shorts than longs"
    )]
    fn calculate_remaining_oi(
        long_oi_amount: Uint128,
        short_oi_amount: Uint128,
        perp_params: PerpParams,
        direction: Direction,
        expected_remaining_oi_amt: Int128,
    ) {
        // For simplicity, we assume that the oracle price is 1
        let perp_oracle_price = Decimal::one();
        let amount = calculate_remaining_oi_amount(
            long_oi_amount,
            short_oi_amount,
            perp_oracle_price,
            &perp_params,
            &direction,
        )
        .unwrap();
        assert_eq!(amount, expected_remaining_oi_amt);
    }
}
