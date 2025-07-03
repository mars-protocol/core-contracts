use cosmwasm_std::{
    coin, coins, Addr, Coin, Decimal, OverflowError, OverflowOperation::Sub, Uint128,
};
use mars_credit_manager::error::{ContractError, ContractError::NotTokenOwner};
use mars_testing::multitest::helpers::uusdc_info;
use mars_types::{
    credit_manager::Action, oracle::ActionKind, params::AssetParamsUpdate::AddOrUpdate,
};
use test_case::test_case;

use super::helpers::{assert_err, uatom_info, uosmo_info, AccountToFund, MockEnv};

#[test]
fn only_owner_of_token_can_withdraw() {
    let coin_info = uosmo_info();
    let owner = Addr::unchecked("owner");
    let mut mock = MockEnv::new().build().unwrap();
    let account_id = mock.create_credit_account(&owner).unwrap();

    let another_user = Addr::unchecked("another_user");
    let res = mock.update_credit_account(
        &account_id,
        &another_user,
        vec![Action::Withdraw(coin_info.to_action_coin(382))],
        &[],
    );

    assert_err(
        res,
        NotTokenOwner {
            user: another_user.into(),
            account_id: account_id.clone(),
        },
    );

    let res = mock.query_positions(&account_id);
    assert_eq!(res.deposits.len(), 0);
}

#[test]
fn withdraw_disabled() {
    let mut coin_info = uosmo_info();
    coin_info.withdraw_enabled = false;
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .build()
        .unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    // Standard withdraw
    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Action::Deposit(coin_info.to_coin(300)),
            Action::Withdraw(coin_info.to_action_coin(400)),
        ],
        &[coin(300, coin_info.denom.clone())],
    );

    assert_err(
        res,
        ContractError::WithdrawNotEnabled {
            denom: coin_info.denom.clone(),
        },
    );

    // Withdraw to wallet
    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Action::Deposit(coin_info.to_coin(300)),
            Action::WithdrawToWallet {
                coin: coin_info.to_action_coin(400),
                recipient: user.to_string(),
            },
        ],
        &[coin(300, coin_info.denom.clone())],
    );

    assert_err(
        res,
        ContractError::WithdrawNotEnabled {
            denom: coin_info.denom.clone(),
        },
    );
}

#[test]
fn withdraw_nothing() {
    let coin_info = uosmo_info();
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new().set_params(&[coin_info.clone()]).build().unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![Action::Withdraw(coin_info.to_action_coin(0))],
        &[],
    );

    assert_err(res, ContractError::NoAmount);
    let res = mock.query_positions(&account_id);
    assert_eq!(res.deposits.len(), 0);
}

#[test]
fn withdraw_but_no_funds() {
    let coin_info = uosmo_info();
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new().set_params(&[coin_info.clone()]).build().unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![Action::Withdraw(coin_info.to_action_coin(234))],
        &[],
    );

    assert_err(
        res,
        ContractError::Overflow(OverflowError {
            operation: Sub,
            operand1: "0".to_string(),
            operand2: "234".to_string(),
        }),
    );

    let res = mock.query_positions(&account_id);
    assert_eq!(res.deposits.len(), 0);
}

#[test]
fn withdraw_but_not_enough_funds() {
    let coin_info = uosmo_info();
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .build()
        .unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Action::Deposit(coin_info.to_coin(300)),
            Action::Withdraw(coin_info.to_action_coin(400)),
        ],
        &[coin(300, coin_info.denom)],
    );

    assert_err(
        res,
        ContractError::Overflow(OverflowError {
            operation: Sub,
            operand1: "300".to_string(),
            operand2: "400".to_string(),
        }),
    );

    let res = mock.query_positions(&account_id);
    assert_eq!(res.deposits.len(), 0);
}

#[test]
fn cannot_withdraw_more_than_healthy() {
    let coin_info = uosmo_info();
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .build()
        .unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Action::Deposit(coin_info.to_coin(200)),
            Action::Borrow(coin_info.to_coin(400)),
            Action::Withdraw(coin_info.to_action_coin(50)),
        ],
        &[coin(200, coin_info.denom)],
    );

    assert_err(
        res,
        ContractError::AboveMaxLTV {
            account_id: account_id.clone(),
            max_ltv_health_factor: "0.940594059405940594".to_string(),
        },
    );

    let res = mock.query_positions(&account_id);
    assert_eq!(res.deposits.len(), 0);
}

#[test]
fn withdraw_success() {
    let coin_info = uosmo_info();
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .build()
        .unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let deposit_amount = 234;
    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Action::Deposit(coin_info.to_coin(deposit_amount)),
            Action::Withdraw(coin_info.to_action_coin(deposit_amount)),
        ],
        &[Coin::new(deposit_amount, coin_info.denom.clone())],
    )
    .unwrap();

    let res = mock.query_positions(&account_id);
    assert_eq!(res.deposits.len(), 0);

    let coin = mock.query_balance(&mock.rover, &coin_info.denom);
    assert_eq!(coin.amount, Uint128::zero())
}

#[test]
fn withdraw_account_balance() {
    let coin_info = uosmo_info();
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .build()
        .unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let deposit_amount = 234;
    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Action::Deposit(coin_info.to_coin(deposit_amount)),
            Action::Withdraw(coin_info.to_action_coin_full_balance()),
        ],
        &[Coin::new(deposit_amount, coin_info.denom.clone())],
    )
    .unwrap();

    let res = mock.query_positions(&account_id);
    assert_eq!(res.deposits.len(), 0);

    let coin = mock.query_balance(&mock.rover, &coin_info.denom);
    assert_eq!(coin.amount, Uint128::zero())
}

