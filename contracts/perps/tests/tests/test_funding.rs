use std::str::FromStr;

use cosmwasm_std::{coin, Coin, Decimal};
use mars_types::{
    math::SignedDecimal,
    params::PerpParamsUpdate,
    perps::{PerpPosition, PnL},
};

use super::helpers::MockEnv;
use crate::tests::helpers::default_perp_params;

const ONE_HOUR_SEC: u64 = 3600u64;

// TODO fix numbers once moved to SignedUint
#[test]
fn computing_funding() {
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let depositor = "peter";

    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000_000u128, &["ueth", "uusdc"]);

    // set usdc price
    mock.set_price(&owner, "uusdc", Decimal::from_str("0.9").unwrap()).unwrap();

    // deposit some big number of uusdc to vault
    mock.deposit_to_vault(&credit_manager, depositor, &[coin(1_000_000_000_000u128, "uusdc")])
        .unwrap();

    // init denoms
    mock.init_denom(
        &owner,
        "ueth",
        Decimal::from_str("3").unwrap(),
        Decimal::from_str("1000000").unwrap(),
    )
    .unwrap();
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: default_perp_params("ueth"),
        },
    );

    // set entry price
    mock.set_price(&owner, "ueth", Decimal::from_str("2000").unwrap()).unwrap();

    // user 1 opens long position
    mock.open_position(&credit_manager, "1", "ueth", SignedDecimal::from_str("300").unwrap(), &[])
        .unwrap();

    // query state for h0
    let user_1_pos = mock.query_position("1", "ueth");
    // assert_eq!(user_1_pos.position.pnl.coins.pnl, PnL::BreakEven);
    assert_eq!(user_1_pos.position.unrealised_pnl.values.pnl, SignedDecimal::zero());
    let ds = mock.query_perp_denom_state("ueth");
    assert_eq!(ds.rate, SignedDecimal::zero());
    assert_eq!(ds.total_entry_cost, SignedDecimal::from_str("600090").unwrap());
    assert_eq!(ds.total_entry_funding, SignedDecimal::zero());
    assert_eq!(ds.pnl_values.price_pnl, SignedDecimal::zero());
    assert_eq!(ds.pnl_values.accrued_funding, SignedDecimal::zero());
    assert_eq!(ds.pnl_values.pnl, SignedDecimal::zero());

    // move time forward by 2 hour
    mock.increment_by_time(2 * ONE_HOUR_SEC);

    // user 2 opens short position
    mock.open_position(&credit_manager, "2", "ueth", SignedDecimal::from_str("-150").unwrap(), &[])
        .unwrap();

    // query state for h2
    let user_1_pos = mock.query_position("1", "ueth");
    // assert_eq!(user_1_pos.position.pnl, PnL::Loss(coin(91u128, "uusdc")));
    assert_eq!(
        user_1_pos.position.unrealised_pnl.values.pnl,
        SignedDecimal::from_str("-91.87499999999939994").unwrap()
    );
    let user_2_pos = mock.query_position("2", "ueth");
    // assert_eq!(user_2_pos.position.pnl, PnL::BreakEven);
    assert_eq!(user_2_pos.position.unrealised_pnl.values.pnl, SignedDecimal::zero());
    let ds = mock.query_perp_denom_state("ueth");
    assert_eq!(ds.rate, SignedDecimal::from_str("0.000074999999999999").unwrap());
    assert_eq!(ds.total_entry_cost, SignedDecimal::from_str("300022.5").unwrap());
    assert_eq!(ds.total_entry_funding, SignedDecimal::from_str("1.0416666666663333").unwrap());
    assert_eq!(ds.pnl_values.price_pnl, SignedDecimal::from_str("-90").unwrap());
    assert_eq!(
        ds.pnl_values.accrued_funding,
        SignedDecimal::from_str("-1.87499999999939994").unwrap()
    );
    assert_eq!(ds.pnl_values.pnl, SignedDecimal::from_str("-91.87499999999939994").unwrap());

    // move time forward by 8 hour
    mock.increment_by_time(8 * ONE_HOUR_SEC);

    // query state for h10
    let user_1_pos = mock.query_position("1", "ueth");
    // assert_eq!(user_1_pos.position.pnl.coins.pnl, PnL::Loss(coin(121u128, "uusdc")));
    assert_eq!(
        user_1_pos.position.unrealised_pnl.values.pnl,
        SignedDecimal::from_str("-121.8749999999987997").unwrap()
    );
    let user_2_pos = mock.query_position("2", "ueth");
    // assert_eq!(user_2_pos.position.pnl.coins.pnl, PnL::Profit(coin(14u128, "uusdc"))); // spreadsheet says 15 (rounding error in SC?)
    assert_eq!(
        user_2_pos.position.unrealised_pnl.values.pnl,
        SignedDecimal::from_str("14.99999999999969988").unwrap()
    );
    let ds = mock.query_perp_denom_state("ueth");
    assert_eq!(ds.rate, SignedDecimal::from_str("0.000224999999999998").unwrap());
    assert_eq!(ds.total_entry_cost, SignedDecimal::from_str("300022.5").unwrap());
    assert_eq!(ds.total_entry_funding, SignedDecimal::from_str("1.0416666666663333").unwrap());
    assert_eq!(ds.pnl_values.price_pnl, SignedDecimal::from_str("-90").unwrap());
    assert_eq!(
        ds.pnl_values.accrued_funding,
        SignedDecimal::from_str("-16.87499999999909982").unwrap()
    );
    assert_eq!(ds.pnl_values.pnl, SignedDecimal::from_str("-106.87499999999909982").unwrap());

    // move time forward by 2 hour
    mock.increment_by_time(2 * ONE_HOUR_SEC);

    // price goes up
    mock.set_price(&owner, "ueth", Decimal::from_str("2020").unwrap()).unwrap();

    // query state for h12
    let user_1_pos = mock.query_position("1", "ueth");
    // assert_eq!(user_1_pos.position.pnl.coins.pnl, PnL::Profit(coin(5865u128, "uusdc")));
    assert_eq!(
        user_1_pos.position.unrealised_pnl.values.pnl,
        SignedDecimal::from_str("5865.51562500000120621").unwrap()
    );
    let user_2_pos = mock.query_position("2", "ueth");
    // assert_eq!(user_2_pos.position.pnl.coins.pnl, PnL::Loss(coin(2979u128, "uusdc")));
    assert_eq!(
        user_2_pos.position.unrealised_pnl.values.pnl,
        SignedDecimal::from_str("-2979.370312500000303075").unwrap()
    );

    // simulate realized pnl for user 1, reopen long position with the same size
    mock.close_position(&credit_manager, "1", "ueth", &from_position_to_coin(user_1_pos.position))
        .unwrap();
    mock.open_position(&credit_manager, "1", "ueth", SignedDecimal::from_str("300").unwrap(), &[])
        .unwrap();

    // query state for h12 after user 1 realized pnl
    let user_1_pos = mock.query_position("1", "ueth");
    // assert_eq!(user_1_pos.position.pnl.coins.pnl, PnL::BreakEven); // realized pnl should be zero
    assert_eq!(user_1_pos.position.unrealised_pnl.values.pnl, SignedDecimal::zero());
    let user_2_pos = mock.query_position("2", "ueth");
    // assert_eq!(user_2_pos.position.pnl.coins.pnl, PnL::Loss(coin(2979u128, "uusdc")));
    assert_eq!(
        user_2_pos.position.unrealised_pnl.values.pnl,
        SignedDecimal::from_str("-2979.370312500000303075").unwrap()
    );
    let ds = mock.query_perp_denom_state("ueth");
    assert_eq!(ds.rate, SignedDecimal::from_str("0.000262499999999998").unwrap());
    assert_eq!(ds.total_entry_cost, SignedDecimal::from_str("305932.5").unwrap());
    assert_eq!(ds.total_entry_funding, SignedDecimal::from_str("-48.3854166666656598").unwrap());
    assert_eq!(ds.pnl_values.price_pnl, SignedDecimal::from_str("-3000.675").unwrap()); // only user 2 has unrealized pnl
    assert_eq!(
        ds.pnl_values.accrued_funding,
        SignedDecimal::from_str("21.304687499999696925").unwrap()
    );
    assert_eq!(ds.pnl_values.pnl, SignedDecimal::from_str("-2979.370312500000303075").unwrap());

    // move time forward by 3 hour
    mock.increment_by_time(3 * ONE_HOUR_SEC);

    // price goes up
    mock.set_price(&owner, "ueth", Decimal::from_str("2040").unwrap()).unwrap();

    // query state for h15
    let user_1_pos = mock.query_position("1", "ueth");
    // assert_eq!(user_1_pos.position.pnl.coins.pnl, PnL::Profit(coin(5977u128, "uusdc")));
    assert_eq!(
        user_1_pos.position.unrealised_pnl.values.pnl,
        SignedDecimal::from_str("5977.76718750000061209").unwrap()
    );
    let user_2_pos = mock.query_position("2", "ueth");
    // assert_eq!(user_2_pos.position.pnl.coins.pnl, PnL::Loss(coin(5968u128, "uusdc")));
    assert_eq!(
        user_2_pos.position.unrealised_pnl.values.pnl,
        SignedDecimal::from_str("-5968.92890625000060912").unwrap()
    );
    let ds = mock.query_perp_denom_state("ueth");
    assert_eq!(ds.rate, SignedDecimal::from_str("0.000318749999999998").unwrap());
    assert_eq!(ds.total_entry_cost, SignedDecimal::from_str("305932.5").unwrap());
    assert_eq!(ds.total_entry_funding, SignedDecimal::from_str("-48.3854166666656598").unwrap());
    assert_eq!(ds.pnl_values.price_pnl, SignedDecimal::from_str("-1.35").unwrap());
    assert_eq!(
        ds.pnl_values.accrued_funding,
        SignedDecimal::from_str("10.18828125000000297").unwrap()
    );
    assert_eq!(ds.pnl_values.pnl, SignedDecimal::from_str("8.83828125000000297").unwrap());

    // simulate realized pnl for user 2, increase short position size by 200 (total 350)
    mock.close_position(&credit_manager, "2", "ueth", &from_position_to_coin(user_2_pos.position))
        .unwrap();
    mock.open_position(&credit_manager, "2", "ueth", SignedDecimal::from_str("-350").unwrap(), &[])
        .unwrap();

    // query state for h15 after user 2 realized pnl
    let user_1_pos = mock.query_position("1", "ueth");
    // assert_eq!(user_1_pos.position.pnl, PnL::Profit(coin(5855u128, "uusdc")));
    assert_eq!(
        user_1_pos.position.unrealised_pnl.values.pnl,
        SignedDecimal::from_str("5855.36718750000061209").unwrap()
    );
    let user_2_pos = mock.query_position("2", "ueth");
    // assert_eq!(user_2_pos.position.pnl, PnL::BreakEven);
    assert_eq!(user_2_pos.position.unrealised_pnl.values.pnl, SignedDecimal::zero());
    let ds = mock.query_perp_denom_state("ueth");
    assert_eq!(ds.rate, SignedDecimal::from_str("0.000318749999999998").unwrap()); // rate shouldn't change after closing and opening the same position size
    assert_eq!(ds.total_entry_cost, SignedDecimal::from_str("-108089.25").unwrap());
    assert_eq!(ds.total_entry_funding, SignedDecimal::from_str("37.0581597222212054").unwrap());
    assert_eq!(ds.pnl_values.price_pnl, SignedDecimal::from_str("5877.6").unwrap());
    assert_eq!(
        ds.pnl_values.accrued_funding,
        SignedDecimal::from_str("-22.23281249999938791").unwrap()
    );
    assert_eq!(ds.pnl_values.pnl, SignedDecimal::from_str("5855.36718750000061209").unwrap());

    // move time forward by 5 hour
    mock.increment_by_time(5 * ONE_HOUR_SEC);

    // price goes down
    mock.set_price(&owner, "ueth", Decimal::from_str("1980").unwrap()).unwrap();

    // query state for h20
    let user_1_pos = mock.query_position("1", "ueth");
    // assert_eq!(user_1_pos.position.pnl, PnL::Loss(coin(12178u128, "uusdc")));
    assert_eq!(
        user_1_pos.position.unrealised_pnl.values.pnl,
        SignedDecimal::from_str("-12178.54453124999899191").unwrap()
    );
    let user_2_pos = mock.query_position("2", "ueth");
    // assert_eq!(user_2_pos.position.pnl, PnL::Profit(coin(21046u128, "uusdc")));
    assert_eq!(
        user_2_pos.position.unrealised_pnl.values.pnl,
        SignedDecimal::from_str("21046.388671874999538").unwrap()
    );
    let ds = mock.query_perp_denom_state("ueth");
    assert_eq!(ds.rate, SignedDecimal::from_str("0.000287499999999999").unwrap());
    assert_eq!(ds.total_entry_cost, SignedDecimal::from_str("-108089.25").unwrap());
    assert_eq!(ds.total_entry_funding, SignedDecimal::from_str("37.0581597222212054").unwrap());
    assert_eq!(ds.pnl_values.price_pnl, SignedDecimal::from_str("8883.825").unwrap());
    assert_eq!(
        ds.pnl_values.accrued_funding,
        SignedDecimal::from_str("-15.98085937499945391").unwrap()
    );
    assert_eq!(ds.pnl_values.pnl, SignedDecimal::from_str("8867.84414062500054609").unwrap());

    // query user 1 realized pnl
}

fn from_position_to_coin(pos: PerpPosition) -> Vec<Coin> {
    if let PnL::Loss(coin) = pos.unrealised_pnl.coins.pnl {
        vec![coin]
    } else {
        vec![]
    }
}
