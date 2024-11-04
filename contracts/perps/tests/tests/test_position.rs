use std::str::FromStr;

use cosmwasm_std::{coin, coins, Addr, Decimal, Int128, SignedDecimal, Uint128};
use mars_perps::{accounting::BalanceExt, error::ContractError};
use mars_types::{
    oracle::ActionKind,
    params::{PerpParams, PerpParamsUpdate},
    perps::{Accounting, Balance, CashFlow, PnL, PnlAmounts, PositionFeesResponse},
};
use test_case::test_case;

use super::helpers::{assert_err, MockEnv};
use crate::tests::helpers::default_perp_params;

#[test]
fn random_user_cannot_open_position() {
    let mut mock = MockEnv::new().build().unwrap();

    let res = mock.execute_perp_order(
        &Addr::unchecked("random-user-123"),
        "2",
        "uatom",
        Int128::from_str("-125").unwrap(),
        None,
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
                opening_fee_rate: Decimal::percent(1),
                closing_fee_rate: Decimal::percent(1),
                ..default_perp_params("uatom")
            },
        },
    );

    let size = Int128::from_str("-125").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    mock.execute_perp_order(&credit_manager, "2", "uatom", size, None, &[atom_opening_fee])
        .unwrap();

    let res = mock.execute_perp_order(
        &Addr::unchecked("random-user-123"),
        "2",
        "uatom",
        Int128::from_str("-125").unwrap(),
        None,
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
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                enabled: false,
                ..default_perp_params("uatom")
            },
        },
    );

    let res = mock.execute_perp_order(
        &credit_manager,
        "2",
        "uatom",
        Int128::from_str("-125").unwrap(),
        None,
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
                opening_fee_rate: Decimal::percent(1),
                closing_fee_rate: Decimal::percent(1),
                ..default_perp_params("uatom")
            },
        },
    );

    let size = Int128::from_str("-125").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    mock.execute_perp_order(&credit_manager, "2", "uatom", size, None, &[atom_opening_fee])
        .unwrap();

    let perp_params = mock.query_perp_params("uatom");
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                enabled: false,
                ..perp_params
            },
        },
    );

    // increase position
    let res = mock.execute_perp_order(
        &credit_manager,
        "2",
        "uatom",
        Int128::from_str("-175").unwrap(),
        None,
        &[], // fees are not important for this test
    );
    assert_err(
        res,
        ContractError::PositionCannotBeModifiedIfDenomDisabled {
            denom: "uatom".to_string(),
        },
    );

    // decrease position
    let res = mock.execute_perp_order(
        &credit_manager,
        "2",
        "uatom",
        Int128::from_str("-100").unwrap(),
        None,
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
                opening_fee_rate: Decimal::percent(1),
                closing_fee_rate: Decimal::percent(1),
                ..default_perp_params("uatom")
            },
        },
    );

    let size = Int128::from_str("125").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    mock.execute_perp_order(&credit_manager, "2", "uatom", size, None, &[atom_opening_fee])
        .unwrap();

    let perp_params = mock.query_perp_params("uatom");
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                enabled: false,
                ..perp_params
            },
        },
    );

    mock.set_price(&owner, "uatom", Decimal::from_str("10.2").unwrap()).unwrap();

    mock.execute_perp_order(
        &credit_manager,
        "2",
        "uatom",
        Int128::zero().checked_sub(size).unwrap(),
        None,
        &[],
    )
    .unwrap();
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
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params("uatom"),
        },
    );

    // open a position for account 2
    mock.execute_perp_order(
        &credit_manager,
        "2",
        "uatom",
        Int128::from_str("-125").unwrap(),
        None,
        &[],
    )
    .unwrap();

    // try to open one more time
    mock.execute_perp_order(
        &credit_manager,
        "2",
        "uatom",
        Int128::from_str("-125").unwrap(),
        None,
        &[],
    )
    .unwrap();

    let position = mock.query_position("2", "uatom");
    assert_eq!(position.position.unwrap().size, Int128::from_str("-250").unwrap());
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
    let res = mock.execute_perp_order(
        &credit_manager,
        "2",
        "uatom",
        Int128::from_str("100").unwrap(),
        None,
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
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params("uatom"),
        },
    );
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params("utia"),
        },
    );
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params("untrn"),
        },
    );

    // open a position for account 2
    mock.execute_perp_order(
        &credit_manager,
        "2",
        "uatom",
        Int128::from_str("-125").unwrap(),
        None,
        &[],
    )
    .unwrap();
    mock.execute_perp_order(
        &credit_manager,
        "2",
        "utia",
        Int128::from_str("100").unwrap(),
        None,
        &[],
    )
    .unwrap();

    // try to open third position
    let res = mock.execute_perp_order(
        &credit_manager,
        "2",
        "untrn",
        Int128::from_str("-125").unwrap(),
        None,
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
                min_position_value,
                opening_fee_rate: Decimal::percent(1),
                closing_fee_rate: Decimal::percent(1),
                ..default_perp_params("uatom")
            },
        },
    );

    // create valid position
    let size = Int128::from_str("200").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    mock.execute_perp_order(&credit_manager, "2", "uatom", size, None, &[atom_opening_fee])
        .unwrap();

    // Position size is too small
    let res = mock.execute_perp_order(
        &credit_manager,
        "2",
        "uatom",
        Int128::from_str("-100").unwrap(),
        None,
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
    let res = mock.execute_perp_order(
        &credit_manager,
        "2",
        "uatom",
        Int128::from_str("100").unwrap(),
        None,
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
                max_position_value: Some(max_position_value),
                opening_fee_rate: Decimal::percent(1),
                closing_fee_rate: Decimal::percent(1),
                ..default_perp_params("uatom")
            },
        },
    );

    // position size is too big
    // 100 * 12.5 = 1250
    let size = Int128::from_str("50").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    mock.execute_perp_order(&credit_manager, "2", "uatom", size, None, &[atom_opening_fee])
        .unwrap();

    let res = mock.execute_perp_order(
        &credit_manager,
        "2",
        "uatom",
        Int128::from_str("50").unwrap(),
        None,
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
    mock.execute_perp_order(
        &credit_manager,
        "1",
        "uatom",
        Int128::from_str("200").unwrap(),
        None,
        &[],
    )
    .unwrap();
    mock.execute_perp_order(
        &credit_manager,
        "2",
        "uatom",
        Int128::from_str("-400").unwrap(),
        None,
        &[],
    )
    .unwrap();

    // long OI is too big
    let res = mock.execute_perp_order(
        &credit_manager,
        "3",
        "uatom",
        Int128::from_str("403").unwrap(),
        None,
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
    let res = mock.execute_perp_order(
        &credit_manager,
        "3",
        "uatom",
        Int128::from_str("401").unwrap(),
        None,
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
    let res = mock.execute_perp_order(
        &credit_manager,
        "4",
        "uatom",
        Int128::from_str("-101").unwrap(),
        None,
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
    let res = mock.execute_perp_order(
        &credit_manager,
        "4",
        "uatom",
        Int128::from_str("-1").unwrap(),
        None,
        &[],
    ); // 400 + 1 = 401, abs(200 - 401) = 201 * 10 = 2010
    assert_err(
        res,
        ContractError::NetOpenInterestReached {
            max: max_net_oi,
            found: max_net_oi + Uint128::one(),
        },
    );
}

#[test]
fn error_when_new_size_equals_old_size() {
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let user = "jake";

    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000_000u128, &["uusdc", "uatom"]);
    mock.set_price(&owner, "uatom", Decimal::from_str("10").unwrap()).unwrap();
    mock.set_price(&owner, "uusdc", Decimal::from_str("0.8").unwrap()).unwrap();

    // deposit some big number of uusdc to vault
    mock.deposit_to_vault(
        &credit_manager,
        Some(user),
        None,
        &[coin(1_000_000_000_000u128, "uusdc")],
    )
    .unwrap();

    // init denoms
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

    // Test with positive size
    let size = Int128::from_str("12").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    mock.execute_perp_order(&credit_manager, "1", "uatom", size, None, &[atom_opening_fee])
        .unwrap();
    // Try to modify position of 12 to 12
    let res = mock.execute_perp_order(&credit_manager, "1", "uatom", Int128::zero(), None, &[]);

    assert_err(
        res,
        ContractError::IllegalPositionModification {
            reason: "new_size is equal to old_size.".to_string(),
        },
    );

    // Test with negative size
    let size = Int128::from_str("-3").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    mock.execute_perp_order(
        &credit_manager,
        "2",
        "uatom",
        Int128::from_str("-3").unwrap(),
        None,
        &[atom_opening_fee],
    )
    .unwrap();
    // Try to modify position of -3 to -3
    let res = mock.execute_perp_order(
        &credit_manager,
        "2",
        "uatom",
        Int128::zero(),
        None,
        &[coin(1, "uusdc")],
    );

    assert_err(
        res,
        ContractError::IllegalPositionModification {
            reason: "new_size is equal to old_size.".to_string(),
        },
    );
}

#[test]
fn error_when_oi_limits_exceeded() {
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
    mock.deposit_to_vault(
        &credit_manager,
        Some(user),
        None,
        &[coin(1_000_000_000_000u128, "uusdc")],
    )
    .unwrap();

    // init denoms
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

    // State:
    // Long OI = (0) + 30 = 30
    // Short OI = (0) -40 = -40
    // Net OI = (0) + 30 - 40 = -10
    let size = Int128::from_str("30").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    mock.execute_perp_order(&credit_manager, "1", "uatom", size, None, &[atom_opening_fee])
        .unwrap();
    let size = Int128::from_str("-40").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    mock.execute_perp_order(&credit_manager, "2", "uatom", size, None, &[atom_opening_fee])
        .unwrap();

    // Check 1: Long OI is too big

    // State
    // Long OI = (30) + 371 = 401
    // Short OI = (-40)
    let res = mock.execute_perp_order(
        &credit_manager,
        "1",
        "uatom",
        Int128::from_str("371").unwrap(),
        None,
        &[],
    );
    // Long OI value = (401 * 10) = 4010
    // Max Long OI value = 4009
    assert_err(
        res,
        ContractError::LongOpenInterestReached {
            max: max_long_oi,
            found: max_long_oi + Uint128::one(),
        },
    );

    // Check 2: Net OI is too big

    // State
    // Long OI = (30)
    // Short OI = (-40) + -41 = -81
    // Net OI = 30 -81 = -51
    let res = mock.execute_perp_order(
        &credit_manager,
        "1",
        "uatom",
        Int128::from_str("-41").unwrap(),
        None,
        &[],
    );
    assert_err(
        res,
        ContractError::NetOpenInterestReached {
            max: max_net_oi,
            found: max_net_oi + Uint128::one(),
        },
    );

    // State
    // Long OI = (30)
    // Short OI = (-40) - 381 = -4210
    // Net OI = 30 - 421 = 391
    let res = mock.execute_perp_order(
        &credit_manager,
        "2",
        "uatom",
        // Move size from +91 to -421
        Int128::from_str("-381").unwrap(),
        None,
        &[],
    );
    assert_err(
        res,
        ContractError::ShortOpenInterestReached {
            max: max_short_oi,
            found: max_short_oi + Uint128::one(),
        },
    );

    // State
    // Long OI = (30)
    // Short OI = (-40) - 41 = -81
    // Net OI = 30 - 81 = -51
    let res = mock.execute_perp_order(
        &credit_manager,
        "2",
        "uatom",
        Int128::from_str("-41").unwrap(),
        None,
        &[],
    ); // abs(30 - 81) = 51 * 10 = 510
    assert_err(
        res,
        ContractError::NetOpenInterestReached {
            max: max_net_oi,
            found: max_net_oi + Uint128::one(),
        },
    );

    // State
    // Long OI = (30) + 421 = 451
    // Short OI = (-40) + 40 = 0
    let res = mock.execute_perp_order(
        &credit_manager,
        "2",
        "uatom",
        Int128::from_str("461").unwrap(),
        None,
        &[],
    );
    assert_err(
        res,
        ContractError::LongOpenInterestReached {
            max: max_long_oi,
            found: Uint128::from_str("4510").unwrap(),
        },
    );

    // State
    // Long OI = (30) - 30 = 0
    // Short OI = (-40) - 512 = 552
    let res = mock.execute_perp_order(
        &credit_manager,
        "1",
        "uatom",
        Int128::from_str("-542").unwrap(),
        None,
        &[],
    );
    assert_err(
        res,
        ContractError::ShortOpenInterestReached {
            max: max_short_oi,
            found: Uint128::from_str("5520").unwrap(),
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
                closing_fee_rate: Decimal::from_str("0.01").unwrap(),
                opening_fee_rate: Decimal::from_str("0.01").unwrap(),
                ..default_perp_params("uatom")
            },
        },
    );

    // prepare some OI
    let size = Int128::from_str("300").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    mock.execute_perp_order(&credit_manager, "1", "uatom", size, None, &[atom_opening_fee.clone()])
        .unwrap();

    // update price - we are now up 10%
    mock.set_price(&owner, "uatom", Decimal::from_str("11").unwrap()).unwrap();

    // how much opening fee we will pay for increase from 300 to 400
    let atom_opening_fee_for_increase =
        mock.query_opening_fee("uatom", Int128::from_str("100").unwrap()).fee;

    // modify and verify that our pnl is realized
    mock.execute_perp_order(
        &credit_manager,
        "1",
        "uatom",
        Int128::from_str("100").unwrap(),
        None,
        &[],
    )
    .unwrap();

    let position = mock.query_position("1", "uatom");

    let atom_opening_fee_total = atom_opening_fee.amount + atom_opening_fee_for_increase.amount;
    let atom_opening_fee_total =
        Int128::zero().checked_sub(atom_opening_fee_total.try_into().unwrap()).unwrap(); // make it negative because it's a cost
    assert_eq!(atom_opening_fee_total, Int128::from_str("-43").unwrap());
    assert_eq!(
        position.position.unwrap().realized_pnl,
        PnlAmounts {
            accrued_funding: Int128::zero(),
            price_pnl: Int128::from_str("300").unwrap(),
            // opening_fee: atom_opening_fee_total,
            opening_fee: Int128::from_str("-43").unwrap(), // rounding error
            closing_fee: Int128::zero(), // increased position does not have closing fee
            pnl: Int128::from_str("257").unwrap(),
        }
    );

    // update price - we fall back to 10
    mock.set_price(&owner, "uatom", Decimal::from_str("10.5").unwrap()).unwrap();

    mock.execute_perp_order(
        &credit_manager,
        "1",
        "uatom",
        Int128::from_str("-100").unwrap(),
        None,
        &[coin(211u128, "uusdc")],
    )
    .unwrap();

    let position = mock.query_position("1", "uatom");

    assert_eq!(
        position.position.unwrap().realized_pnl,
        PnlAmounts {
            accrued_funding: Int128::zero(),
            price_pnl: Int128::from_str("100").unwrap(),
            // opening_fee: atom_opening_fee_total, // we are not paying opening fee for decrease
            opening_fee: Int128::from_str("-43").unwrap(), // rounding error
            closing_fee: Int128::from_str("-11").unwrap(),
            pnl: Int128::from_str("46").unwrap(),
        }
    );
}

#[test]
fn shouldnt_open_when_reduce_only() {
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
                max_position_value: Some(max_position_value),
                opening_fee_rate: Decimal::percent(1),
                closing_fee_rate: Decimal::percent(1),
                ..default_perp_params("uatom")
            },
        },
    );

    let size = Int128::from_str("50").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    let res = mock.execute_perp_order(
        &credit_manager,
        "2",
        "uatom",
        size,
        Some(true),
        &[atom_opening_fee],
    );

    assert_err(
        res,
        ContractError::IllegalPositionModification {
            reason: "Cannot open position if reduce_only = true".to_string(),
        },
    );
}

