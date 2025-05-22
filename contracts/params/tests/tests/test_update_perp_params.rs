use std::str::FromStr;

use cosmwasm_std::{Addr, Decimal, Uint128};
use mars_params::error::ContractError;
use mars_types::params::{PerpParams, PerpParamsUpdate};

use super::helpers::{assert_contents_equal, assert_err, default_perp_params, MockEnv};

#[test]
fn initial_state_of_perp_params() {
    let mock = MockEnv::new().build().unwrap();
    let params = mock.query_all_perp_params(None, None);
    assert!(params.is_empty());
}

#[test]
fn only_owner_can_init_perp_params() {
    let mut mock =
        MockEnv::new().build_with_risk_manager(Some("risk_manager_123".to_string())).unwrap();

    mock.set_price_source_fixed("xyz", Decimal::one());

    let bad_guy = Addr::unchecked("doctor_otto_983");
    let mut res = mock.update_perp_params(
        &bad_guy,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params("xyz"),
        },
    );
    assert_err(res, ContractError::NotOwnerOrRiskManager {});

    let risk_manager = mock.query_risk_manager();
    res = mock.update_perp_params(
        &risk_manager,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params("xyz"),
        },
    );
    assert_err(
        res,
        ContractError::RiskManagerUnauthorized {
            reason: "new perp".to_string(),
        },
    );

    let owner = mock.query_owner();
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params("xyz"),
        },
    )
    .unwrap();
}

#[test]
fn only_owner_and_risk_manager_can_update_perp_params() {
    let mut mock =
        MockEnv::new().build_with_risk_manager(Some("risk_manager_123".to_string())).unwrap();

    mock.set_price_source_fixed("xyz", Decimal::one());

    // Add perp param as owner
    mock.update_perp_params(
        &mock.query_owner(),
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params("xyz"),
        },
    )
    .unwrap();

    // Baddie can't update perp params
    let bad_guy = Addr::unchecked("doctor_otto_983");
    let res = mock.update_perp_params(
        &bad_guy,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params("xyz"),
        },
    );
    assert_err(res, ContractError::NotOwnerOrRiskManager {});

    // Risk Manager can update perp params
    let risk_manager = mock.query_risk_manager();
    mock.update_perp_params(
        &risk_manager,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params("xyz"),
        },
    )
    .unwrap();

    // Owner can update perp params
    let owner = mock.query_owner();
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params("xyz"),
        },
    )
    .unwrap();
}

#[test]
fn only_owner_can_update_perp_params_liquidation_threshold() {
    let mut mock =
        MockEnv::new().build_with_risk_manager(Some("risk_manager_123".to_string())).unwrap();

    mock.set_price_source_fixed("xyz", Decimal::one());

    // Add perp param as owner
    let mut params = default_perp_params("xyz");
    mock.update_perp_params(
        &mock.query_owner(),
        PerpParamsUpdate::AddOrUpdate {
            params: params.clone(),
        },
    )
    .unwrap();

    // Update the liq threshold from 0.7 to 0.99
    params.liquidation_threshold = Decimal::from_str("0.98").unwrap();

    // Fail updating as baddie
    let bad_guy = Addr::unchecked("doctor_otto_983");
    let res = mock.update_perp_params(
        &bad_guy,
        PerpParamsUpdate::AddOrUpdate {
            params: params.clone(),
        },
    );
    assert_err(res, ContractError::NotOwnerOrRiskManager {});

    // Fail updating as risk mananger if changing liq threshold
    let res = mock.update_perp_params(
        &mock.query_risk_manager(),
        PerpParamsUpdate::AddOrUpdate {
            params: params.clone(),
        },
    );
    assert_err(
        res,
        ContractError::RiskManagerUnauthorized {
            reason: "perp param liquidation threshold".to_string(),
        },
    );

    // Succeed updating as owner if changing liq threshold
    mock.update_perp_params(
        &mock.query_owner(),
        PerpParamsUpdate::AddOrUpdate {
            params: params.clone(),
        },
    )
    .unwrap();
}

