use std::str::FromStr;

use cosmwasm_std::{Addr, Decimal, Uint128};
use mars_perps::error::ContractError;
use mars_types::{
    math::SignedDecimal,
    params::{PerpParams, PerpParamsUpdate},
};

use super::helpers::{assert_err, MockEnv};
use crate::tests::helpers::default_perp_params;

#[test]
fn random_user_cannot_open_position() {
    let mut mock = MockEnv::new().build().unwrap();

    let res = mock.open_position(
        &Addr::unchecked("random-user-123"),
        "2",
        "uatom",
        SignedDecimal::from_str("-125").unwrap(),
    );
    assert_err(res, ContractError::SenderIsNotCreditManager);
}

#[test]
fn cannot_open_position_for_disabled_denom() {
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();

    // set prices
    mock.set_price(&owner, "uusdc", Decimal::from_str("1").unwrap()).unwrap();
    mock.set_price(&owner, "uatom", Decimal::from_str("7.2").unwrap()).unwrap();

    // init denoms
    mock.init_denom(
        &owner,
        "uatom",
        Decimal::from_str("3").unwrap(),
        Decimal::from_str("1000000").unwrap(),
    )
    .unwrap();
    mock.disable_denom(&owner, "uatom").unwrap();

    let res =
        mock.open_position(&credit_manager, "2", "uatom", SignedDecimal::from_str("-125").unwrap());
    assert_err(
        res,
        ContractError::DenomNotEnabled {
            denom: "uatom".to_string(),
        },
    );
}

#[test]
fn only_one_position_possible_for_denom() {
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();

    // set prices
    mock.set_price(&owner, "uusdc", Decimal::from_str("1").unwrap()).unwrap();
    mock.set_price(&owner, "uatom", Decimal::from_str("7.2").unwrap()).unwrap();

    // init denoms
    mock.init_denom(
        &owner,
        "uatom",
        Decimal::from_str("3").unwrap(),
        Decimal::from_str("1000000").unwrap(),
    )
    .unwrap();
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params("uatom"),
        },
    );

    // open a position for account 2
    mock.open_position(&credit_manager, "2", "uatom", SignedDecimal::from_str("-125").unwrap())
        .unwrap();

    // try to open one more time
    let res =
        mock.open_position(&credit_manager, "2", "uatom", SignedDecimal::from_str("-125").unwrap());
    assert_err(
        res,
        ContractError::PositionExists {
            account_id: "2".to_string(),
            denom: "uatom".to_string(),
        },
    );
}

#[test]
fn open_position_cannot_be_too_small() {
    let min_position_in_base_denom = Uint128::new(1251u128);
    let mut mock =
        MockEnv::new().min_position_in_base_denom(min_position_in_base_denom).build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();

    // set prices
    mock.set_price(&owner, "uusdc", Decimal::from_str("0.8").unwrap()).unwrap();
    mock.set_price(&owner, "uatom", Decimal::from_str("10").unwrap()).unwrap();

    // init denoms
    mock.init_denom(
        &owner,
        "uatom",
        Decimal::from_str("3").unwrap(),
        Decimal::from_str("1000000").unwrap(),
    )
    .unwrap();
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params("uatom"),
        },
    );

    // position size is too small
    // 100 * 10 / 0.8 = 1250
    let res =
        mock.open_position(&credit_manager, "2", "uatom", SignedDecimal::from_str("100").unwrap());
    assert_err(
        res,
        ContractError::PositionTooSmall {
            min: min_position_in_base_denom,
            found: min_position_in_base_denom - Uint128::one(),
            base_denom: "uusdc".to_string(),
        },
    );
}

#[test]
fn open_position_cannot_be_too_big() {
    let max_position_in_base_denom = Uint128::new(1249u128);
    let mut mock = MockEnv::new()
        .min_position_in_base_denom(Uint128::zero())
        .max_position_in_base_denom(Some(max_position_in_base_denom))
        .build()
        .unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();

    // set prices
    mock.set_price(&owner, "uusdc", Decimal::from_str("0.8").unwrap()).unwrap();
    mock.set_price(&owner, "uatom", Decimal::from_str("10").unwrap()).unwrap();

    // init denoms
    mock.init_denom(
        &owner,
        "uatom",
        Decimal::from_str("3").unwrap(),
        Decimal::from_str("1000000").unwrap(),
    )
    .unwrap();
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params("uatom"),
        },
    );

    // position size is too big
    // 100 * 10 / 0.8 = 1250
    let res =
        mock.open_position(&credit_manager, "2", "uatom", SignedDecimal::from_str("100").unwrap());
    assert_err(
        res,
        ContractError::PositionTooBig {
            max: max_position_in_base_denom,
            found: max_position_in_base_denom + Uint128::one(),
            base_denom: "uusdc".to_string(),
        },
    );
}

#[test]
fn validate_opening_position() {
    let mut mock = MockEnv::new()
        .min_position_in_base_denom(Uint128::zero())
        .max_position_in_base_denom(None)
        .build()
        .unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();

    // set prices
    mock.set_price(&owner, "uusdc", Decimal::from_str("0.8").unwrap()).unwrap();
    mock.set_price(&owner, "uatom", Decimal::from_str("10").unwrap()).unwrap();

    // init denoms
    mock.init_denom(
        &owner,
        "uatom",
        Decimal::from_str("3").unwrap(),
        Decimal::from_str("1000000").unwrap(),
    )
    .unwrap();
    let max_net_oi = Uint128::new(500);
    let max_long_oi = Uint128::new(4000);
    let max_short_oi = Uint128::new(4200);
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                denom: "uatom".to_string(),
                max_net_oi,
                max_long_oi,
                max_short_oi,
            },
        },
    );

    // prepare some OI
    mock.open_position(&credit_manager, "1", "uatom", SignedDecimal::from_str("300").unwrap())
        .unwrap();
    mock.open_position(&credit_manager, "2", "uatom", SignedDecimal::from_str("-400").unwrap())
        .unwrap();

    // long OI is too big
    let res =
        mock.open_position(&credit_manager, "3", "uatom", SignedDecimal::from_str("3701").unwrap()); // 300 + 3701 = 4001
    assert_err(
        res,
        ContractError::LongOpenInterestReached {
            max: max_long_oi,
            found: max_long_oi + Uint128::one(),
        },
    );

    // net OI is too big
    let res =
        mock.open_position(&credit_manager, "3", "uatom", SignedDecimal::from_str("601").unwrap()); // 300 + 601 = 901, abs(901 - 400) = 501
    assert_err(
        res,
        ContractError::NetOpenInterestReached {
            max: max_net_oi,
            found: max_net_oi + Uint128::one(),
        },
    );

    // short OI is too big
    let res = mock.open_position(
        &credit_manager,
        "4",
        "uatom",
        SignedDecimal::from_str("-3801").unwrap(),
    ); // 400 + 3801 = 4201
    assert_err(
        res,
        ContractError::ShortOpenInterestReached {
            max: max_short_oi,
            found: max_short_oi + Uint128::one(),
        },
    );

    // net OI is too big
    let res =
        mock.open_position(&credit_manager, "4", "uatom", SignedDecimal::from_str("-401").unwrap()); // 400 + 401 = 801, abs(300 - 801) = 501
    assert_err(
        res,
        ContractError::NetOpenInterestReached {
            max: max_net_oi,
            found: max_net_oi + Uint128::one(),
        },
    );
}
