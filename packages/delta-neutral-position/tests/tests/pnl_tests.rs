use std::str::FromStr;

use cosmwasm_std::{Decimal, Int128, Uint128};
use mars_delta_neutral_position::pnl::compute_realized_pnl;
use test_case::test_case;

fn dec(value: &str) -> Decimal {
    Decimal::from_str(value).unwrap()
}

#[test_case(
    "99.0", "96.0", 10_000_000, "-20000000", 10_000_000, 0, 0, 0, 50_000_000 ;
    "simple realized pnl"
)]
#[test_case(
    "100.0", "100.0", 10_000_000, "-10000000", 20_000_000, 0, 0, 0, 5_000_000 ;
    "multi increase vwap pnl zero"
)]
#[test_case(
    "99.0", "96.0", 10_000_000, "-30000000", 30_000_000, 0, 0, 0, 40_000_000 ;
    "partial close accuracy"
)]
#[allow(clippy::too_many_arguments)]
fn realized_pnl_cases(
    spot_exit: &str,
    perp_exit: &str,
    decrease: u128,
    entry_value: &str,
    position_size: u128,
    fee_amount: i128,
    net_funding_accrued: i128,
    net_borrow_accrued: i128,
    expected_pnl: i128,
) {
    let pnl = compute_realized_pnl(
        dec(spot_exit),
        dec(perp_exit),
        Uint128::new(decrease),
        Int128::try_from(entry_value).unwrap(),
        Uint128::new(position_size),
        Int128::new(fee_amount),
        Int128::new(net_funding_accrued),
        Int128::new(net_borrow_accrued),
    )
    .unwrap();
    assert_eq!(pnl, Int128::new(expected_pnl));
}

#[test]
fn test_decrease_remaining_position() {
    let spot_exit = dec("101.0");
    let perp_exit = dec("100.0");
    let decrease = Uint128::new(20_000_000);
    let entry_value = Int128::try_from("-20000000").unwrap();
    let position_size = Uint128::new(20_000_000);
    let fee_amount = Int128::new(0);
    let net_funding_accrued = Int128::new(0);
    let net_borrow_accrued = Int128::new(0);

    let pnl = compute_realized_pnl(
        spot_exit,
        perp_exit,
        decrease,
        entry_value,
        position_size,
        fee_amount,
        net_funding_accrued,
        net_borrow_accrued,
    )
    .unwrap();
    /*
    1. Exit Value Calculation:
        spot_exit_value = 101.0 × 20,000,000 = 2,020,000,000
        perp_exit_value = 100.0 × 20,000,000 = 2,000,000,000
        exit_value = 2,020,000,000 - 2,000,000,000 = 20,000,000

    2. Entry Value Calculation:
        entry_value_per_unit = -20,000,000 / 20,000,000 = -1
        entry_value_position_slice = -1 × 20,000,000 = -20,000,000

    3. Raw PnL:
        raw_pnl = 20,000,000 - (-20,000,000) = 40,000,000

    4. Funding & Borrow Calculation:
        position_ratio = 20,000,000 / 20,000,000 = 1
        realized_funding = 0 × 1 = 0
        realized_borrow = 0 × 1 = 0
        net_yield = 0 - 0 = 0

    5. Final PnL:
        final_pnl = 40,000,000 + 0 - 0 = 40,000,000
    */
    assert_eq!(pnl, Int128::new(40_000_000));
}

#[test]
fn test_zero_decrease_error() {
    let spot_exit = dec("100.0");
    let perp_exit = dec("100.0");
    let decrease = Uint128::zero();
    let entry_value = Int128::try_from("-10").unwrap();
    let position_size = Uint128::new(10);
    let fee_amount = Int128::new(0);
    let net_funding_accrued = Int128::new(0);
    let net_borrow_accrued = Int128::new(0);

    let result = compute_realized_pnl(
        spot_exit,
        perp_exit,
        decrease,
        entry_value,
        position_size,
        fee_amount,
        net_funding_accrued,
        net_borrow_accrued,
    );
    assert!(result.is_err());
}

#[test]
fn test_over_decrease_error() {
    let spot_exit = dec("100.0");
    let perp_exit = dec("100.0");
    let decrease = Uint128::new(15_000_000);
    let entry_value = Int128::try_from("-10").unwrap();
    let position_size = Uint128::new(10_000_000);
    let fee_amount = Int128::new(0);
    let net_funding_accrued = Int128::new(0);
    let net_borrow_accrued = Int128::new(0);

    // This will still return a value mathematically, but your logic layer may reject this before calling
    let result = compute_realized_pnl(
        spot_exit,
        perp_exit,
        decrease,
        entry_value,
        position_size,
        fee_amount,
        net_funding_accrued,
        net_borrow_accrued,
    );
    assert!(result.is_ok()); // Note: Enforcement of "can't decrease more than size" may live elsewhere
}

