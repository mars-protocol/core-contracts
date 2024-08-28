use std::{collections::HashMap, str::FromStr};

use cosmwasm_std::{coin, Addr, Decimal, Uint128};
use mars_perps::{denom::SECONDS_IN_DAY, error::ContractError};
use mars_types::{
    error::MarsError,
    math::SignedDecimal,
    params::{EmergencyUpdate, PerpParams, PerpParamsUpdate, PerpsEmergencyUpdate},
    perps::{DenomStateResponse, Funding},
    signed_uint::SignedUint,
};

use super::helpers::MockEnv;
use crate::tests::helpers::{assert_err, default_perp_params};

#[test]
fn random_addr_cannot_update_params() {
    let mut mock = MockEnv::new().build().unwrap();

    let res = mock.update_params(&Addr::unchecked("dawid"), default_perp_params("uosmo"));
    assert_err(res, ContractError::Mars(MarsError::Unauthorized {}));
}

#[test]
fn initialize_denom() {
    let mut mock = MockEnv::new().build().unwrap();

    let params_addr = mock.params.clone();

    mock.update_params(
        &params_addr,
        PerpParams {
            max_funding_velocity: Decimal::from_str("3").unwrap(),
            skew_scale: Uint128::new(1000000u128),
            ..default_perp_params("perp/osmo/usd")
        },
    )
    .unwrap();

    let ds = mock.query_denom_state("perp/osmo/usd");
    let block_time = mock.query_block_time();
    assert_eq!(
        ds,
        DenomStateResponse {
            denom: "perp/osmo/usd".to_string(),
            enabled: true,
            total_cost_base: SignedUint::zero(),
            funding: Funding {
                max_funding_velocity: Decimal::from_str("3").unwrap(),
                skew_scale: Uint128::new(1000000u128),
                last_funding_rate: SignedDecimal::zero(),
                last_funding_accrued_per_unit_in_base_denom: SignedDecimal::zero()
            },
            last_updated: block_time
        }
    )
}

#[test]
fn update_denom() {
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();
    let params_addr = mock.params.clone();

    mock.set_price(&owner, "uusdc", Decimal::from_str("1").unwrap()).unwrap();
    mock.set_price(&owner, "perp/osmo/usd", Decimal::from_str("1").unwrap()).unwrap();

    mock.update_params(
        &params_addr,
        PerpParams {
            max_funding_velocity: Decimal::from_str("389").unwrap(),
            skew_scale: Uint128::new(1234000u128),
            ..default_perp_params("perp/osmo/usd")
        },
    )
    .unwrap();

    let block_time = mock.query_block_time();

    let ds = mock.query_denom_state("perp/osmo/usd");
    assert_eq!(
        ds,
        DenomStateResponse {
            denom: "perp/osmo/usd".to_string(),
            enabled: true,
            total_cost_base: SignedUint::zero(),
            funding: Funding {
                max_funding_velocity: Decimal::from_str("389").unwrap(),
                skew_scale: Uint128::new(1234000u128),
                last_funding_rate: SignedDecimal::zero(),
                last_funding_accrued_per_unit_in_base_denom: SignedDecimal::zero()
            },
            last_updated: block_time
        }
    );

    mock.update_params(
        &params_addr,
        PerpParams {
            enabled: false,
            max_funding_velocity: Decimal::from_str("36").unwrap(),
            skew_scale: Uint128::new(8976543u128),
            ..default_perp_params("perp/osmo/usd")
        },
    )
    .unwrap();

    let ds = mock.query_denom_state("perp/osmo/usd");
    assert_eq!(
        ds,
        DenomStateResponse {
            denom: "perp/osmo/usd".to_string(),
            enabled: false,
            total_cost_base: SignedUint::zero(),
            funding: Funding {
                max_funding_velocity: Decimal::from_str("36").unwrap(),
                skew_scale: Uint128::new(8976543u128),
                last_funding_rate: SignedDecimal::zero(),
                last_funding_accrued_per_unit_in_base_denom: SignedDecimal::zero()
            },
            last_updated: block_time
        }
    );
}

#[test]
fn emergency_disable_trading() {
    let emergency_owner = Addr::unchecked("miles_morales");
    let mut mock = MockEnv::new().emergency_owner(emergency_owner.as_str()).build().unwrap();

    let owner = mock.owner.clone();

    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params("ueth"),
        },
    );
    let ds = mock.query_denom_state("ueth");
    assert!(ds.enabled);

    mock.emergency_params_update(
        &emergency_owner,
        EmergencyUpdate::Perps(PerpsEmergencyUpdate::DisableTrading("ueth".to_string())),
    )
    .unwrap();
    let ds = mock.query_denom_state("ueth");
    assert!(!ds.enabled);
}