#[test]
fn should_open_when_reduce_only_false_or_none() {
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
                max_position_value: Some(max_position_value),
                opening_fee_rate: Decimal::percent(1),
                closing_fee_rate: Decimal::percent(1),
                ..default_perp_params("uatom")
            },
        },
    );

    let size = Int128::from_str("50").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    mock.execute_perp_order(
        &credit_manager,
        "2",
        "uatom",
        size,
        Some(false),
        &[atom_opening_fee.clone()],
    )
    .unwrap();

    mock.execute_perp_order(&credit_manager, "3", "uatom", size, None, &[atom_opening_fee])
        .unwrap();

    let position_a = mock.query_position("2", "uatom").position.unwrap();
    let position_b = mock.query_position("3", "uatom").position.unwrap();

    assert_eq!(position_a.size, size);
    assert_eq!(position_a.denom, "uatom");

    assert_eq!(position_b.size, size);
    assert_eq!(position_b.denom, "uatom");
}

#[test]
fn should_reduce_when_reduce_only_true() {
    let max_position_value = Uint128::new(1249u128);
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let user = "jake";
    let denom = "uatom";
    let base_denom = "uusdc";
    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000_000u128, &["uosmo", "uatom", "uusdc"]);

    // set prices
    mock.set_price(&owner, base_denom, Decimal::from_str("0.8").unwrap()).unwrap();
    mock.set_price(&owner, denom, Decimal::from_str("12.5").unwrap()).unwrap();

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
                max_position_value: Some(max_position_value),
                opening_fee_rate: Decimal::percent(1),
                closing_fee_rate: Decimal::percent(1),
                ..default_perp_params(denom)
            },
        },
    );

    let size_long_position = Int128::from_str("50").unwrap();
    let size_short_position = Int128::from_str("-50").unwrap();

    let atom_opening_fee = mock.query_opening_fee(denom, size_long_position).fee;

    mock.execute_perp_order(
        &credit_manager,
        "2",
        denom,
        size_long_position,
        None,
        &[atom_opening_fee.clone()],
    )
    .unwrap();

    mock.execute_perp_order(
        &credit_manager,
        "3",
        denom,
        size_short_position,
        None,
        &[atom_opening_fee.clone()],
    )
    .unwrap();

    let long_position = mock.query_position("2", "uatom").position.unwrap();
    assert_eq!(long_position.size, size_long_position);
    assert_eq!(long_position.denom, denom);

    let new_long_size = Int128::from_str("25").unwrap();
    let new_short_size = Int128::from_str("-25").unwrap();
    let long_modification_size = new_long_size.checked_sub(size_long_position).unwrap();
    let short_modification_size = new_short_size.checked_sub(size_short_position).unwrap();

    let atom_closing_fee_long: PositionFeesResponse =
        mock.query_position_fees("2", denom, new_long_size);

    let long_pnl_losses = if long_position.unrealized_pnl.price_pnl.is_negative() {
        long_position.unrealized_pnl.price_pnl.unsigned_abs()
    } else {
        Uint128::zero()
    };

    // Reduce long
    mock.execute_perp_order(
        &credit_manager,
        "2",
        denom,
        long_modification_size,
        Some(true),
        &coins(
            // add pnl to the closing fee
            atom_closing_fee_long.closing_fee.checked_add(long_pnl_losses).unwrap().into(),
            base_denom,
        ),
    )
    .unwrap();

    let atom_closing_fee_short: PositionFeesResponse =
        mock.query_position_fees("3", denom, new_short_size);
    let short_position = mock.query_position("3", "uatom").position.unwrap();
    assert_eq!(short_position.size, size_short_position);
    assert_eq!(short_position.denom, denom);

    let short_pnl_losses = if short_position.unrealized_pnl.price_pnl.is_negative() {
        short_position.unrealized_pnl.price_pnl.unsigned_abs()
    } else {
        Uint128::zero()
    };

    // Reduce short
    mock.execute_perp_order(
        &credit_manager,
        "3",
        denom,
        short_modification_size,
        Some(true),
        &coins(
            atom_closing_fee_short.closing_fee.checked_add(short_pnl_losses).unwrap().into(),
            base_denom,
        ),
    )
    .unwrap();

    // Verify updates occurred
    let updated_long_position = mock.query_position("2", denom).position.unwrap();
    assert_eq!(updated_long_position.size, new_long_size);
    assert_eq!(updated_long_position.denom, denom);

    let short_position = mock.query_position("3", denom).position.unwrap();
    assert_eq!(short_position.size, new_short_size);
    assert_eq!(short_position.denom, denom);
}