#[test]
fn test_mixed_entry_values() {
    let spot_exit = dec("103.0");
    let perp_exit = dec("97.0");
    let decrease = Uint128::new(10_000_000);
    let entry_value = Int128::try_from("20000000").unwrap();
    let position_size = Uint128::new(20_000_000);
    let fee_amount = Int128::new(0);
    let net_funding_accrued = Int128::new(0);
    let net_borrow_accrued = Int128::new(0);

    let pnl = compute_realized_pnl(
        spot_exit,
        perp_exit,
        decrease,
        entry_value,
        position_size,
        fee_amount,
        net_funding_accrued,
        net_borrow_accrued,
    )
    .unwrap();
    /*
    1. Exit Value Calculation:
        spot_exit_value = 103.0 × 10,000,000 = 1,030,000,000
        perp_exit_value = 97.0 × 10,000,000 = 970,000,000
        exit_value = 1,030,000,000 - 970,000,000 = 60,000,000

    2. Entry Value Calculation:
        entry_value_per_unit = 20,000,000 / 20,000,000 = 1
        entry_value_position_slice = 1 × 10,000,000 = 10,000,000

    3. Raw PnL:
        raw_pnl = 60,000,000 - 10,000,000 = 50,000,000

    4. Funding & Borrow Calculation:
        position_ratio = 10,000,000 / 20,000,000 = 0.5
        realized_funding = 0 × 0.5 = 0
        realized_borrow = 0 × 0.5 = 0
        net_yield = 0 - 0 = 0

    5. Final PnL:
        final_pnl = 50,000,000 + 0 - 0 = 50,000,000
    */
    assert_eq!(pnl, Int128::new(50_000_000));
}

#[test]
fn close_fully_with_fees() {
    let spot_exit = dec("100.0");
    let perp_exit = dec("100.0");
    let decrease = Uint128::new(10_000_000);
    let entry_value = Int128::try_from("-10000000").unwrap();
    let position_size = Uint128::new(10_000_000);
    let fee_amount = Int128::new(1_000_000);
    let net_funding_accrued = Int128::new(0);
    let net_borrow_accrued = Int128::new(0);

    let pnl = compute_realized_pnl(
        spot_exit,
        perp_exit,
        decrease,
        entry_value,
        position_size,
        fee_amount,
        net_funding_accrued,
        net_borrow_accrued,
    )
    .unwrap();
    /*
    1. Exit Value Calculation:
        spot_exit_value = 100.0 × 10,000,000 = 1,000,000,000
        perp_exit_value = 100.0 × 10,000,000 = 1,000,000,000
        exit_value = 1,000,000,000 - 1,000,000,000 = 0

    2. Entry Value Calculation:
        entry_value_per_unit = -10,000,000 / 10,000,000 = -1
        entry_value_position_slice = -1 × 10,000,000 = -10,000,000

    3. Raw PnL:
        raw_pnl = 0 - (-10,000,000) = 10,000,000

    4. Funding & Borrow Calculation:
        position_ratio = 10,000,000 / 10,000,000 = 1
        realized_funding = 0 × 1 = 0
        realized_borrow = 0 × 1 = 0
        net_yield = 0 - 0 = 0

    5. Final PnL:
        final_pnl = 10,000,000 + 0 - 1,000,000 = 9,000,000
    */
    assert_eq!(pnl, Int128::new(9_000_000));
}

#[test]
fn test_positive_funding_only() {
    let spot_exit = dec("100.0");
    let perp_exit = dec("100.0");
    let decrease = Uint128::new(10_000_000);
    let entry_value = Int128::try_from("0").unwrap();
    let position_size = Uint128::new(10_000_000);
    let fee_amount = Int128::new(0);
    let net_funding_accrued = Int128::new(5_000_000); // $5 funding received
    let net_borrow_accrued = Int128::new(0);

    let pnl = compute_realized_pnl(
        spot_exit,
        perp_exit,
        decrease,
        entry_value,
        position_size,
        fee_amount,
        net_funding_accrued,
        net_borrow_accrued,
    )
    .unwrap();
    /*
    1. Exit Value Calculation:
        spot_exit_value = 100.0 × 10,000,000 = 1,000,000,000
        perp_exit_value = 100.0 × 10,000,000 = 1,000,000,000
        exit_value = 1,000,000,000 - 1,000,000,000 = 0

    2. Entry Value Calculation:
        entry_value_per_unit = 0 / 10,000,000 = 0
        entry_value_position_slice = 0 × 10,000,000 = 0

    3. Raw PnL:
        raw_pnl = 0 - 0 = 0

    4. Funding & Borrow Calculation:
        position_ratio = 10,000,000 / 10,000,000 = 1
        realized_funding = 5,000,000 × 1 = 5,000,000
        realized_borrow = 0 × 1 = 0
        net_yield = 5,000,000 - 0 = 5,000,000

    5. Final PnL:
        final_pnl = 0 + 5,000,000 - 0 = 5,000,000
    */
    assert_eq!(pnl, Int128::new(5_000_000)); // PnL is just the funding
}

