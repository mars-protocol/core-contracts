use std::str::FromStr;

use cosmwasm_std::{coin, Addr, Decimal, Uint128};
use mars_perps::{accounting::BalanceExt, error::ContractError};
use mars_types::{
    math::SignedDecimal,
    oracle::ActionKind,
    params::{PerpParams, PerpParamsUpdate},
    perps::{Accounting, Balance, CashFlow, PnL, PnlAmounts, PnlValues, PositionFeesResponse},
    signed_uint::SignedUint,
};
use test_case::test_case;

use super::helpers::{assert_err, MockEnv};
use crate::tests::helpers::default_perp_params;

#[test]
fn random_user_cannot_open_position() {
    let mut mock = MockEnv::new().build().unwrap();

    let res = mock.open_position(
        &Addr::unchecked("random-user-123"),
        "2",
        "uatom",
        SignedUint::from_str("-125").unwrap(),
        &[],
    );
    assert_err(res, ContractError::SenderIsNotCreditManager);
}

#[test]
fn random_user_cannot_modify_position() {
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let user = "jake";

    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000_000u128, &["uosmo", "uatom", "uusdc"]);

    // set prices
    mock.set_price(&owner, "uusdc", Decimal::from_str("1").unwrap()).unwrap();
    mock.set_price(&owner, "uatom", Decimal::from_str("7.2").unwrap()).unwrap();

    // deposit some big number of uusdc to vault
    mock.deposit_to_vault(&credit_manager, Some(user), &[coin(1_000_000_000_000u128, "uusdc")])
        .unwrap();

    // init denoms
    mock.init_denom(&owner, "uatom", Decimal::from_str("3").unwrap(), Uint128::new(1000000u128))
        .unwrap();

    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                opening_fee_rate: Decimal::percent(1),
                closing_fee_rate: Decimal::percent(1),
                ..default_perp_params("uatom")
            },
        },
    );

    let size = SignedUint::from_str("-125").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    mock.open_position(&credit_manager, "2", "uatom", size, &[atom_opening_fee]).unwrap();

    let res = mock.modify_position(
        &Addr::unchecked("random-user-123"),
        "2",
        "uatom",
        SignedUint::from_str("-125").unwrap(),
        &[],
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
    mock.init_denom(&owner, "uatom", Decimal::from_str("3").unwrap(), Uint128::new(1000000u128))
        .unwrap();
    mock.disable_denom(&owner, "uatom").unwrap();

    let res = mock.open_position(
        &credit_manager,
        "2",
        "uatom",
        SignedUint::from_str("-125").unwrap(),
        &[],
    );
    assert_err(
        res,
        ContractError::DenomNotEnabled {
            denom: "uatom".to_string(),
        },
    );
}

#[test]
fn cannot_modify_position_for_disabled_denom() {
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let user = "jake";

    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000_000u128, &["uosmo", "uatom", "uusdc"]);

    // set prices
    mock.set_price(&owner, "uusdc", Decimal::from_str("1").unwrap()).unwrap();
    mock.set_price(&owner, "uatom", Decimal::from_str("7.2").unwrap()).unwrap();

    // deposit some big number of uusdc to vault
    mock.deposit_to_vault(&credit_manager, Some(user), &[coin(1_000_000_000_000u128, "uusdc")])
        .unwrap();

    // init denoms
    mock.init_denom(&owner, "uatom", Decimal::from_str("3").unwrap(), Uint128::new(1000000u128))
        .unwrap();

    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                opening_fee_rate: Decimal::percent(1),
                closing_fee_rate: Decimal::percent(1),
                ..default_perp_params("uatom")
            },
        },
    );

    let size = SignedUint::from_str("-125").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    mock.open_position(&credit_manager, "2", "uatom", size, &[atom_opening_fee]).unwrap();

    mock.disable_denom(&owner, "uatom").unwrap();

    // increase position
    let res = mock.modify_position(
        &credit_manager,
        "2",
        "uatom",
        SignedUint::from_str("-175").unwrap(),
        &[], // fees are not important for this test
    );
    assert_err(
        res,
        ContractError::DenomNotEnabled {
            denom: "uatom".to_string(),
        },
    );

    // decrease position
    let res = mock.modify_position(
        &credit_manager,
        "2",
        "uatom",
        SignedUint::from_str("-100").unwrap(),
        &[], // fees are not important for this test
    );
    assert_err(
        res,
        ContractError::PositionCannotBeModifiedIfDenomDisabled {
            denom: "uatom".to_string(),
        },
    );
}