#[test]
fn shouldnt_increase_when_reduce_only_true() {
    let max_position_value = Uint128::new(6250u128);
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let user = "jake";
    let denom = "uatom";
    let base_denom = "uusdc";
    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000_000u128, &["uosmo", "uatom", "uusdc"]);

    // set prices
    mock.set_price(&owner, base_denom, Decimal::from_str("0.8").unwrap()).unwrap();
    mock.set_price(&owner, denom, Decimal::from_str("12.5").unwrap()).unwrap();

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
                max_position_value: Some(max_position_value),
                opening_fee_rate: Decimal::percent(1),
                closing_fee_rate: Decimal::percent(1),
                ..default_perp_params(denom)
            },
        },
    );

    let size_long_position = Int128::from_str("500").unwrap();
    let size_short_position = Int128::from_str("-500").unwrap();

    let atom_opening_fee = mock.query_opening_fee(denom, size_long_position).fee;
    mock.execute_perp_order(
        &credit_manager,
        "2",
        denom,
        size_long_position,
        None,
        &[atom_opening_fee.clone()],
    )
    .unwrap();

    let atom_opening_fee = mock.query_opening_fee(denom, size_short_position).fee;
    mock.execute_perp_order(
        &credit_manager,
        "3",
        denom,
        size_short_position,
        None,
        &[atom_opening_fee.clone()],
    )
    .unwrap();

    let long_position = mock.query_position("2", "uatom").position.unwrap();
    assert_eq!(long_position.size, size_long_position);
    assert_eq!(long_position.denom, denom);

    let new_long_size = Int128::from_str("700").unwrap();
    let new_short_size = Int128::from_str("-750").unwrap();
    let long_modification_size = new_long_size.checked_sub(size_long_position).unwrap();
    let short_modification_size = new_short_size.checked_sub(size_short_position).unwrap();

    let atom_closing_fee_long: PositionFeesResponse =
        mock.query_position_fees("2", denom, new_long_size);

    let long_pnl_losses = if long_position.unrealized_pnl.price_pnl.is_negative() {
        long_position.unrealized_pnl.price_pnl.unsigned_abs()
    } else {
        Uint128::zero()
    };

    // increase long
    let res = mock.execute_perp_order(
        &credit_manager,
        "2",
        denom,
        long_modification_size,
        Some(true),
        &coins(
            // add pnl to the closing fee
            atom_closing_fee_long.closing_fee.checked_add(long_pnl_losses).unwrap().into(),
            base_denom,
        ),
    );

    assert_err(
        res,
        ContractError::IllegalPositionModification {
            reason: "Cannot increase position if reduce_only = true".to_string(),
        },
    );

    let short_position = mock.query_position("3", "uatom").position.unwrap();
    assert_eq!(short_position.size, size_short_position);
    assert_eq!(short_position.denom, denom);

    // Reduce short
    let res = mock.execute_perp_order(
        &credit_manager,
        "3",
        denom,
        short_modification_size,
        Some(true),
        &[],
    );

    assert_err(
        res,
        ContractError::IllegalPositionModification {
            reason: "Cannot increase position if reduce_only = true".to_string(),
        },
    );

    // Verify updates occurred
    let updated_long_position = mock.query_position("2", denom).position.unwrap();
    assert_eq!(updated_long_position.size, size_long_position);
    assert_eq!(updated_long_position.denom, denom);

    let short_position = mock.query_position("3", denom).position.unwrap();
    assert_eq!(short_position.size, size_short_position);
    assert_eq!(short_position.denom, denom);
}

