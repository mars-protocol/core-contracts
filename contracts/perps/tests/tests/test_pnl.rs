use std::str::FromStr;

use cosmwasm_std::{coin, Addr, Coin, Decimal};
use mars_types::{
    math::SignedDecimal,
    params::PerpParamsUpdate,
    perps::{PerpPosition, PnL},
};

use super::helpers::MockEnv;
use crate::tests::helpers::default_perp_params;

// TODO fix numbers once moved to SignedUint
#[test]
fn computing_total_pnl() {
    let mut mock = MockEnv::new().opening_fee_rate(Decimal::zero()).build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let user = Addr::unchecked("jake");

    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(
        &[&credit_manager, &user],
        1_000_000_000_000u128,
        &["uosmo", "uatom", "utia", "uusdc"],
    );

    // deposit some big number of uusdc to vault
    mock.deposit_to_vault(&user, &[coin(1_000_000_000_000u128, "uusdc")]).unwrap();

    // init denoms
    mock.init_denom(&owner, "uosmo", Decimal::zero(), Decimal::one()).unwrap();
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params("uosmo"),
        },
    );
    mock.init_denom(&owner, "uatom", Decimal::zero(), Decimal::one()).unwrap();
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params("uatom"),
        },
    );
    mock.init_denom(&owner, "utia", Decimal::zero(), Decimal::one()).unwrap();
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params("utia"),
        },
    );

    mock.set_price(&owner, "uusdc", Decimal::from_str("1").unwrap()).unwrap();

    // set entry prices
    mock.set_price(&owner, "uosmo", Decimal::from_str("0.25").unwrap()).unwrap();
    mock.set_price(&owner, "uatom", Decimal::from_str("7.2").unwrap()).unwrap();
    mock.set_price(&owner, "utia", Decimal::from_str("2.65").unwrap()).unwrap();

    // open few positions for account 1
    mock.open_position(&credit_manager, "1", "uosmo", SignedDecimal::from_str("100").unwrap(), &[])
        .unwrap();
    mock.open_position(&credit_manager, "1", "utia", SignedDecimal::from_str("-250").unwrap(), &[])
        .unwrap();

    // open few positions for account 2
    mock.open_position(&credit_manager, "2", "uosmo", SignedDecimal::from_str("500").unwrap(), &[])
        .unwrap();
    mock.open_position(
        &credit_manager,
        "2",
        "uatom",
        SignedDecimal::from_str("-125").unwrap(),
        &[],
    )
    .unwrap();
    mock.open_position(&credit_manager, "2", "utia", SignedDecimal::from_str("1245").unwrap(), &[])
        .unwrap();

    // calculate total PnL if no price change
    let total_pnl = mock.query_total_pnl();
    assert_eq!(total_pnl.pnl, SignedDecimal::from_str("-832084.82375").unwrap());

    // change only uatom price
    mock.set_price(&owner, "uatom", Decimal::from_str("10").unwrap()).unwrap();

    // calculate total PnL after uatom price change
    let total_pnl = mock.query_total_pnl();
    assert_eq!(total_pnl.pnl, SignedDecimal::from_str("-810344.57375").unwrap());

    // change the rest of the prices
    mock.set_price(&owner, "uosmo", Decimal::from_str("0.1").unwrap()).unwrap();
    mock.set_price(&owner, "utia", Decimal::from_str("3.10").unwrap()).unwrap();

    // calculate total PnL
    let total_pnl = mock.query_total_pnl();
    assert_eq!(total_pnl.pnl, SignedDecimal::from_str("-764801.4575").unwrap());

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
    assert_eq!(total_pnl.pnl, SignedDecimal::from_str("22293.75").unwrap());

    // close uatom position
    let pos = mock.query_position("2", "uatom");
    mock.close_position(&credit_manager, "2", "uatom", &from_position_to_coin(pos.position))
        .unwrap();

    // after closing all positions, total PnL should be 0
    let total_pnl = mock.query_total_pnl();
    assert_eq!(total_pnl.pnl, SignedDecimal::from_str("0").unwrap());
}

fn from_position_to_coin(pos: PerpPosition) -> Vec<Coin> {
    if let PnL::Loss(coin) = pos.pnl.coins.pnl {
        vec![coin]
    } else {
        vec![]
    }
}
