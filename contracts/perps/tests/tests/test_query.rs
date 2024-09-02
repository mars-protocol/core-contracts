use std::str::FromStr;

use cosmwasm_std::{coin, Decimal, Uint128};
use mars_types::{
    math::SignedDecimal,
    params::{PerpParams, PerpParamsUpdate},
    perps::{Funding, PerpDenomState, PnlValues},
    signed_uint::SignedUint,
};

use crate::tests::helpers::{default_perp_params, MockEnv};

#[test]
fn perp_denom_state() {
    let mut mock = MockEnv::new().build().unwrap();
    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let user = "terry";

    let base_denom = "uusdc";
    let denom1 = "ueth";

    let base_price = "0.9";
    let denom1_price = "311.56";

    let initial_skew_scale = Uint128::new(1000000u128);
    let initial_funding_velocity = Decimal::from_str("32").unwrap();

    mock.set_price(&owner, base_denom, Decimal::from_str(base_price).unwrap()).unwrap();
    mock.set_price(&owner, denom1, Decimal::from_str(denom1_price).unwrap()).unwrap();
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                max_funding_velocity: initial_funding_velocity,
                skew_scale: initial_skew_scale,
                ..default_perp_params(denom1)
            },
        },
    );

    // Test initial state
    let perp_denom_state = mock.query_perp_denom_state(denom1);

    let expected_perp_denom_state = PerpDenomState {
        denom: denom1.to_string(),
        enabled: true,
        long_oi: Uint128::zero(),
        short_oi: Uint128::zero(),
        total_entry_cost: SignedUint::zero(),
        total_entry_funding: SignedUint::zero(),
        rate: SignedDecimal::zero(),
        pnl_values: PnlValues::default(),
        funding: Funding {
            skew_scale: initial_skew_scale,
            max_funding_velocity: initial_funding_velocity,
            ..Funding::default()
        },
    };

    assert_eq!(perp_denom_state, expected_perp_denom_state);

    // Add some position to the perp denom state
    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000_000u128, &[denom1, base_denom]);

    mock.deposit_to_vault(&credit_manager, Some(user), &[coin(1_000_000_000_000u128, "uusdc")])
        .unwrap();

    let amount = Uint128::from(200u32);
    let size = SignedUint::from(amount);

    mock.execute_perp_order(&credit_manager, "2", denom1, size, None, &[]).unwrap();

    let perp_denom_state = mock.query_perp_denom_state(denom1);

    let expected_perp_denom_state = PerpDenomState {
        long_oi: amount,
        total_entry_cost: SignedUint::from_str("62318").unwrap(),
        ..expected_perp_denom_state
    };

    assert_eq!(perp_denom_state, expected_perp_denom_state);
}

