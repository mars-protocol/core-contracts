use cosmwasm_std::{Decimal, Int128, Uint128};
use mars_delta_neutral_position::helpers::{prorate_i128_by_amount, weighted_avg};
use proptest::prelude::*;

proptest! {
    #[test]
    fn weighted_avg_never_panics(
        old_amt in 0u128..1_000_000_000u128,
        new_amt in 0u128..1_000_000_000u128,
        old_price_int in 0u64..500_000,
        new_price_int in 0u64..500_000,
    ) {
        let old_price = Decimal::from_ratio(old_price_int, 1u64);
        let new_price = Decimal::from_ratio(new_price_int, 1u64);

        let _ = weighted_avg(old_price, Uint128::new(old_amt), new_price, Uint128::new(new_amt));
    }

    #[test]
    fn prorate_i128_does_not_crash(
        total in -10_000_000i128..10_000_000i128,
        slice in 0u128..10_000_000u128,
        total_size in 1u128..10_000_000u128, // skip zero to avoid div-by-zero
    ) {
        let _ = prorate_i128_by_amount(Int128::new(total), Uint128::new(slice), Uint128::new(total_size));
    }

    #[test]
    fn prorate_i128_full_slice_equals_total(
        total in -1_000_000i128..1_000_000i128,
        total_size in 1u128..1_000_000u128
    ) {
        let result = prorate_i128_by_amount(Int128::new(total), Uint128::new(total_size), Uint128::new(total_size)).unwrap();
        assert_eq!(result, Int128::new(total));
    }
}
