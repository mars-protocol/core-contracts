use std::str::FromStr;

use cosmwasm_std::{coin, Coin, Decimal, Uint128};
use mars_types::{
    params::{PerpParams, PerpParamsUpdate},
    signed_uint::SignedUint,
};
use test_case::test_case;

use crate::tests::helpers::{default_perp_params, MockEnv};

#[test_case(Decimal::percent(25), Decimal::percent(2), Decimal::percent(1), SignedUint::from_str("1000").unwrap(), Uint128::from(7u128), Uint128::from(4u128); "25 percent on open and close")]
#[test_case(Decimal::percent(25), Decimal::percent(2), Decimal::percent(0), SignedUint::from_str("1000").unwrap(), Uint128::from(7u128), Uint128::from(0u128); "25 percent on open")]
#[test_case(Decimal::percent(25), Decimal::percent(0), Decimal::percent(2), SignedUint::from_str("1000").unwrap(), Uint128::from(0u128), Uint128::from(7u128); "25 percent on close")]
#[test_case(Decimal::percent(0), Decimal::percent(3), Decimal::percent(3), SignedUint::from_str("5000").unwrap(), Uint128::from(0u128), Uint128::from(0u128); "0 percent on open and close")]
#[test_case(Decimal::percent(100), Decimal::percent(2), Decimal::percent(3), SignedUint::from_str("1000").unwrap(), Uint128::from(28u128), Uint128::from(42u128); "100 percent on open and close")]
#[test_case(Decimal::percent(1), Decimal::percent(2), Decimal::percent(1), SignedUint::from_str("1").unwrap(), Uint128::from(1u128), Uint128::from(1u128); "1 percent on open and close, testing rounding")]
fn protocol_fee_sent_to_rewards_collector(
    protocol_fee_rate: Decimal,
    opening_fee_rate: Decimal,
    closing_fee_rate: Decimal,
    size: SignedUint,
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
    mock.deposit_to_vault(&credit_manager, Some(user), &[coin(1_000_000_000_000u128, "uusdc")])
        .unwrap();

    // init denoms
    mock.init_denom(&owner, "uosmo", Decimal::from_str("32").unwrap(), Uint128::new(1000000u128))
        .unwrap();
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                closing_fee_rate,
                opening_fee_rate,
                ..default_perp_params("uosmo")
            },
        },
    );

    mock.set_price(&owner, "uosmo", Decimal::from_str("1.25").unwrap()).unwrap();

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

    let osmo_closing_fee = mock.query_position_fees("1", "uosmo", SignedUint::zero()).closing_fee;

    let funds = if osmo_closing_fee.is_zero() {
        vec![]
    } else {
        vec![Coin::new(osmo_closing_fee.u128(), "uusdc")]
    };
    mock.execute_perp_order(&credit_manager, "1", "uosmo", size.neg(), None, &funds).unwrap();

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