#[test]
fn increase_when_reduce_only_false() {
    let max_position_value = Uint128::new(1249u128);
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let user = "jake";
    let denom = "uatom";
    let base_denom = "uusdc";
    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000_000u128, &["uosmo", "uatom", "uusdc"]);

    // set prices
    mock.set_price(&owner, base_denom, Decimal::from_str("0.8").unwrap()).unwrap();
    mock.set_price(&owner, denom, Decimal::from_str("12.5").unwrap()).unwrap();

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
                max_position_value: Some(max_position_value),
                opening_fee_rate: Decimal::percent(1),
                closing_fee_rate: Decimal::percent(1),
                ..default_perp_params(denom)
            },
        },
    );

    let size_long_position = Int128::from_str("50").unwrap();
    let size_short_position = Int128::from_str("-50").unwrap();

    let atom_opening_fee = mock.query_opening_fee(denom, size_long_position).fee;

    mock.execute_perp_order(
        &credit_manager,
        "2",
        denom,
        size_long_position,
        None,
        &[atom_opening_fee.clone()],
    )
    .unwrap();

    mock.execute_perp_order(
        &credit_manager,
        "3",
        denom,
        size_short_position,
        None,
        &[atom_opening_fee.clone()],
    )
    .unwrap();

    let long_position = mock.query_position("2", "uatom").position.unwrap();
    assert_eq!(long_position.size, size_long_position);
    assert_eq!(long_position.denom, denom);

    let new_long_size = Int128::from_str("75").unwrap();
    let new_short_size = Int128::from_str("-75").unwrap();
    let long_modification_size = new_long_size.checked_sub(size_long_position).unwrap();
    let short_modification_size = new_short_size.checked_sub(size_short_position).unwrap();

    // increase long
    mock.execute_perp_order(
        &credit_manager,
        "2",
        denom,
        long_modification_size,
        Some(false),
        &coins(
            // add pnl to the closing fee
            4, base_denom,
        ),
    )
    .unwrap();

    let short_position = mock.query_position("3", "uatom").position.unwrap();
    assert_eq!(short_position.size, size_short_position);
    assert_eq!(short_position.denom, denom);

    // Reduce short
    mock.execute_perp_order(
        &credit_manager,
        "3",
        denom,
        short_modification_size,
        Some(false),
        &coins(4, base_denom),
    )
    .unwrap();

    // Verify updates occurred
    let updated_long_position = mock.query_position("2", denom).position.unwrap();
    assert_eq!(updated_long_position.size, new_long_size);
    assert_eq!(updated_long_position.denom, denom);

    let short_position = mock.query_position("3", denom).position.unwrap();
    assert_eq!(short_position.size, new_short_size);
    assert_eq!(short_position.denom, denom);
}

