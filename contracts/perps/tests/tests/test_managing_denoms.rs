use std::{collections::HashMap, str::FromStr};

use cosmwasm_std::{coin, Addr, Decimal, Uint128};
use mars_owner::OwnerError;
use mars_perps::{denom::SECONDS_IN_DAY, error::ContractError};
use mars_types::{
    math::SignedDecimal,
    params::PerpParamsUpdate,
    perps::{DenomStateResponse, Funding},
    signed_uint::SignedUint,
};

use super::helpers::MockEnv;
use crate::tests::helpers::{assert_err, default_perp_params};

#[test]
fn non_owner_cannot_init_denom() {
    let mut mock = MockEnv::new().build().unwrap();

    let res = mock.init_denom(&Addr::unchecked("dawid"), "uosmo", Decimal::one(), Uint128::one());
    assert_err(res, ContractError::Owner(OwnerError::NotOwner {}));
}

#[test]
fn denom_already_exists() {
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();

    mock.init_denom(&owner, "uosmo", Decimal::one(), Uint128::one()).unwrap();
    let res = mock.init_denom(&owner, "uosmo", Decimal::one(), Uint128::one());
    assert_err(
        res,
        ContractError::DenomAlreadyExists {
            denom: "uosmo".to_string(),
        },
    );
}

#[test]
fn skew_scale_cannot_be_zero() {
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();

    let res = mock.init_denom(&owner, "uosmo", Decimal::one(), Uint128::zero());
    assert_err(
        res,
        ContractError::InvalidParam {
            reason: "skew_scale cannot be zero".to_string(),
        },
    );
}

#[test]
fn initialize_denom() {
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();

    mock.init_denom(
        &owner,
        "perp/osmo/usd",
        Decimal::from_str("3").unwrap(),
        Uint128::new(1000000u128),
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
                last_funding_accrued_per_unit_in_base_denom: SignedUint::zero()
            },
            last_updated: block_time
        }
    )
}

#[test]
fn non_owner_cannot_enable_denom() {
    let mut mock = MockEnv::new().build().unwrap();

    let res = mock.enable_denom(&Addr::unchecked("pumpkin"), "perp/osmo/usd");
    assert_err(res, ContractError::Owner(OwnerError::NotOwner {}));
}

#[test]
fn cannot_enable_denom_if_not_found() {
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();

    let res = mock.enable_denom(&owner, "perp/osmo/usd");
    assert_err(
        res,
        ContractError::DenomNotFound {
            denom: "perp/osmo/usd".to_string(),
        },
    );
}

#[test]
fn owner_can_enable_denom() {
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();

    mock.init_denom(&owner, "perp/eth/eur", Decimal::zero(), Uint128::one()).unwrap();
    mock.init_denom(&owner, "perp/btc/usd", Decimal::zero(), Uint128::one()).unwrap();

    let dss = mock.query_denom_states(None, None);
    assert_eq!(dss.len(), 2);
    let dss = dss.into_map();
    assert!(dss.get("perp/btc/usd").unwrap().enabled);
    assert!(dss.get("perp/eth/eur").unwrap().enabled);
}

#[test]
fn non_owner_cannot_disable_denom() {
    let mut mock = MockEnv::new().build().unwrap();

    let res = mock.disable_denom(&Addr::unchecked("jake"), "perp/btc/usd");
    assert_err(res, ContractError::Owner(OwnerError::NotOwner {}));
}

#[test]
fn cannot_disable_denom_if_not_found() {
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();

    let res = mock.disable_denom(&owner, "perp/osmo/usd");
    assert_err(
        res,
        ContractError::DenomNotFound {
            denom: "perp/osmo/usd".to_string(),
        },
    );
}

#[test]
fn owner_can_disable_denom() {
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();

    mock.init_denom(&owner, "perp/eth/eur", Decimal::zero(), Uint128::one()).unwrap();
    mock.init_denom(&owner, "perp/btc/usd", Decimal::zero(), Uint128::one()).unwrap();

    mock.set_price(&owner, "uusdc", Decimal::from_str("1").unwrap()).unwrap();
    mock.set_price(&owner, "perp/eth/eur", Decimal::from_str("1").unwrap()).unwrap();
    mock.set_price(&owner, "perp/btc/usd", Decimal::from_str("1").unwrap()).unwrap();

    mock.disable_denom(&owner, "perp/btc/usd").unwrap();

    let dss = mock.query_denom_states(None, None);
    assert_eq!(dss.len(), 2);
    let dss = dss.into_map();
    assert!(!dss.get("perp/btc/usd").unwrap().enabled);
    assert!(dss.get("perp/eth/eur").unwrap().enabled);
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
    mock.init_denom(&owner, "ueth", Decimal::from_str("30").unwrap(), Uint128::new(1000000u128))
        .unwrap();
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params("ueth"),
        },
    );
    mock.open_position(&credit_manager, "1", "ueth", SignedUint::from_str("300").unwrap(), &[])
        .unwrap();
    mock.disable_denom(&owner, "ueth").unwrap();

    // query denom state for h0
    let ds_h0 = mock.query_denom_state("ueth");

    // move time forward by 24 hour
    mock.increment_by_time(SECONDS_IN_DAY);

    // enable denom
    mock.enable_denom(&owner, "ueth").unwrap();

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
    mock.disable_denom(&owner, "ueth").unwrap();

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
                last_funding_accrued_per_unit_in_base_denom: SignedUint::from_str("-9").unwrap(),
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