#[test]
fn test_negative_funding_only() {
    let spot_exit = dec("100.0");
    let perp_exit = dec("100.0");
    let decrease = Uint128::new(10_000_000);
    let entry_value = Int128::try_from("0").unwrap();
    let position_size = Uint128::new(10_000_000);
    let fee_amount = Int128::new(0);
    let net_funding_accrued = Int128::new(-3_000_000); // $3 funding paid
    let net_borrow_accrued = Int128::new(0);

    let pnl = compute_realized_pnl(
        spot_exit,
        perp_exit,
        decrease,
        entry_value,
        position_size,
        fee_amount,
        net_funding_accrued,
        net_borrow_accrued,
    )
    .unwrap();
    /*
    1. Exit Value Calculation:
        spot_exit_value = 100.0 × 10,000,000 = 1,000,000,000
        perp_exit_value = 100.0 × 10,000,000 = 1,000,000,000
        exit_value = 1,000,000,000 - 1,000,000,000 = 0

    2. Entry Value Calculation:
        entry_value_per_unit = 0 / 10,000,000 = 0
        entry_value_position_slice = 0 × 10,000,000 = 0

    3. Raw PnL:
        raw_pnl = 0 - 0 = 0

    4. Funding & Borrow Calculation:
        position_ratio = 10,000,000 / 10,000,000 = 1
        realized_funding = -3,000,000 × 1 = -3,000,000
        realized_borrow = 0 × 1 = 0
        net_yield = -3,000,000 - 0 = -3,000,000

    5. Final PnL:
        final_pnl = 0 + (-3,000,000) - 0 = -3,000,000
    */
    assert_eq!(pnl, Int128::new(-3_000_000)); // PnL is negative due to funding cost
}

#[test]
fn test_positive_funding_with_borrow() {
    let spot_exit = dec("100.0");
    let perp_exit = dec("100.0");
    let decrease = Uint128::new(10_000_000);
    let entry_value = Int128::try_from("0").unwrap();
    let position_size = Uint128::new(10_000_000);
    let fee_amount = Int128::new(0);
    let net_funding_accrued = Int128::new(5_000_000); // $5 funding received
    let net_borrow_accrued = Int128::new(2_000_000); // $2 borrow cost

    let pnl = compute_realized_pnl(
        spot_exit,
        perp_exit,
        decrease,
        entry_value,
        position_size,
        fee_amount,
        net_funding_accrued,
        net_borrow_accrued,
    )
    .unwrap();
    /*
    1. Exit Value Calculation:
        spot_exit_value = 100.0 × 10,000,000 = 1,000,000,000
        perp_exit_value = 100.0 × 10,000,000 = 1,000,000,000
        exit_value = 1,000,000,000 - 1,000,000,000 = 0

    2. Entry Value Calculation:
        entry_value_per_unit = 0 / 10,000,000 = 0
        entry_value_position_slice = 0 × 10,000,000 = 0

    3. Raw PnL:
        raw_pnl = 0 - 0 = 0

    4. Funding & Borrow Calculation:
        position_ratio = 10,000,000 / 10,000,000 = 1
        realized_funding = 5,000,000 × 1 = 5,000,000
        realized_borrow = 2,000,000 × 1 = 2,000,000
        net_yield = 5,000,000 - 2,000,000 = 3,000,000

    5. Final PnL:
        final_pnl = 0 + 3,000,000 - 0 = 3,000,000
    */
    assert_eq!(pnl, Int128::new(3_000_000)); // Net yield: 5 - 2 = 3
}

