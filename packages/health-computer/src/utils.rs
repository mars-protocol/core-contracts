use cosmwasm_std::{Decimal, Uint128};
use mars_types::{health::HealthResult, params::PerpParams};

use crate::Direction;

pub fn calculate_remaining_oi_amount(
    long_oi_amount: Uint128,
    short_oi_amount: Uint128,
    perp_oracle_price: Decimal,
    perp_params: &PerpParams,
    direction: &Direction,
) -> HealthResult<Uint128> {
    let long_oi_value = long_oi_amount.checked_mul_floor(perp_oracle_price)?;
    let short_oi_value = short_oi_amount.checked_mul_floor(perp_oracle_price)?;
    let net_oi_value = long_oi_value.abs_diff(short_oi_value);

    let increasing_net_oi = match direction {
        // Determine if the action is increasing the net OI.
        // - If we are opening or increasing a Long position, the net OI increases when Long OI > Short OI.
        // - If we are opening or increasing a Short position, the net OI increases when Short OI > Long OI.
        Direction::Long => long_oi_value > short_oi_value,
        Direction::Short => short_oi_value > long_oi_value,
    };

    // If the action is increasing the net OI AND the net OI limit has been reached or exceeded,
    // we cannot increase OI further. In this case, we return `Uint128::zero()` to signal that no
    // additional Open Interest (OI) can be added.
    //
    // However, if the action is reducing net OI (i.e., going in the opposite direction of the skew),
    // we can still estimate the remaining OI even if the net OI limit is exceeded.
    // For example:
    // - If Long OI > Short OI and we are adding a Short position, this reduces net OI and is allowed.
    if net_oi_value >= perp_params.max_net_oi_value && increasing_net_oi {
        return Ok(Uint128::zero());
    }

    // Check if the direction-specific OI limits are exceeded
    let (remaining_net_oi_value, q_max_direction_value) = match direction {
        Direction::Long if long_oi_value >= perp_params.max_long_oi_value => {
            return Ok(Uint128::zero())
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
            return Ok(Uint128::zero())
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
    let q_max_amount = q_max_value.checked_div_floor(perp_oracle_price)?;
    Ok(q_max_amount)
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
        Uint128::zero();
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
        Uint128::zero();
        "long position - exceeded max net oi can't increase oi"
    )]
    #[test_case(
        Uint128::from(40u128),
        Uint128::from(90u128),
        PerpParams {
            max_net_oi_value: Uint128::from(50u128),
            max_long_oi_value: Uint128::from(250u128),
            max_short_oi_value: Uint128::from(260u128),
            ..Default::default()
        },
        Direction::Long,
        Uint128::from(100u128);
        "long position - exceeded max net oi can decrease oi"
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
        Uint128::from(80u128);
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
        Uint128::from(40u128);
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
        Uint128::from(60u128);
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
        Uint128::zero();
        "short position - exceeded max short oi"
    )]
    #[test_case(
        Uint128::from(40u128),
        Uint128::from(90u128),
        PerpParams {
            max_net_oi_value: Uint128::from(50u128),
            max_long_oi_value: Uint128::from(100u128),
            max_short_oi_value: Uint128::from(110u128),
            ..Default::default()
        },
        Direction::Short,
        Uint128::zero();
        "short position - exceeded max net oi can't increase oi"
    )]
    #[test_case(
        Uint128::from(90u128),
        Uint128::from(40u128),
        PerpParams {
            max_net_oi_value: Uint128::from(50u128),
            max_long_oi_value: Uint128::from(250u128),
            max_short_oi_value: Uint128::from(260u128),
            ..Default::default()
        },
        Direction::Short,
        Uint128::from(100u128);
        "short position - exceeded max net oi can decrease oi"
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
        Uint128::from(70u128);
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
        Uint128::from(60u128);
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
        Uint128::from(40u128);
        "short position - remaining max net oi if more shorts than longs"
    )]
    fn calculate_remaining_oi(
        long_oi_amount: Uint128,
        short_oi_amount: Uint128,
        perp_params: PerpParams,
        direction: Direction,
        expected_remaining_oi_amt: Uint128,
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
