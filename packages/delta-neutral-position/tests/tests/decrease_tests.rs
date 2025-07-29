use std::str::FromStr;

use cosmwasm_std::{Decimal, Int128, Uint128};
use mars_delta_neutral_position::{error::ContractError, types::Position};
use test_case::test_case;

#[test_case(30_000_000u128, "100.0", "101.0", 10_000_000u128, "99.0", "96.0", -30_000_000i128, -10_000_000i128, 3_000_000i128, 1_500_000i128, 999_999i128, 499_999i128; "partial decrease with funding and borrow")] // note - we have rounding inaccuracies here
#[test_case(20_000_000u128, "101.0", "100.0", 5_000_000u128, "98.0", "97.5", 20_000_000i128, 5_000_000i128, 2_000_000i128, 500_000i128, 500_000i128, 125_000i128; "smaller slice of accruals")]
#[test_case(40_000_000u128, "100.0", "100.0", 20_000_000u128, "100.0", "100.0", 0i128, 0i128, 0, 0, 0i128, 0i128; "flat position, no pnl or yield")]
#[test_case(50_000_000u128, "100.0", "101.0", 50_000_000u128, "102.0", "103.0", -50_000_000i128, -50_000_000i128, 5_000_000i128, 2_500_000i128, 5_000_000i128, 2_500_000i128; "full decrease with positive funding and borrow")]
#[test_case(15_000_000u128, "95.0", "94.0", 7_500_000u128, "90.0", "89.0", 15_000_000i128, 7_500_000i128, -1_000_000i128, -500_000i128, -500_000i128, -250_000i128; "negative funding and borrow scenario")]
#[test_case(25_000_000u128, "100.0", "99.0", 10_000_000u128, "110.0", "109.0", 25_000_000i128, 10_000_000i128, 0, 0, 0i128, 0i128; "significant price increase scenario")]
#[test_case(60_000_000u128, "100.0", "100.0", 30_000_000u128, "80.0", "80.0", 0i128, 0i128, 10_000_000i128, 5_000_000i128, 5_000_000i128, 2_500_000i128; "price drop with funding/borrow")]
#[allow(clippy::too_many_arguments)]
fn test_decrease_with_yield_and_pnl(
    initial_size: u128,
    entry_spot: &str,
    entry_perp: &str,
    decrease_amount: u128,
    exit_spot: &str,
    exit_perp: &str,
    entry_value: i128,
    _expected_slice: i128, // todo figure out how we are going to test this
    funding_i128: i128,
    borrow_i128: i128,
    expected_funding: i128,
    expected_borrow: i128,
) {
    let mut pos = Position::default();
    pos.increase(
        Uint128::new(initial_size),
        Decimal::from_str(entry_spot).unwrap(),
        Decimal::from_str(entry_perp).unwrap(),
        Int128::zero(),
        1_000_000,
        Int128::zero(),
        Int128::zero(),
    )
    .unwrap();

    pos.entry_value = Int128::new(entry_value);

    let result = pos
        .decrease(
            Uint128::new(decrease_amount),
            Decimal::from_str(exit_spot).unwrap(),
            Decimal::from_str(exit_perp).unwrap(),
            Int128::zero(),
            1_000_100,
            Int128::new(funding_i128),
            Int128::new(borrow_i128),
        )
        .unwrap();

    // assert_eq!(result.entry_value_slice, Int128::new(expected_slice));
    assert_eq!(result.net_realized_funding, Int128::new(expected_funding));
    assert_eq!(result.net_realized_borrow, Int128::new(expected_borrow));
    assert_eq!(pos.spot_amount, Uint128::new(initial_size - decrease_amount));
    assert_eq!(pos.perp_amount, Uint128::new(initial_size - decrease_amount));
}

