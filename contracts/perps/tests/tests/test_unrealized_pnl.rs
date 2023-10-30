use std::str::FromStr;

use cosmwasm_std::{coin, Addr, Coin, Decimal};
use mars_types::{
    math::SignedDecimal,
    perps::{PerpPosition, PnL},
};

use super::helpers::MockEnv;

#[test]
fn computing_total_unrealized_pnl() {
    let mut mock = MockEnv::new().build().unwrap();

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

    // enable all denoms
    mock.enable_denom(&owner, "uosmo").unwrap();
    mock.enable_denom(&owner, "uatom").unwrap();
    mock.enable_denom(&owner, "utia").unwrap();

    // set entry prices
    mock.set_price(&owner, "uosmo", Decimal::from_str("0.25").unwrap()).unwrap();
    mock.set_price(&owner, "uatom", Decimal::from_str("7.2").unwrap()).unwrap();
    mock.set_price(&owner, "utia", Decimal::from_str("2.65").unwrap()).unwrap();

    // open few positions for account 1
    mock.open_position(&credit_manager, "1", "uosmo", SignedDecimal::from_str("100").unwrap())
        .unwrap();
    mock.open_position(&credit_manager, "1", "utia", SignedDecimal::from_str("-250").unwrap())
        .unwrap();

    // open few positions for account 2
    mock.open_position(&credit_manager, "2", "uosmo", SignedDecimal::from_str("500").unwrap())
        .unwrap();
    mock.open_position(&credit_manager, "2", "uatom", SignedDecimal::from_str("-125").unwrap())
        .unwrap();
    mock.open_position(&credit_manager, "2", "utia", SignedDecimal::from_str("1245").unwrap())
        .unwrap();

    // calculate total unrealized PnL if no price change
    let total_unrealized_pnl = mock.query_total_unrealized_pnl();
    assert_eq!(total_unrealized_pnl, SignedDecimal::from_str("0").unwrap());

    // change only uatom price
    mock.set_price(&owner, "uatom", Decimal::from_str("10").unwrap()).unwrap();

    // calculate total unrealized PnL after uatom price change
    let total_unrealized_pnl = mock.query_total_unrealized_pnl();
    // -125 * (10 - 7.2)
    assert_eq!(total_unrealized_pnl, SignedDecimal::from_str("-350").unwrap());

    // change the rest of the prices
    mock.set_price(&owner, "uosmo", Decimal::from_str("0.1").unwrap()).unwrap();
    mock.set_price(&owner, "utia", Decimal::from_str("3.10").unwrap()).unwrap();

    // calculate total unrealized PnL
    let total_unrealized_pnl = mock.query_total_unrealized_pnl();
    // 100 * (0.1 - 0.25) + -250 * (3.10 - 2.65) + 500 * (0.1 - 0.25) + -125 * (10 - 7.2) + 1245 * (3.10 - 2.65)
    assert_eq!(total_unrealized_pnl, SignedDecimal::from_str("7.75").unwrap());

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
    let total_unrealized_pnl = mock.query_total_unrealized_pnl();
    // -125 * (10 - 7.2)
    assert_eq!(total_unrealized_pnl, SignedDecimal::from_str("-350").unwrap());

    // close uatom position
    let pos = mock.query_position("2", "uatom");
    mock.close_position(&credit_manager, "2", "uatom", &from_position_to_coin(pos.position))
        .unwrap();

    // after closing all positions, total unrealized PnL should be 0
    let total_unrealized_pnl = mock.query_total_unrealized_pnl();
    assert_eq!(total_unrealized_pnl, SignedDecimal::from_str("0").unwrap());
}

fn from_position_to_coin(pos: PerpPosition) -> Vec<Coin> {
    if let PnL::Loss(coin) = pos.pnl {
        vec![coin]
    } else {
        vec![]
    }
}
