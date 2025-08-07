use std::str::FromStr;

use cosmwasm_std::{coin, Coin, Decimal, Int128, Uint128};
use mars_types::params::{PerpParams, PerpParamsUpdate};
use test_case::test_case;

use crate::tests::helpers::{default_perp_params, MockEnv};

#[test_case(Decimal::percent(25), Decimal::percent(2), Decimal::percent(1), Int128::from_str("1000").unwrap(), Uint128::from(7u128), Uint128::from(4u128); "25 percent on open and close")]
#[test_case(Decimal::percent(25), Decimal::percent(2), Decimal::percent(0), Int128::from_str("1000").unwrap(), Uint128::from(7u128), Uint128::from(0u128); "25 percent on open")]
#[test_case(Decimal::percent(25), Decimal::percent(0), Decimal::percent(2), Int128::from_str("1000").unwrap(), Uint128::from(0u128), Uint128::from(7u128); "25 percent on close")]
#[test_case(Decimal::percent(0), Decimal::percent(3), Decimal::percent(3), Int128::from_str("5000").unwrap(), Uint128::from(0u128), Uint128::from(0u128); "0 percent on open and close")]
#[test_case(Decimal::percent(100), Decimal::percent(2), Decimal::percent(3), Int128::from_str("1000").unwrap(), Uint128::from(28u128), Uint128::from(42u128); "100 percent on open and close")]
#[test_case(Decimal::percent(1), Decimal::percent(2), Decimal::percent(1), Int128::from_str("1").unwrap(), Uint128::from(1u128), Uint128::from(1u128); "1 percent on open and close, testing rounding")]
fn protocol_fee_sent_to_rewards_collector(
    protocol_fee_rate: Decimal,
    opening_fee_rate: Decimal,
    closing_fee_rate: Decimal,
    size: Int128,
    expected_protocol_fee_opening: Uint128,
    expected_protocol_fee_closing: Uint128,
) {
    let mut mock = MockEnv::new().protocol_fee_rate(protocol_fee_rate).build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let rewards_collector = mock.rewards_collector.clone();
    let user = "jake";

    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000_000u128, &["uosmo", "uusdc"]);

    let base_denom_price = Decimal::from_str("0.9").unwrap();
    mock.set_price(&owner, "uusdc", base_denom_price).unwrap();

    // deposit some big number of uusdc to vault
    mock.deposit_to_vault(
        &credit_manager,
        Some(user),
        None,
        &[coin(1_000_000_000_000u128, "uusdc")],
    )
    .unwrap();

    mock.set_price(&owner, "uosmo", Decimal::from_str("1.25").unwrap()).unwrap();

    // init denoms
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                closing_fee_rate,
                opening_fee_rate,
                max_funding_velocity: Decimal::from_str("32").unwrap(),
                skew_scale: Uint128::new(1000000u128),
                ..default_perp_params("uosmo")
            },
        },
    );

    let vault_state_before_opening = mock.query_vault().total_liquidity;

    let rewards_collector_before_opening = mock.query_balance(&rewards_collector, "uusdc");

    // open perps position
    let osmo_opening_fee = mock.query_position_fees("1", "uosmo", size).opening_fee;

    let funds = if osmo_opening_fee.is_zero() {
        vec![]
    } else {
        vec![Coin::new(osmo_opening_fee.u128(), "uusdc")]
    };

    mock.execute_perp_order(&credit_manager, "1", "uosmo", size, None, &funds).unwrap();

    // check vault state after opening position
    let rewards_collector_after_opening = mock.query_balance(&rewards_collector, "uusdc");
    let vault_state_after_opening = mock.query_vault().total_liquidity;

    assert_eq!(rewards_collector_before_opening.amount, Uint128::zero());
    assert_eq!(rewards_collector_after_opening.amount, expected_protocol_fee_opening);
    assert_eq!(
        vault_state_after_opening,
        vault_state_before_opening + osmo_opening_fee - expected_protocol_fee_opening
    );

    let osmo_closing_fee = mock.query_position_fees("1", "uosmo", Int128::zero()).closing_fee;

    let funds = if osmo_closing_fee.is_zero() {
        vec![]
    } else {
        vec![Coin::new(osmo_closing_fee.u128(), "uusdc")]
    };
    mock.execute_perp_order(&credit_manager, "1", "uosmo", Int128::zero() - size, None, &funds)
        .unwrap();

    // check vault state after closing position
    let rewards_collector_after_closing = mock.query_balance(&rewards_collector, "uusdc");
    let vault_state_after_closing = mock.query_vault().total_liquidity;

    assert_eq!(
        rewards_collector_after_closing.amount,
        expected_protocol_fee_opening + expected_protocol_fee_closing
    );
    assert_eq!(
        vault_state_after_closing,
        vault_state_after_opening + osmo_closing_fee - expected_protocol_fee_closing
    );
}