#[test]
fn only_close_position_possible_for_disabled_denom() {
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let user = "jake";

    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000_000u128, &["uosmo", "uatom", "uusdc"]);

    // set prices
    mock.set_price(&owner, "uusdc", Decimal::from_str("1").unwrap()).unwrap();
    mock.set_price(&owner, "uatom", Decimal::from_str("7.2").unwrap()).unwrap();

    // deposit some big number of uusdc to vault
    mock.deposit_to_vault(&credit_manager, Some(user), &[coin(1_000_000_000_000u128, "uusdc")])
        .unwrap();

    // init denoms
    mock.init_denom(&owner, "uatom", Decimal::from_str("3").unwrap(), Uint128::new(1000000u128))
        .unwrap();

    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                opening_fee_rate: Decimal::percent(1),
                closing_fee_rate: Decimal::percent(1),
                ..default_perp_params("uatom")
            },
        },
    );

    let size = SignedUint::from_str("125").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    mock.open_position(&credit_manager, "2", "uatom", size, &[atom_opening_fee]).unwrap();

    mock.disable_denom(&owner, "uatom").unwrap();

    mock.set_price(&owner, "uatom", Decimal::from_str("10.2").unwrap()).unwrap();

    mock.close_position(&credit_manager, "2", "uatom", &[]).unwrap();
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
    mock.init_denom(&owner, "uatom", Decimal::from_str("3").unwrap(), Uint128::new(1000000u128))
        .unwrap();
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params("uatom"),
        },
    );

    // open a position for account 2
    mock.open_position(&credit_manager, "2", "uatom", SignedUint::from_str("-125").unwrap(), &[])
        .unwrap();

    // try to open one more time
    let res = mock.open_position(
        &credit_manager,
        "2",
        "uatom",
        SignedUint::from_str("-125").unwrap(),
        &[],
    );
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
    let min_position_value = Uint128::new(1251u128);
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();

    // set prices
    mock.set_price(&owner, "uusdc", Decimal::from_str("0.8").unwrap()).unwrap();
    mock.set_price(&owner, "uatom", Decimal::from_str("12.5").unwrap()).unwrap();

    // init denoms
    mock.init_denom(&owner, "uatom", Decimal::from_str("3").unwrap(), Uint128::new(1000000u128))
        .unwrap();
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                min_position_value,
                ..default_perp_params("uatom")
            },
        },
    );

    // position size is too small
    // 100 * 12.5 = 1250
    let res = mock.open_position(
        &credit_manager,
        "2",
        "uatom",
        SignedUint::from_str("100").unwrap(),
        &[],
    );
    assert_err(
        res,
        ContractError::PositionTooSmall {
            min: min_position_value,
            found: min_position_value - Uint128::one(),
        },
    );
}

#[test]
fn max_open_perps_reached() {
    let mut mock = MockEnv::new().max_positions(2).build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();

    // set prices
    mock.set_price(&owner, "uusdc", Decimal::from_str("1").unwrap()).unwrap();
    mock.set_price(&owner, "uatom", Decimal::from_str("7.2").unwrap()).unwrap();
    mock.set_price(&owner, "utia", Decimal::from_str("10.5").unwrap()).unwrap();
    mock.set_price(&owner, "untrn", Decimal::from_str("1.5").unwrap()).unwrap();

    // init denoms
    mock.init_denom(&owner, "uatom", Decimal::from_str("3").unwrap(), Uint128::new(1000000u128))
        .unwrap();
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params("uatom"),
        },
    );
    mock.init_denom(&owner, "utia", Decimal::from_str("3").unwrap(), Uint128::new(1000000u128))
        .unwrap();
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params("utia"),
        },
    );
    mock.init_denom(&owner, "untrn", Decimal::from_str("3").unwrap(), Uint128::new(1000000u128))
        .unwrap();
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params("untrn"),
        },
    );

    // open a position for account 2
    mock.open_position(&credit_manager, "2", "uatom", SignedUint::from_str("-125").unwrap(), &[])
        .unwrap();
    mock.open_position(&credit_manager, "2", "utia", SignedUint::from_str("100").unwrap(), &[])
        .unwrap();

    // try to open third position
    let res = mock.open_position(
        &credit_manager,
        "2",
        "untrn",
        SignedUint::from_str("-125").unwrap(),
        &[],
    );
    assert_err(
        res,
        ContractError::MaxPositionsReached {
            account_id: "2".to_string(),
            max_positions: 2,
        },
    );
}

