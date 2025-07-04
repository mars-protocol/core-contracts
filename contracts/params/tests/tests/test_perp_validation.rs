//

use std::str::FromStr;

use cosmwasm_std::{Decimal, Uint128};
use mars_params::error::ContractError;
use mars_types::{
    error::MarsError::Validation,
    params::{PerpParams, PerpParamsUpdate},
};
use mars_utils::error::ValidationError::InvalidParam;

use super::helpers::{assert_err, default_perp_params, MockEnv};

#[test]
fn liquidation_threshold_must_be_le_one() {
    let mut mock = MockEnv::new().build().unwrap();
    let denom = "btc/perp/usd".to_string();
    let res = mock.update_perp_params(
        &mock.query_owner(),
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                liquidation_threshold: Decimal::from_str("1.1").unwrap(),
                ..default_perp_params(&denom)
            },
        },
    );

    assert_err(
        res,
        ContractError::Mars(Validation(InvalidParam {
            param_name: "liquidation_threshold".to_string(),
            invalid_value: "1.1".to_string(),
            predicate: "<= 1".to_string(),
        })),
    );
}

#[test]
fn liquidation_threshold_usdc_must_be_le_one() {
    let mut mock = MockEnv::new().build().unwrap();
    let denom = "btc/perp/usd".to_string();
    let res = mock.update_perp_params(
        &mock.query_owner(),
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                liquidation_threshold_usdc: Some(Decimal::from_str("1.1").unwrap()),
                ..default_perp_params(&denom)
            },
        },
    );

    assert_err(
        res,
        ContractError::Mars(Validation(InvalidParam {
            param_name: "liquidation_threshold_usdc".to_string(),
            invalid_value: "1.1".to_string(),
            predicate: "<= 1".to_string(),
        })),
    );
}

#[test]
fn max_ltv_must_be_less_than_liquidation_threshold() {
    let mut mock = MockEnv::new().build().unwrap();
    let denom = "btc/perp/usd".to_string();
    let res = mock.update_perp_params(
        &mock.query_owner(),
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                liquidation_threshold: Decimal::from_str("0.95").unwrap(),
                max_loan_to_value: Decimal::from_str("0.96").unwrap(),
                ..default_perp_params(&denom)
            },
        },
    );

    assert_err(
        res,
        ContractError::Mars(Validation(InvalidParam {
            param_name: "liquidation_threshold".to_string(),
            invalid_value: "0.95".to_string(),
            predicate: "> 0.96 (max LTV)".to_string(),
        })),
    );
}

#[test]
fn max_loan_to_value_usdc_must_be_less_than_liquidation_threshold_usdc() {
    let mut mock = MockEnv::new().build().unwrap();
    let denom = "btc/perp/usd".to_string();
    let res = mock.update_perp_params(
        &mock.query_owner(),
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                liquidation_threshold_usdc: Some(Decimal::from_str("0.95").unwrap()),
                max_loan_to_value_usdc: Some(Decimal::from_str("0.96").unwrap()),
                ..default_perp_params(&denom)
            },
        },
    );

    assert_err(
        res,
        ContractError::Mars(Validation(InvalidParam {
            param_name: "liquidation_threshold_usdc".to_string(),
            invalid_value: "0.95".to_string(),
            predicate: "> 0.96 (max LTV USDC)".to_string(),
        })),
    );
}

#[test]
fn opening_fee_rate_must_be_less_than_one() {
    let mut mock = MockEnv::new().build().unwrap();
    let denom = "btc/perp/usd".to_string();
    let res = mock.update_perp_params(
        &mock.query_owner(),
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                opening_fee_rate: Decimal::from_str("1").unwrap(),
                ..default_perp_params(&denom)
            },
        },
    );

    assert_err(
        res,
        ContractError::Mars(Validation(InvalidParam {
            param_name: "opening_fee_rate".to_string(),
            invalid_value: "1".to_string(),
            predicate: "< 1".to_string(),
        })),
    );
}

#[test]
fn closing_fee_rate_must_be_less_than_one() {
    let mut mock = MockEnv::new().build().unwrap();
    let denom = "btc/perp/usd".to_string(); // Invalid native denom length
    let res = mock.update_perp_params(
        &mock.query_owner(),
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                closing_fee_rate: Decimal::from_str("1").unwrap(),
                ..default_perp_params(&denom)
            },
        },
    );

    assert_err(
        res,
        ContractError::Mars(Validation(InvalidParam {
            param_name: "closing_fee_rate".to_string(),
            invalid_value: "1".to_string(),
            predicate: "< 1".to_string(),
        })),
    );
}

#[test]
fn max_oi_long_must_be_ge_than_max_net_oi() {
    let mut mock = MockEnv::new().build().unwrap();
    let denom = "btc/perp/usd".to_string();
    let res = mock.update_perp_params(
        &mock.query_owner(),
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                max_net_oi_value: Uint128::new(1001),
                max_long_oi_value: Uint128::new(1000),
                ..default_perp_params(&denom)
            },
        },
    );

    assert_err(
        res,
        ContractError::Mars(Validation(InvalidParam {
            param_name: "max_long_oi_value".to_string(),
            invalid_value: "1000".to_string(),
            predicate: ">= 1001 (max_net_oi_value)".to_string(),
        })),
    );
}

#[test]
fn max_oi_short_must_be_ge_than_max_net_oi() {
    let mut mock = MockEnv::new().build().unwrap();
    let denom = "btc/perp/usd".to_string();
    let res = mock.update_perp_params(
        &mock.query_owner(),
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                max_net_oi_value: Uint128::new(1001),
                max_long_oi_value: Uint128::new(1002),
                max_short_oi_value: Uint128::new(1000),
                ..default_perp_params(&denom)
            },
        },
    );

    assert_err(
        res,
        ContractError::Mars(Validation(InvalidParam {
            param_name: "max_short_oi_value".to_string(),
            invalid_value: "1000".to_string(),
            predicate: ">= 1001 (max_net_oi_value)".to_string(),
        })),
    );
}

#[test]
fn max_size_cannot_be_less_than_min() {
    let mut mock = MockEnv::new().build().unwrap();
    let denom = "btc/perp/usd".to_string();
    let res = mock.update_perp_params(
        &mock.query_owner(),
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                max_position_value: Some(Uint128::one()),
                min_position_value: Uint128::new(2),
                ..default_perp_params(&denom)
            },
        },
    );

    assert_err(
        res,
        ContractError::Mars(Validation(InvalidParam {
            param_name: "max_position_value".to_string(),
            invalid_value: "1".to_string(),
            predicate: ">= 2 (min position value)".to_string(),
        })),
    );
}

#[test]
fn skew_scale_cannot_be_zero() {
    let mut mock = MockEnv::new().build().unwrap();
    let denom = "btc/perp/usd".to_string();
    let res = mock.update_perp_params(
        &mock.query_owner(),
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                skew_scale: Uint128::zero(),
                ..default_perp_params(&denom)
            },
        },
    );

    assert_err(
        res,
        ContractError::Mars(Validation(InvalidParam {
            param_name: "skew_scale".to_string(),
            invalid_value: "0".to_string(),
            predicate: "> 0".to_string(),
        })),
    );
}