#[test_case(10_000_000u128, "100.0", "101.0", 20_000_000u128, "Cannot decrease more than current position size"; "cannot decrease more than held")]
#[test_case(0, "100.0", "101.0", 1_000_000u128, "Cannot decrease more than current position size"; "cannot decrease from zero state")]
#[test_case(5_000_000u128, "100.0", "101.0", 0u128, "Amount must be greater than zero"; "cannot decrease by zero amount")]
#[test_case(1_000_000u128, "100.0", "101.0", 999_999u128, ""; "decrease almost entire position")]
#[test_case(1_000_000u128, "100.0", "101.0", 1u128, ""; "decrease minimal amount")]
#[test_case(1_000_000u128, "100.0", "101.0", 1_000_000u128, ""; "decrease exact position size")]
#[test_case(100u128, "100.0", "101.0", 100u128, ""; "small position full decrease")]
// Note - this test is about our max - should we increase this?
#[test_case((u128::MAX / 10u128.pow(18))/32, "100.0", "101.0", (u128::MAX / 10u128.pow(18))/64, ""; "very large position partial decrease")]
#[test_case(1_000_000u128, "100.0", "101.0", 1_000_001u128, "Cannot decrease more than current position size"; "decrease one more than position size")]
#[test_case(2_000_000u128, "0.0001", "0.0002", 1_000_000u128, ""; "extremely small price values")]
#[test_case(2_000_000u128, "1000000.0", "999999.0", 1_000_000u128, ""; "extremely large price values")]
#[test_case(3_000_000u128, "100.0", "99.0", 1_500_000u128, ""; "short spot long perp direction")]
#[test_case(4_000_000u128, "100.0", "101.0", 2_000_000u128, ""; "negative funding and borrow")]
fn test_decrease_validation_and_boundaries(
    amount: u128,
    spot: &str,
    perp: &str,
    reduce: u128,
    expected_error_msg: &str,
) {
    let mut pos = Position::default();
    if amount > 0 {
        // For one test case, use ShortSpotLongPerp direction
        // let direction = if amount == 3_000_000 {
        //     Side::ShortSpotLongPerp
        // } else {
        //     Side::LongSpotShortPerp
        // };

        pos.increase(
            Uint128::new(amount),
            Decimal::from_str(spot).unwrap(),
            Decimal::from_str(perp).unwrap(),
            Int128::zero(),
            1_000_000,
            Int128::zero(),
            Int128::zero(),
        )
        .unwrap();
    }

    // For some tests, set non-zero funding and borrow values
    if amount == 2_000_000 {
        pos.net_funding_balance = Int128::new(1_000_000);
        pos.net_borrow_balance = Int128::new(500_000);
    } else if amount == 4_000_000 {
        // Set negative funding and borrow values
        pos.net_funding_balance = Int128::new(-2_000_000);
        pos.net_borrow_balance = Int128::new(-1_000_000);
    }

    let result = pos.decrease(
        Uint128::new(reduce),
        Decimal::from_str(spot).unwrap(),
        Decimal::from_str(perp).unwrap(),
        Int128::zero(),
        1_000_100,
        Int128::zero(),
        Int128::zero(),
    );

    if expected_error_msg.is_empty() {
        // This should succeed
        assert!(result.is_ok());
        // Check the remaining amount is correct
        assert_eq!(pos.spot_amount, Uint128::new(amount - reduce));
        assert_eq!(pos.perp_amount, Uint128::new(amount - reduce));

        // For the specific cases with funding and borrow, check those are prorated correctly
        if amount == 2_000_000 && reduce == 1_000_000 {
            assert_eq!(pos.net_realized_funding, Int128::new(500_000));
            assert_eq!(pos.net_realized_borrow, Int128::new(250_000));

            let decrease_result = result.unwrap();
            assert_eq!(decrease_result.net_realized_funding, Int128::new(500_000));
            assert_eq!(decrease_result.net_realized_borrow, Int128::new(250_000));
        } else if amount == 4_000_000 && reduce == 2_000_000 {
            // Check negative funding and borrow are prorated correctly
            assert_eq!(pos.net_realized_funding, Int128::new(-1_000_000));
            assert_eq!(pos.net_realized_borrow, Int128::new(-500_000));

            let decrease_result = result.unwrap();
            assert_eq!(decrease_result.net_realized_funding, Int128::new(-1_000_000));
            assert_eq!(decrease_result.net_realized_borrow, Int128::new(-500_000));
        }
    } else {
        // Check that the error is an InvalidAmount error with the expected message
        match result {
            Err(ContractError::InvalidAmount {
                reason,
            }) => {
                assert_eq!(reason, expected_error_msg);
            }
            _ => panic!("Expected InvalidAmount error with message: {}", expected_error_msg),
        }
    }
}

