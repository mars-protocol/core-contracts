use std::str::FromStr;

use cosmwasm_std::{coin, Decimal, Int128, SignedDecimal, Uint128};
use mars_types::{
    params::{PerpParams, PerpParamsUpdate},
    perps::MarketResponse,
};

use crate::tests::helpers::{default_perp_params, MockEnv};

#[test]
fn query_market() {
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
    let perp_market_state = mock.query_market(denom1);

    let expected_perp_market_state = MarketResponse {
        denom: denom1.to_string(),
        enabled: true,
        long_oi: Uint128::zero(),
        long_oi_value: Uint128::zero(),
        short_oi: Uint128::zero(),
        short_oi_value: Uint128::zero(),
        current_funding_rate: SignedDecimal::zero(),
    };

    assert_eq!(perp_market_state, expected_perp_market_state);

    // Add some position to the perp market state
    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000_000u128, &[denom1, base_denom]);

    mock.deposit_to_vault(
        &credit_manager,
        Some(user),
        None,
        &[coin(1_000_000_000_000u128, "uusdc")],
    )
    .unwrap();

    // Open a LONG position
    let amount = Uint128::from(200u32);
    let size = Int128::try_from(amount).unwrap();

    mock.execute_perp_order(&credit_manager, "2", denom1, size, None, &[]).unwrap();

    let perp_market_state = mock.query_market(denom1);

    let expected_long_oi_value =
        amount.checked_mul_floor(Decimal::from_str(denom1_price).unwrap()).unwrap();
    let expected_perp_market_state = MarketResponse {
        long_oi: amount,
        long_oi_value: expected_long_oi_value,
        ..expected_perp_market_state
    };

    assert_eq!(perp_market_state, expected_perp_market_state);

    // Open a SHORT position
    let amount = Uint128::from(350u32);
    let size = -Int128::try_from(amount).unwrap();

    mock.execute_perp_order(&credit_manager, "3", denom1, size, None, &[]).unwrap();

    let perp_market_state = mock.query_market(denom1);

    let expected_short_oi_value =
        amount.checked_mul_floor(Decimal::from_str(denom1_price).unwrap()).unwrap();
    let expected_perp_market_state = MarketResponse {
        short_oi: amount,
        short_oi_value: expected_short_oi_value,
        ..expected_perp_market_state
    };

    assert_eq!(perp_market_state, expected_perp_market_state);
}

#[test]
fn query_markets() {
    let mut mock = MockEnv::new().build().unwrap();
    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();

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

    // Setup all expected perp market states
    let expected_perp_market_state_base = MarketResponse {
        denom: base_denom.to_string(),
        enabled: true,
        long_oi: Uint128::zero(),
        long_oi_value: Uint128::zero(),
        short_oi: Uint128::zero(),
        short_oi_value: Uint128::zero(),
        current_funding_rate: SignedDecimal::zero(),
    };

    let expected_perp_market_state1 = MarketResponse {
        denom: denom1.to_string(),
        ..expected_perp_market_state_base.clone()
    };

    let long_amount = Uint128::from(200u32);
    let long_oi_value =
        long_amount.checked_mul_floor(Decimal::from_str(denom2_price).unwrap()).unwrap();
    let size = Int128::try_from(long_amount).unwrap();
    mock.execute_perp_order(&credit_manager, "2", denom2, size, None, &[]).unwrap();

    let expected_perp_market_state2 = MarketResponse {
        denom: denom2.to_string(),
        long_oi: long_amount,
        long_oi_value,
        ..expected_perp_market_state_base.clone()
    };

    let short_amount = Uint128::from(420u32);
    let short_oi_value =
        short_amount.checked_mul_floor(Decimal::from_str(denom3_price).unwrap()).unwrap();
    let size = -Int128::try_from(short_amount).unwrap();
    mock.execute_perp_order(&credit_manager, "3", denom3, size, None, &[]).unwrap();

    let expected_perp_market_state3 = MarketResponse {
        denom: denom3.to_string(),
        short_oi: short_amount,
        short_oi_value,
        ..expected_perp_market_state_base.clone()
    };

    // Test to query all perp market states
    let perp_market_states_res = mock.query_markets(None, None);
    assert_eq!(
        perp_market_states_res.data,
        vec![
            expected_perp_market_state1.clone(),
            expected_perp_market_state2.clone(),
            expected_perp_market_state3.clone()
        ]
    );
    assert!(!perp_market_states_res.metadata.has_more);

    // Test to query after the first perp market state
    let perp_market_states_res = mock.query_markets(Some(denom1.to_string()), None);
    assert_eq!(
        perp_market_states_res.data,
        vec![expected_perp_market_state2.clone(), expected_perp_market_state3.clone()]
    );
    assert!(!perp_market_states_res.metadata.has_more);

    // Test the limit parameter
    let perp_market_states_res = mock.query_markets(None, Some(1));
    assert_eq!(perp_market_states_res.data, vec![expected_perp_market_state1.clone()]);
    assert!(perp_market_states_res.metadata.has_more);
}

#[test]
fn query_positions() {
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
    let size = Int128::from_str("50").unwrap();
    mock.execute_perp_order(&credit_manager, "1", "uatom", size, None, &[]).unwrap();

    let size = Int128::from_str("70").unwrap();
    mock.execute_perp_order(&credit_manager, "2", "uatom", size, None, &[]).unwrap();

    let size = Int128::from_str("40").unwrap();
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