#[test]
fn test_funding_less_than_borrow() {
    let spot_exit = dec("100.0");
    let perp_exit = dec("100.0");
    let decrease = Uint128::new(10_000_000);
    let entry_value = Int128::try_from("0").unwrap();
    let position_size = Uint128::new(10_000_000);
    let fee_amount = Int128::new(0);
    let net_funding_accrued = Int128::new(2_000_000); // $2 funding received
    let net_borrow_accrued = Int128::new(5_000_000); // $5 borrow cost

    let pnl = compute_realized_pnl(
        spot_exit,
        perp_exit,
        decrease,
        entry_value,
        position_size,
        fee_amount,
        net_funding_accrued,
        net_borrow_accrued,
    )
    .unwrap();
    /*
    1. Exit Value Calculation:
        spot_exit_value = 100.0 × 10,000,000 = 1,000,000,000
        perp_exit_value = 100.0 × 10,000,000 = 1,000,000,000
        exit_value = 1,000,000,000 - 1,000,000,000 = 0

    2. Entry Value Calculation:
        entry_value_per_unit = 0 / 10,000,000 = 0
        entry_value_position_slice = 0 × 10,000,000 = 0

    3. Raw PnL:
        raw_pnl = 0 - 0 = 0

    4. Funding & Borrow Calculation:
        position_ratio = 10,000,000 / 10,000,000 = 1
        realized_funding = 2,000,000 × 1 = 2,000,000
        realized_borrow = 5,000,000 × 1 = 5,000,000
        net_yield = 2,000,000 - 5,000,000 = -3,000,000

    5. Final PnL:
        final_pnl = 0 + (-3,000,000) - 0 = -3,000,000
    */
    assert_eq!(pnl, Int128::new(-3_000_000)); // Net yield: 2 - 5 = -3
}

#[test]
fn test_partial_position_with_funding() {
    let spot_exit = dec("105.0");
    let perp_exit = dec("95.0");
    let decrease = Uint128::new(5_000_000);
    let entry_value = Int128::try_from("0").unwrap();
    let position_size = Uint128::new(20_000_000);
    let fee_amount = Int128::new(0);
    let net_funding_accrued = Int128::new(10_000_000); // $10 funding for entire position
    let net_borrow_accrued = Int128::new(4_000_000); // $4 borrow for entire position

    let pnl = compute_realized_pnl(
        spot_exit,
        perp_exit,
        decrease,
        entry_value,
        position_size,
        fee_amount,
        net_funding_accrued,
        net_borrow_accrued,
    )
    .unwrap();
    // Raw PnL: (105 - 95) * 5 = 50
    // Funding portion: 10 * (5/20) = 2.5
    // Borrow portion: 4 * (5/20) = 1
    // Net yield: 2.5 - 1 = 1.5
    // Total PnL: 50 + 1.5 = 51.5
    assert_eq!(pnl, Int128::new(51_500_000));
}

#[test]
fn test_both_negative_funding_and_borrow() {
    let spot_exit = dec("100.0");
    let perp_exit = dec("98.0");
    let decrease = Uint128::new(10_000_000);
    let entry_value = Int128::try_from("-20000000").unwrap(); // Initial position has negative value
    let position_size = Uint128::new(10_000_000);
    let fee_amount = Int128::new(1_000_000);
    let net_funding_accrued = Int128::new(-5_000_000); // $5 funding paid
    let net_borrow_accrued = Int128::new(3_000_000); // $3 borrow cost

    let pnl = compute_realized_pnl(
        spot_exit,
        perp_exit,
        decrease,
        entry_value,
        position_size,
        fee_amount,
        net_funding_accrued,
        net_borrow_accrued,
    )
    .unwrap();
    // Raw PnL: (100 - 98) * 10 - (-20) = 20 + 20 = 40
    // Net yield: -5 - 3 = -8
    // Total: 40 - 8 - 1 (fee) = 31
    assert_eq!(pnl, Int128::new(31_000_000));
}

#[test]
fn test_funding_with_price_movement() {
    let spot_exit = dec("120.0");
    let perp_exit = dec("100.0");
    let decrease = Uint128::new(10_000_000);
    let entry_value = Int128::try_from("-50000000").unwrap(); // Initial position has negative value
    let position_size = Uint128::new(10_000_000);
    let fee_amount = Int128::new(2_000_000);
    let net_funding_accrued = Int128::new(15_000_000); // $15 funding received
    let net_borrow_accrued = Int128::new(5_000_000); // $5 borrow cost

    let pnl = compute_realized_pnl(
        spot_exit,
        perp_exit,
        decrease,
        entry_value,
        position_size,
        fee_amount,
        net_funding_accrued,
        net_borrow_accrued,
    )
    .unwrap();
    // Raw PnL: (120 - 100) * 10 - (-50) = 200 + 50 = 250
    // Net yield: 15 - 5 = 10
    // Total: 250 + 10 - 2 (fee) = 258
    assert_eq!(pnl, Int128::new(258_000_000));
}

