use std::str::FromStr;

use cosmwasm_std::{coin, Coin, Decimal, Uint128};
use mars_types::{
    params::{PerpParams, PerpParamsUpdate},
    perps::{PerpPosition, PnL},
    signed_uint::SignedUint,
};

use super::helpers::MockEnv;
use crate::tests::helpers::default_perp_params;

// TODO fix numbers once moved to SignedUint
#[test]
fn computing_total_pnl() {
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let user = "jake";

    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(
        &[&credit_manager],
        1_000_000_000_000_000u128,
        &["uosmo", "uatom", "utia", "uusdc"],
    );

    mock.set_price(&owner, "uusdc", Decimal::from_str("1").unwrap()).unwrap();

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
                closing_fee_rate: Decimal::percent(1),
                ..default_perp_params("uosmo")
            },
        },
    );
    mock.init_denom(&owner, "uatom", Decimal::from_str("3").unwrap(), Uint128::new(1000000u128))
        .unwrap();
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                closing_fee_rate: Decimal::percent(1),
                ..default_perp_params("uatom")
            },
        },
    );
    mock.init_denom(&owner, "utia", Decimal::from_str("3").unwrap(), Uint128::new(1000000u128))
        .unwrap();
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                closing_fee_rate: Decimal::percent(1),
                ..default_perp_params("utia")
            },
        },
    );

    // set entry prices
    mock.set_price(&owner, "uosmo", Decimal::from_str("0.25").unwrap()).unwrap();
    mock.set_price(&owner, "uatom", Decimal::from_str("7.2").unwrap()).unwrap();
    mock.set_price(&owner, "utia", Decimal::from_str("2.65").unwrap()).unwrap();

    // open few positions for account 1
    mock.open_position(&credit_manager, "1", "uosmo", SignedUint::from_str("100").unwrap(), &[])
        .unwrap();
    mock.open_position(&credit_manager, "1", "utia", SignedUint::from_str("-250").unwrap(), &[])
        .unwrap();

    // open few positions for account 2
    mock.open_position(&credit_manager, "2", "uosmo", SignedUint::from_str("500").unwrap(), &[])
        .unwrap();
    mock.open_position(&credit_manager, "2", "uatom", SignedUint::from_str("-125").unwrap(), &[])
        .unwrap();
    mock.open_position(&credit_manager, "2", "utia", SignedUint::from_str("1245").unwrap(), &[])
        .unwrap();

    // calculate total PnL if no price change
    let total_pnl = mock.query_total_pnl();
    assert_eq!(total_pnl.pnl, SignedUint::from_str("-52").unwrap());

    // change only uatom price
    mock.set_price(&owner, "uatom", Decimal::from_str("10").unwrap()).unwrap();

    // calculate total PnL after uatom price change
    let total_pnl = mock.query_total_pnl();
    assert_eq!(total_pnl.pnl, SignedUint::from_str("-406").unwrap());

    // change the rest of the prices
    mock.set_price(&owner, "uosmo", Decimal::from_str("0.1").unwrap()).unwrap();
    mock.set_price(&owner, "utia", Decimal::from_str("3.10").unwrap()).unwrap();

    // calculate total PnL
    let total_pnl = mock.query_total_pnl();
    assert_eq!(total_pnl.pnl, SignedUint::from_str("-54").unwrap());

    // close all positions except uatom
    let pos = mock.query_position("1", "uosmo");
    mock.close_position(&credit_manager, "1", "uosmo", &from_position_to_coin(pos.position))
        .unwrap();
    let pos = mock.query_position("1", "utia");
    mock.close_position(&credit_manager, "1", "utia", &from_position_to_coin(pos.position))
        .unwrap();
    let pos = mock.query_position("2", "uosmo");
    mock.close_position(&credit_manager, "2", "uosmo", &from_position_to_coin(pos.position))
        .unwrap();
    let pos = mock.query_position("2", "utia");
    mock.close_position(&credit_manager, "2", "utia", &from_position_to_coin(pos.position))
        .unwrap();

    // only uatom position is left
    let total_pnl = mock.query_total_pnl();
    assert_eq!(total_pnl.pnl, SignedUint::from_str("-363").unwrap());

    // close uatom position
    let pos = mock.query_position("2", "uatom");
    mock.close_position(&credit_manager, "2", "uatom", &from_position_to_coin(pos.position))
        .unwrap();

    // after closing all positions, total PnL should be 0
    let total_pnl = mock.query_total_pnl();
    assert_eq!(total_pnl.pnl, SignedUint::from_str("0").unwrap());
}

fn from_position_to_coin(pos: PerpPosition) -> Vec<Coin> {
    if let PnL::Loss(coin) = pos.unrealised_pnl.to_coins(&pos.base_denom).pnl {
        vec![coin]
    } else {
        vec![]
    }
}