#[test]
fn reduced_position_cannot_be_too_small() {
    let min_position_value = Uint128::new(1251u128);
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let user = "jake";

    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000_000u128, &["uosmo", "uatom", "uusdc"]);

    // set prices
    mock.set_price(&owner, "uusdc", Decimal::from_str("0.8").unwrap()).unwrap();
    mock.set_price(&owner, "uatom", Decimal::from_str("12.5").unwrap()).unwrap();

    // deposit some big number of uusdc to vault
    mock.deposit_to_vault(&credit_manager, Some(user), &[coin(1_000_000_000_000u128, "uusdc")])
        .unwrap();

    // init denoms
    mock.init_denom(&owner, "uatom", Decimal::from_str("3").unwrap(), Uint128::new(1000000u128))
        .unwrap();
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                min_position_value,
                opening_fee_rate: Decimal::percent(1),
                closing_fee_rate: Decimal::percent(1),
                ..default_perp_params("uatom")
            },
        },
    );

    // create valid position
    let size = SignedUint::from_str("200").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    mock.open_position(&credit_manager, "2", "uatom", size, &[atom_opening_fee]).unwrap();

    // Position size is too small
    let res = mock.modify_position(
        &credit_manager,
        "2",
        "uatom",
        SignedUint::from_str("100").unwrap(),
        &[],
    );

    assert_err(
        res,
        ContractError::PositionTooSmall {
            min: min_position_value,
            found: min_position_value - Uint128::one(),
        },
    );
}

#[test]
fn open_position_cannot_be_too_big() {
    let max_position_value = Uint128::new(1249u128);
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();

    // set prices
    mock.set_price(&owner, "uusdc", Decimal::from_str("0.8").unwrap()).unwrap();
    mock.set_price(&owner, "uatom", Decimal::from_str("12.5").unwrap()).unwrap();

    // init denoms
    mock.init_denom(&owner, "uatom", Decimal::from_str("3").unwrap(), Uint128::new(1000000u128))
        .unwrap();
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                max_position_value: Some(max_position_value),
                ..default_perp_params("uatom")
            },
        },
    );

    // position size is too big
    // 100 * 12.5 = 1250
    let res = mock.open_position(
        &credit_manager,
        "2",
        "uatom",
        SignedUint::from_str("100").unwrap(),
        &[],
    );
    assert_err(
        res,
        ContractError::PositionTooBig {
            max: max_position_value,
            found: max_position_value + Uint128::one(),
        },
    );
}

#[test]
fn increased_position_cannot_be_too_big() {
    let max_position_value = Uint128::new(1249u128);
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let user = "jake";

    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000_000u128, &["uosmo", "uatom", "uusdc"]);

    // set prices
    mock.set_price(&owner, "uusdc", Decimal::from_str("0.8").unwrap()).unwrap();
    mock.set_price(&owner, "uatom", Decimal::from_str("12.5").unwrap()).unwrap();

    // deposit some big number of uusdc to vault
    mock.deposit_to_vault(&credit_manager, Some(user), &[coin(1_000_000_000_000u128, "uusdc")])
        .unwrap();

    // init denoms
    mock.init_denom(&owner, "uatom", Decimal::from_str("3").unwrap(), Uint128::new(1000000u128))
        .unwrap();
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                max_position_value: Some(max_position_value),
                opening_fee_rate: Decimal::percent(1),
                closing_fee_rate: Decimal::percent(1),
                ..default_perp_params("uatom")
            },
        },
    );

    // position size is too big
    // 100 * 12.5 = 1250
    let size = SignedUint::from_str("50").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    mock.open_position(&credit_manager, "2", "uatom", size, &[atom_opening_fee]).unwrap();

    let res = mock.modify_position(
        &credit_manager,
        "2",
        "uatom",
        SignedUint::from_str("100").unwrap(),
        &[], // fees are not important for this test
    );
    assert_err(
        res,
        ContractError::PositionTooBig {
            max: max_position_value,
            found: max_position_value + Uint128::one(),
        },
    );
}

