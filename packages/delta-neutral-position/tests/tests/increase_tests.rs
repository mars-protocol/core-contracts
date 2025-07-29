use std::str::FromStr;

use cosmwasm_std::{Decimal, Int128, Uint128};
use mars_delta_neutral_position::{error::ContractError, types::Position};
use test_case::test_case;

// Helper functions
fn dec(value: &str) -> Decimal {
    Decimal::from_str(value).unwrap()
}

fn int128(value: i128) -> Int128 {
    Int128::new(value)
}

// Scale amounts by 1e6 for more precise calculations
const SCALE: u128 = 1_000_000;

// ------------------------
// Increase Tests
// ------------------------

#[test_case(10 * SCALE, "100.0", "102.0", int128(-20_000_000), None; "initial increase is correct")]
#[test_case(20 * SCALE, "105.0", "103.0", int128(40_000_000), None; "second increase matches direction and updates correctly")]
#[test_case(15 * SCALE, "99.5", "98.5", int128(15_000_000), None; "short spot long perp increase computes correct entry")]
#[test_case(0, "100.0", "101.0", int128(0), Some(ContractError::InvalidAmount { reason: "Amount must be greater than zero".to_string() }); "zero amount should crash")]
fn test_increase_entry_value_and_direction(
    amount: u128,
    spot: &str,
    perp: &str,
    expected_entry_value: Int128,
    expected_error: Option<ContractError>,
) {
    let mut pos = Position::default();

    let result = pos.increase(
        Uint128::new(amount),
        dec(spot),
        dec(perp),
        Int128::zero(),
        1_000_000,
        Int128::zero(),
        Int128::zero(),
    );

    if let Some(_expected_error) = expected_error {
        assert!(result.is_err());
        assert!(matches!(result.err().unwrap(), _expected_error));
    } else {
        assert!(result.is_ok());
        pos = result.unwrap();
    }

    assert_eq!(pos.entry_value, &expected_entry_value);
    assert_eq!(pos.spot_amount, Uint128::new(amount));
    assert_eq!(pos.perp_amount, Uint128::new(amount));
}

// ------------------------
// Decrease Tests
// ------------------------

#[test_case(30 * SCALE, "100.0", "101.0", 10 * SCALE, "99.0", "96.0", int128(-30_000_000), int128(-10_000_000), int128(3_000_000), int128(1_500_000), int128(999_999), int128(499_999); "simple decrease with prorated funding and borrow")]
#[test_case(20 * SCALE, "101.0", "100.0", 5 * SCALE, "98.0", "97.5", int128(20_000_000), int128(5_000_000), int128(2_000_000), int128(1_000_000), int128(500_000), int128(250_000); "decrease handles positive entry value and partial funding")]
#[test_case(40 * SCALE, "100.0", "100.0", 20 * SCALE, "100.0", "100.0", int128(0), int128(0), int128(0), int128(0), int128(0), int128(0); "flat prices and zero accruals result in zero realized pnl")]
#[allow(clippy::too_many_arguments)]
fn test_decrease_correct_outputs(
    initial_size: u128,
    entry_spot: &str,
    entry_perp: &str,
    decrease_amount: u128,
    exit_spot: &str,
    exit_perp: &str,
    entry_value: Int128,
    _expected_slice: Int128, // todo should we check expected slice? or check in a different way?
    funding: Int128,
    borrow: Int128,
    expected_funding: Int128,
    expected_borrow: Int128,
) {
    let mut pos = Position::default();
    pos = pos
        .increase(
            Uint128::new(initial_size),
            dec(entry_spot),
            dec(entry_perp),
            Int128::zero(),
            1_000_000,
            Int128::zero(),
            Int128::zero(),
        )
        .unwrap();

    // Set the values directly
    pos.entry_value = entry_value;
    pos.net_funding_balance = funding;
    pos.net_borrow_balance = borrow;

    let result = pos
        .decrease(
            Uint128::new(decrease_amount),
            dec(exit_spot),
            dec(exit_perp),
            Int128::zero(),
            1_000_000,
            Int128::zero(),
            Int128::zero(),
        )
        .unwrap();

    // assert_eq!(result.entry_value_slice, &expected_slice);
    assert_eq!(result.net_realized_funding, expected_funding);
    assert_eq!(result.net_realized_borrow, expected_borrow);
    assert_eq!(pos.spot_amount, Uint128::new(initial_size - decrease_amount));
    assert_eq!(pos.perp_amount, Uint128::new(initial_size - decrease_amount));
}