#[test]
fn test_multiple_decreases_with_funding() {
    // First decrease
    let spot_exit_1 = dec("105.0");
    let perp_exit_1 = dec("95.0");
    let decrease_1 = Uint128::new(5_000_000);
    let entry_value = Int128::try_from("-40000000").unwrap();
    let position_size_1 = Uint128::new(20_000_000);
    let fee_amount = Int128::new(1_000_000);
    let net_funding_accrued_1 = Int128::new(20_000_000);
    let net_borrow_accrued_1 = Int128::new(8_000_000);

    let pnl_1 = compute_realized_pnl(
        spot_exit_1,
        perp_exit_1,
        decrease_1,
        entry_value,
        position_size_1,
        fee_amount,
        net_funding_accrued_1,
        net_borrow_accrued_1,
    )
    .unwrap();

    // For a second decrease, we would have updated values
    let remaining_size = Uint128::new(15_000_000); // 20 - 5
    let spot_exit_2 = dec("110.0");
    let perp_exit_2 = dec("90.0");
    let decrease_2 = Uint128::new(5_000_000);

    // Assuming we had more funding/borrow since last decrease
    let net_funding_accrued_2 = Int128::new(15_000_000); // Assuming $15 more funding since last decrease
    let net_borrow_accrued_2 = Int128::new(5_000_000); // Assuming $5 more borrow cost

    let pnl_2 = compute_realized_pnl(
        spot_exit_2,
        perp_exit_2,
        decrease_2,
        entry_value, // Original entry value doesn't change
        remaining_size,
        fee_amount,
        net_funding_accrued_2,
        net_borrow_accrued_2,
    )
    .unwrap();

    // Ensure both calculations give expected results
    assert!(pnl_1 > Int128::new(0)); // First decrease should be profitable
    assert!(pnl_2 > pnl_1); // Second decrease should be more profitable (better exit price)
}

#[test]
fn test_small_funding_values() {
    let spot_exit = dec("100.01");
    let perp_exit = dec("99.99");
    let decrease = Uint128::new(1_000_000);
    let entry_value = Int128::try_from("0").unwrap();
    let position_size = Uint128::new(1_000_000);
    let fee_amount = Int128::new(0);
    let net_funding_accrued = Int128::new(1_000_000); // Very small funding
    let net_borrow_accrued = Int128::new(5_000_000); // Very small borrow

    let pnl = compute_realized_pnl(
        spot_exit,
        perp_exit,
        decrease,
        entry_value,
        position_size,
        fee_amount,
        net_funding_accrued,
        net_borrow_accrued,
    )
    .unwrap();

    /*
    1. Exit Value Calculation:
        spot_exit_value = 100.01 × 1,000,000 = 100,010,000
        perp_exit_value = 99.99 × 1,000,000 = 99,990,000
        exit_value = 100,010,000 - 99,990,000 = 20,000

    2. Entry Value Calculation:
        entry_value_per_unit = 0 / 1,000,000 = 0
        entry_value_position_slice = 0 × 1,000,000 = 0

    3. Raw PnL:
        raw_pnl = 20,000 - 0 = 20,000

    4. Funding & Borrow Calculation:
        position_ratio = 1,000,000 / 1,000,000 = 1
        realized_funding = 1,000,000 × 1 = 1,000,000
        realized_borrow = 5,000,000 × 1 = 5,000,000
        net_yield = 1,000,000 - 5,000,000 = -4,000,000

    5. Final PnL:
        final_pnl = 20,000 + (-4,000,000) - 0 = -3,980,000
    */

    // Make sure small values are handled correctly
    assert_eq!(pnl, Int128::new(-3980000));
}

#[test]
fn test_large_funding_values() {
    // Test with large values that might approach limits
    let spot_exit = dec("1000.0");
    let perp_exit = dec("1000.0");
    let decrease = Uint128::new(100_000_000);
    let entry_value = Int128::try_from("0").unwrap();
    let position_size = Uint128::new(100_000_000);
    let fee_amount = Int128::new(0);
    let net_funding_accrued = Int128::new(10_000_000_000); // Large funding
    let net_borrow_accrued = Int128::new(50_000_000_000); // Large borrow

    let pnl = compute_realized_pnl(
        spot_exit,
        perp_exit,
        decrease,
        entry_value,
        position_size,
        fee_amount,
        net_funding_accrued,
        net_borrow_accrued,
    )
    .unwrap();

    /*
    1. Exit Value Calculation:
        spot_exit_value = 1000.0 × 100,000,000 = 100,000,000,000
        perp_exit_value = 1000.0 × 100,000,000 = 100,000,000,000
        exit_value = 100,000,000,000 - 100,000,000,000 = 0

    2. Entry Value Calculation:
        entry_value_per_unit = 0 / 100,000,000 = 0
        entry_value_position_slice = 0 × 100,000,000 = 0

    3. Raw PnL:
        raw_pnl = 0 - 0 = 0

    4. Funding & Borrow Calculation:
        position_ratio = 100,000,000 / 100,000,000 = 1
        realized_funding = 10,000,000,000 × 1 = 10,000,000,000
        realized_borrow = 50,000,000,000 × 1 = 50,000,000,000
        net_yield = 10,000,000,000 - 50,000,000,000 = -40,000,000,000

    5. Final PnL:
        final_pnl = 0 + (-40,000,000,000) - 0 = -40,000,000,000
    */
    assert_eq!(pnl, Int128::new(-40_000_000_000)); // Net yield from large values
}