#[test]
fn validate_opening_position() {
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();

    // set prices
    mock.set_price(&owner, "uusdc", Decimal::from_str("0.8").unwrap()).unwrap();
    mock.set_price(&owner, "uatom", Decimal::from_str("10").unwrap()).unwrap();

    // init denoms
    mock.init_denom(&owner, "uatom", Decimal::from_str("3").unwrap(), Uint128::new(1000000u128))
        .unwrap();
    let max_net_oi = Uint128::new(2009);
    let max_long_oi = Uint128::new(6029);
    let max_short_oi = Uint128::new(5009);
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                max_net_oi_value: max_net_oi,
                max_long_oi_value: max_long_oi,
                max_short_oi_value: max_short_oi,
                ..default_perp_params("uatom")
            },
        },
    );

    // prepare some OI
    mock.open_position(&credit_manager, "1", "uatom", SignedUint::from_str("200").unwrap(), &[])
        .unwrap();
    mock.open_position(&credit_manager, "2", "uatom", SignedUint::from_str("-400").unwrap(), &[])
        .unwrap();

    // long OI is too big
    let res = mock.open_position(
        &credit_manager,
        "3",
        "uatom",
        SignedUint::from_str("403").unwrap(),
        &[],
    ); // (200 + 403) * 10 = 6030
    assert_err(
        res,
        ContractError::LongOpenInterestReached {
            max: max_long_oi,
            found: max_long_oi + Uint128::one(),
        },
    );

    // net OI is too big
    let res = mock.open_position(
        &credit_manager,
        "3",
        "uatom",
        SignedUint::from_str("401").unwrap(),
        &[],
    ); // 200 + 401 = 601, abs(601 - 400) = 201 * 10 = 2010
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
        SignedUint::from_str("-101").unwrap(),
        &[],
    ); // (400 + 101) * 10 = 5010
    assert_err(
        res,
        ContractError::ShortOpenInterestReached {
            max: max_short_oi,
            found: max_short_oi + Uint128::one(),
        },
    );

    // net OI is too big
    let res =
        mock.open_position(&credit_manager, "4", "uatom", SignedUint::from_str("-1").unwrap(), &[]); // 400 + 1 = 401, abs(200 - 401) = 201 * 10 = 2010
    assert_err(
        res,
        ContractError::NetOpenInterestReached {
            max: max_net_oi,
            found: max_net_oi + Uint128::one(),
        },
    );
}

#[test]
fn validate_modify_position() {
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let user = "jake";

    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000_000u128, &["uosmo", "uatom", "uusdc"]);

    // set prices
    mock.set_price(&owner, "uusdc", Decimal::from_str("0.8").unwrap()).unwrap();
    mock.set_price(&owner, "uatom", Decimal::from_str("10").unwrap()).unwrap();

    // deposit some big number of uusdc to vault
    mock.deposit_to_vault(&credit_manager, Some(user), &[coin(1_000_000_000_000u128, "uusdc")])
        .unwrap();

    // init denoms
    mock.init_denom(&owner, "uatom", Decimal::from_str("3").unwrap(), Uint128::new(1000000u128))
        .unwrap();
    let max_net_oi = Uint128::new(509);
    let max_long_oi = Uint128::new(4009);
    let max_short_oi = Uint128::new(4209);

    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                max_net_oi_value: max_net_oi,
                max_long_oi_value: max_long_oi,
                max_short_oi_value: max_short_oi,
                opening_fee_rate: Decimal::percent(1),
                closing_fee_rate: Decimal::percent(1),
                ..default_perp_params("uatom")
            },
        },
    );

    // prepare some OI
    let size = SignedUint::from_str("30").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    mock.open_position(&credit_manager, "1", "uatom", size, &[atom_opening_fee]).unwrap();
    let size = SignedUint::from_str("-40").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    mock.open_position(&credit_manager, "2", "uatom", size, &[atom_opening_fee]).unwrap();

    // long OI is too big
    let res = mock.modify_position(
        &credit_manager,
        "1",
        "uatom",
        SignedUint::from_str("401").unwrap(),
        &[],
    );
    assert_err(
        res,
        ContractError::LongOpenInterestReached {
            max: max_long_oi,
            found: max_long_oi + Uint128::one(),
        },
    );

    // net OI is too big
    let res = mock.modify_position(
        &credit_manager,
        "1",
        "uatom",
        SignedUint::from_str("91").unwrap(),
        &[],
    ); // abs(91 - 40) = 51 * 10 = 510
    assert_err(
        res,
        ContractError::NetOpenInterestReached {
            max: max_net_oi,
            found: max_net_oi + Uint128::one(),
        },
    );

    // short OI is too big
    let res = mock.modify_position(
        &credit_manager,
        "2",
        "uatom",
        SignedUint::from_str("-421").unwrap(),
        &[],
    );
    assert_err(
        res,
        ContractError::ShortOpenInterestReached {
            max: max_short_oi,
            found: max_short_oi + Uint128::one(),
        },
    );

    // net OI is too big
    let res = mock.modify_position(
        &credit_manager,
        "2",
        "uatom",
        SignedUint::from_str("-81").unwrap(),
        &[],
    ); // abs(30 - 81) = 51 * 10 = 510
    assert_err(
        res,
        ContractError::NetOpenInterestReached {
            max: max_net_oi,
            found: max_net_oi + Uint128::one(),
        },
    );
}