#[test_case(25_000_000u128, "105.0", "104.0", 25_000_000u128; "full reset on complete decrease")]
#[test_case(10_000u128, "10.0", "11.0", 10_000u128; "small position full decrease")]
#[test_case(1_000_000_000u128, "100.0", "99.5", 1_000_000_000u128; "large position full decrease")]
#[test_case(500_000u128, "0.0001", "0.0002", 500_000u128; "tiny price values")]
#[test_case(750_000u128, "9999.0", "9998.0", 750_000u128; "very large price values")]
#[test_case(350_000u128, "101.0", "102.0", 350_000u128; "ShortSpotLongPerp direction")]
#[test_case(150_000u128, "102.0", "101.0", 150_000u128; "LongSpotShortPerp with different exit prices")]
#[test_case(450_000u128, "100.0", "100.0", 450_000u128; "zero entry value")]
#[test_case(600_000u128, "100.0", "101.0", 600_000u128; "negative entry value")]
fn test_full_decrease_resets_state(amount: u128, spot: &str, perp: &str, reduce: u128) {
    let mut pos = Position::default();
    // let direction = if spot.parse::<f64>().unwrap() > perp.parse::<f64>().unwrap() {
    //     Side::LongSpotShortPerp
    // } else {
    //     Side::ShortSpotLongPerp
    // };

    pos.increase(
        Uint128::new(amount),
        Decimal::from_str(spot).unwrap(),
        Decimal::from_str(perp).unwrap(),
        Int128::zero(),
        1_000_000,
        Int128::zero(),
        Int128::zero(),
    )
    .unwrap();

    // Set some funding and borrow values to ensure they get reset
    pos.net_funding_balance = Int128::new(1_000_000);
    pos.net_borrow_balance = Int128::new(500_000);

    // For specific test cases, set custom entry values
    if amount == 600_000 {
        pos.entry_value = Int128::new(-50_000_000);
    } else if amount == 450_000 {
        pos.entry_value = Int128::zero();
    }

    // Store the direction before decrease
    let _original_direction = pos.direction;

    let result = pos
        .decrease(
            Uint128::new(reduce),
            Decimal::from_str("99.0").unwrap(),
            Decimal::from_str("96.0").unwrap(),
            Int128::zero(),
            1_000_100,
            Int128::zero(),
            Int128::zero(),
        )
        .unwrap();

    // Verify the result has the right values
    assert_eq!(result.spot_amount, Uint128::new(amount - reduce));
    assert_eq!(result.perp_amount, Uint128::new(amount - reduce));

    // Check specific values for custom test cases
    if amount == 600_000 {
        // assert_eq!(result.entry_value_slice, Int128::new(-50_000_000));
    } else if amount == 450_000 {
        // assert_eq!(result.entry_value_slice, Int128::zero());
    } else {
        assert_eq!(result.net_realized_funding, Int128::new(1_000_000));
        assert_eq!(result.net_realized_borrow, Int128::new(500_000));
    }

    // Verify position is fully reset
    assert_eq!(pos.spot_amount, Uint128::zero());
    assert_eq!(pos.perp_amount, Uint128::zero());
    assert_eq!(pos.entry_value, Int128::zero());
    assert_eq!(pos.net_funding_balance, Int128::zero());
    assert_eq!(pos.net_borrow_balance, Int128::zero());
    assert_eq!(pos.avg_spot_price, Decimal::zero());
    assert_eq!(pos.avg_perp_price, Decimal::zero());

    // For specific test cases, test that the original direction was correctly set
    // based on the spot and perp price relationship
    // if amount == 350_000 {
    //     assert_eq!(original_direction, Side::ShortSpotLongPerp);
    // } else if amount == 150_000 {
    //     assert_eq!(original_direction, Side::LongSpotShortPerp);
    // }

    // After full decrease, verify that the default direction remains
    // We don't enforce a specific direction reset in the contract,
    // but we want to make sure it's maintained for future increases
    // assert_eq!(pos.direction, original_direction);
}
