use cosmwasm_std::{coin, coins, Addr, OverflowError, OverflowOperation, Uint128};
use mars_credit_manager::error::ContractError;
use mars_types::{
    credit_manager::{
        Action::{Deposit, DepositToPerpVault, UnlockFromPerpVault, WithdrawFromPerpVault},
        ActionAmount, ActionCoin,
    },
    perps::{PerpVaultDeposit, PerpVaultPosition, PerpVaultUnlock},
};

use super::helpers::{assert_err, blacklisted_coin, coin_info, AccountToFund, MockEnv};

#[test]
fn can_only_deposit_to_perp_vault_what_is_whitelisted() {
    let coin_info = blacklisted_coin();
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new().set_params(&[coin_info.clone()]).build().unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![DepositToPerpVault(coin_info.to_action_coin(50))],
        &[],
    );

    assert_err(res, ContractError::NotWhitelisted(String::from("uluna")))
}

#[test]
fn deposit_zero_to_perp_vault_throws_error() {
    let coin_info = coin_info("uusdc");
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new().set_params(&[coin_info.clone()]).build().unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![DepositToPerpVault(coin_info.to_action_coin(0))],
        &[],
    );

    assert_err(res, ContractError::NoAmount)
}

#[test]
fn raises_when_not_enough_assets_to_deposit_to_perp_vault() {
    let coin_info = coin_info("uusdc");
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
        vec![Deposit(coin_info.to_coin(300)), DepositToPerpVault(coin_info.to_action_coin(500))],
        &[coin_info.to_coin(300)],
    );

    assert_err(
        res,
        ContractError::Overflow(OverflowError {
            operation: OverflowOperation::Sub,
            operand1: "300".to_string(),
            operand2: "500".to_string(),
        }),
    )
}

#[test]
fn deposit_account_balance_to_perp_vault_if_no_funds() {
    let coin_info = coin_info("uusdc");

    let user_a = Addr::unchecked("user_a");

    let mut mock = MockEnv::new()
        .set_params(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: user_a.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .build()
        .unwrap();

    let account_id_a = mock.create_credit_account(&user_a).unwrap();

    let position = mock.query_positions(&account_id_a);
    assert_eq!(position.deposits.len(), 0);
    assert!(position.perp_vault.is_none());

    let res = mock.update_credit_account(
        &account_id_a,
        &user_a,
        vec![DepositToPerpVault(ActionCoin {
            denom: coin_info.denom.clone(),
            amount: ActionAmount::AccountBalance,
        })],
        &[],
    );

    assert_err(res, ContractError::NoAmount)
}

#[test]
fn successful_deposit_to_perp_vault() {
    let coin_info = coin_info("uusdc");

    let user = Addr::unchecked("user_abc");

    let mut mock = MockEnv::new()
        .set_params(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .build()
        .unwrap();

    let account_id = mock.create_credit_account(&user).unwrap();

    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 0);
    assert!(position.perp_vault.is_none());

    let vault_deposit_amt = Uint128::new(50);
    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(coin_info.to_coin(300)),
            DepositToPerpVault(coin_info.to_action_coin(vault_deposit_amt.u128())),
        ],
        &[coin(300, coin_info.denom.clone())],
    )
    .unwrap();

    // Assert deposits decreased
    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 1);
    let deposit_res = position.deposits.first().unwrap();
    let expected_net_deposit_amount = Uint128::new(250);
    assert_eq!(deposit_res.amount, expected_net_deposit_amount);

    // Assert perp vault position increased
    assert_eq!(
        position.perp_vault.unwrap(),
        PerpVaultPosition {
            denom: coin_info.denom.clone(),
            deposit: PerpVaultDeposit {
                shares: Uint128::new(50_000_000),
                amount: vault_deposit_amt
            },
            unlocks: vec![]
        }
    );

    // Assert CM has indeed sent those tokens to Perps contract
    let balance = mock.query_balance(&mock.rover, &coin_info.denom);
    assert_eq!(balance.amount, expected_net_deposit_amount);
    let balance = mock.query_balance(mock.perps.address(), &coin_info.denom);
    assert_eq!(balance.amount, vault_deposit_amt);
}

#[test]
fn successful_account_balance_deposit_to_perp_vault() {
    let coin_info = coin_info("uusdc");

    let user = Addr::unchecked("user_abc");

    let mut mock = MockEnv::new()
        .set_params(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .build()
        .unwrap();

    let account_id = mock.create_credit_account(&user).unwrap();

    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 0);
    assert!(position.perp_vault.is_none());

    let vault_deposit_amt = Uint128::new(300);
    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(coin_info.to_coin(vault_deposit_amt.u128())),
            DepositToPerpVault(ActionCoin {
                denom: coin_info.denom.clone(),
                amount: ActionAmount::AccountBalance,
            }),
        ],
        &[coin(vault_deposit_amt.u128(), coin_info.denom.clone())],
    )
    .unwrap();

    // Assert deposits decreased
    let position = mock.query_positions(&account_id);
    assert!(position.deposits.is_empty());

    // Assert perp vault position increased
    assert_eq!(
        position.perp_vault.unwrap(),
        PerpVaultPosition {
            denom: coin_info.denom.clone(),
            deposit: PerpVaultDeposit {
                shares: Uint128::new(300_000_000),
                amount: vault_deposit_amt
            },
            unlocks: vec![]
        }
    );

    // Assert CM has indeed sent those tokens to Perps contract
    let balance = mock.query_balance(&mock.rover, &coin_info.denom);
    assert_eq!(balance.amount, Uint128::zero());
    let balance = mock.query_balance(mock.perps.address(), &coin_info.denom);
    assert_eq!(balance.amount, vault_deposit_amt);
}