#[test]
fn paginate_denom_states() {
    let mut mock = MockEnv::new().build().unwrap();

    let params_addr = mock.params.clone();

    mock.update_params(
        &params_addr,
        PerpParams {
            enabled: false,
            max_funding_velocity: Decimal::from_str("389").unwrap(),
            skew_scale: Uint128::new(1234000u128),
            ..default_perp_params("perp/osmo/usd")
        },
    )
    .unwrap();

    mock.update_params(
        &params_addr,
        PerpParams {
            enabled: true,
            max_funding_velocity: Decimal::from_str("100").unwrap(),
            skew_scale: Uint128::new(23400u128),
            ..default_perp_params("perp/ntrn/usd")
        },
    )
    .unwrap();

    let block_time = mock.query_block_time();

    let dss = mock.query_denom_states(None, None);
    assert_eq!(dss.len(), 2);
    let dss = dss.into_map();
    assert_eq!(
        dss.get("perp/osmo/usd").unwrap(),
        &DenomStateResponse {
            denom: "perp/osmo/usd".to_string(),
            enabled: false,
            total_cost_base: SignedUint::zero(),
            funding: Funding {
                max_funding_velocity: Decimal::from_str("389").unwrap(),
                skew_scale: Uint128::new(1234000u128),
                last_funding_rate: SignedDecimal::zero(),
                last_funding_accrued_per_unit_in_base_denom: SignedDecimal::zero()
            },
            last_updated: block_time
        }
    );
    assert_eq!(
        dss.get("perp/ntrn/usd").unwrap(),
        &DenomStateResponse {
            denom: "perp/ntrn/usd".to_string(),
            enabled: true,
            total_cost_base: SignedUint::zero(),
            funding: Funding {
                max_funding_velocity: Decimal::from_str("100").unwrap(),
                skew_scale: Uint128::new(23400u128),
                last_funding_rate: SignedDecimal::zero(),
                last_funding_accrued_per_unit_in_base_denom: SignedDecimal::zero()
            },
            last_updated: block_time
        }
    );
}

#[test]
fn funding_change_accordingly_to_denom_state_modification() {
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let depositor = "peter";

    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000u128, &["ueth", "uusdc"]);

    mock.set_price(&owner, "uusdc", Decimal::from_str("1").unwrap()).unwrap();
    mock.set_price(&owner, "ueth", Decimal::from_str("2000").unwrap()).unwrap();

    // deposit some big number of uusdc to vault
    mock.deposit_to_vault(
        &credit_manager,
        Some(depositor),
        &[coin(1_000_000_000_000u128, "uusdc")],
    )
    .unwrap();

    // prepare denom state
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                max_funding_velocity: Decimal::from_str("30").unwrap(),
                skew_scale: Uint128::new(1000000u128),
                ..default_perp_params("ueth")
            },
        },
    );
    mock.execute_perp_order(
        &credit_manager,
        "1",
        "ueth",
        SignedUint::from_str("300").unwrap(),
        None,
        &[],
    )
    .unwrap();
    let perp_params = mock.query_perp_params("ueth");
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                enabled: false,
                ..perp_params
            },
        },
    );

    // query denom state for h0
    let ds_h0 = mock.query_denom_state("ueth");

    // move time forward by 24 hour
    mock.increment_by_time(SECONDS_IN_DAY);

    // enable denom
    let perp_params = mock.query_perp_params("ueth");
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                enabled: true,
                ..perp_params
            },
        },
    );

    // Query state for h24.
    // Should be the same as h0 with last_updated changed and enabled set to true.
    // When denom is disabled there is no activity so funding shouldn't be changed.
    // We just shift the last_updated time.
    let ds_h24 = mock.query_denom_state("ueth");
    assert_eq!(
        ds_h24,
        DenomStateResponse {
            enabled: true,
            last_updated: ds_h0.last_updated + SECONDS_IN_DAY,
            ..ds_h0
        }
    );

    // move time forward by 24 hour
    mock.increment_by_time(SECONDS_IN_DAY);

    // disable denom
    let perp_params = mock.query_perp_params("ueth");
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                enabled: false,
                ..perp_params
            },
        },
    );

    // Query state for h48.
    // When denom is disabled, funding should be updated accordingly.
    let ds_h48 = mock.query_denom_state("ueth");
    assert_ne!(ds_h48.funding.last_funding_rate, ds_h24.funding.last_funding_rate);
    assert_ne!(
        ds_h48.funding.last_funding_accrued_per_unit_in_base_denom,
        ds_h24.funding.last_funding_accrued_per_unit_in_base_denom
    );
    assert_eq!(
        ds_h48,
        DenomStateResponse {
            enabled: false,
            last_updated: ds_h24.last_updated + SECONDS_IN_DAY,
            funding: Funding {
                last_funding_rate: SignedDecimal::from_str("0.009").unwrap(),
                last_funding_accrued_per_unit_in_base_denom: SignedDecimal::from_str("-9").unwrap(),
                ..ds_h24.funding
            },
            ..ds_h24
        }
    );
}

trait DenomStateResponseVecExt {
    fn into_map(self) -> HashMap<String, DenomStateResponse>;
}

impl DenomStateResponseVecExt for Vec<DenomStateResponse> {
    fn into_map(self) -> HashMap<String, DenomStateResponse> {
        self.into_iter().map(|ds| (ds.denom.clone(), ds)).collect()
    }
}