#[test]
fn initializing_perp_params() {
    let mut mock = MockEnv::new().build().unwrap();
    let owner = mock.query_owner();
    let denom0 = "atom".to_string();
    let denom1 = "osmo".to_string();

    mock.set_price_source_fixed(&denom0, Decimal::one());
    mock.set_price_source_fixed(&denom1, Decimal::one());

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

    mock.set_price_source_fixed(&denom0, Decimal::one());

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

    mock.set_price_source_fixed(&denom0, Decimal::one());

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
                enabled: false,
                max_net_oi_value: Uint128::new(888_999_000),
                max_long_oi_value: Uint128::new(1_123_000_000),
                max_short_oi_value: Uint128::new(1_321_000_000),
                closing_fee_rate: Decimal::from_str("0.018").unwrap(),
                opening_fee_rate: Decimal::from_str("0.016").unwrap(),
                liquidation_threshold: Decimal::from_str("0.85").unwrap(),
                max_loan_to_value: Decimal::from_str("0.8").unwrap(),
                max_position_value: None,
                min_position_value: Uint128::zero(),
                max_funding_velocity: Decimal::from_str("36").unwrap(),
                skew_scale: Uint128::new(7227323000000),
            },
        },
    )
    .unwrap();

    let all_perp_params = mock.query_all_perp_params(None, None);
    assert_eq!(1, all_perp_params.len());

    let perp_params = mock.query_perp_params(&denom0);
    assert_ne!(perp_params.enabled, old_perp_params.enabled);
    assert_ne!(perp_params.max_net_oi_value, old_perp_params.max_net_oi_value);
    assert_ne!(perp_params.max_long_oi_value, old_perp_params.max_long_oi_value);
    assert_ne!(perp_params.max_short_oi_value, old_perp_params.max_short_oi_value);
    assert_ne!(perp_params.closing_fee_rate, old_perp_params.closing_fee_rate);
    assert_ne!(perp_params.opening_fee_rate, old_perp_params.opening_fee_rate);
    assert_ne!(perp_params.max_funding_velocity, old_perp_params.max_funding_velocity);
    assert_ne!(perp_params.skew_scale, old_perp_params.skew_scale);
    assert_eq!(perp_params.max_net_oi_value, Uint128::new(888_999_000));
    assert_eq!(perp_params.max_long_oi_value, Uint128::new(1_123_000_000));
    assert_eq!(perp_params.max_short_oi_value, Uint128::new(1_321_000_000));
    assert_eq!(perp_params.closing_fee_rate, Decimal::from_str("0.018").unwrap());
    assert_eq!(perp_params.opening_fee_rate, Decimal::from_str("0.016").unwrap());
    assert_eq!(perp_params.max_funding_velocity, Decimal::from_str("36").unwrap());
    assert_eq!(perp_params.skew_scale, Uint128::new(7227323000000));
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
    let denoms = [
        denom0.clone(),
        denom1.clone(),
        denom2.clone(),
        denom3.clone(),
        denom4.clone(),
        denom5.clone(),
    ];

    for denom in denoms.iter() {
        mock.set_price_source_fixed(denom, Decimal::one());
        mock.update_perp_params(
            &owner,
            PerpParamsUpdate::AddOrUpdate {
                params: default_perp_params(denom),
            },
        )
        .unwrap();
    }

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

#[test]
fn pagination_query_v2() {
    let mut mock = MockEnv::new().build().unwrap();
    let owner = mock.query_owner();
    let denom0 = "atom".to_string();
    let denom1 = "osmo".to_string();
    let denom2 = "juno".to_string();
    let denom3 = "mars".to_string();
    let denom4 = "ion".to_string();
    let denom5 = "usdc".to_string();
    let mut denoms = [
        denom0.clone(),
        denom1.clone(),
        denom2.clone(),
        denom3.clone(),
        denom4.clone(),
        denom5.clone(),
    ];
    denoms.sort();

    for denom in denoms.iter() {
        mock.set_price_source_fixed(denom, Decimal::one());
        mock.update_perp_params(
            &owner,
            PerpParamsUpdate::AddOrUpdate {
                params: default_perp_params(denom),
            },
        )
        .unwrap();
    }

    let perp_params_a_res = mock.query_all_perp_params_v2(None, Some(2));
    assert!(perp_params_a_res.metadata.has_more);
    assert_eq!(perp_params_a_res.data.len(), 2);
    let perp_params_b_res = mock
        .query_all_perp_params_v2(perp_params_a_res.data.last().map(|r| r.denom.clone()), Some(2));
    assert!(perp_params_b_res.metadata.has_more);
    assert_eq!(perp_params_b_res.data.len(), 2);
    let perp_params_c_res =
        mock.query_all_perp_params_v2(perp_params_b_res.data.last().map(|r| r.denom.clone()), None);
    assert!(!perp_params_c_res.metadata.has_more);
    assert_eq!(perp_params_c_res.data.len(), 2);

    let combined = perp_params_a_res
        .data
        .iter()
        .cloned()
        .chain(perp_params_b_res.data.iter().cloned())
        .chain(perp_params_c_res.data.iter().cloned())
        .map(|r| r.denom)
        .collect::<Vec<_>>();

    assert_eq!(combined.len(), 6);
    assert_eq!(&denoms, combined.as_slice());
}

#[test]
fn max_perp_params_reached() {
    let max_perp_params = 22;
    let mut mock = MockEnv::new().max_perp_params(max_perp_params).build().unwrap();
    let owner = mock.query_owner();

    for i in 0..max_perp_params {
        mock.set_price_source_fixed(&format!("denom{}", i), Decimal::one());
        mock.update_perp_params(
            &owner,
            PerpParamsUpdate::AddOrUpdate {
                params: default_perp_params(&format!("denom{}", i)),
            },
        )
        .unwrap();
    }
    let res = mock.query_all_perp_params_v2(None, Some(max_perp_params as u32 + 1));
    assert!(!res.metadata.has_more);
    assert_eq!(res.data.len(), max_perp_params as usize);

    // max_perp_params is already reached
    mock.set_price_source_fixed("uatom", Decimal::one());
    let res = mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params("uatom"),
        },
    );
    assert_err(
        res,
        ContractError::MaxPerpParamsReached {
            max: max_perp_params,
        },
    );
}

#[test]
fn can_not_update_perp_params_if_price_source_is_not_set() {
    let mut mock = MockEnv::new().build().unwrap();
    let owner = mock.query_owner();
    let denom0 = "atom".to_string();

    let res = mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params(&denom0),
        },
    );
    assert_err(
        res,
        ContractError::PriceSourceNotFound {
            denom: denom0.clone(),
        },
    );
}
