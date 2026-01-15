use cosmwasm_std::{coin, coins, Addr, Uint128};
use cw_multi_test::Executor;
use mars_types::{
    credit_manager::{
        Action::{Borrow, Deposit, Lend, Withdraw},
        MigrateMsg,
    },
    red_bank::ExecuteMsg as RedBankExecuteMsg,
};

use super::helpers::{get_debt, uusdc_info, AccountToFund, MockEnv};

#[test]
fn write_off_bad_debt_accounts_same_owner() {
    let coin_info = uusdc_info();
    let denom = coin_info.denom.clone();

    let owner = Addr::unchecked("bad_debt_owner");
    let admin = Addr::unchecked("migration_admin");

    let mut mock = MockEnv::new()
        .set_params(&[coin_info.clone()])
        .set_rover_admin(&admin)
        .fund_account(AccountToFund {
            addr: owner.clone(),
            funds: coins(1_000, denom.clone()),
        })
        .build()
        .unwrap();

    let good_account = mock.create_credit_account(&owner).unwrap();
    let bad_account_1 = mock.create_credit_account(&owner).unwrap();
    let bad_account_2 = mock.create_credit_account(&owner).unwrap();

    // Good account has collateral and should be skipped
    mock.update_credit_account(
        &good_account,
        &owner,
        vec![
            Deposit(coin(150, denom.clone())),
            Lend(coin_info.to_action_coin(150)),
            Borrow(coin(50, denom.clone())),
        ],
        &[coin(150, denom.clone())],
    )
    .unwrap();
    mock.update_credit_account(
        &good_account,
        &owner,
        vec![Withdraw(coin_info.to_action_coin(50))],
        &[],
    )
    .unwrap();

    // Bad account 1: create debt, then remove coin balances and collateral
    mock.update_credit_account(
        &bad_account_1,
        &owner,
        vec![
            Deposit(coin(200, denom.clone())),
            Lend(coin_info.to_action_coin(200)),
            Borrow(coin(100, denom.clone())),
        ],
        &[coin(200, denom.clone())],
    )
    .unwrap();
    mock.update_credit_account(
        &bad_account_1,
        &owner,
        vec![Withdraw(coin_info.to_action_coin(100))],
        &[],
    )
    .unwrap();

    // Bad account 2: create debt, then remove coin balances and collateral
    mock.update_credit_account(
        &bad_account_2,
        &owner,
        vec![
            Deposit(coin(200, denom.clone())),
            Lend(coin_info.to_action_coin(200)),
            Borrow(coin(100, denom.clone())),
        ],
        &[coin(200, denom.clone())],
    )
    .unwrap();
    mock.update_credit_account(
        &bad_account_2,
        &owner,
        vec![Withdraw(coin_info.to_action_coin(100))],
        &[],
    )
    .unwrap();

    let config = mock.query_config();
    let red_bank_addr = Addr::unchecked(config.red_bank.clone());

    // Remove red-bank collateral directly to create bad debt
    let bad_1_collateral = mock.query_red_bank_collateral(&bad_account_1, &denom);
    mock.app
        .execute_contract(
            mock.rover.clone(),
            red_bank_addr.clone(),
            &RedBankExecuteMsg::Withdraw {
                denom: denom.clone(),
                amount: Some(bad_1_collateral.amount),
                recipient: None,
                account_id: Some(bad_account_1.clone()),
                liquidation_related: None,
            },
            &[],
        )
        .unwrap();

    let bad_2_collateral = mock.query_red_bank_collateral(&bad_account_2, &denom);
    mock.app
        .execute_contract(
            mock.rover.clone(),
            red_bank_addr.clone(),
            &RedBankExecuteMsg::Withdraw {
                denom: denom.clone(),
                amount: Some(bad_2_collateral.amount),
                recipient: None,
                account_id: Some(bad_account_2.clone()),
                liquidation_related: None,
            },
            &[],
        )
        .unwrap();

    let total_collateral = mock.query_red_bank_collateral(&good_account, &denom).amount
        + mock.query_red_bank_collateral(&bad_account_1, &denom).amount
        + mock.query_red_bank_collateral(&bad_account_2, &denom).amount;
    let total_debt_before = mock.query_red_bank_debt(&denom).amount;

    assert!(total_debt_before > total_collateral);

    let bad_shares_1 = get_debt(&denom, &mock.query_positions(&bad_account_1).debts).shares;
    let bad_shares_2 = get_debt(&denom, &mock.query_positions(&bad_account_2).debts).shares;
    let bad_shares_total = bad_shares_1 + bad_shares_2;
    let total_shares_before = mock.query_total_debt_shares(&denom).shares;

    let expected_writeoff = if total_debt_before.is_zero() {
        Uint128::zero()
    } else {
        total_debt_before
            .checked_mul_ceil((bad_shares_total, total_shares_before))
            .unwrap()
    };

    let code_id = mock.query_code_id(&mock.rover);
    mock.app
        .migrate_contract(
            admin,
            mock.rover.clone(),
            &MigrateMsg::WriteOffBadDebt {
                address_provider: mock.address_provider.to_string(),
                bad_debt_owner: owner.to_string(),
                denom: denom.clone(),
                start_after: None,
                limit: None,
            },
            code_id,
        )
        .unwrap();

    let bad_1_after = get_debt(&denom, &mock.query_positions(&bad_account_1).debts).shares;
    let bad_2_after = get_debt(&denom, &mock.query_positions(&bad_account_2).debts).shares;
    assert!(bad_1_after.is_zero());
    assert!(bad_2_after.is_zero());

    let total_shares_after = mock.query_total_debt_shares(&denom).shares;
    assert_eq!(total_shares_after, total_shares_before - bad_shares_total);

    let total_debt_after = mock.query_red_bank_debt(&denom).amount;
    assert_eq!(
        total_debt_after,
        total_debt_before - expected_writeoff
    );

    // good account still has collateral
    let good_collateral_after = mock.query_red_bank_collateral(&good_account, &denom).amount;
    assert!(!good_collateral_after.is_zero());
}