#[test]
fn flip_position_when_reduce_only_true() {
    let max_position_value = Uint128::new(1249u128);
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let user = "jake";
    let denom = "uatom";
    let base_denom = "uusdc";
    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000_000u128, &["uosmo", "uatom", "uusdc"]);

    // set prices
    mock.set_price(&owner, base_denom, Decimal::from_str("0.8").unwrap()).unwrap();
    mock.set_price(&owner, denom, Decimal::from_str("12.5").unwrap()).unwrap();

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
                max_position_value: Some(max_position_value),
                opening_fee_rate: Decimal::percent(1),
                closing_fee_rate: Decimal::percent(1),
                ..default_perp_params(denom)
            },
        },
    );

    let size_long_position = Int128::from_str("50").unwrap();
    let size_short_position = Int128::from_str("-50").unwrap();

    let atom_opening_fee = mock.query_opening_fee(denom, size_long_position).fee;

    mock.execute_perp_order(
        &credit_manager,
        "2",
        denom,
        size_long_position,
        None,
        &[atom_opening_fee.clone()],
    )
    .unwrap();

    mock.execute_perp_order(
        &credit_manager,
        "3",
        denom,
        size_short_position,
        None,
        &[atom_opening_fee.clone()],
    )
    .unwrap();

    let long_position = mock.query_position("2", "uatom").position.unwrap();
    assert_eq!(long_position.size, size_long_position);
    assert_eq!(long_position.denom, denom);

    // Flip short
    let new_long_size = Int128::from_str("-25").unwrap();
    // Flip long
    let new_short_size = Int128::from_str("25").unwrap();
    let long_modification_size = new_long_size.checked_sub(size_long_position).unwrap();
    let short_modification_size = new_short_size.checked_sub(size_short_position).unwrap();

    let atom_closing_fee_long: PositionFeesResponse =
        mock.query_position_fees("2", denom, new_long_size);

    let long_pnl_losses = if long_position.unrealized_pnl.price_pnl.is_negative() {
        long_position.unrealized_pnl.price_pnl.unsigned_abs()
    } else {
        Uint128::zero()
    };

    // Reduce long
    mock.execute_perp_order(
        &credit_manager,
        "2",
        denom,
        long_modification_size,
        Some(true),
        &coins(
            // add pnl to the closing fee
            atom_closing_fee_long.closing_fee.checked_add(long_pnl_losses).unwrap().into(),
            base_denom,
        ),
    )
    .unwrap();

    let atom_closing_fee_short: PositionFeesResponse =
        mock.query_position_fees("3", denom, new_short_size);
    let short_position = mock.query_position("3", "uatom").position.unwrap();
    assert_eq!(short_position.size, size_short_position);
    assert_eq!(short_position.denom, denom);

    let short_pnl_losses = if short_position.unrealized_pnl.price_pnl.is_negative() {
        short_position.unrealized_pnl.price_pnl.unsigned_abs()
    } else {
        Uint128::zero()
    };

    // Reduce short
    mock.execute_perp_order(
        &credit_manager,
        "3",
        denom,
        short_modification_size,
        Some(true),
        &coins(
            atom_closing_fee_short.closing_fee.checked_add(short_pnl_losses).unwrap().into(),
            base_denom,
        ),
    )
    .unwrap();

    // Verify all positions are closed
    let positions_2 = mock.query_positions_by_account_id("2", ActionKind::Default);
    assert_eq!(positions_2.positions.len(), 0);

    let positions_3 = mock.query_positions_by_account_id("3", ActionKind::Default);
    assert_eq!(positions_3.positions.len(), 0);
}