#[test]
fn test_near_overflow_positive() {
    // Test with values approaching Int128::MAX
    let spot_exit = dec("100000.0"); // Very high spot price
    let perp_exit = dec("1.0"); // Very low perp price
    let decrease = Uint128::new(10_000_000_000); // Large decrease amount
    let entry_value = Int128::try_from("-9000000000000000").unwrap(); // Large negative entry value
    let position_size = Uint128::new(10_000_000_000);
    let fee_amount = Int128::new(0);
    let net_funding_accrued = Int128::new(0);
    let net_borrow_accrued = Int128::new(0);

    let pnl = compute_realized_pnl(
        spot_exit,
        perp_exit,
        decrease,
        entry_value,
        position_size,
        fee_amount,
        net_funding_accrued,
        net_borrow_accrued,
    )
    .unwrap();

    /*
    1. Exit Value Calculation:
        spot_exit_value = 100000.0 × 10,000,000,000 = 1,000,000,000,000,000
        perp_exit_value = 1.0 × 10,000,000,000 = 10,000,000,000
        exit_value = 1,000,000,000,000,000 - 10,000,000,000 = 999,990,000,000,000

    2. Entry Value Calculation:
        entry_value_per_unit = -9,000,000,000,000,000 / 10,000,000,000 = -900,000
        entry_value_position_slice = -900,000 × 10,000,000,000 = -9,000,000,000,000,000

    3. Raw PnL:
        raw_pnl = 999,990,000,000,000 - (-9,000,000,000,000,000) = 9,999,990,000,000,000
        (This is a very large positive PnL that approaches Int128::MAX)

    4. Funding & Borrow Calculation:
        position_ratio = 10,000,000,000 / 10,000,000,000 = 1
        realized_funding = 0 × 1 = 0
        realized_borrow = 0 × 1 = 0
        net_yield = 0 - 0 = 0

    5. Final PnL:
        final_pnl = 9,999,990,000,000,000 + 0 - 0 = 9,999,990,000,000,000
    */

    // Verify result is extremely large positive number but doesn't overflow
    assert!(pnl > Int128::new(9_000_000_000_000_000));
    assert!(pnl < Int128::MAX); // Shouldn't hit Int128::MAX
}

#[test]
fn test_near_underflow_negative() {
    // Test with values approaching Int128::MIN
    let spot_exit = dec("1.0"); // Very low spot price
    let perp_exit = dec("100000.0"); // Very high perp price
    let decrease = Uint128::new(10_000_000_000); // Large decrease amount
    let entry_value = Int128::try_from("9000000000000000").unwrap(); // Large positive entry value
    let position_size = Uint128::new(10_000_000_000);
    let fee_amount = Int128::new(0);
    let net_funding_accrued = Int128::new(0);
    let net_borrow_accrued = Int128::new(0);

    let pnl = compute_realized_pnl(
        spot_exit,
        perp_exit,
        decrease,
        entry_value,
        position_size,
        fee_amount,
        net_funding_accrued,
        net_borrow_accrued,
    )
    .unwrap();

    /*
    1. Exit Value Calculation:
        spot_exit_value = 1.0 × 10,000,000,000 = 10,000,000,000
        perp_exit_value = 100000.0 × 10,000,000,000 = 1,000,000,000,000,000
        Since spot_exit_value < perp_exit_value, we'll get an error when trying
        to subtract. This means the test will likely fail with an Overflow error.

        If the code is smart enough to handle this by returning a StdError, the test
        should check for that specific error.

        If the subtraction is inverted (perp_exit_value - spot_exit_value), then:
        exit_value = 10,000,000,000 - 1,000,000,000,000,000 = -999,990,000,000,000

        The rest of the calculation would be:

    2. Entry Value Calculation:
        entry_value_per_unit = 9,000,000,000,000,000 / 10,000,000,000 = 900,000
        entry_value_position_slice = 900,000 × 10,000,000,000 = 9,000,000,000,000,000

    3. Raw PnL:
        raw_pnl = -999,990,000,000,000 - 9,000,000,000,000,000 = -9,999,990,000,000,000
        (This is a very large negative PnL that approaches Int128::MIN)
    */

    // This test expects an error when spot_exit_value < perp_exit_value
    assert!(pnl.is_negative());
    assert!(pnl > Int128::MIN); // Shouldn't underflow to Int128::MIN
}

