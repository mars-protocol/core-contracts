use std::str::FromStr;

use cosmwasm_std::{coin, Addr, Decimal, Uint128};
use mars_mock_oracle::msg::CoinPrice;
use mars_types::{
    credit_manager::{
        Action::{ClosePerp, Deposit, OpenPerp},
        Positions,
    },
    math::SignedDecimal,
    oracle::ActionKind,
    perps::PnL,
};

use super::helpers::{coin_info, uatom_info, uosmo_info, AccountToFund, MockEnv};

#[test]
fn perp_position_when_usdc_in_account() {
    let osmo_info = uosmo_info();
    let atom_info = uatom_info();
    let usdc_info = coin_info("uusdc");

    let contract_owner = Addr::unchecked("owner");
    let cm_user = Addr::unchecked("user");
    let vault_depositor = Addr::unchecked("vault_depositor");

    let osmo_coin_deposited = osmo_info.to_coin(10000);
    let usdc_coin_deposited = usdc_info.to_coin(1000);

    let mut mock = MockEnv::new()
        .owner(contract_owner.as_str())
        .set_params(&[osmo_info, atom_info.clone(), usdc_info.clone()])
        .fund_account(AccountToFund {
            addr: cm_user.clone(),
            funds: vec![osmo_coin_deposited.clone(), usdc_coin_deposited.clone()],
        })
        .fund_account(AccountToFund {
            addr: vault_depositor.clone(),
            funds: vec![coin(100000, usdc_info.denom.clone())],
        })
        .build()
        .unwrap();
    let account_id = mock.create_credit_account(&cm_user).unwrap();

    // setup perp contract
    mock.init_perp_denom(
        &contract_owner,
        &atom_info.denom,
        Decimal::from_str("3").unwrap(),
        Decimal::from_str("1000000").unwrap(),
    )
    .unwrap();
    mock.deposit_to_perp_vault(&vault_depositor, &coin(100000, usdc_info.denom.clone())).unwrap();

    let perp_size = SignedDecimal::from_str("200").unwrap();

    // check perp data before any action
    let perp_usdc_balance = mock.query_balance(mock.perps.address(), &usdc_info.denom);
    let opening_fee = mock.query_perp_opening_fee(&atom_info.denom, perp_size);

    // open perp position
    mock.update_credit_account(
        &account_id,
        &cm_user,
        vec![
            Deposit(osmo_coin_deposited.clone()),
            Deposit(usdc_coin_deposited.clone()),
            OpenPerp {
                denom: atom_info.denom.clone(),
                size: perp_size,
            },
        ],
        &[osmo_coin_deposited.clone(), usdc_coin_deposited.clone()],
    )
    .unwrap();

    // check position data
    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 2);
    assert_present(&position, &osmo_coin_deposited.denom, osmo_coin_deposited.amount);
    let expected_pos_usdc_amt_after_opening_perp =
        usdc_coin_deposited.amount - opening_fee.fee.amount; // opening fee deducted from deposit
    assert_present(&position, &usdc_coin_deposited.denom, expected_pos_usdc_amt_after_opening_perp);
    assert_eq!(position.lends.len(), 0);
    assert_eq!(position.debts.len(), 0);
    assert_eq!(position.perps.len(), 1);
    let perp_position = position.perps.first().unwrap().clone();
    let expected_perp_position = mock.query_perp_position(&account_id, &atom_info.denom).position;
    assert_eq!(perp_position, expected_perp_position);

    // check if perp balance increased by opening fee
    let current_perp_usdc_balance = mock.query_balance(mock.perps.address(), &usdc_info.denom);
    let expected_perp_usdc_balance_after_opening_perp =
        perp_usdc_balance.amount + opening_fee.fee.amount;
    assert_eq!(current_perp_usdc_balance.amount, expected_perp_usdc_balance_after_opening_perp);

    // simulate loss in perp position
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: atom_info.denom.clone(),
        price: atom_info.price * Decimal::percent(90u64), // 10% loss in price
    });

    // check perp position pnl
    let perp_position = mock.query_perp_position(&account_id, &atom_info.denom).position;
    let loss_amt = pnl_loss(perp_position.pnl.coins.pnl);

    // close perp position
    mock.update_credit_account(
        &account_id,
        &cm_user,
        vec![ClosePerp {
            denom: atom_info.denom.clone(),
        }],
        &[],
    )
    .unwrap();

    // check position data
    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 2);
    assert_present(&position, &osmo_coin_deposited.denom, osmo_coin_deposited.amount);
    assert_present(
        &position,
        &usdc_coin_deposited.denom,
        expected_pos_usdc_amt_after_opening_perp - loss_amt, // loss deducted from deposit
    );
    assert_eq!(position.lends.len(), 0);
    assert_eq!(position.debts.len(), 0);
    assert_eq!(position.perps.len(), 0);

    // check if perp balance increased by position loss
    let current_perp_usdc_balance = mock.query_balance(mock.perps.address(), &usdc_info.denom);
    assert_eq!(
        current_perp_usdc_balance.amount,
        expected_perp_usdc_balance_after_opening_perp + loss_amt
    );
}