#[test]
fn modify_position_realises_pnl() {
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let user = "jake";

    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000_000u128, &["uosmo", "uatom", "uusdc"]);

    // set prices
    mock.set_price(&owner, "uusdc", Decimal::from_str("1").unwrap()).unwrap();
    mock.set_price(&owner, "uatom", Decimal::from_str("10").unwrap()).unwrap();

    // deposit some big number of uusdc to vault
    mock.deposit_to_vault(&credit_manager, Some(user), &[coin(1_000_000_000_000u128, "uusdc")])
        .unwrap();

    // init denoms
    mock.init_denom(&owner, "uatom", Decimal::from_str("3").unwrap(), Uint128::new(1000000u128))
        .unwrap();
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                closing_fee_rate: Decimal::from_str("0.01").unwrap(),
                opening_fee_rate: Decimal::from_str("0.01").unwrap(),
                ..default_perp_params("uatom")
            },
        },
    );

    // prepare some OI
    let size = SignedUint::from_str("300").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    mock.open_position(&credit_manager, "1", "uatom", size, &[atom_opening_fee.clone()]).unwrap();

    // update price - we are now up 10%
    mock.set_price(&owner, "uatom", Decimal::from_str("11").unwrap()).unwrap();

    // how much opening fee we will pay for increase from 300 to 400
    let atom_opening_fee_for_increase =
        mock.query_opening_fee("uatom", SignedUint::from_str("100").unwrap()).fee;

    // modify and verify that our pnl is realised
    mock.modify_position(&credit_manager, "1", "uatom", SignedUint::from_str("400").unwrap(), &[])
        .unwrap();

    let position = mock.query_position("1", "uatom");

    let atom_opening_fee_total = atom_opening_fee.amount + atom_opening_fee_for_increase.amount;
    let atom_opening_fee_total =
        SignedDecimal::zero().checked_sub(atom_opening_fee_total.into()).unwrap(); // make it negative because it's a cost
    assert_eq!(atom_opening_fee_total, SignedDecimal::from_str("-43").unwrap());
    assert_eq!(
        position.position.realised_pnl,
        PnlAmounts {
            accrued_funding: SignedUint::zero(),
            price_pnl: SignedUint::from_str("300").unwrap(),
            // opening_fee: atom_opening_fee_total,
            opening_fee: SignedUint::from_str("-43").unwrap(), // rounding error
            closing_fee: SignedUint::zero(), // increased position does not have closing fee
            pnl: SignedUint::from_str("257").unwrap(),
        }
    );

    // update price - we fall back to 10
    mock.set_price(&owner, "uatom", Decimal::from_str("10.5").unwrap()).unwrap();

    mock.modify_position(
        &credit_manager,
        "1",
        "uatom",
        SignedUint::from_str("300").unwrap(),
        &[coin(213u128, "uusdc")],
    )
    .unwrap();

    let position = mock.query_position("1", "uatom");

    assert_eq!(
        position.position.realised_pnl,
        PnlAmounts {
            accrued_funding: SignedUint::zero(),
            price_pnl: SignedUint::from_str("98").unwrap(),
            // opening_fee: atom_opening_fee_total, // we are not paying opening fee for decrease
            opening_fee: SignedUint::from_str("-43").unwrap(), // rounding error
            closing_fee: SignedUint::from_str("-11").unwrap(),
            pnl: SignedUint::from_str("44").unwrap(),
        }
    );
}

