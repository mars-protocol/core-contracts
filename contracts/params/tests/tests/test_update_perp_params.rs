use cosmwasm_std::{Addr, Uint128};
use mars_owner::OwnerError;
use mars_params::error::ContractError::Owner;
use mars_types::params::{PerpParams, PerpParamsUpdate};

use super::helpers::{assert_contents_equal, assert_err, default_perp_params, MockEnv};

#[test]
fn initial_state_of_perp_params() {
    let mock = MockEnv::new().build().unwrap();
    let params = mock.query_all_perp_params(None, None);
    assert!(params.is_empty());
}

#[test]
fn only_owner_can_update_perp_params() {
    let mut mock = MockEnv::new().build().unwrap();
    let bad_guy = Addr::unchecked("doctor_otto_983");
    let res = mock.update_perp_params(
        &bad_guy,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params("xyz"),
        },
    );
    assert_err(res, Owner(OwnerError::NotOwner {}));
}

#[test]
fn initializing_perp_params() {
    let mut mock = MockEnv::new().build().unwrap();
    let owner = mock.query_owner();
    let denom0 = "atom".to_string();
    let denom1 = "osmo".to_string();

    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params(&denom0),
        },
    )
    .unwrap();

    let all_perp_params = mock.query_all_perp_params(None, None);
    assert_eq!(1, all_perp_params.len());
    let res = all_perp_params.first().unwrap();
    assert_eq!(&denom0, &res.denom);

    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params(&denom1),
        },
    )
    .unwrap();

    let all_perp_params = mock.query_all_perp_params(None, None);
    assert_eq!(2, all_perp_params.len());
    assert_eq!(&denom1, &all_perp_params.get(1).unwrap().denom);
}

#[test]
fn add_same_perp_multiple_times() {
    let mut mock = MockEnv::new().build().unwrap();
    let owner = mock.query_owner();
    let denom0 = "atom".to_string();

    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params(&denom0),
        },
    )
    .unwrap();
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params(&denom0),
        },
    )
    .unwrap();

    let all_perp_params = mock.query_all_perp_params(None, None);
    assert_eq!(1, all_perp_params.len());
    assert_eq!(denom0, all_perp_params.first().unwrap().denom);
}

#[test]
fn update_existing_perp_params() {
    let mut mock = MockEnv::new().build().unwrap();
    let owner = mock.query_owner();
    let denom0 = "atom".to_string();

    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params(&denom0),
        },
    )
    .unwrap();

    let old_perp_params = mock.query_perp_params(&denom0);

    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                denom: denom0.clone(),
                max_net_oi: Uint128::new(1_888_999_000),
                max_long_oi: Uint128::new(1_123_000_000),
                max_short_oi: Uint128::new(1_321_000_000),
            },
        },
    )
    .unwrap();

    let all_perp_params = mock.query_all_perp_params(None, None);
    assert_eq!(1, all_perp_params.len());

    let perp_params = mock.query_perp_params(&denom0);
    assert_ne!(perp_params.max_net_oi, old_perp_params.max_net_oi);
    assert_ne!(perp_params.max_long_oi, old_perp_params.max_long_oi);
    assert_ne!(perp_params.max_short_oi, old_perp_params.max_short_oi);
    assert_eq!(perp_params.max_net_oi, Uint128::new(1_888_999_000));
    assert_eq!(perp_params.max_long_oi, Uint128::new(1_123_000_000));
    assert_eq!(perp_params.max_short_oi, Uint128::new(1_321_000_000));
}

#[test]
fn pagination_query() {
    let mut mock = MockEnv::new().build().unwrap();
    let owner = mock.query_owner();
    let denom0 = "atom".to_string();
    let denom1 = "osmo".to_string();
    let denom2 = "juno".to_string();
    let denom3 = "mars".to_string();
    let denom4 = "ion".to_string();
    let denom5 = "usdc".to_string();

    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params(&denom0),
        },
    )
    .unwrap();
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params(&denom1),
        },
    )
    .unwrap();
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params(&denom2),
        },
    )
    .unwrap();
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params(&denom3),
        },
    )
    .unwrap();
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params(&denom4),
        },
    )
    .unwrap();
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params(&denom5),
        },
    )
    .unwrap();

    let perp_params_a = mock.query_all_perp_params(None, Some(2));
    let perp_params_b =
        mock.query_all_perp_params(perp_params_a.last().map(|r| r.denom.clone()), Some(2));
    let perp_params_c =
        mock.query_all_perp_params(perp_params_b.last().map(|r| r.denom.clone()), None);

    let combined = perp_params_a
        .iter()
        .cloned()
        .chain(perp_params_b.iter().cloned())
        .chain(perp_params_c.iter().cloned())
        .map(|r| r.denom)
        .collect::<Vec<_>>();

    assert_eq!(6, combined.len());

    assert_contents_equal(&[denom0, denom1, denom2, denom3, denom4, denom5], &combined)
}
