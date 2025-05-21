use std::{collections::HashMap, str::FromStr};

use cosmwasm_std::{coin, Addr, Decimal, Int128, SignedDecimal, Uint128};
use mars_perps::{error::ContractError, market::SECONDS_IN_DAY};
use mars_types::{
    error::MarsError,
    params::{EmergencyUpdate, PerpParams, PerpParamsUpdate, PerpsEmergencyUpdate},
    perps::{Funding, MarketResponse, MarketState, MarketStateResponse},
};

use super::helpers::MockEnv;
use crate::tests::helpers::{assert_err, default_perp_params};

#[test]
fn random_addr_cannot_update_market() {
    let mut mock = MockEnv::new().build().unwrap();

    let res = mock.update_market(&Addr::unchecked("dawid"), default_perp_params("uosmo"));
    assert_err(res, ContractError::Mars(MarsError::Unauthorized {}));
}

#[test]
fn initialize_market() {
    let mut mock = MockEnv::new().build().unwrap();

    let params_addr = mock.params.clone();

    mock.update_market(
        &params_addr,
        PerpParams {
            max_funding_velocity: Decimal::from_str("3").unwrap(),
            skew_scale: Uint128::new(1000000u128),
            ..default_perp_params("perp/osmo/usd")
        },
    )
    .unwrap();

    let ms = mock.query_market_state("perp/osmo/usd");
    let block_time = mock.query_block_time();
    assert_eq!(
        ms,
        MarketStateResponse {
            denom: "perp/osmo/usd".to_string(),
            market_state: MarketState {
                enabled: true,
                funding: Funding {
                    max_funding_velocity: Decimal::from_str("3").unwrap(),
                    skew_scale: Uint128::new(1000000u128),
                    last_funding_rate: SignedDecimal::zero(),
                    last_funding_accrued_per_unit_in_base_denom: SignedDecimal::zero()
                },
                last_updated: block_time,
                ..Default::default()
            }
        }
    )
}

#[test]
fn update_market() {
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();
    let params_addr = mock.params.clone();

    mock.set_price(&owner, "uusdc", Decimal::from_str("1").unwrap()).unwrap();
    mock.set_price(&owner, "perp/osmo/usd", Decimal::from_str("1").unwrap()).unwrap();

    mock.update_market(
        &params_addr,
        PerpParams {
            max_funding_velocity: Decimal::from_str("389").unwrap(),
            skew_scale: Uint128::new(1234000u128),
            ..default_perp_params("perp/osmo/usd")
        },
    )
    .unwrap();

    let block_time = mock.query_block_time();

    let ms = mock.query_market_state("perp/osmo/usd");
    assert_eq!(
        ms,
        MarketStateResponse {
            denom: "perp/osmo/usd".to_string(),
            market_state: MarketState {
                enabled: true,
                funding: Funding {
                    max_funding_velocity: Decimal::from_str("389").unwrap(),
                    skew_scale: Uint128::new(1234000u128),
                    last_funding_rate: SignedDecimal::zero(),
                    last_funding_accrued_per_unit_in_base_denom: SignedDecimal::zero()
                },
                last_updated: block_time,
                ..Default::default()
            }
        }
    );

    mock.update_market(
        &params_addr,
        PerpParams {
            enabled: false,
            max_funding_velocity: Decimal::from_str("36").unwrap(),
            skew_scale: Uint128::new(8976543u128),
            ..default_perp_params("perp/osmo/usd")
        },
    )
    .unwrap();

    let ms = mock.query_market_state("perp/osmo/usd");
    assert_eq!(
        ms,
        MarketStateResponse {
            denom: "perp/osmo/usd".to_string(),
            market_state: MarketState {
                enabled: false,
                funding: Funding {
                    max_funding_velocity: Decimal::from_str("36").unwrap(),
                    skew_scale: Uint128::new(8976543u128),
                    last_funding_rate: SignedDecimal::zero(),
                    last_funding_accrued_per_unit_in_base_denom: SignedDecimal::zero()
                },
                last_updated: block_time,
                ..Default::default()
            }
        }
    );
}