#[test_case(
    None,
    SignedUint::from_str("250").unwrap(),
    PositionFeesResponse {
        base_denom: "uusdc".to_string(),
        opening_fee: Uint128::new(2u128),
        closing_fee: Uint128::zero(),
        opening_exec_price: Some(Decimal::from_str("1.26265625").unwrap()),
        closing_exec_price: None,
    };
    "open long"
)]
#[test_case(
    Some(SignedUint::from_str("1200").unwrap()),
    SignedUint::from_str("2500").unwrap(),
    PositionFeesResponse {
        base_denom: "uusdc".to_string(),
        opening_fee: Uint128::new(8u128),
        closing_fee: Uint128::zero(),
        opening_exec_price: Some(Decimal::from_str("1.2655625").unwrap()),
        closing_exec_price: Some(Decimal::from_str("1.26325").unwrap()),
    };
    "increase long"
)]
#[test_case(
    Some(SignedUint::from_str("1200").unwrap()),
    SignedUint::from_str("800").unwrap(),
    PositionFeesResponse {
        base_denom: "uusdc".to_string(),
        opening_fee: Uint128::zero(),
        closing_fee: Uint128::new(4u128),
        opening_exec_price: Some(Decimal::from_str("1.2645").unwrap()),
        closing_exec_price: Some(Decimal::from_str("1.26325").unwrap()),
    };
    "decrease long"
)]
#[test_case(
    Some(SignedUint::from_str("1200").unwrap()),
    SignedUint::from_str("0").unwrap(),
    PositionFeesResponse {
        base_denom: "uusdc".to_string(),
        opening_fee: Uint128::zero(),
        closing_fee: Uint128::new(11u128),
        opening_exec_price: None,
        closing_exec_price: Some(Decimal::from_str("1.26325").unwrap()),
    };
    "close long"
)]
#[test_case(
    None,
    SignedUint::from_str("-2500").unwrap(),
    PositionFeesResponse {
        base_denom: "uusdc".to_string(),
        opening_fee: Uint128::new(15u128),
        closing_fee: Uint128::zero(),
        opening_exec_price: Some(Decimal::from_str("1.2609375").unwrap()),
        closing_exec_price: None,
    };
    "open short"
)]
#[test_case(
    Some(SignedUint::from_str("-1200").unwrap()),
    SignedUint::from_str("-2500").unwrap(),
    PositionFeesResponse {
        base_denom: "uusdc".to_string(),
        opening_fee: Uint128::new(8u128),
        closing_fee: Uint128::zero(),
        opening_exec_price: Some(Decimal::from_str("1.2594375").unwrap()),
        closing_exec_price: Some(Decimal::from_str("1.26175").unwrap()),
    };
    "increase short"
)]
#[test_case(
    Some(SignedUint::from_str("-1200").unwrap()),
    SignedUint::from_str("-600").unwrap(),
    PositionFeesResponse {
        base_denom: "uusdc".to_string(),
        opening_fee: Uint128::zero(),
        closing_fee: Uint128::new(6u128),
        opening_exec_price: Some(Decimal::from_str("1.260625").unwrap()),
        closing_exec_price: Some(Decimal::from_str("1.26175").unwrap()),
    };
    "decrease short"
)]
#[test_case(
    Some(SignedUint::from_str("-1200").unwrap()),
    SignedUint::from_str("0").unwrap(),
    PositionFeesResponse {
        base_denom: "uusdc".to_string(),
        opening_fee: Uint128::zero(),
        closing_fee: Uint128::new(11u128),
        opening_exec_price: None,
        closing_exec_price: Some(Decimal::from_str("1.26175").unwrap()),
    };
    "close short"
)]
fn query_position_fees(
    old_size: Option<SignedUint>,
    new_size: SignedUint,
    expected_fees: PositionFeesResponse,
) {
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let user = "jake";

    // set prices
    mock.set_price(&owner, "uusdc", Decimal::from_str("0.9").unwrap()).unwrap();
    mock.set_price(&owner, "uosmo", Decimal::from_str("1.25").unwrap()).unwrap();

    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000_000u128, &["uosmo", "uusdc"]);

    // deposit some big number of uusdc to vault
    mock.deposit_to_vault(&credit_manager, Some(user), &[coin(1_000_000_000_000u128, "uusdc")])
        .unwrap();

    // init denoms
    mock.init_denom(&owner, "uosmo", Decimal::from_str("3").unwrap(), Uint128::new(1000000u128))
        .unwrap();
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                opening_fee_rate: Decimal::from_str("0.004").unwrap(),
                closing_fee_rate: Decimal::from_str("0.006").unwrap(),
                ..default_perp_params("uosmo")
            },
        },
    );

    // open a position to change skew
    let size = SignedUint::from_str("10000").unwrap();
    let opening_fee = mock.query_opening_fee("uosmo", size).fee;
    mock.open_position(&credit_manager, "2", "uosmo", size, &[opening_fee]).unwrap();

    // open a position if specified
    if let Some(old_size) = old_size {
        let opening_fee = mock.query_opening_fee("uosmo", old_size).fee;
        mock.open_position(&credit_manager, "1", "uosmo", old_size, &[opening_fee]).unwrap();
    }

    // check expected fees
    let position_fees = mock.query_position_fees("1", "uosmo", new_size);
    assert_eq!(position_fees, expected_fees);
}

#[test]
fn random_user_cannot_close_all_positions() {
    let mut mock = MockEnv::new().build().unwrap();

    let res = mock.close_all_positions(&Addr::unchecked("random-user-123"), "2", &[]);
    assert_err(res, ContractError::SenderIsNotCreditManager);
}