#[test]
fn flip_position_when_reduce_only_false() {
    let max_position_value = Uint128::new(1249u128);
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let user = "jake";
    let denom = "uatom";
    let base_denom = "uusdc";
    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000_000u128, &["uosmo", "uatom", "uusdc"]);

    // set prices
    mock.set_price(&owner, base_denom, Decimal::from_str("0.8").unwrap()).unwrap();
    mock.set_price(&owner, denom, Decimal::from_str("12.5").unwrap()).unwrap();

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
                max_position_value: Some(max_position_value),
                opening_fee_rate: Decimal::percent(1),
                closing_fee_rate: Decimal::percent(1),
                ..default_perp_params(denom)
            },
        },
    );

    let size_long_position = Int128::from_str("50").unwrap();
    let size_short_position = Int128::from_str("-50").unwrap();

    let atom_opening_fee = mock.query_opening_fee(denom, size_long_position).fee;

    mock.execute_perp_order(
        &credit_manager,
        "2",
        denom,
        size_long_position,
        None,
        &[atom_opening_fee.clone()],
    )
    .unwrap();

    mock.execute_perp_order(
        &credit_manager,
        "3",
        denom,
        size_short_position,
        None,
        &[atom_opening_fee.clone()],
    )
    .unwrap();

    let long_position = mock.query_position("2", "uatom").position.unwrap();
    assert_eq!(long_position.size, size_long_position);
    assert_eq!(long_position.denom, denom);

    // Flip short
    let new_long_size = Int128::from_str("-25").unwrap();
    // Flip long
    let new_short_size = Int128::from_str("25").unwrap();
    let long_modification_size = new_long_size.checked_sub(size_long_position).unwrap();
    let short_modification_size = new_short_size.checked_sub(size_short_position).unwrap();

    // Reduce long
    mock.execute_perp_order(
        &credit_manager,
        "2",
        denom,
        long_modification_size,
        Some(false),
        &coins(12, base_denom),
    )
    .unwrap();

    let short_position = mock.query_position("3", "uatom").position.unwrap();
    assert_eq!(short_position.size, size_short_position);
    assert_eq!(short_position.denom, denom);

    // Reduce short
    mock.execute_perp_order(
        &credit_manager,
        "3",
        denom,
        short_modification_size,
        Some(false),
        &coins(12, base_denom),
    )
    .unwrap();

    // Verify updates occurred
    let updated_long_position = mock.query_position("2", denom).position.unwrap();
    assert_eq!(updated_long_position.size, new_long_size);
    assert_eq!(updated_long_position.denom, denom);

    let short_position = mock.query_position("3", denom).position.unwrap();
    assert_eq!(short_position.size, new_short_size);
    assert_eq!(short_position.denom, denom);
}