#[test]
fn emergency_disable_trading() {
    let emergency_owner = Addr::unchecked("miles_morales");
    let mut mock = MockEnv::new().emergency_owner(emergency_owner.as_str()).build().unwrap();

    let owner = mock.owner.clone();

    mock.set_price(&owner, "ueth", Decimal::one()).unwrap();

    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params("ueth"),
        },
    );
    let ms = mock.query_market_state("ueth");
    assert!(ms.market_state.enabled);

    mock.emergency_params_update(
        &emergency_owner,
        EmergencyUpdate::Perps(PerpsEmergencyUpdate::DisableTrading("ueth".to_string())),
    )
    .unwrap();
    let ms = mock.query_market_state("ueth");
    assert!(!ms.market_state.enabled);
}

#[test]
fn paginate_markets() {
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();
    let params_addr = mock.params.clone();

    mock.set_price(&owner, "uusdc", Decimal::from_str("1").unwrap()).unwrap();
    mock.set_price(&owner, "perp/osmo/usd", Decimal::from_str("1").unwrap()).unwrap();
    mock.set_price(&owner, "perp/ntrn/usd", Decimal::from_str("1").unwrap()).unwrap();

    mock.update_market(
        &params_addr,
        PerpParams {
            enabled: false,
            max_funding_velocity: Decimal::from_str("389").unwrap(),
            skew_scale: Uint128::new(1234000u128),
            ..default_perp_params("perp/osmo/usd")
        },
    )
    .unwrap();

    mock.update_market(
        &params_addr,
        PerpParams {
            enabled: true,
            max_funding_velocity: Decimal::from_str("100").unwrap(),
            skew_scale: Uint128::new(23400u128),
            ..default_perp_params("perp/ntrn/usd")
        },
    )
    .unwrap();

    let dss = mock.query_markets(None, None);
    assert_eq!(dss.data.len(), 2);
    let dss = dss.data.into_map();
    assert_eq!(
        dss.get("perp/osmo/usd").unwrap(),
        &MarketResponse {
            denom: "perp/osmo/usd".to_string(),
            enabled: false,
            ..Default::default()
        }
    );
    assert_eq!(
        dss.get("perp/ntrn/usd").unwrap(),
        &MarketResponse {
            denom: "perp/ntrn/usd".to_string(),
            enabled: true,
            ..Default::default()
        }
    );
}

#[test]
fn funding_change_accordingly_to_market_state_modification() {
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
        None,
        &[coin(1_000_000_000_000u128, "uusdc")],
    )
    .unwrap();

    // prepare market state
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
        Int128::from_str("300").unwrap(),
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

    // query market state for h0
    let ds_h0 = mock.query_market_state("ueth");

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
    let ds_h24 = mock.query_market_state("ueth");
    assert_eq!(
        ds_h24,
        MarketStateResponse {
            denom: ds_h0.denom.clone(),
            market_state: MarketState {
                enabled: true,
                last_updated: ds_h0.market_state.last_updated + SECONDS_IN_DAY,
                ..ds_h0.market_state
            }
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
    let ds_h48 = mock.query_market_state("ueth");
    assert_ne!(
        ds_h48.market_state.funding.last_funding_rate,
        ds_h24.market_state.funding.last_funding_rate
    );
    assert_ne!(
        ds_h48.market_state.funding.last_funding_accrued_per_unit_in_base_denom,
        ds_h24.market_state.funding.last_funding_accrued_per_unit_in_base_denom
    );
    assert_eq!(
        ds_h48,
        MarketStateResponse {
            denom: ds_h24.denom.clone(),
            market_state: MarketState {
                enabled: false,
                last_updated: ds_h24.market_state.last_updated + SECONDS_IN_DAY,
                funding: Funding {
                    last_funding_rate: SignedDecimal::from_str("0.009").unwrap(),
                    last_funding_accrued_per_unit_in_base_denom: SignedDecimal::from_str("-9")
                        .unwrap(),
                    ..ds_h24.market_state.funding
                },
                ..ds_h24.market_state
            }
        }
    );
}

trait MarketStateResponseVecExt {
    fn into_map(self) -> HashMap<String, MarketResponse>;
}

impl MarketStateResponseVecExt for Vec<MarketResponse> {
    fn into_map(self) -> HashMap<String, MarketResponse> {
        self.into_iter().map(|ms| (ms.denom.clone(), ms)).collect()
    }
}