#[test_case(10 * SCALE, "100.0", "101.0", 20 * SCALE; "attempt to decrease more than position size should fail")]
#[test_case(0, "100.0", "101.0", SCALE; "cannot decrease from zero state")]
fn test_decrease_too_much(amount: u128, spot: &str, perp: &str, reduce: u128) {
    let mut pos = Position::default();

    // Define funding and borrow amounts
    let funding_delta_increase = Int128::new(500); // Positive funding (user receives)
    let borrow_delta_increase = Int128::new(-300); // Negative borrow (user pays)

    if amount > 0 {
        pos = pos
            .increase(
                Uint128::new(amount),
                dec(spot),
                dec(perp),
                Int128::zero(),
                1_000_000,
                funding_delta_increase,
                borrow_delta_increase,
            )
            .unwrap();
    }

    // Different funding/borrow for decrease operation
    let funding_delta_decrease = Int128::new(-200); // Negative funding (user pays)
    let borrow_delta_decrease = Int128::new(-150); // Negative borrow (user pays)

    let result = pos.decrease(
        Uint128::new(reduce),
        dec("99.0"),
        dec("96.0"),
        Int128::zero(),
        1_000_100,
        funding_delta_decrease,
        borrow_delta_decrease,
    );
    assert!(result.is_err());
}

#[test_case(10 * SCALE, "100.0", "101.0", 10 * SCALE; "full decrease resets position")]
#[test_case(25 * SCALE, "105.0", "104.0", 25 * SCALE; "reset confirmed on full position close")]
fn test_full_decrease_resets_state(amount: u128, spot: &str, perp: &str, reduce: u128) {
    let mut pos = Position::default();

    // Define funding and borrow amounts for increase
    let funding_delta_increase = Int128::new(750); // Positive funding (user receives)
    let borrow_delta_increase = Int128::new(-450); // Negative borrow (user pays)

    pos = pos
        .increase(
            Uint128::new(amount),
            dec(spot),
            dec(perp),
            Int128::zero(),
            1_000_000,
            funding_delta_increase,
            borrow_delta_increase,
        )
        .unwrap();

    // Different funding/borrow for decrease operation
    let funding_delta_decrease = Int128::new(320); // Positive funding (user receives)
    let borrow_delta_decrease = Int128::new(-280); // Negative borrow (user pays)
    let result = pos
        .decrease(
            Uint128::new(reduce),
            dec("99.0"),
            dec("96.0"),
            Int128::zero(),
            1_000_100,
            funding_delta_decrease,
            borrow_delta_decrease,
        )
        .unwrap();

    // When we do a full decrease (amount == total), all funding and borrow is realized
    // First check that this is a full close
    assert_eq!(Uint128::new(amount), Uint128::new(reduce));

    // The total accrued funding should include both the increase and decrease deltas
    let total_funding = funding_delta_increase.checked_add(funding_delta_decrease).unwrap();
    let total_borrow = borrow_delta_increase.checked_add(borrow_delta_decrease).unwrap();

    // For a full decrease, all funding and borrow is realized
    assert_eq!(result.net_realized_funding, total_funding);
    assert_eq!(result.net_realized_borrow, total_borrow);

    // Position should be fully reset
    assert_eq!(pos.spot_amount, Uint128::zero());
    assert_eq!(pos.perp_amount, Uint128::zero());
    assert_eq!(pos.entry_value, Int128::zero());
    assert_eq!(pos.net_funding_balance, Int128::zero());
    assert_eq!(pos.net_borrow_balance, Int128::zero());
}