#[test_case(
    None,
    Int128::from_str("250").unwrap(),
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
    Some(Int128::from_str("1200").unwrap()),
    Int128::from_str("2500").unwrap(),
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
    Some(Int128::from_str("1200").unwrap()),
    Int128::from_str("800").unwrap(),
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
    Some(Int128::from_str("1200").unwrap()),
    Int128::from_str("0").unwrap(),
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
    Int128::from_str("-2500").unwrap(),
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
    Some(Int128::from_str("-1200").unwrap()),
    Int128::from_str("-2500").unwrap(),
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
    Some(Int128::from_str("-1200").unwrap()),
    Int128::from_str("-600").unwrap(),
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
    Some(Int128::from_str("-1200").unwrap()),
    Int128::from_str("0").unwrap(),
    PositionFeesResponse {
        base_denom: "uusdc".to_string(),
        opening_fee: Uint128::zero(),
        closing_fee: Uint128::new(11u128),
        opening_exec_price: None,
        closing_exec_price: Some(Decimal::from_str("1.26175").unwrap()),
    };
    "close short"
)]
#[test_case(
    Some(Int128::from_str("1200").unwrap()),
    Int128::from_str("-2500").unwrap(),
    PositionFeesResponse {
    base_denom: "uusdc".to_string(),
    opening_fee: Uint128::new(15u128),
    closing_fee: Uint128::new(11u128),
    opening_exec_price: Some(Decimal::from_str("1.2609375").unwrap()),
    closing_exec_price: Some(Decimal::from_str("1.26325").unwrap()),
    };
    "flip long to short"
)]
#[test_case(
    Some(Int128::from_str("-500").unwrap()),
    Int128::from_str("500").unwrap(),
    PositionFeesResponse {
    base_denom: "uusdc".to_string(),
    opening_fee: Uint128::new(3u128),
    closing_fee: Uint128::new(5u128),
    opening_exec_price: Some(Decimal::from_str("1.2628125").unwrap()),
    closing_exec_price: Some(Decimal::from_str("1.2621875").unwrap()),
    };
    "flip short to long"
)]
fn query_position_fees(
    old_size: Option<Int128>,
    new_size: Int128,
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
                opening_fee_rate: Decimal::from_str("0.004").unwrap(),
                closing_fee_rate: Decimal::from_str("0.006").unwrap(),
                ..default_perp_params("uosmo")
            },
        },
    );

    // open a position to change skew
    let size = Int128::from_str("10000").unwrap();
    let opening_fee = mock.query_opening_fee("uosmo", size).fee;
    mock.execute_perp_order(&credit_manager, "2", "uosmo", size, None, &[opening_fee]).unwrap();

    // open a position if specified
    if let Some(old_size) = old_size {
        let opening_fee = mock.query_opening_fee("uosmo", old_size).fee;
        mock.execute_perp_order(&credit_manager, "1", "uosmo", old_size, None, &[opening_fee])
            .unwrap();
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
    mock.deposit_to_vault(
        &credit_manager,
        Some(user),
        None,
        &[coin(1_000_000_000_000u128, "uusdc")],
    )
    .unwrap();

    // init perps
    for denom in denoms {
        if denom == "uusdc" {
            continue;
        }
        // init denoms
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
    let size = Int128::from_str("300").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    mock.execute_perp_order(&credit_manager, "1", "uatom", size, None, &[atom_opening_fee.clone()])
        .unwrap();

    let size = Int128::from_str("-500").unwrap();
    let ntrn_opening_fee = mock.query_opening_fee("untrn", size).fee;
    mock.execute_perp_order(&credit_manager, "1", "untrn", size, None, &[ntrn_opening_fee.clone()])
        .unwrap();

    let size = Int128::from_str("100").unwrap();
    let osmo_opening_fee = mock.query_opening_fee("uosmo", size).fee;
    mock.execute_perp_order(&credit_manager, "1", "uosmo", size, None, &[osmo_opening_fee.clone()])
        .unwrap();

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
    let atom_pos_before_close = mock.query_position("1", "uatom").position.unwrap();
    let ntrn_pos_before_close = mock.query_position("1", "untrn").position.unwrap();
    let osmo_pos_before_close = mock.query_position("1", "uosmo").position.unwrap();

    // compute funds to be sent to close all positions
    let mut pnl_amounts_acc = PnlAmounts::default();
    pnl_amounts_acc.add(&atom_pos_before_close.unrealized_pnl).unwrap();
    pnl_amounts_acc.add(&ntrn_pos_before_close.unrealized_pnl).unwrap();
    pnl_amounts_acc.add(&osmo_pos_before_close.unrealized_pnl).unwrap();

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
    let atom_realized_pnl = mock.query_realized_pnl_by_account_and_market("1", "uatom");
    let mut atom_pnl = PnlAmounts::default();
    atom_pnl.add(&atom_pos_before_close.unrealized_pnl).unwrap();
    atom_pnl.add(&atom_pos_before_close.realized_pnl).unwrap();
    assert_eq!(atom_realized_pnl, atom_pnl);

    let ntrn_realized_pnl = mock.query_realized_pnl_by_account_and_market("1", "untrn");
    let mut ntrn_pnl = PnlAmounts::default();
    ntrn_pnl.add(&ntrn_pos_before_close.unrealized_pnl).unwrap();
    ntrn_pnl.add(&ntrn_pos_before_close.realized_pnl).unwrap();
    assert_eq!(ntrn_realized_pnl, ntrn_pnl);

    let osmo_realized_pnl = mock.query_realized_pnl_by_account_and_market("1", "uosmo");
    let mut osmo_pnl = PnlAmounts::default();
    osmo_pnl.add(&osmo_pos_before_close.unrealized_pnl).unwrap();
    osmo_pnl.add(&osmo_pos_before_close.realized_pnl).unwrap();
    assert_eq!(osmo_realized_pnl, osmo_pnl);

    // calculate user total realized pnl
    let mut user_realized_pnl = PnlAmounts::default();
    user_realized_pnl.add(&atom_realized_pnl).unwrap();
    user_realized_pnl.add(&ntrn_realized_pnl).unwrap();
    user_realized_pnl.add(&osmo_realized_pnl).unwrap();

    let accounting = mock.query_total_accounting().accounting;

    // profit for a user is a loss for the contract and vice versa
    let expected_cash_flow = CashFlow {
        price_pnl: Int128::zero().checked_sub(user_realized_pnl.price_pnl).unwrap(),
        opening_fee: Int128::zero().checked_sub(user_realized_pnl.opening_fee).unwrap(),
        closing_fee: Int128::zero().checked_sub(user_realized_pnl.closing_fee).unwrap(),
        accrued_funding: Int128::zero().checked_sub(user_realized_pnl.accrued_funding).unwrap(),
        protocol_fee: Uint128::zero(),
    };
    assert_eq!(
        accounting,
        Accounting {
            cash_flow: expected_cash_flow.clone(),
            balance: Balance::compute_balance(&expected_cash_flow, &PnlAmounts::default(),)
                .unwrap(),
            withdrawal_balance: Balance::compute_withdrawal_balance(
                &expected_cash_flow,
                &PnlAmounts::default(),
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

    // no unrealized pnl after updating market states
    let total_pnl = mock.query_total_accounting().unrealized_pnl;
    assert_eq!(total_pnl, PnlAmounts::default());
}

#[test]
fn open_very_small_position_with_zero_opening_fee() {
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let user = "jake";

    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000_000u128, &["uosmo", "uatom", "uusdc"]);

    // set prices
    mock.set_price(&owner, "uusdc", Decimal::from_str("0.98").unwrap()).unwrap();
    mock.set_price(&owner, "uatom", Decimal::from_str("0.01").unwrap()).unwrap();

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
                opening_fee_rate: Decimal::from_str("0.00000000000000001").unwrap(),
                ..default_perp_params("uatom")
            },
        },
    );

    // openining fee is zero
    let size = Int128::from_str("1").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    assert!(atom_opening_fee.amount.is_zero());

    // open a very small position where opening fee is zero but opening_fee_rate is not zero
    mock.execute_perp_order(&credit_manager, "1", "uatom", size, None, &[]).unwrap();
}