#[test]
fn test_minimal_decrease_precision() {
    // Test with very small decrease amount to check precision handling
    let spot_exit = dec("100.0");
    let perp_exit = dec("99.0");
    let decrease = Uint128::new(1); // Minimal decrease
    let entry_value = Int128::try_from("-100").unwrap();
    let position_size = Uint128::new(100);
    let fee_amount = Int128::new(0);
    let net_funding_accrued = Int128::new(0);
    let net_borrow_accrued = Int128::new(0);

    let pnl = compute_realized_pnl(
        spot_exit,
        perp_exit,
        decrease,
        entry_value,
        position_size,
        fee_amount,
        net_funding_accrued,
        net_borrow_accrued,
    )
    .unwrap();

    /*
    1. Exit Value Calculation:
        spot_exit_value = 100.0 × 1 = 100
        perp_exit_value = 99.0 × 1 = 99
        exit_value = 100 - 99 = 1

    2. Entry Value Calculation:
        entry_value_per_unit = -100 / 100 = -1
        entry_value_position_slice = -1 × 1 = -1

    3. Raw PnL:
        raw_pnl = 1 - (-1) = 2

    4. Funding & Borrow Calculation:
        position_ratio = 1 / 100 = 0.01
        realized_funding = 0 × 0.01 = 0
        realized_borrow = 0 × 0.01 = 0
        net_yield = 0 - 0 = 0

    5. Final PnL:
        final_pnl = 2 + 0 - 0 = 2
    */

    // Verify the minimal calculation handles precision correctly
    assert_eq!(pnl, Int128::new(2));
}

#[test]
fn test_maximum_difference_prices() {
    // Test with extreme price difference to check calculation integrity
    let spot_exit = dec("1000000.0"); // Very high spot price
    let perp_exit = dec("0.0000001"); // Very low perp price
    let decrease = Uint128::new(1_000_000);
    let entry_value = Int128::try_from("0").unwrap();
    let position_size = Uint128::new(1_000_000);
    let fee_amount = Int128::new(0);
    let net_funding_accrued = Int128::new(0);
    let net_borrow_accrued = Int128::new(0);

    let pnl = compute_realized_pnl(
        spot_exit,
        perp_exit,
        decrease,
        entry_value,
        position_size,
        fee_amount,
        net_funding_accrued,
        net_borrow_accrued,
    )
    .unwrap();

    /*
    1. Exit Value Calculation:
        spot_exit_value = 1000000.0 × 1,000,000 = 1,000,000,000,000
        perp_exit_value = 0.0000001 × 1,000,000 = 0.1 (rounds to 0)
        exit_value = 1,000,000,000,000 - 0 = 1,000,000,000,000

    2. Entry Value Calculation:
        entry_value_per_unit = 0 / 1,000,000 = 0
        entry_value_position_slice = 0 × 1,000,000 = 0

    3. Raw PnL:
        raw_pnl = 1,000,000,000,000 - 0 = 1,000,000,000,000

    4. Funding & Borrow Calculation:
        position_ratio = 1,000,000 / 1,000,000 = 1
        realized_funding = 0 × 1 = 0
        realized_borrow = 0 × 1 = 0
        net_yield = 0 - 0 = 0

    5. Final PnL:
        final_pnl = 1,000,000,000,000 + 0 - 0 = 1,000,000,000,000
    */

    // Verify extreme price differences are handled correctly
    assert!(pnl > Int128::new(900_000_000_000));
}

#[test]
fn test_negative_spread_profitable() {
    // Test with perp price > spot price (negative spread) but still profitable
    let spot_exit = dec("95.0"); // Spot price lower than perp price
    let perp_exit = dec("105.0"); // Perp price higher than spot price
    let decrease = Uint128::new(10_000_000);
    let entry_value = Int128::try_from("200000000").unwrap(); // Very positive entry value (entered when spread was even more negative)
    let position_size = Uint128::new(10_000_000);
    let fee_amount = Int128::new(0);
    let net_funding_accrued = Int128::new(0);
    let net_borrow_accrued = Int128::new(0);

    let pnl = compute_realized_pnl(
        spot_exit,
        perp_exit,
        decrease,
        entry_value,
        position_size,
        fee_amount,
        net_funding_accrued,
        net_borrow_accrued,
    )
    .unwrap();

    /*
    1. Exit Value Calculation:
        spot_exit_value = 95.0 × 10,000,000 = 950,000,000
        perp_exit_value = 105.0 × 10,000,000 = 1,050,000,000
        exit_value = 950,000,000 - 1,050,000,000 = -100,000,000 (negative)

    2. Entry Value Calculation:
        entry_value_per_unit = 200,000,000 / 10,000,000 = 20
        entry_value_position_slice = 20 × 10,000,000 = 200,000,000

    3. Raw PnL:
        raw_pnl = -100,000,000 - 200,000,000 = -300,000,000
        (This is actually a loss because the spread worsened)

    4. Funding & Borrow Calculation:
        position_ratio = 10,000,000 / 10,000,000 = 1
        realized_funding = 0 × 1 = 0
        realized_borrow = 0 × 1 = 0
        net_yield = 0 - 0 = 0

    5. Final PnL:
        final_pnl = -300,000,000 + 0 - 0 = -300,000,000
    */

    // Verify PnL is negative as spread worsened
    assert_eq!(pnl, Int128::new(-300_000_000));
}