fn assert_present(res: &Positions, denom: &str, amount: Uint128) {
    res.deposits.iter().find(|item| item.denom == denom && item.amount == amount).unwrap();
}

fn pnl_loss(pnl: PnL) -> Uint128 {
    match pnl {
        PnL::Loss(coin) => coin.amount,
        _ => panic!("expected loss"),
    }
}

#[test]
fn perp_position_when_not_enough_usdc_in_account() {
    let osmo_info = uosmo_info();
    let atom_info = uatom_info();
    let usdc_info = coin_info("uusdc");

    let contract_owner = Addr::unchecked("owner");
    let cm_user = Addr::unchecked("user");
    let vault_depositor = Addr::unchecked("vault_depositor");

    let osmo_coin_deposited = osmo_info.to_coin(10000);
    let usdc_coin_deposited = usdc_info.to_coin(2);

    let mut mock = MockEnv::new()
        .owner(contract_owner.as_str())
        .set_params(&[osmo_info, atom_info.clone(), usdc_info.clone()])
        .fund_account(AccountToFund {
            addr: cm_user.clone(),
            funds: vec![
                osmo_coin_deposited.clone(),
                usdc_coin_deposited.clone(),
                usdc_coin_deposited.clone(),
            ], // deposit usdc twice
        })
        .fund_account(AccountToFund {
            addr: vault_depositor.clone(),
            funds: vec![coin(100000, usdc_info.denom.clone())],
        })
        .build()
        .unwrap();
    let account_id = mock.create_credit_account(&cm_user).unwrap();

    // setup perp contract
    mock.init_perp_denom(
        &contract_owner,
        &atom_info.denom,
        Decimal::from_str("3").unwrap(),
        Decimal::from_str("1000000").unwrap(),
    )
    .unwrap();
    mock.deposit_to_perp_vault(&vault_depositor, &coin(100000, usdc_info.denom.clone())).unwrap();

    let perp_size = SignedDecimal::from_str("400").unwrap();

    // check perp data before any action
    let perp_usdc_balance = mock.query_balance(mock.perps.address(), &usdc_info.denom);
    let opening_fee = mock.query_perp_opening_fee(&atom_info.denom, perp_size);

    // open perp position
    mock.update_credit_account(
        &account_id,
        &cm_user,
        vec![
            Deposit(osmo_coin_deposited.clone()),
            Deposit(usdc_coin_deposited.clone()),
            OpenPerp {
                denom: atom_info.denom.clone(),
                size: perp_size,
            },
        ],
        &[osmo_coin_deposited.clone(), usdc_coin_deposited.clone()],
    )
    .unwrap();

    // check position data
    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 1); // only osmo left, usdc is taken for opening fee payment
    assert_present(&position, &osmo_coin_deposited.denom, osmo_coin_deposited.amount);
    assert_eq!(position.lends.len(), 0);
    assert_eq!(position.debts.len(), 1);
    let debt = position.debts.first().unwrap();
    let expected_pos_debt_after_opening_perp =
        opening_fee.fee.amount - usdc_coin_deposited.amount + Uint128::new(1); // simulated interest
    assert_eq!(debt.amount, expected_pos_debt_after_opening_perp);
    assert_eq!(position.perps.len(), 1);
    let perp_position = position.perps.first().unwrap().clone();
    let expected_perp_position = mock.query_perp_position(&account_id, &atom_info.denom).position;
    assert_eq!(perp_position, expected_perp_position);

    // check if perp balance increased by opening fee
    let current_perp_usdc_balance = mock.query_balance(mock.perps.address(), &usdc_info.denom);
    let expected_perp_usdc_balance_after_opening_perp =
        perp_usdc_balance.amount + opening_fee.fee.amount;
    assert_eq!(current_perp_usdc_balance.amount, expected_perp_usdc_balance_after_opening_perp);

    // deposit usdc again
    mock.update_credit_account(
        &account_id,
        &cm_user,
        vec![Deposit(usdc_coin_deposited.clone())],
        &[usdc_coin_deposited.clone()],
    )
    .unwrap();

    // simulate loss in perp position
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: atom_info.denom.clone(),
        price: atom_info.price * Decimal::percent(90u64), // 10% loss in price
    });

    // check perp position pnl
    let perp_position = mock.query_perp_position(&account_id, &atom_info.denom).position;
    let loss_amt = pnl_loss(perp_position.pnl.coins.pnl);

    // close perp position
    mock.update_credit_account(
        &account_id,
        &cm_user,
        vec![ClosePerp {
            denom: atom_info.denom.clone(),
        }],
        &[],
    )
    .unwrap();

    // check position data
    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 1); // only osmo left, usdc is taken for closing perp payment
    assert_present(&position, &osmo_coin_deposited.denom, osmo_coin_deposited.amount);
    assert_eq!(position.lends.len(), 0);
    assert_eq!(position.debts.len(), 1);
    let debt = position.debts.first().unwrap();
    let expected_debt = expected_pos_debt_after_opening_perp + loss_amt
        - usdc_coin_deposited.amount
        + Uint128::new(1); // simulated interest
    assert_eq!(debt.amount, expected_debt);
    assert_eq!(position.perps.len(), 0);

    // check if perp balance increased by position loss
    let current_perp_usdc_balance = mock.query_balance(mock.perps.address(), &usdc_info.denom);
    assert_eq!(
        current_perp_usdc_balance.amount,
        expected_perp_usdc_balance_after_opening_perp + loss_amt
    );
}