#[test]
fn multiple_withdraw_actions() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[uosmo_info.clone(), uatom_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![coin(234, uosmo_info.denom.clone()), coin(25, uatom_info.denom.clone())],
        })
        .build()
        .unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let uosmo_amount = Uint128::new(234);
    let uatom_amount = Uint128::new(25);

    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Action::Deposit(uosmo_info.to_coin(uosmo_amount.u128())),
            Action::Deposit(uatom_info.to_coin(uatom_amount.u128())),
        ],
        &[coin(234, uosmo_info.denom.clone()), coin(25, uatom_info.denom.clone())],
    )
    .unwrap();

    let res = mock.query_positions(&account_id);
    assert_eq!(res.deposits.len(), 2);

    let coin = mock.query_balance(&user, &uosmo_info.denom);
    assert_eq!(coin.amount, Uint128::zero());

    let coin = mock.query_balance(&user, &uatom_info.denom);
    assert_eq!(coin.amount, Uint128::zero());

    mock.update_credit_account(
        &account_id,
        &user,
        vec![Action::Withdraw(uosmo_info.to_action_coin(uosmo_amount.u128()))],
        &[],
    )
    .unwrap();

    let res = mock.query_positions(&account_id);
    assert_eq!(res.deposits.len(), 1);

    let coin = mock.query_balance(&mock.rover, &uosmo_info.denom);
    assert_eq!(coin.amount, Uint128::zero());

    let coin = mock.query_balance(&user, &uosmo_info.denom);
    assert_eq!(coin.amount, uosmo_amount);

    mock.update_credit_account(
        &account_id,
        &user,
        vec![Action::Withdraw(uatom_info.to_action_coin(20))],
        &[],
    )
    .unwrap();

    let res = mock.query_positions(&account_id);
    assert_eq!(res.deposits.len(), 1);

    let coin = mock.query_balance(&mock.rover, &uatom_info.denom);
    assert_eq!(coin.amount, Uint128::new(5));

    let coin = mock.query_balance(&user, &uatom_info.denom);
    assert_eq!(coin.amount, Uint128::new(20));

    mock.update_credit_account(
        &account_id,
        &user,
        vec![Action::Withdraw(uatom_info.to_action_coin(5))],
        &[],
    )
    .unwrap();

    let res = mock.query_positions(&account_id);
    assert_eq!(res.deposits.len(), 0);

    let coin = mock.query_balance(&mock.rover, &uatom_info.denom);
    assert_eq!(coin.amount, Uint128::zero());

    let coin = mock.query_balance(&user, &uatom_info.denom);
    assert_eq!(coin.amount, uatom_amount);
}

#[test_case(
    false,
    true;
    "delisting asset; uusdc not whitelisted, max ltv non-zero"
)]
#[test_case(
    true,
    false;
    "delisting asset; uusdc whitelisted, max ltv zero"
)]
#[test_case(
    false,
    false;
    "delisting asset; uusdc not whitelisted, max ltv zero"
)]
fn withdraw_delisted_asset(uusdc_whitelisted: bool, uusdc_max_ltv_non_zero: bool) {
    let uusdc_coin_info = uusdc_info();
    let uosmo_coin_info = uosmo_info();
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[uusdc_coin_info.clone(), uosmo_coin_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(1000, uusdc_coin_info.denom.clone()),
        })
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(1000, uosmo_coin_info.denom.clone()),
        })
        .build()
        .unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let uusdc_deposit_amount = 300;
    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Action::Deposit(uusdc_coin_info.to_coin(uusdc_deposit_amount)),
            Action::Borrow(uosmo_coin_info.to_coin(100)),
        ],
        &[Coin::new(uusdc_deposit_amount, uusdc_coin_info.denom.clone())],
    )
    .unwrap();

    // Account is healthy
    let health = mock.query_health(&account_id, ActionKind::Default);
    assert!(!health.above_max_ltv);
    assert!(!health.liquidatable);

    // Withdrawing uusdc should not be allowed because it would make the account unhealthy
    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![Action::Withdraw(uusdc_coin_info.to_action_coin(uusdc_deposit_amount))],
        &[],
    );
    assert_err(
        res,
        ContractError::AboveMaxLTV {
            account_id: account_id.clone(),
            max_ltv_health_factor: "0.653846153846153846".to_string(),
        },
    );

    // Delist uusdc
    let mut uusdc_asset_param = mock.query_asset_params(&uusdc_coin_info.denom);
    uusdc_asset_param.credit_manager.whitelisted = uusdc_whitelisted;
    if !uusdc_max_ltv_non_zero {
        uusdc_asset_param.max_loan_to_value = Decimal::zero();
    }
    mock.update_asset_params(AddOrUpdate {
        params: uusdc_asset_param.into(),
    });

    // Account is unhealthy (maxLTV < 1) but not liquidatable (liqLTV > 1)
    let health = mock.query_health(&account_id, ActionKind::Default);
    assert!(health.above_max_ltv);
    assert!(!health.liquidatable);

    // Withdrawing uusdc is now allowed. The account will be liquidatable after the withdrawal.
    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![Action::Withdraw(uusdc_coin_info.to_action_coin(uusdc_deposit_amount))],
        &[],
    );
    assert_err(
        res,
        ContractError::UnhealthyLiquidationHfDecrease {
            prev_hf: "12.5".to_string(),
            new_hf: "0.730769230769230769".to_string(),
        },
    );
}