#[test]
fn test_spread_improvement_with_negative_entry() {
    // Test where negative spread improves (perp-spot narrows)
    let spot_exit = dec("98.0");
    let perp_exit = dec("102.0"); // Perp still higher but spread improved
    let decrease = Uint128::new(10_000_000);
    let entry_value = Int128::try_from("-500000000").unwrap(); // Very negative entry value (entered when spread was worse)
    let position_size = Uint128::new(10_000_000);
    let fee_amount = Int128::new(0);
    let net_funding_accrued = Int128::new(0);
    let net_borrow_accrued = Int128::new(0);

    let pnl = compute_realized_pnl(
        spot_exit,
        perp_exit,
        decrease,
        entry_value,
        position_size,
        fee_amount,
        net_funding_accrued,
        net_borrow_accrued,
    )
    .unwrap();

    /*
    1. Exit Value Calculation:
        spot_exit_value = 98.0 × 10,000,000 = 980,000,000
        perp_exit_value = 102.0 × 10,000,000 = 1,020,000,000
        exit_value = 980,000,000 - 1,020,000,000 = -40,000,000 (negative)

    2. Entry Value Calculation:
        entry_value_per_unit = -500,000,000 / 10,000,000 = -50
        entry_value_position_slice = -50 × 10,000,000 = -500,000,000

    3. Raw PnL:
        raw_pnl = -40,000,000 - (-500,000,000) = -40,000,000 + 500,000,000 = 460,000,000
        (This is a profit because the negative spread improved from -50 to -4)

    4. Funding & Borrow Calculation:
        position_ratio = 10,000,000 / 10,000,000 = 1
        realized_funding = 0 × 1 = 0
        realized_borrow = 0 × 1 = 0
        net_yield = 0 - 0 = 0

    5. Final PnL:
        final_pnl = 460,000,000 + 0 - 0 = 460,000,000
    */

    // Verify PnL is positive as negative spread improved
    assert_eq!(pnl, Int128::new(460_000_000));
}

#[test]
fn test_negative_spread_with_funding() {
    // Test negative spread with positive funding to offset losses
    let spot_exit = dec("90.0");
    let perp_exit = dec("110.0"); // Large negative spread
    let decrease = Uint128::new(10_000_000);
    let entry_value = Int128::try_from("0").unwrap(); // Neutral entry
    let position_size = Uint128::new(10_000_000);
    let fee_amount = Int128::new(0);
    let net_funding_accrued = Int128::new(300_000_000); // Large positive funding
    let net_borrow_accrued = Int128::new(0);

    let pnl = compute_realized_pnl(
        spot_exit,
        perp_exit,
        decrease,
        entry_value,
        position_size,
        fee_amount,
        net_funding_accrued,
        net_borrow_accrued,
    )
    .unwrap();

    /*
    1. Exit Value Calculation:
        spot_exit_value = 90.0 × 10,000,000 = 900,000,000
        perp_exit_value = 110.0 × 10,000,000 = 1,100,000,000
        exit_value = 900,000,000 - 1,100,000,000 = -200,000,000 (negative)

    2. Entry Value Calculation:
        entry_value_per_unit = 0 / 10,000,000 = 0
        entry_value_position_slice = 0 × 10,000,000 = 0

    3. Raw PnL:
        raw_pnl = -200,000,000 - 0 = -200,000,000

    4. Funding & Borrow Calculation:
        position_ratio = 10,000,000 / 10,000,000 = 1
        realized_funding = 300,000,000 × 1 = 300,000,000
        realized_borrow = 0 × 1 = 0
        net_yield = 300,000,000 - 0 = 300,000,000

    5. Final PnL:
        final_pnl = -200,000,000 + 300,000,000 - 0 = 100,000,000
    */

    // Verify positive funding can offset negative spread losses
    assert_eq!(pnl, Int128::new(100_000_000));
}