#[test]
fn perp_position_when_no_usdc_in_account() {
    let osmo_info = uosmo_info();
    let atom_info = uatom_info();
    let usdc_info = coin_info("uusdc");

    let contract_owner = Addr::unchecked("owner");
    let cm_user = Addr::unchecked("user");
    let vault_depositor = Addr::unchecked("vault_depositor");

    let osmo_coin_deposited = osmo_info.to_coin(10000);

    let mut mock = MockEnv::new()
        .owner(contract_owner.as_str())
        .set_params(&[osmo_info, atom_info.clone(), usdc_info.clone()])
        .fund_account(AccountToFund {
            addr: cm_user.clone(),
            funds: vec![osmo_coin_deposited.clone()],
        })
        .fund_account(AccountToFund {
            addr: vault_depositor.clone(),
            funds: vec![coin(100000, usdc_info.denom.clone())],
        })
        .build()
        .unwrap();
    let account_id = mock.create_credit_account(&cm_user).unwrap();

    // setup perp contract
    mock.init_perp_denom(
        &contract_owner,
        &atom_info.denom,
        Decimal::from_str("3").unwrap(),
        Decimal::from_str("1000000").unwrap(),
    )
    .unwrap();
    mock.deposit_to_perp_vault(&vault_depositor, &coin(100000, usdc_info.denom.clone())).unwrap();

    let perp_size = SignedDecimal::from_str("400").unwrap();

    // check perp data before any action
    let perp_usdc_balance = mock.query_balance(mock.perps.address(), &usdc_info.denom);
    let opening_fee = mock.query_perp_opening_fee(&atom_info.denom, perp_size);

    // open perp position
    mock.update_credit_account(
        &account_id,
        &cm_user,
        vec![
            Deposit(osmo_coin_deposited.clone()),
            OpenPerp {
                denom: atom_info.denom.clone(),
                size: perp_size,
            },
        ],
        &[osmo_coin_deposited.clone()],
    )
    .unwrap();

    // check position data
    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 1);
    assert_present(&position, &osmo_coin_deposited.denom, osmo_coin_deposited.amount);
    assert_eq!(position.lends.len(), 0);
    assert_eq!(position.debts.len(), 1);
    let debt = position.debts.first().unwrap();
    let expected_pos_debt_after_opening_perp = opening_fee.fee.amount + Uint128::new(1); // simulated interest
    assert_eq!(debt.amount, expected_pos_debt_after_opening_perp);
    assert_eq!(position.perps.len(), 1);
    let perp_position = position.perps.first().unwrap().clone();
    let expected_perp_position = mock.query_perp_position(&account_id, &atom_info.denom).position;
    assert_eq!(perp_position, expected_perp_position);

    // check if perp balance increased by opening fee
    let current_perp_usdc_balance = mock.query_balance(mock.perps.address(), &usdc_info.denom);
    let expected_perp_usdc_balance_after_opening_perp =
        perp_usdc_balance.amount + opening_fee.fee.amount;
    assert_eq!(current_perp_usdc_balance.amount, expected_perp_usdc_balance_after_opening_perp);

    // simulate loss in perp position
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: atom_info.denom.clone(),
        price: atom_info.price * Decimal::percent(90u64), // 10% loss in price
    });

    // check perp position pnl
    let perp_position = mock.query_perp_position(&account_id, &atom_info.denom).position;
    let loss_amt = pnl_loss(perp_position.pnl.coins.pnl);

    // close perp position
    mock.update_credit_account(
        &account_id,
        &cm_user,
        vec![ClosePerp {
            denom: atom_info.denom.clone(),
        }],
        &[],
    )
    .unwrap();

    // check position data
    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 1);
    assert_present(&position, &osmo_coin_deposited.denom, osmo_coin_deposited.amount);
    assert_eq!(position.lends.len(), 0);
    assert_eq!(position.debts.len(), 1);
    let debt = position.debts.first().unwrap();
    let expected_debt = expected_pos_debt_after_opening_perp + loss_amt + Uint128::one(); // simulated interest
    assert_eq!(debt.amount, expected_debt);
    assert_eq!(position.perps.len(), 0);

    // check if perp balance increased by position loss
    let current_perp_usdc_balance = mock.query_balance(mock.perps.address(), &usdc_info.denom);
    assert_eq!(
        current_perp_usdc_balance.amount,
        expected_perp_usdc_balance_after_opening_perp + loss_amt
    );
}