#[test]
fn global_realized_pnl_matches_positions_realized_pnl() {
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let user = "jake";

    // Credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000_000u128, &["uosmo", "uatom", "uusdc"]);

    // Set prices
    mock.set_price(&owner, "uusdc", Decimal::from_str("1").unwrap()).unwrap();
    mock.set_price(&owner, "uatom", Decimal::from_str("10").unwrap()).unwrap();

    // Deposit some big number of uusdc to vault
    mock.deposit_to_vault(
        &credit_manager,
        Some(user),
        None,
        &[coin(1_000_000_000_000u128, "uusdc")],
    )
    .unwrap();

    // Init perp
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

    // Open a LONG position
    let size = Int128::from_str("300000").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    mock.execute_perp_order(&credit_manager, "1", "uatom", size, None, &[atom_opening_fee.clone()])
        .unwrap();

    // Increase price
    mock.set_price(&owner, "uatom", Decimal::from_str("11").unwrap()).unwrap();

    // Move blocks to generate funding
    mock.increment_by_blocks(100);

    // Check upcoming realized pnl
    let perp_position = mock.query_position("1", "uatom").position.unwrap();
    let mut realized_pnl = perp_position.realized_pnl;
    realized_pnl.add(&perp_position.unrealized_pnl).unwrap();

    // Close the LONG position
    let closing_size = Int128::zero().checked_sub(size).unwrap();
    mock.execute_perp_order(&credit_manager, "1", "uatom", closing_size, Some(true), &[]).unwrap();

    // Check global realized pnl
    let global_realized_pnl = mock.query_realized_pnl_by_account_and_market("1", "uatom");
    assert_eq!(global_realized_pnl, realized_pnl);

    // Open a SHORT position
    let size = Int128::from_str("-300000").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    mock.execute_perp_order(&credit_manager, "1", "uatom", size, None, &[atom_opening_fee.clone()])
        .unwrap();

    // Decrease price
    mock.set_price(&owner, "uatom", Decimal::from_str("9").unwrap()).unwrap();

    // Move blocks to generate funding
    mock.increment_by_blocks(100);

    // Check upcoming realized pnl
    let perp_position = mock.query_position("1", "uatom").position.unwrap();
    realized_pnl.add(&perp_position.realized_pnl).unwrap();
    realized_pnl.add(&perp_position.unrealized_pnl).unwrap();

    // Close the SHORT position
    let closing_size = Int128::zero().checked_sub(size).unwrap();
    mock.execute_perp_order(&credit_manager, "1", "uatom", closing_size, Some(true), &[]).unwrap();

    // Check global realized pnl
    let global_realized_pnl = mock.query_realized_pnl_by_account_and_market("1", "uatom");
    assert_eq!(global_realized_pnl, realized_pnl);
}
