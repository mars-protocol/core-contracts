use std::str::FromStr;

use cosmwasm_std::{coin, Addr, Decimal, Int128, Uint128};
use mars_types::{
    params::{PerpParams, PerpParamsUpdate},
    perps::Accounting,
};

use super::helpers::MockEnv;
use crate::tests::helpers::default_perp_params;

#[test]
fn accounting_with_discount_fees() {
    let protocol_fee_rate = Decimal::percent(2);
    let mut mock = MockEnv::new().protocol_fee_rate(protocol_fee_rate).build().unwrap();

    // Set up dao staking after building
    let dao_staking_addr = Addr::unchecked("mock-dao-staking");
    mock.set_dao_staking_address(&dao_staking_addr);

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let user = "jake";

    // Fund credit manager and set up prices
    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000_000u128, &["uosmo", "uatom", "uusdc"]);
    mock.set_price(&owner, "uusdc", Decimal::from_str("0.9").unwrap()).unwrap();
    mock.set_price(&owner, "uosmo", Decimal::from_str("1.25").unwrap()).unwrap();
    mock.set_price(&owner, "uatom", Decimal::from_str("10.5").unwrap()).unwrap();

    // Deposit USDC to vault
    mock.deposit_to_vault(
        &credit_manager,
        Some(user),
        None,
        &[coin(1_000_000_000_000u128, "uusdc")],
    )
    .unwrap();

    // Set up perp markets with different fee rates
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                closing_fee_rate: Decimal::percent(1),
                opening_fee_rate: Decimal::percent(2),
                max_funding_velocity: Decimal::from_str("32").unwrap(),
                ..default_perp_params("uosmo")
            },
        },
    );
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                closing_fee_rate: Decimal::percent(1),
                opening_fee_rate: Decimal::percent(2),
                max_funding_velocity: Decimal::from_str("30").unwrap(),
                ..default_perp_params("uatom")
            },
        },
    );

    // Check accounting in the beginning
    let osmo_accounting_before = mock.query_market_accounting("uosmo").accounting;
    let atom_accounting_before = mock.query_market_accounting("uatom").accounting;
    let total_accounting_before = mock.query_total_accounting().accounting;

    assert_eq!(osmo_accounting_before, Accounting::default());
    assert_eq!(atom_accounting_before, Accounting::default());
    assert_eq!(total_accounting_before, Accounting::default());

    // Test opening fees with and without discount
    let atom_size = Int128::from_str("1000000").unwrap();

    // Query opening fee without discount
    let atom_opening_fee_no_discount = mock.query_opening_fee("uatom", atom_size, None).fee;

    // Query opening fee with 50% discount
    let discount_pct = Decimal::percent(50);
    let atom_opening_fee_with_discount =
        mock.query_opening_fee("uatom", atom_size, Some(discount_pct)).fee;

    // Verify discount is applied correctly
    assert!(atom_opening_fee_with_discount.amount < atom_opening_fee_no_discount.amount);
    let expected_discount_amount =
        atom_opening_fee_no_discount.amount.checked_mul_ceil(discount_pct).unwrap();
    let expected_fee_with_discount =
        atom_opening_fee_no_discount.amount.checked_sub(expected_discount_amount).unwrap();

    // Allow for small rounding differences (within 1 unit)
    let difference = if expected_fee_with_discount > atom_opening_fee_with_discount.amount {
        expected_fee_with_discount.checked_sub(atom_opening_fee_with_discount.amount).unwrap()
    } else {
        atom_opening_fee_with_discount.amount.checked_sub(expected_fee_with_discount).unwrap()
    };
    assert!(
        difference <= Uint128::new(1),
        "Discount calculation difference too large: expected {}, got {}, difference {}",
        expected_fee_with_discount,
        atom_opening_fee_with_discount.amount,
        difference
    );

    // Test different discount percentages
    let discount_25 = Decimal::percent(25);
    let atom_opening_fee_25_discount =
        mock.query_opening_fee("uatom", atom_size, Some(discount_25)).fee;
    assert!(atom_opening_fee_25_discount.amount < atom_opening_fee_no_discount.amount);
    assert!(atom_opening_fee_25_discount.amount > atom_opening_fee_with_discount.amount);

    let discount_75 = Decimal::percent(75);
    let atom_opening_fee_75_discount =
        mock.query_opening_fee("uatom", atom_size, Some(discount_75)).fee;
    assert!(atom_opening_fee_75_discount.amount < atom_opening_fee_25_discount.amount);

    // Test that 100% discount results in 0 fee
    let discount_100 = Decimal::percent(100);
    let atom_opening_fee_100_discount =
        mock.query_opening_fee("uatom", atom_size, Some(discount_100)).fee;
    assert_eq!(atom_opening_fee_100_discount.amount, Uint128::zero());

    // Test that 0% discount is same as no discount
    let discount_0 = Decimal::zero();
    let atom_opening_fee_0_discount =
        mock.query_opening_fee("uatom", atom_size, Some(discount_0)).fee;
    assert_eq!(atom_opening_fee_0_discount.amount, atom_opening_fee_no_discount.amount);

    // Test with different position sizes
    let small_size = Int128::from_str("100000").unwrap();
    let small_fee_no_discount = mock.query_opening_fee("uatom", small_size, None).fee;
    let small_fee_with_discount =
        mock.query_opening_fee("uatom", small_size, Some(discount_pct)).fee;

    // Verify proportional discount
    let small_expected_discount =
        small_fee_no_discount.amount.checked_mul_ceil(discount_pct).unwrap();
    let small_expected_fee =
        small_fee_no_discount.amount.checked_sub(small_expected_discount).unwrap();
    let small_difference = if small_expected_fee > small_fee_with_discount.amount {
        small_expected_fee.checked_sub(small_fee_with_discount.amount).unwrap()
    } else {
        small_fee_with_discount.amount.checked_sub(small_expected_fee).unwrap()
    };
    assert!(small_difference <= Uint128::new(1));

    // Test with negative size (short position)
    let short_size = Int128::from_str("-500000").unwrap();
    let short_fee_no_discount = mock.query_opening_fee("uatom", short_size, None).fee;
    let short_fee_with_discount =
        mock.query_opening_fee("uatom", short_size, Some(discount_pct)).fee;

    assert!(short_fee_with_discount.amount < short_fee_no_discount.amount);
    let short_expected_discount =
        short_fee_no_discount.amount.checked_mul_ceil(discount_pct).unwrap();
    let short_expected_fee =
        short_fee_no_discount.amount.checked_sub(short_expected_discount).unwrap();
    let short_difference = if short_expected_fee > short_fee_with_discount.amount {
        short_expected_fee.checked_sub(short_fee_with_discount.amount).unwrap()
    } else {
        short_fee_with_discount.amount.checked_sub(short_expected_fee).unwrap()
    };
    assert!(short_difference <= Uint128::new(1));

    // Test discount consistency across different denoms
    let osmo_size = Int128::from_str("500000").unwrap();
    let osmo_opening_fee_no_discount = mock.query_opening_fee("uosmo", osmo_size, None).fee;
    let osmo_opening_fee_with_discount =
        mock.query_opening_fee("uosmo", osmo_size, Some(discount_pct)).fee;

    // Verify discount is applied consistently
    assert!(osmo_opening_fee_with_discount.amount < osmo_opening_fee_no_discount.amount);
    let osmo_expected_discount =
        osmo_opening_fee_no_discount.amount.checked_mul_ceil(discount_pct).unwrap();
    let osmo_expected_fee =
        osmo_opening_fee_no_discount.amount.checked_sub(osmo_expected_discount).unwrap();
    let osmo_difference = if osmo_expected_fee > osmo_opening_fee_with_discount.amount {
        osmo_expected_fee.checked_sub(osmo_opening_fee_with_discount.amount).unwrap()
    } else {
        osmo_opening_fee_with_discount.amount.checked_sub(osmo_expected_fee).unwrap()
    };
    assert!(osmo_difference <= Uint128::new(1));

    // Verify that accounting is still clean after all the queries
    let osmo_accounting_after = mock.query_market_accounting("uosmo").accounting;
    let atom_accounting_after = mock.query_market_accounting("uatom").accounting;
    let total_accounting_after = mock.query_total_accounting().accounting;

    // Since we didn't execute any orders, accounting should still be default
    assert_eq!(osmo_accounting_after, Accounting::default());
    assert_eq!(atom_accounting_after, Accounting::default());
    assert_eq!(total_accounting_after, Accounting::default());
}