#[test]
fn close_all_positions_applies_fees() {
    let protocol_fee_rate = Decimal::percent(25);
    let opening_fee_rate = Decimal::percent(2);
    let closing_fee_rate = Decimal::percent(1);

    let mut mock = MockEnv::new().protocol_fee_rate(protocol_fee_rate).build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let rewards_collector = mock.rewards_collector.clone();
    let user = "jake";

    let denom_1 = "uosmo";
    let denom_2 = "umars";

    let size_1 = Int128::from_str("1000").unwrap();
    let size_2 = Int128::from_str("500").unwrap();

    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000_000u128, &[denom_1, "uusdc"]);

    let base_denom_price = Decimal::from_str("0.9").unwrap();
    mock.set_price(&owner, "uusdc", base_denom_price).unwrap();
    mock.set_price(&owner, denom_1, Decimal::from_str("1.25").unwrap()).unwrap();
    mock.set_price(&owner, denom_2, Decimal::from_str("2.7").unwrap()).unwrap();

    // deposit some big number of uusdc to vault
    mock.deposit_to_vault(
        &credit_manager,
        Some(user),
        None,
        &[coin(1_000_000_000_000u128, "uusdc")],
    )
    .unwrap();

    // init denoms
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                closing_fee_rate,
                opening_fee_rate,
                max_funding_velocity: Decimal::from_str("32").unwrap(),
                skew_scale: Uint128::new(1000000u128),
                ..default_perp_params(denom_1)
            },
        },
    );
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                closing_fee_rate,
                opening_fee_rate,
                max_funding_velocity: Decimal::from_str("32").unwrap(),
                skew_scale: Uint128::new(1000000u128),
                ..default_perp_params(denom_2)
            },
        },
    );

    let vault_state_before_opening = mock.query_vault().total_liquidity;
    let rewards_collector_before_opening = mock.query_balance(&rewards_collector, "uusdc");

    // open perps positions
    // Position 1
    let opening_fee_1 = mock.query_position_fees("1", denom_1, size_1).opening_fee;
    let funds = if opening_fee_1.is_zero() {
        vec![]
    } else {
        vec![Coin::new(opening_fee_1.u128(), "uusdc")]
    };

    mock.execute_perp_order(&credit_manager, "1", denom_1, size_1, None, &funds).unwrap();

    // Position 2
    let opening_fee_2 = mock.query_position_fees("1", denom_2, size_2).opening_fee;
    let funds = if opening_fee_2.is_zero() {
        vec![]
    } else {
        vec![Coin::new(opening_fee_2.u128(), "uusdc")]
    };
    mock.execute_perp_order(&credit_manager, "1", denom_2, size_2, None, &funds).unwrap();

    // check vault state after opening position
    let rewards_collector_after_opening = mock.query_balance(&rewards_collector, "uusdc");
    let vault_state_after_opening = mock.query_vault().total_liquidity;

    // Fees are rounded up in favor of protocol
    let expected_protocol_fee_opening_1 =
        opening_fee_1.checked_mul_ceil(protocol_fee_rate).unwrap();
    let expected_protocol_fee_opening_2 =
        opening_fee_2.checked_mul_ceil(protocol_fee_rate).unwrap();

    assert_eq!(rewards_collector_before_opening.amount, Uint128::zero());
    assert_eq!(
        rewards_collector_after_opening.amount,
        expected_protocol_fee_opening_1 + expected_protocol_fee_opening_2
    );
    assert_eq!(
        vault_state_after_opening,
        vault_state_before_opening + opening_fee_1 + opening_fee_2
            - expected_protocol_fee_opening_1
            - expected_protocol_fee_opening_2
    );

    let closing_fee_1 = mock.query_position_fees("1", denom_1, Int128::zero()).closing_fee;
    assert!(!closing_fee_1.is_zero());
    let closing_fee_2 = mock.query_position_fees("1", denom_2, Int128::zero()).closing_fee;
    assert!(!closing_fee_2.is_zero());

    let total_closing_fee = closing_fee_1 + closing_fee_2;

    let funds = if total_closing_fee.is_zero() {
        vec![]
    } else {
        vec![Coin::new(total_closing_fee.u128(), "uusdc")]
    };
    mock.close_all_positions(&credit_manager, "1", &funds).unwrap();

    // // check vault state after closing position
    let rewards_collector_after_closing = mock.query_balance(&rewards_collector, "uusdc");
    let vault_state_after_closing = mock.query_vault().total_liquidity;

    // Fees are rounded up in favor of protocol
    let expected_protocol_fee_closing_1 =
        closing_fee_1.checked_mul_ceil(protocol_fee_rate).unwrap();
    assert!(!expected_protocol_fee_closing_1.is_zero());
    let expected_protocol_fee_closing_2 =
        closing_fee_2.checked_mul_ceil(protocol_fee_rate).unwrap();
    assert!(!expected_protocol_fee_closing_2.is_zero());

    assert_eq!(
        vault_state_after_closing,
        vault_state_after_opening + closing_fee_1 + closing_fee_2
            - expected_protocol_fee_closing_1
            - expected_protocol_fee_closing_2
    );
    assert_eq!(
        rewards_collector_after_closing.amount,
        rewards_collector_after_opening.amount
            + expected_protocol_fee_closing_1
            + expected_protocol_fee_closing_2
    );
}