#[test]
fn unlock_zero_shares_from_perp_vault_throws_error() {
    let coin_info = coin_info("uusdc");
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new().set_params(&[coin_info.clone()]).build().unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![UnlockFromPerpVault {
            shares: Uint128::zero(),
        }],
        &[],
    );

    assert_err(res, ContractError::NoAmount)
}

#[test]
fn unlock_more_shares_than_deposited_throws_error() {
    let coin_info = coin_info("uusdc");

    let user = Addr::unchecked("user_abc");

    let mut mock = MockEnv::new()
        .set_params(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .build()
        .unwrap();

    let account_id = mock.create_credit_account(&user).unwrap();

    let vault_deposit_amt = Uint128::new(50);
    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(coin_info.to_coin(300)),
            DepositToPerpVault(coin_info.to_action_coin(vault_deposit_amt.u128())),
        ],
        &[coin(300, coin_info.denom.clone())],
    )
    .unwrap();

    let position = mock.query_positions(&account_id);

    mock.update_credit_account(
        &account_id,
        &user,
        vec![UnlockFromPerpVault {
            shares: position.perp_vault.unwrap().deposit.shares + Uint128::new(1),
        }],
        &[],
    )
    .unwrap_err();
}

#[test]
fn successful_unlock_and_withdraw_from_perp_vault() {
    let coin_info = coin_info("uusdc");

    let user = Addr::unchecked("user_abc");

    let mut mock = MockEnv::new()
        .set_params(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .build()
        .unwrap();

    let account_id = mock.create_credit_account(&user).unwrap();

    let vault_deposit_amt = Uint128::new(50);
    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(coin_info.to_coin(300)),
            DepositToPerpVault(coin_info.to_action_coin(vault_deposit_amt.u128())),
        ],
        &[coin(300, coin_info.denom.clone())],
    )
    .unwrap();

    // Read state before unlock
    let perp_vault_position = mock.query_positions(&account_id);
    let cm_balance = mock.query_balance(&mock.rover, &coin_info.denom);
    let perps_balance = mock.query_balance(mock.perps.address(), &coin_info.denom);

    let unlock_current_time = mock.query_block_time();
    let unlock_shares = perp_vault_position.perp_vault.unwrap().deposit.shares / Uint128::new(5);
    mock.update_credit_account(
        &account_id,
        &user,
        vec![UnlockFromPerpVault {
            shares: unlock_shares,
        }],
        &[],
    )
    .unwrap();

    // Balances should be the same
    let cm_balance_after_unlock = mock.query_balance(&mock.rover, &coin_info.denom);
    assert_eq!(cm_balance, cm_balance_after_unlock);
    let perps_balance_after_unlock = mock.query_balance(mock.perps.address(), &coin_info.denom);
    assert_eq!(perps_balance, perps_balance_after_unlock);

    let perp_config = mock.query_perp_config();
    let perp_vault_position_after_unlock = mock.query_positions(&account_id);

    // Deposits should be the same
    assert_eq!(perp_vault_position.deposits, perp_vault_position_after_unlock.deposits);

    // Perp vault position should be updated
    let expected_unlock_amt = Uint128::new(10);
    assert_eq!(
        perp_vault_position_after_unlock.perp_vault.unwrap(),
        PerpVaultPosition {
            denom: coin_info.denom.clone(),
            deposit: PerpVaultDeposit {
                shares: Uint128::new(40_000_000),
                amount: Uint128::new(40),
            },
            unlocks: vec![PerpVaultUnlock {
                created_at: unlock_current_time,
                cooldown_end: unlock_current_time + perp_config.cooldown_period,
                shares: unlock_shares,
                amount: expected_unlock_amt
            }]
        }
    );

    // Move time forward to pass cooldown period
    mock.set_block_time(unlock_current_time + perp_config.cooldown_period + 1);

    mock.update_credit_account(&account_id, &user, vec![WithdrawFromPerpVault {}], &[]).unwrap();

    // Check contract balances after withdraw
    let cm_balance_after_withdraw = mock.query_balance(&mock.rover, &coin_info.denom);
    assert_eq!(
        cm_balance_after_withdraw.amount,
        cm_balance_after_unlock.amount + expected_unlock_amt
    );
    let perps_balance_after_withdraw = mock.query_balance(mock.perps.address(), &coin_info.denom);
    assert_eq!(
        perps_balance_after_withdraw.amount,
        perps_balance_after_unlock.amount - expected_unlock_amt
    );

    // Check positions are updated
    let perp_vault_position_after_withdraw = mock.query_positions(&account_id);
    assert_eq!(
        perp_vault_position_after_withdraw.perp_vault.unwrap(),
        PerpVaultPosition {
            denom: coin_info.denom.clone(),
            deposit: PerpVaultDeposit {
                shares: Uint128::new(40_000_000),
                amount: Uint128::new(40),
            },
            unlocks: vec![]
        }
    );
    let deposit_after_unlock = perp_vault_position_after_unlock.deposits.first().unwrap();
    let deposit_after_withdraw = perp_vault_position_after_withdraw.deposits.first().unwrap();
    assert_eq!(perp_vault_position_after_withdraw.deposits.len(), 1);
    assert_eq!(deposit_after_withdraw.amount, deposit_after_unlock.amount + expected_unlock_amt);
}