#[test]
fn discount_fee_edge_cases() {
    let mut mock = MockEnv::new().build().unwrap();
    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let user = "alice";

    // Set up basic environment
    mock.fund_accounts(&[&credit_manager], 1_000_000_000u128, &["uusdc"]);
    mock.set_price(&owner, "uusdc", Decimal::from_str("1").unwrap()).unwrap();
    mock.set_price(&owner, "uatom", Decimal::from_str("10").unwrap()).unwrap();

    mock.deposit_to_vault(&credit_manager, Some(user), None, &[coin(1_000_000_000u128, "uusdc")])
        .unwrap();

    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                opening_fee_rate: Decimal::percent(1),
                closing_fee_rate: Decimal::percent(1),
                ..default_perp_params("uatom")
            },
        },
    );

    let size = Int128::from_str("1000").unwrap();

    // Test 100% discount (should result in 0 fee)
    let fee_100_discount = mock.query_opening_fee("uatom", size, Some(Decimal::percent(100))).fee;
    assert_eq!(fee_100_discount.amount, Uint128::zero());

    // Test 0% discount (should be same as no discount)
    let fee_0_discount = mock.query_opening_fee("uatom", size, Some(Decimal::zero())).fee;
    let fee_no_discount = mock.query_opening_fee("uatom", size, None).fee;
    assert_eq!(fee_0_discount.amount, fee_no_discount.amount);

    // Test very small discount
    let small_discount = Decimal::from_str("0.001").unwrap(); // 0.1%
    let fee_small_discount = mock.query_opening_fee("uatom", size, Some(small_discount)).fee;
    assert!(fee_small_discount.amount < fee_no_discount.amount);
    assert!(fee_small_discount.amount > Uint128::zero());
}