#[test_case(
    Decimal::from_str("14").unwrap(),
    Decimal::from_str("1").unwrap(),
    Decimal::from_str("4").unwrap(),
    SignedDecimal::from_str("1").unwrap();
    "close all positions with profit"
)]
#[test_case(
    Decimal::from_str("8").unwrap(),
    Decimal::from_str("1").unwrap(),
    Decimal::from_str("1").unwrap(),
    SignedDecimal::from_str("-1").unwrap();
    "close all positions with loss"
)]
#[test_case(
    Decimal::from_str("10").unwrap(),
    Decimal::from_str("2").unwrap(),
    Decimal::from_str("2.5").unwrap(),
    SignedDecimal::zero();
    "close all positions with break even"
)]
fn close_all_positions(
    new_atom_price: Decimal,
    new_ntrn_price: Decimal,
    new_osmo_price: Decimal,
    pnl_direction_expected: SignedDecimal, // 1 - profit, -1 - loss, 0 - break even
) {
    // 0 closing fee if we want to simulate break even pnl
    let closing_fee_rate = if pnl_direction_expected.is_zero() {
        Decimal::zero()
    } else {
        Decimal::from_str("0.006").unwrap()
    };

    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let perps = mock.perps.clone();
    let user = "jake";

    let denoms = vec!["uosmo", "untrn", "uatom", "uusdc"];

    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000_000u128, &denoms);

    // set prices
    let usdc_price = Decimal::from_str("0.9").unwrap();
    mock.set_price(&owner, "uusdc", usdc_price).unwrap();
    mock.set_price(&owner, "uatom", Decimal::from_str("10").unwrap()).unwrap();
    mock.set_price(&owner, "untrn", Decimal::from_str("2").unwrap()).unwrap();
    mock.set_price(&owner, "uosmo", Decimal::from_str("2.5").unwrap()).unwrap();

    // deposit some big number of uusdc to vault
    mock.deposit_to_vault(&credit_manager, Some(user), &[coin(1_000_000_000_000u128, "uusdc")])
        .unwrap();

    // init perps
    for denom in denoms {
        if denom == "uusdc" {
            continue;
        }
        // init denoms
        mock.init_denom(&owner, denom, Decimal::from_str("3").unwrap(), Uint128::new(1000000u128))
            .unwrap();
        mock.update_perp_params(
            &owner,
            PerpParamsUpdate::AddOrUpdate {
                params: PerpParams {
                    opening_fee_rate: Decimal::percent(1),
                    closing_fee_rate,
                    ..default_perp_params(denom)
                },
            },
        );
    }

    // open few positions
    let size = SignedUint::from_str("300").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    mock.open_position(&credit_manager, "1", "uatom", size, &[atom_opening_fee.clone()]).unwrap();

    let size = SignedUint::from_str("-500").unwrap();
    let ntrn_opening_fee = mock.query_opening_fee("untrn", size).fee;
    mock.open_position(&credit_manager, "1", "untrn", size, &[ntrn_opening_fee.clone()]).unwrap();

    let size = SignedUint::from_str("100").unwrap();
    let osmo_opening_fee = mock.query_opening_fee("uosmo", size).fee;
    mock.open_position(&credit_manager, "1", "uosmo", size, &[osmo_opening_fee.clone()]).unwrap();

    // update prices
    mock.set_price(&owner, "uatom", new_atom_price).unwrap();
    mock.set_price(&owner, "untrn", new_ntrn_price).unwrap();
    mock.set_price(&owner, "uosmo", new_osmo_price).unwrap();

    // move few blocks for profit/loss pnl
    if !pnl_direction_expected.is_zero() {
        let current_time = mock.query_block_time();
        mock.set_block_time(current_time + 115_200); // move by 32h
    }

    // check balances before closing perp positions
    let cm_usdc_balance = mock.query_balance(&credit_manager, "uusdc");
    let perps_usdc_balance = mock.query_balance(&perps, "uusdc");

    // query positions
    let atom_pos_before_close = mock.query_position("1", "uatom").position;
    let ntrn_pos_before_close = mock.query_position("1", "untrn").position;
    let osmo_pos_before_close = mock.query_position("1", "uosmo").position;

    // compute funds to be sent to close all positions
    let mut pnl_amounts_acc = PnlAmounts::default();
    pnl_amounts_acc.add(&atom_pos_before_close.unrealised_pnl).unwrap();
    pnl_amounts_acc.add(&ntrn_pos_before_close.unrealised_pnl).unwrap();
    pnl_amounts_acc.add(&osmo_pos_before_close.unrealised_pnl).unwrap();

    let pnl = pnl_amounts_acc.to_coins("uusdc").pnl;
    let funds = match pnl.clone() {
        PnL::Profit(_) if pnl_direction_expected == SignedDecimal::one() => vec![],
        PnL::Loss(coin) if pnl_direction_expected == SignedDecimal::from_str("-1").unwrap() => {
            vec![coin]
        }
        PnL::BreakEven if pnl_direction_expected == SignedDecimal::zero() => vec![],
        _ => panic!("unexpected pnl"),
    };
    mock.close_all_positions(&credit_manager, "1", &funds).unwrap();

    // no open positions after closing
    let acc_positions = mock.query_positions_by_account_id("1", ActionKind::Default);
    assert!(acc_positions.positions.is_empty());

    // realized pnl after closing position is equal to opening fee paid (included in realized pnl before closing position) + unrealized pnl
    let atom_realized_pnl = mock.query_denom_realized_pnl_for_account("1", "uatom");
    let mut atom_pnl = PnlAmounts::default();
    atom_pnl.add(&atom_pos_before_close.unrealised_pnl).unwrap();
    atom_pnl.add(&atom_pos_before_close.realised_pnl).unwrap();
    assert_eq!(atom_realized_pnl, atom_pnl);

    let ntrn_realized_pnl = mock.query_denom_realized_pnl_for_account("1", "untrn");
    let mut ntrn_pnl = PnlAmounts::default();
    ntrn_pnl.add(&ntrn_pos_before_close.unrealised_pnl).unwrap();
    ntrn_pnl.add(&ntrn_pos_before_close.realised_pnl).unwrap();
    assert_eq!(ntrn_realized_pnl, ntrn_pnl);

    let osmo_realized_pnl = mock.query_denom_realized_pnl_for_account("1", "uosmo");
    let mut osmo_pnl = PnlAmounts::default();
    osmo_pnl.add(&osmo_pos_before_close.unrealised_pnl).unwrap();
    osmo_pnl.add(&osmo_pos_before_close.realised_pnl).unwrap();
    assert_eq!(osmo_realized_pnl, osmo_pnl);

    // calculate user total realized pnl
    let mut user_realized_pnl = PnlAmounts::default();
    user_realized_pnl.add(&atom_realized_pnl).unwrap();
    user_realized_pnl.add(&ntrn_realized_pnl).unwrap();
    user_realized_pnl.add(&osmo_realized_pnl).unwrap();

    let accounting = mock.query_total_accounting();

    // profit for a user is a loss for the contract and vice versa
    let expected_cash_flow = CashFlow {
        price_pnl: SignedUint::zero().checked_sub(user_realized_pnl.price_pnl).unwrap(),
        opening_fee: SignedUint::zero().checked_sub(user_realized_pnl.opening_fee).unwrap(),
        closing_fee: SignedUint::zero().checked_sub(user_realized_pnl.closing_fee).unwrap(),
        accrued_funding: SignedUint::zero().checked_sub(user_realized_pnl.accrued_funding).unwrap(),
    };
    assert_eq!(
        accounting,
        Accounting {
            cash_flow: expected_cash_flow.clone(),
            balance: Balance::compute_balance(
                &expected_cash_flow,
                &PnlValues::default(),
                usdc_price
            )
            .unwrap(),
            withdrawal_balance: Balance::compute_withdrawal_balance(
                &expected_cash_flow,
                &PnlValues::default(),
                usdc_price
            )
            .unwrap()
        }
    );

    // check balances after closing perp positions
    let cm_usdc_balance_after_close = mock.query_balance(&credit_manager, "uusdc");
    let perps_usdc_balance_after_close = mock.query_balance(&perps, "uusdc");
    let (expected_cm_usdc_balance, expected_perps_usdc_balance) = match pnl {
        PnL::Profit(coin) => {
            (cm_usdc_balance.amount + coin.amount, perps_usdc_balance.amount - coin.amount)
        }
        PnL::Loss(coin) => {
            (cm_usdc_balance.amount - coin.amount, perps_usdc_balance.amount + coin.amount)
        }
        PnL::BreakEven => (cm_usdc_balance.amount, perps_usdc_balance.amount),
    };
    assert_eq!(cm_usdc_balance_after_close.amount, expected_cm_usdc_balance);
    assert_eq!(perps_usdc_balance_after_close.amount, expected_perps_usdc_balance);

    // no unrealized pnl after updating denom states
    let total_pnl = mock.query_total_pnl();
    assert_eq!(total_pnl, PnlValues::default());
}
