use std::str::FromStr;

use cosmwasm_std::{coin, Coin, Decimal, Uint128};
use mars_types::{
    params::{PerpParams, PerpParamsUpdate},
    perps::{Accounting, Balance, CashFlow, PerpPosition, PnL},
    signed_uint::SignedUint,
};

use super::helpers::MockEnv;
use crate::tests::helpers::{default_perp_params, ONE_HOUR_SEC};

#[test]
fn accounting() {
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let user = "jake";

    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000_000u128, &["uosmo", "uatom", "uusdc"]);

    mock.set_price(&owner, "uusdc", Decimal::from_str("0.9").unwrap()).unwrap();

    // deposit some big number of uusdc to vault
    mock.deposit_to_vault(&credit_manager, Some(user), &[coin(1_000_000_000_000u128, "uusdc")])
        .unwrap();

    // init denoms
    mock.init_denom(&owner, "uosmo", Decimal::from_str("32").unwrap(), Uint128::new(1000000u128))
        .unwrap();
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                closing_fee_rate: Decimal::percent(1),
                opening_fee_rate: Decimal::percent(2),
                ..default_perp_params("uosmo")
            },
        },
    );
    mock.init_denom(&owner, "uatom", Decimal::from_str("30").unwrap(), Uint128::new(1000000u128))
        .unwrap();
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                closing_fee_rate: Decimal::percent(1),
                opening_fee_rate: Decimal::percent(2),
                ..default_perp_params("uatom")
            },
        },
    );

    // set entry prices
    mock.set_price(&owner, "uosmo", Decimal::from_str("1.25").unwrap()).unwrap();
    mock.set_price(&owner, "uatom", Decimal::from_str("10.5").unwrap()).unwrap();

    // check accounting in the beginning
    let osmo_accounting = mock.query_denom_accounting("uosmo");
    assert_eq!(osmo_accounting, Accounting::default());
    let atom_accounting = mock.query_denom_accounting("uatom");
    assert_eq!(atom_accounting, Accounting::default());
    let total_accounting = mock.query_total_accounting();
    assert_eq!(total_accounting, Accounting::default());

    let vault_state_before_opening = mock.query_vault_state();

    // open few positions for account 1
    let osmo_size = SignedUint::from_str("10000000").unwrap();
    let osmo_opening_fee = mock.query_opening_fee("uosmo", osmo_size).fee;
    mock.execute_perp_order(
        &credit_manager,
        "1",
        "uosmo",
        osmo_size,
        None,
        &[osmo_opening_fee.clone()],
    )
    .unwrap();
    let atom_size = SignedUint::from_str("-260000").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", atom_size).fee;
    mock.execute_perp_order(
        &credit_manager,
        "1",
        "uatom",
        atom_size,
        None,
        &[atom_opening_fee.clone()],
    )
    .unwrap();

    // check vault state after opening positions
    let vault_state = mock.query_vault_state();
    assert_eq!(vault_state_before_opening, vault_state);

    // check accounting after opening positions
    let osmo_accounting = mock.query_denom_accounting("uosmo");
    assert_eq!(
        osmo_accounting.cash_flow,
        CashFlow {
            opening_fee: SignedUint::from(osmo_opening_fee.amount),
            ..Default::default()
        }
    );
    let atom_accounting = mock.query_denom_accounting("uatom");
    assert_eq!(
        atom_accounting.cash_flow,
        CashFlow {
            opening_fee: SignedUint::from(atom_opening_fee.amount),
            ..Default::default()
        }
    );
    let total_accounting = mock.query_total_accounting();
    assert_eq!(
        total_accounting.cash_flow,
        CashFlow {
            opening_fee: SignedUint::from(osmo_opening_fee.amount + atom_opening_fee.amount),
            ..Default::default()
        }
    );
    assert_accounting(&total_accounting, &osmo_accounting, &atom_accounting);

    // move time forward by 12 hour
    mock.increment_by_time(12 * ONE_HOUR_SEC);

    // change only uosmo price
    mock.set_price(&owner, "uosmo", Decimal::from_str("2").unwrap()).unwrap();

    let vault_state_before_closing = mock.query_vault_state();

    // close uosmo position
    let pos = mock.query_position("1", "uosmo");
    mock.execute_perp_order(
        &credit_manager,
        "1",
        "uosmo",
        SignedUint::zero().checked_sub(osmo_size).unwrap(),
        Some(true),
        &from_position_to_coin(pos.position.unwrap()),
    )
    .unwrap();

    // check vault state after closing uosmo position
    let vault_state = mock.query_vault_state();
    assert_eq!(vault_state_before_closing, vault_state);

    // check accounting after closing uosmo position
    let osmo_accounting = mock.query_denom_accounting("uosmo");
    let atom_accounting = mock.query_denom_accounting("uatom");
    let total_accounting = mock.query_total_accounting();
    assert_accounting(&total_accounting, &osmo_accounting, &atom_accounting);

    // compare realized PnL
    let osmo_realized_pnl = mock.query_denom_realized_pnl_for_account("1", "uosmo");
    assert_eq!(osmo_realized_pnl.price_pnl.abs, osmo_accounting.cash_flow.price_pnl.abs);
    assert!(!osmo_realized_pnl.price_pnl.negative);
    assert_ne!(osmo_realized_pnl.price_pnl.negative, osmo_accounting.cash_flow.price_pnl.negative);
    assert_eq!(
        osmo_realized_pnl.accrued_funding.abs,
        osmo_accounting.cash_flow.accrued_funding.abs
    );
    assert!(osmo_realized_pnl.accrued_funding.negative);
    assert_ne!(
        osmo_realized_pnl.accrued_funding.negative,
        osmo_accounting.cash_flow.accrued_funding.negative
    );
    assert_eq!(osmo_realized_pnl.opening_fee.abs, osmo_accounting.cash_flow.opening_fee.abs);
    assert!(osmo_realized_pnl.opening_fee.negative);
    assert_ne!(
        osmo_realized_pnl.opening_fee.negative,
        osmo_accounting.cash_flow.opening_fee.negative
    );
    assert_eq!(osmo_realized_pnl.closing_fee.abs, osmo_accounting.cash_flow.closing_fee.abs);
    assert!(osmo_realized_pnl.closing_fee.negative);
    assert_ne!(
        osmo_realized_pnl.closing_fee.negative,
        osmo_accounting.cash_flow.opening_fee.negative
    );

    // move time forward by 12 hour
    mock.increment_by_time(12 * ONE_HOUR_SEC);

    // change only uatom price
    mock.set_price(&owner, "uatom", Decimal::from_str("15").unwrap()).unwrap();

    let vault_state_before_closing = mock.query_vault_state();

    // close uatom position
    let pos = mock.query_position("1", "uatom");
    mock.execute_perp_order(
        &credit_manager,
        "1",
        "uatom",
        SignedUint::zero().checked_sub(atom_size).unwrap(),
        Some(true),
        &from_position_to_coin(pos.position.unwrap()),
    )
    .unwrap();

    // check vault state after closing uatom position
    let vault_state = mock.query_vault_state();
    assert_eq!(vault_state_before_closing, vault_state);

    // check accounting after closing uatom position
    let osmo_accounting = mock.query_denom_accounting("uosmo");
    let atom_accounting = mock.query_denom_accounting("uatom");
    let total_accounting = mock.query_total_accounting();
    assert_accounting(&total_accounting, &osmo_accounting, &atom_accounting);

    // compare realized PnL
    let atom_realized_pnl = mock.query_denom_realized_pnl_for_account("1", "uatom");
    assert_eq!(atom_realized_pnl.price_pnl.abs, atom_accounting.cash_flow.price_pnl.abs);
    assert!(atom_realized_pnl.price_pnl.negative);
    assert_ne!(atom_realized_pnl.price_pnl.negative, atom_accounting.cash_flow.price_pnl.negative);
    assert_eq!(
        atom_realized_pnl.accrued_funding.abs,
        atom_accounting.cash_flow.accrued_funding.abs
    );
    assert!(atom_realized_pnl.accrued_funding.negative);
    assert_ne!(
        atom_realized_pnl.accrued_funding.negative,
        atom_accounting.cash_flow.accrued_funding.negative
    );
    assert_eq!(atom_realized_pnl.opening_fee.abs, atom_accounting.cash_flow.opening_fee.abs);
    assert!(atom_realized_pnl.opening_fee.negative);
    assert_ne!(
        atom_realized_pnl.opening_fee.negative,
        atom_accounting.cash_flow.opening_fee.negative
    );
    assert_eq!(atom_realized_pnl.closing_fee.abs, atom_accounting.cash_flow.closing_fee.abs);
    assert!(atom_realized_pnl.closing_fee.negative);
    assert_ne!(
        atom_realized_pnl.closing_fee.negative,
        atom_accounting.cash_flow.opening_fee.negative
    );
}

