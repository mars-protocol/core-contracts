use std::str::FromStr;

use cosmwasm_std::{coin, Addr, Decimal, Uint128};
use mars_perps::error::ContractError;
use mars_types::{
    math::SignedDecimal,
    params::{PerpParams, PerpParamsUpdate},
    perps::{PnlAmounts, PositionFeesResponse},
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
        SignedDecimal::from_str("-125").unwrap(),
        &[],
    );
    assert_err(res, ContractError::SenderIsNotCreditManager);
}

#[test]
fn random_user_cannot_modify_position() {
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let user = Addr::unchecked("jake");

    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(
        &[&credit_manager, &user],
        1_000_000_000_000u128,
        &["uosmo", "uatom", "uusdc"],
    );

    // deposit some big number of uusdc to vault
    mock.deposit_to_vault(&user, &[coin(1_000_000_000_000u128, "uusdc")]).unwrap();

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

    let size = SignedDecimal::from_str("-125").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    mock.open_position(&credit_manager, "2", "uatom", size, &[atom_opening_fee]).unwrap();

    let res = mock.modify_position(
        &Addr::unchecked("random-user-123"),
        "2",
        "uatom",
        SignedDecimal::from_str("-125").unwrap(),
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
    mock.init_denom(
        &owner,
        "uatom",
        Decimal::from_str("3").unwrap(),
        Decimal::from_str("1000000").unwrap(),
    )
    .unwrap();
    mock.disable_denom(&owner, "uatom").unwrap();

    let res = mock.open_position(
        &credit_manager,
        "2",
        "uatom",
        SignedDecimal::from_str("-125").unwrap(),
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
fn cannot_increase_position_for_disabled_denom() {
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let user = Addr::unchecked("jake");

    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(
        &[&credit_manager, &user],
        1_000_000_000_000u128,
        &["uosmo", "uatom", "uusdc"],
    );

    // deposit some big number of uusdc to vault
    mock.deposit_to_vault(&user, &[coin(1_000_000_000_000u128, "uusdc")]).unwrap();

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

    let size = SignedDecimal::from_str("-125").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    mock.open_position(&credit_manager, "2", "uatom", size, &[atom_opening_fee]).unwrap();

    mock.disable_denom(&owner, "uatom").unwrap();

    let res = mock.modify_position(
        // FIXME: provide fees
        &credit_manager,
        "2",
        "uatom",
        SignedDecimal::from_str("-175").unwrap(),
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
fn only_one_position_possible_for_denom() {
    let mut mock = MockEnv::new().opening_fee_rate(Decimal::zero()).build().unwrap();

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
    mock.open_position(
        &credit_manager,
        "2",
        "uatom",
        SignedDecimal::from_str("-125").unwrap(),
        &[],
    )
    .unwrap();

    // try to open one more time
    let res = mock.open_position(
        &credit_manager,
        "2",
        "uatom",
        SignedDecimal::from_str("-125").unwrap(),
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
    let min_position_in_base_denom = Uint128::new(1251u128);
    let mut mock = MockEnv::new()
        .opening_fee_rate(Decimal::zero())
        .min_position_in_base_denom(min_position_in_base_denom)
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

    // position size is too small
    // 100 * 10 / 0.8 = 1250
    let res = mock.open_position(
        &credit_manager,
        "2",
        "uatom",
        SignedDecimal::from_str("100").unwrap(),
        &[],
    );
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
fn reduced_position_cannot_be_too_small() {
    let min_position_in_base_denom = Uint128::new(1251u128);
    let mut mock =
        MockEnv::new().min_position_in_base_denom(min_position_in_base_denom).build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let user = Addr::unchecked("jake");

    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(
        &[&credit_manager, &user],
        1_000_000_000_000u128,
        &["uosmo", "uatom", "uusdc"],
    );

    // deposit some big number of uusdc to vault
    mock.deposit_to_vault(&user, &[coin(1_000_000_000_000u128, "uusdc")]).unwrap();

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

    // create valid position
    let size = SignedDecimal::from_str("200").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    mock.open_position(&credit_manager, "2", "uatom", size, &[atom_opening_fee]).unwrap();

    // Position size is too small
    let res = mock.modify_position(
        &credit_manager,
        "2",
        "uatom",
        SignedDecimal::from_str("100").unwrap(),
        &[],
    );

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
        .opening_fee_rate(Decimal::zero())
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
    let res = mock.open_position(
        &credit_manager,
        "2",
        "uatom",
        SignedDecimal::from_str("100").unwrap(),
        &[],
    );
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
fn increased_position_cannot_be_too_big() {
    let max_position_in_base_denom = Uint128::new(1249u128);
    let mut mock = MockEnv::new()
        .min_position_in_base_denom(Uint128::zero())
        .max_position_in_base_denom(Some(max_position_in_base_denom))
        .build()
        .unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let user = Addr::unchecked("jake");

    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(
        &[&credit_manager, &user],
        1_000_000_000_000u128,
        &["uosmo", "uatom", "uusdc"],
    );

    // deposit some big number of uusdc to vault
    mock.deposit_to_vault(&user, &[coin(1_000_000_000_000u128, "uusdc")]).unwrap();

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
    let size = SignedDecimal::from_str("50").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    mock.open_position(&credit_manager, "2", "uatom", size, &[atom_opening_fee]).unwrap();

    let res = mock.modify_position(
        // FIXME: provide fees
        &credit_manager,
        "2",
        "uatom",
        SignedDecimal::from_str("100").unwrap(),
        &[],
    );
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
        .opening_fee_rate(Decimal::zero())
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
    mock.open_position(&credit_manager, "1", "uatom", SignedDecimal::from_str("300").unwrap(), &[])
        .unwrap();
    mock.open_position(
        &credit_manager,
        "2",
        "uatom",
        SignedDecimal::from_str("-400").unwrap(),
        &[],
    )
    .unwrap();

    // long OI is too big
    let res = mock.open_position(
        &credit_manager,
        "3",
        "uatom",
        SignedDecimal::from_str("3701").unwrap(),
        &[],
    ); // 300 + 3701 = 4001
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
        SignedDecimal::from_str("601").unwrap(),
        &[],
    ); // 300 + 601 = 901, abs(901 - 400) = 501
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
        &[],
    ); // 400 + 3801 = 4201
    assert_err(
        res,
        ContractError::ShortOpenInterestReached {
            max: max_short_oi,
            found: max_short_oi + Uint128::one(),
        },
    );

    // net OI is too big
    let res = mock.open_position(
        &credit_manager,
        "4",
        "uatom",
        SignedDecimal::from_str("-401").unwrap(),
        &[],
    ); // 400 + 401 = 801, abs(300 - 801) = 501
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
    let mut mock = MockEnv::new()
        .min_position_in_base_denom(Uint128::zero())
        .max_position_in_base_denom(None)
        .build()
        .unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let user = Addr::unchecked("jake");

    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(
        &[&credit_manager, &user],
        1_000_000_000_000u128,
        &["uosmo", "uatom", "uusdc"],
    );

    // deposit some big number of uusdc to vault
    mock.deposit_to_vault(&user, &[coin(1_000_000_000_000u128, "uusdc")]).unwrap();

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
    let size = SignedDecimal::from_str("300").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    mock.open_position(&credit_manager, "1", "uatom", size, &[atom_opening_fee]).unwrap();
    let size = SignedDecimal::from_str("-400").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    mock.open_position(&credit_manager, "2", "uatom", size, &[atom_opening_fee]).unwrap();

    // long OI is too big
    let res = mock.modify_position(
        &credit_manager,
        "1",
        "uatom",
        SignedDecimal::from_str("4001").unwrap(),
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
        SignedDecimal::from_str("901").unwrap(),
        &[],
    ); // 300 + 601 = 901, abs(901 - 400) = 501
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
        SignedDecimal::from_str("-4201").unwrap(),
        &[],
    ); // 400 + 3801 = 4201
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
        SignedDecimal::from_str("-801").unwrap(),
        &[],
    ); // 400 + 401 = 801, abs(300 - 801) = 501
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
    let mut mock = MockEnv::new()
        .min_position_in_base_denom(Uint128::zero())
        .max_position_in_base_denom(None)
        .build()
        .unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let user = Addr::unchecked("jake");

    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(
        &[&credit_manager, &user],
        1_000_000_000_000u128,
        &["uosmo", "uatom", "uusdc"],
    );

    // deposit some big number of uusdc to vault
    mock.deposit_to_vault(&user, &[coin(1_000_000_000_000u128, "uusdc")]).unwrap();

    // set prices
    mock.set_price(&owner, "uusdc", Decimal::from_str("1").unwrap()).unwrap();
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
            params: PerpParams {
                denom: "uatom".to_string(),
                max_net_oi: Uint128::new(500),
                max_long_oi: Uint128::new(4000),
                max_short_oi: Uint128::new(4200),
            },
        },
    );

    // prepare some OI
    let size = SignedDecimal::from_str("300").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    mock.open_position(&credit_manager, "1", "uatom", size, &[atom_opening_fee.clone()]).unwrap();

    // update price - we are now up 10%
    mock.set_price(&owner, "uatom", Decimal::from_str("11").unwrap()).unwrap();

    // how much opening fee we will pay for increase from 300 to 400
    let atom_opening_fee_for_increase =
        mock.query_opening_fee("uatom", SignedDecimal::from_str("100").unwrap()).fee;

    // modify and verify that our pnl is realised
    mock.modify_position(
        &credit_manager,
        "1",
        "uatom",
        SignedDecimal::from_str("400").unwrap(),
        &[],
    )
    .unwrap();

    let position = mock.query_position("1", "uatom");

    let atom_opening_fee_total = atom_opening_fee.amount + atom_opening_fee_for_increase.amount;
    let atom_opening_fee_total =
        SignedDecimal::zero().checked_sub(atom_opening_fee_total.into()).unwrap(); // make it negative because it's a cost
    assert_eq!(atom_opening_fee_total, SignedDecimal::from_str("-43").unwrap());
    assert_eq!(
        position.position.realised_pnl,
        PnlAmounts {
            accrued_funding: SignedDecimal::zero(),
            price_pnl: SignedDecimal::from_str("300.045").unwrap(),
            // opening_fee: atom_opening_fee_total,
            opening_fee: SignedDecimal::from_str("-42.00385").unwrap(), // FIXME: rounding error
            closing_fee: SignedDecimal::zero(), // increased position does not have closing fee
            pnl: SignedDecimal::from_str("258.04115").unwrap(),
        }
    );

    // update price - we fall back to 10
    mock.set_price(&owner, "uatom", Decimal::from_str("10.5").unwrap()).unwrap();

    mock.modify_position(
        &credit_manager,
        "1",
        "uatom",
        SignedDecimal::from_str("300").unwrap(),
        &[coin(211u128, "uusdc")],
    )
    .unwrap();

    let position = mock.query_position("1", "uatom");

    assert_eq!(
        position.position.realised_pnl,
        PnlAmounts {
            accrued_funding: SignedDecimal::zero(),
            price_pnl: SignedDecimal::from_str("98.685").unwrap(),
            // opening_fee: atom_opening_fee_total, // we are not paying opening fee for decrease
            opening_fee: SignedDecimal::from_str("-42.00385").unwrap(), // FIXME: rounding error
            closing_fee: SignedDecimal::from_str("-10.503675").unwrap(),
            pnl: SignedDecimal::from_str("46.177475").unwrap(),
        }
    );
}

#[test_case(
    None,
    SignedDecimal::from_str("250").unwrap(),
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
    Some(SignedDecimal::from_str("1200").unwrap()),
    SignedDecimal::from_str("2500").unwrap(),
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
    Some(SignedDecimal::from_str("1200").unwrap()),
    SignedDecimal::from_str("800").unwrap(),
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
    Some(SignedDecimal::from_str("1200").unwrap()),
    SignedDecimal::from_str("0").unwrap(),
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
    SignedDecimal::from_str("-2500").unwrap(),
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
    Some(SignedDecimal::from_str("-1200").unwrap()),
    SignedDecimal::from_str("-2500").unwrap(),
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
    Some(SignedDecimal::from_str("-1200").unwrap()),
    SignedDecimal::from_str("-600").unwrap(),
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
    Some(SignedDecimal::from_str("-1200").unwrap()),
    SignedDecimal::from_str("0").unwrap(),
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
    old_size: Option<SignedDecimal>,
    new_size: SignedDecimal,
    expected_fees: PositionFeesResponse,
) {
    let mut mock = MockEnv::new()
        .opening_fee_rate(Decimal::from_str("0.004").unwrap())
        .closing_fee_rate(Decimal::from_str("0.006").unwrap())
        .build()
        .unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let user = Addr::unchecked("jake");

    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(&[&credit_manager, &user], 1_000_000_000_000u128, &["uosmo", "uusdc"]);

    // deposit some big number of uusdc to vault
    mock.deposit_to_vault(&user, &[coin(1_000_000_000_000u128, "uusdc")]).unwrap();

    // init denoms
    mock.init_denom(
        &owner,
        "uosmo",
        Decimal::from_str("3").unwrap(),
        Decimal::from_str("1000000").unwrap(),
    )
    .unwrap();
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params("uosmo"),
        },
    );

    // set prices
    mock.set_price(&owner, "uusdc", Decimal::from_str("0.9").unwrap()).unwrap();
    mock.set_price(&owner, "uosmo", Decimal::from_str("1.25").unwrap()).unwrap();

    // open a position to change skew
    let size = SignedDecimal::from_str("10000").unwrap();
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