#[test]
fn perp_denom_states() {
    let mut mock = MockEnv::new().build().unwrap();
    let owner = mock.owner.clone();

    // Setup the environment for 3 different perp markets
    let base_denom = "uusdc";
    let denom1 = "uandme";
    let denom2 = "uatom";
    let denom3 = "ueth";

    let base_price = "0.9";
    let denom1_price = "311.56";
    let denom2_price = "10.8";
    let denom3_price = "66.67";

    let initial_skew_scale1 = Uint128::new(1000000u128);
    let initial_funding_velocity1 = Decimal::from_str("32").unwrap();
    let initial_skew_scale2 = Uint128::new(5000000u128);
    let initial_funding_velocity2 = Decimal::from_str("14").unwrap();
    let initial_skew_scale3 = Uint128::new(8000000u128);
    let initial_funding_velocity3 = Decimal::from_str("22").unwrap();

    mock.set_price(&owner, base_denom, Decimal::from_str(base_price).unwrap()).unwrap();
    mock.set_price(&owner, denom1, Decimal::from_str(denom1_price).unwrap()).unwrap();
    mock.set_price(&owner, denom2, Decimal::from_str(denom2_price).unwrap()).unwrap();
    mock.set_price(&owner, denom3, Decimal::from_str(denom3_price).unwrap()).unwrap();
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                max_funding_velocity: initial_funding_velocity1,
                skew_scale: initial_skew_scale1,
                ..default_perp_params(denom1)
            },
        },
    );
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                max_funding_velocity: initial_funding_velocity2,
                skew_scale: initial_skew_scale2,
                ..default_perp_params(denom2)
            },
        },
    );
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                max_funding_velocity: initial_funding_velocity3,
                skew_scale: initial_skew_scale3,
                ..default_perp_params(denom3)
            },
        },
    );

    // Setup all expected perp denom states
    let expected_perp_denom_state_base = PerpDenomState {
        denom: base_denom.to_string(),
        enabled: true,
        long_oi: Uint128::zero(),
        short_oi: Uint128::zero(),
        total_entry_cost: SignedUint::zero(),
        total_entry_funding: SignedUint::zero(),
        rate: SignedDecimal::zero(),
        pnl_values: PnlValues::default(),
        funding: Funding::default(),
    };

    let expected_perp_denom_state1 = PerpDenomState {
        denom: denom1.to_string(),
        funding: Funding {
            skew_scale: initial_skew_scale1,
            max_funding_velocity: initial_funding_velocity1,
            ..Funding::default()
        },
        ..expected_perp_denom_state_base.clone()
    };

    let expected_perp_denom_state2 = PerpDenomState {
        denom: denom2.to_string(),
        funding: Funding {
            skew_scale: initial_skew_scale2,
            max_funding_velocity: initial_funding_velocity2,
            ..Funding::default()
        },
        ..expected_perp_denom_state_base.clone()
    };

    let expected_perp_denom_state3 = PerpDenomState {
        denom: denom3.to_string(),
        funding: Funding {
            skew_scale: initial_skew_scale3,
            max_funding_velocity: initial_funding_velocity3,
            ..Funding::default()
        },
        ..expected_perp_denom_state_base.clone()
    };

    // Test to query all perp denom states
    let perp_denom_states_res = mock.query_perp_denom_states(None, None);
    assert_eq!(
        perp_denom_states_res.data,
        vec![
            expected_perp_denom_state1.clone(),
            expected_perp_denom_state2.clone(),
            expected_perp_denom_state3.clone()
        ]
    );
    assert!(!perp_denom_states_res.metadata.has_more);

    // Test to query after the first perp denom state
    let perp_denom_states_res = mock.query_perp_denom_states(Some(denom1.to_string()), None);
    assert_eq!(
        perp_denom_states_res.data,
        vec![expected_perp_denom_state2.clone(), expected_perp_denom_state3.clone()]
    );
    assert!(!perp_denom_states_res.metadata.has_more);

    // Test the limit parameter
    let perp_denom_states_res = mock.query_perp_denom_states(None, Some(1));
    assert_eq!(perp_denom_states_res.data, vec![expected_perp_denom_state1.clone()]);
    assert!(perp_denom_states_res.metadata.has_more);
}

#[test]
fn perp_positions() {
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let user = "jake";

    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(
        &[&credit_manager],
        1_000_000_000_000_000u128,
        &["uosmo", "uatom", "utia", "uusdc"],
    );

    // set prices
    mock.set_price(&owner, "uusdc", Decimal::from_str("0.8").unwrap()).unwrap();
    mock.set_price(&owner, "uatom", Decimal::from_str("12.5").unwrap()).unwrap();
    mock.set_price(&owner, "utia", Decimal::from_str("6.2").unwrap()).unwrap();

    // deposit some big number of uusdc to vault
    mock.deposit_to_vault(&credit_manager, Some(user), &[coin(1_000_000_000_000u128, "uusdc")])
        .unwrap();

    // init denoms
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                max_funding_velocity: Decimal::from_str("3").unwrap(),
                skew_scale: Uint128::new(1000000u128),
                ..default_perp_params("uatom")
            },
        },
    );
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                max_funding_velocity: Decimal::from_str("3").unwrap(),
                skew_scale: Uint128::new(1200000u128),
                ..default_perp_params("utia")
            },
        },
    );

    // open few positions
    let size = SignedUint::from_str("50").unwrap();
    mock.execute_perp_order(&credit_manager, "1", "uatom", size, None, &[]).unwrap();

    let size = SignedUint::from_str("70").unwrap();
    mock.execute_perp_order(&credit_manager, "2", "uatom", size, None, &[]).unwrap();

    let size = SignedUint::from_str("40").unwrap();
    mock.execute_perp_order(&credit_manager, "2", "utia", size, None, &[]).unwrap();

    let acc_1_atom_position = mock.query_position("1", "uatom").position.unwrap();
    let acc_2_atom_position = mock.query_position("2", "uatom").position.unwrap();
    let acc_2_tia_position = mock.query_position("2", "utia").position.unwrap();

    let positions = mock.query_positions(None, None);
    assert_eq!(positions.len(), 3);
    assert_eq!(positions[0].clone().position.unwrap(), acc_1_atom_position);
    assert_eq!(positions[1].clone().position.unwrap(), acc_2_atom_position);
    assert_eq!(positions[2].clone().position.unwrap(), acc_2_tia_position);
}