fn from_position_to_coin(pos: PerpPosition) -> Vec<Coin> {
    if let PnL::Loss(coin) = pos.unrealised_pnl.to_coins(&pos.base_denom).pnl {
        vec![coin]
    } else {
        vec![]
    }
}

fn assert_accounting(
    total_accounting: &Accounting,
    osmo_accounting: &Accounting,
    atom_accounting: &Accounting,
) {
    assert_eq!(
        total_accounting.cash_flow,
        add_cash_flows(&osmo_accounting.cash_flow, &atom_accounting.cash_flow)
    );
    assert_eq!(
        total_accounting.balance,
        add_balances(&osmo_accounting.balance, &atom_accounting.balance)
    );
    assert_eq!(
        total_accounting.withdrawal_balance,
        add_balances(&osmo_accounting.withdrawal_balance, &atom_accounting.withdrawal_balance)
    );
}

fn add_cash_flows(a: &CashFlow, b: &CashFlow) -> CashFlow {
    CashFlow {
        price_pnl: a.price_pnl.checked_add(b.price_pnl).unwrap(),
        opening_fee: a.opening_fee.checked_add(b.opening_fee).unwrap(),
        closing_fee: a.closing_fee.checked_add(b.closing_fee).unwrap(),
        accrued_funding: a.accrued_funding.checked_add(b.accrued_funding).unwrap(),
    }
}

fn add_balances(a: &Balance, b: &Balance) -> Balance {
    Balance {
        price_pnl: a.price_pnl.checked_add(b.price_pnl).unwrap(),
        opening_fee: a.opening_fee.checked_add(b.opening_fee).unwrap(),
        closing_fee: a.closing_fee.checked_add(b.closing_fee).unwrap(),
        accrued_funding: a.accrued_funding.checked_add(b.accrued_funding).unwrap(),
        total: a.total.checked_add(b.total).unwrap(),
    }
}
