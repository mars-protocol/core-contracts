use std::str::FromStr;

use cosmwasm_std::{coin, Addr, Coin, Decimal};
use mars_types::{
    math::SignedDecimal,
    perps::{PerpPosition, PnL},
};

use super::helpers::MockEnv;

const ONE_HOUR_SEC: u64 = 3600u64;

#[test]
fn computing_funding() {
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let depositor = Addr::unchecked("peter");

    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(&[&credit_manager, &depositor], 1_000_000_000_000u128, &["ueth", "uusdc"]);

    // deposit some big number of uusdc to vault
    mock.deposit_to_vault(&depositor, &[coin(1_000_000_000_000u128, "uusdc")]).unwrap();

    // init denoms
    mock.init_denom(
        &owner,
        "ueth",
        Decimal::from_str("3").unwrap(),
        Decimal::from_str("1000000").unwrap(),
    )
    .unwrap();

    // set entry price
    mock.set_price(&owner, "ueth", Decimal::from_str("2000").unwrap()).unwrap();

    // user 1 opens long position
    mock.open_position(&credit_manager, "1", "ueth", SignedDecimal::from_str("300").unwrap())
        .unwrap();

    // user 2 opens short position
    mock.open_position(&credit_manager, "2", "ueth", SignedDecimal::from_str("-150").unwrap())
        .unwrap();

    // query state for h0
    let user_1_pos = mock.query_position("1", "ueth");
    assert_eq!(user_1_pos.position.pnl, PnL::BreakEven);
    let user_2_pos = mock.query_position("2", "ueth");
    assert_eq!(user_2_pos.position.pnl, PnL::BreakEven);
    let ds = mock.query_perp_denom_state("ueth");
    assert_eq!(ds.rate, SignedDecimal::zero());
    assert_eq!(ds.index, SignedDecimal::one());
    assert_eq!(ds.total_size, SignedDecimal::from_str("150").unwrap());
    assert_eq!(ds.pnl_values.unrealized_pnl, SignedDecimal::zero());
    assert_eq!(ds.pnl_values.accrued_funding, SignedDecimal::zero());
    assert_eq!(ds.pnl_values.pnl, SignedDecimal::zero());

    // move time forward by 10 hour
    mock.increment_by_time(10 * ONE_HOUR_SEC);

    // query state for h10
    let user_1_pos = mock.query_position("1", "ueth");
    assert_eq!(user_1_pos.position.pnl, PnL::Loss(coin(112u128, "uusdc"))); // unrealized_pnl - (index_h10 / index_h0 - 1) * 300 * 2000
    let user_2_pos = mock.query_position("2", "ueth");
    assert_eq!(user_2_pos.position.pnl, PnL::Profit(coin(56u128, "uusdc"))); // unrealized_pnl - (index_h10 / index_h0 - 1) * -150 * 2000
    let ds = mock.query_perp_denom_state("ueth");
    assert_eq!(ds.rate, SignedDecimal::from_str("0.000187499999999999").unwrap());
    assert_eq!(ds.index, SignedDecimal::from_str("1.000187499999999999").unwrap());
    assert_eq!(ds.total_size, SignedDecimal::from_str("150").unwrap()); // longs pay shorts to incentivize opening short position
    assert_eq!(ds.pnl_values.unrealized_pnl, SignedDecimal::zero()); // price doesn't change so no unrealized pnl
    assert_eq!(ds.pnl_values.accrued_funding, SignedDecimal::from_str("56.2499999999997").unwrap()); // user 1 pays 112, user 2 receives 56, net 56 goes to vault
    assert_eq!(ds.pnl_values.pnl, SignedDecimal::from_str("-56.2499999999997").unwrap()); // unrealized_pnl - accrued_funding, sum of pnl for all positions

    // move time forward by 2 hour
    mock.increment_by_time(2 * ONE_HOUR_SEC);

    // price goes up
    mock.set_price(&owner, "ueth", Decimal::from_str("2020").unwrap()).unwrap();

    // query state for h12
    let user_1_pos = mock.query_position("1", "ueth");
    assert_eq!(user_1_pos.position.pnl, PnL::Profit(coin(5863u128, "uusdc")));
    let user_2_pos = mock.query_position("2", "ueth");
    assert_eq!(user_2_pos.position.pnl, PnL::Loss(coin(2931u128, "uusdc")));
    let ds = mock.query_perp_denom_state("ueth");
    assert_eq!(ds.rate, SignedDecimal::from_str("0.000225").unwrap());
    assert_eq!(ds.index, SignedDecimal::from_str("1.000225").unwrap());
    assert_eq!(ds.total_size, SignedDecimal::from_str("150").unwrap());
    assert_eq!(ds.pnl_values.unrealized_pnl, SignedDecimal::from_str("3000").unwrap());
    assert_eq!(ds.pnl_values.accrued_funding, SignedDecimal::from_str("68.175").unwrap()); // if pnl is realized, vault receives 68.175
    assert_eq!(ds.pnl_values.pnl, SignedDecimal::from_str("2931.825").unwrap()); // if pnl is realized, vault decreases by 2931.825

    // simulate realized pnl for user 1, reopen long position with the same size
    mock.close_position(&credit_manager, "1", "ueth", &from_position_to_coin(user_1_pos.position))
        .unwrap();
    mock.open_position(&credit_manager, "1", "ueth", SignedDecimal::from_str("300").unwrap())
        .unwrap();

    // query state for h12 after user 1 realized pnl
    let user_1_pos = mock.query_position("1", "ueth");
    assert_eq!(user_1_pos.position.pnl, PnL::BreakEven); // realized pnl should be zero
    let user_2_pos = mock.query_position("2", "ueth");
    assert_eq!(user_2_pos.position.pnl, PnL::Loss(coin(2931u128, "uusdc")));
    let ds = mock.query_perp_denom_state("ueth");
    assert_eq!(ds.rate, SignedDecimal::from_str("0.000225").unwrap()); // rate and index shouldn't change after closing and opening the same position size
    assert_eq!(ds.index, SignedDecimal::from_str("1.000225").unwrap());
    assert_eq!(ds.total_size, SignedDecimal::from_str("150").unwrap());
    assert_eq!(ds.pnl_values.unrealized_pnl, SignedDecimal::from_str("-3000").unwrap()); // only user 2 has unrealized pnl
    assert_eq!(
        ds.pnl_values.accrued_funding,
        SignedDecimal::from_str("-68.175000000000000468").unwrap()
    );
    assert_eq!(ds.pnl_values.pnl, SignedDecimal::from_str("-2931.824999999999999532").unwrap());

    // move time forward by 3 hour
    mock.increment_by_time(3 * ONE_HOUR_SEC);

    // price goes up
    mock.set_price(&owner, "ueth", Decimal::from_str("2040").unwrap()).unwrap();

    // query state for h15
    let user_1_pos = mock.query_position("1", "ueth");
    assert_eq!(user_1_pos.position.pnl, PnL::Profit(coin(5827u128, "uusdc")));
    let user_2_pos = mock.query_position("2", "ueth");
    assert_eq!(user_2_pos.position.pnl, PnL::Loss(coin(5845u128, "uusdc")));
    let ds = mock.query_perp_denom_state("ueth");
    assert_eq!(ds.rate, SignedDecimal::from_str("0.00028125").unwrap());
    assert_eq!(ds.index, SignedDecimal::from_str("1.00050631328125").unwrap());
    assert_eq!(ds.total_size, SignedDecimal::from_str("150").unwrap());
    assert_eq!(ds.pnl_values.unrealized_pnl, SignedDecimal::from_str("0").unwrap());
    assert_eq!(
        ds.pnl_values.accrued_funding,
        SignedDecimal::from_str("17.193135937499999527").unwrap()
    );
    assert_eq!(ds.pnl_values.pnl, SignedDecimal::from_str("-17.193135937499999527").unwrap());

    // simulate realized pnl for user 2, increase short position size by 200 (total 350)
    mock.close_position(&credit_manager, "2", "ueth", &from_position_to_coin(user_2_pos.position))
        .unwrap();
    mock.open_position(&credit_manager, "2", "ueth", SignedDecimal::from_str("-350").unwrap())
        .unwrap();

    // query state for h15 after user 2 realized pnl
    let user_1_pos = mock.query_position("1", "ueth");
    assert_eq!(user_1_pos.position.pnl, PnL::Profit(coin(5827u128, "uusdc")));
    let user_2_pos = mock.query_position("2", "ueth");
    assert_eq!(user_2_pos.position.pnl, PnL::BreakEven);
    let ds = mock.query_perp_denom_state("ueth");
    assert_eq!(ds.rate, SignedDecimal::from_str("0.00028125").unwrap()); // rate and index shouldn't change after closing and opening the same position size
    assert_eq!(ds.index, SignedDecimal::from_str("1.00050631328125").unwrap());
    assert_eq!(ds.total_size, SignedDecimal::from_str("-50").unwrap());
    assert_eq!(ds.pnl_values.unrealized_pnl, SignedDecimal::from_str("6000").unwrap());
    assert_eq!(
        ds.pnl_values.accrued_funding,
        SignedDecimal::from_str("172.125000000000000729").unwrap()
    );
    assert_eq!(ds.pnl_values.pnl, SignedDecimal::from_str("5827.874999999999999271").unwrap());

    // move time forward by 5 hour
    mock.increment_by_time(5 * ONE_HOUR_SEC);

    // price goes down
    mock.set_price(&owner, "ueth", Decimal::from_str("1980").unwrap()).unwrap();

    // query state for h20
    let user_1_pos = mock.query_position("1", "ueth");
    assert_eq!(user_1_pos.position.pnl, PnL::Loss(coin(12315u128, "uusdc")));
    let user_2_pos = mock.query_position("2", "ueth");
    assert_eq!(user_2_pos.position.pnl, PnL::Profit(coin(21173u128, "uusdc")));
    let ds = mock.query_perp_denom_state("ueth");
    assert_eq!(ds.rate, SignedDecimal::from_str("0.000250000000000001").unwrap());
    assert_eq!(ds.index, SignedDecimal::from_str("1.000756439859570313").unwrap());
    assert_eq!(ds.total_size, SignedDecimal::from_str("-50").unwrap());
    assert_eq!(ds.pnl_values.unrealized_pnl, SignedDecimal::from_str("9000").unwrap());
    assert_eq!(
        ds.pnl_values.accrued_funding,
        SignedDecimal::from_str("142.354265624999951316").unwrap()
    );
    assert_eq!(ds.pnl_values.pnl, SignedDecimal::from_str("8857.645734375000048684").unwrap());

    // query user 1 realized pnl
}

fn from_position_to_coin(pos: PerpPosition) -> Vec<Coin> {
    if let PnL::Loss(coin) = pos.pnl {
        vec![coin]
    } else {
        vec![]
    }
}
