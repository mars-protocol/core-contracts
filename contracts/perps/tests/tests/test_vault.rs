use std::str::FromStr;

use cosmwasm_std::{coin, Addr, Decimal, Uint128};
use mars_perps::error::ContractError;
use mars_types::{
    math::SignedDecimal,
    oracle::ActionKind,
    params::{PerpParams, PerpParamsUpdate},
    perps::{PerpVaultDeposit, PerpVaultPosition, UnlockState},
};

use super::helpers::MockEnv;
use crate::tests::helpers::{assert_err, default_perp_params};

#[test]
fn random_user_cannot_deposit_to_vault() {
    let mut mock = MockEnv::new().build().unwrap();
    let random_sender = Addr::unchecked("random-user-123");
    mock.fund_accounts(&[&random_sender], 1_000_000_000_000u128, &["uusdc"]);

    let res = mock.deposit_to_vault(&random_sender, "2", &[coin(1_000_000_000u128, "uusdc")]);
    assert_err(res, ContractError::SenderIsNotCreditManager);
}

#[test]
fn random_user_cannot_unlock_from_vault() {
    let mut mock = MockEnv::new().build().unwrap();
    let random_sender = Addr::unchecked("random-user-123");
    mock.fund_accounts(&[&random_sender], 1_000_000_000_000u128, &["uusdc"]);

    let res = mock.unlock_from_vault(&random_sender, "2", Uint128::new(100));
    assert_err(res, ContractError::SenderIsNotCreditManager);
}

#[test]
fn random_user_cannot_withdraw_from_vault() {
    let mut mock = MockEnv::new().build().unwrap();
    let random_sender = Addr::unchecked("random-user-123");
    mock.fund_accounts(&[&random_sender], 1_000_000_000_000u128, &["uusdc"]);

    let res = mock.withdraw_from_vault(&random_sender, "2");
    assert_err(res, ContractError::SenderIsNotCreditManager);
}

#[test]
fn unlock_few_times() {
    let depositor = "depositor";
    let cooldown_period = 1225u64;
    let mut mock = MockEnv::new().cooldown_period(cooldown_period).build().unwrap();
    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();

    // set usdc price
    mock.set_price(&owner, "uusdc", Decimal::one()).unwrap();

    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000u128, &["uusdc"]);

    mock.deposit_to_vault(&credit_manager, depositor, &[coin(1_000_000_000u128, "uusdc")]).unwrap();

    // unlocks should be empty
    let unlocks = mock.query_unlocks(depositor);
    assert!(unlocks.is_empty());

    // amounts to unlock
    let deposit = mock.query_deposit(depositor);
    let shares_1 = deposit.shares.multiply_ratio(1u128, 2u128); // 50%
    let amt_1 = deposit.amount.multiply_ratio(1u128, 2u128);
    let shares_2 = deposit.shares.multiply_ratio(1u128, 4u128); // 25%
    let amt_2 = deposit.amount.multiply_ratio(1u128, 4u128);
    let shares_3 = deposit.shares.multiply_ratio(1u128, 4u128); // 25%
    let amt_3 = deposit.amount.multiply_ratio(1u128, 4u128);

    // first unlock
    mock.unlock_from_vault(&credit_manager, depositor, shares_1).unwrap();
    let unlocks = mock.query_unlocks(depositor);
    let current_time = mock.query_block_time();
    let unlock_1_expected = UnlockState {
        created_at: current_time,
        cooldown_end: current_time + cooldown_period,
        amount: amt_1,
    };
    assert_eq!(unlocks, vec![unlock_1_expected.clone()]);

    // move time forward
    mock.increment_by_time(3600);

    // second unlock
    mock.unlock_from_vault(&credit_manager, depositor, shares_2).unwrap();
    let unlocks = mock.query_unlocks(depositor);
    let current_time = mock.query_block_time();
    let unlock_2_expected = UnlockState {
        created_at: current_time,
        cooldown_end: current_time + cooldown_period,
        amount: amt_2,
    };
    assert_eq!(unlocks, vec![unlock_1_expected.clone(), unlock_2_expected.clone()]);

    // move time forward
    mock.increment_by_time(3600);

    // third unlock
    mock.unlock_from_vault(&credit_manager, depositor, shares_3).unwrap();
    let unlocks = mock.query_unlocks(depositor);
    let current_time = mock.query_block_time();
    let unlock_3_expected = UnlockState {
        created_at: current_time,
        cooldown_end: current_time + cooldown_period,
        amount: amt_3,
    };
    assert_eq!(unlocks, vec![unlock_1_expected, unlock_2_expected, unlock_3_expected]);

    // deposit should be empty after all unlocks
    let deposit = mock.query_deposit(depositor);
    assert!(deposit.amount.is_zero());
    assert!(deposit.shares.is_zero());
}

#[test]
fn withdraw_not_possible_if_cooldown_not_ended() {
    let depositor = "depositor";
    let cooldown_period = 86400u64;
    let mut mock = MockEnv::new().cooldown_period(cooldown_period).build().unwrap();
    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();

    // set usdc price
    mock.set_price(&owner, "uusdc", Decimal::one()).unwrap();

    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000u128, &["uusdc"]);

    mock.deposit_to_vault(&credit_manager, depositor, &[coin(1_000_000_000u128, "uusdc")]).unwrap();

    mock.unlock_from_vault(&credit_manager, depositor, Uint128::new(1_000_000)).unwrap();

    let unlocks = mock.query_unlocks(depositor);
    assert!(!unlocks.is_empty());

    // cooldown period should be passed for at least one unlock
    let res = mock.withdraw_from_vault(&credit_manager, depositor);
    assert_err(res, ContractError::UnlockedPositionsNotFound {});
}

#[test]
fn withdraw_unlocked_shares() {
    let depositor = "depositor";
    let cooldown_period = 86400u64;
    let mut mock = MockEnv::new().cooldown_period(cooldown_period).build().unwrap();
    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();

    // set usdc price
    mock.set_price(&owner, "uusdc", Decimal::one()).unwrap();

    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000u128, &["uusdc"]);

    mock.deposit_to_vault(&credit_manager, depositor, &[coin(1_000_000_000u128, "uusdc")]).unwrap();

    // unlocks should be empty
    let unlocks = mock.query_unlocks(depositor);
    assert!(unlocks.is_empty());

    // amounts to unlock
    let deposit = mock.query_deposit(depositor);
    let shares_1 = deposit.shares.multiply_ratio(1u128, 2u128); // 50%
    let amt_1 = deposit.amount.multiply_ratio(1u128, 2u128);
    let shares_2 = deposit.shares.multiply_ratio(1u128, 4u128); // 25%
    let amt_2 = deposit.amount.multiply_ratio(1u128, 4u128);
    let shares_3 = deposit.shares.multiply_ratio(1u128, 4u128); // 25%
    let amt_3 = deposit.amount.multiply_ratio(1u128, 4u128);

    // first unlock
    mock.unlock_from_vault(&credit_manager, depositor, shares_1).unwrap();

    // move time forward
    mock.increment_by_time(3600);

    // second unlock
    let unlock_2_current_time = mock.query_block_time();
    mock.unlock_from_vault(&credit_manager, depositor, shares_2).unwrap();

    // move time forward
    mock.increment_by_time(3600);

    // second unlock
    let unlock_3_current_time = mock.query_block_time();
    mock.unlock_from_vault(&credit_manager, depositor, shares_3).unwrap();

    // move time forward to pass cooldown period for first and second unlock
    mock.set_block_time(unlock_2_current_time + cooldown_period);

    // check balances before withdraw
    let balance_1 = mock.query_balance(&credit_manager, "uusdc");

    // withdraw from vault should succeed for two unlocks
    mock.withdraw_from_vault(&credit_manager, depositor).unwrap();

    // check balances after withdraw, it should be increased by amount of two unlocks
    let balance_2 = mock.query_balance(&credit_manager, "uusdc");
    assert_eq!(balance_2.amount, balance_1.amount + amt_1 + amt_2);

    // check unlocks after withdraw, it should be one unlock left
    let unlocks = mock.query_unlocks(depositor);
    assert_eq!(
        unlocks,
        vec![UnlockState {
            created_at: unlock_3_current_time,
            cooldown_end: unlock_3_current_time + cooldown_period,
            amount: amt_3,
        }]
    );

    // move time forward to pass cooldown period for last unlock
    mock.set_block_time(unlock_3_current_time + cooldown_period);

    // withdraw from vault should succeed for last unlock
    mock.withdraw_from_vault(&credit_manager, depositor).unwrap();

    // check balances after withdraw, it should be increased by amount of last unlock
    let balance_3 = mock.query_balance(&credit_manager, "uusdc");
    assert_eq!(balance_3.amount, balance_2.amount + amt_3);

    // check unlocks after withdraw, it should be empty
    let unlocks = mock.query_unlocks(depositor);
    assert!(unlocks.is_empty());
}

#[test]
fn cannot_unlock_if_zero_withdrawal_balance() {
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let user = "jake";

    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000u128, &["uatom", "uusdc"]);

    // init denoms
    mock.init_denom(
        &owner,
        "uatom",
        Decimal::from_str("3").unwrap(),
        Decimal::from_str("1000000").unwrap(),
    )
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

    // set prices
    mock.set_price(&owner, "uusdc", Decimal::from_str("0.9").unwrap()).unwrap();
    mock.set_price(&owner, "uatom", Decimal::from_str("10").unwrap()).unwrap();

    // deposit uusdc to vault
    mock.deposit_to_vault(&credit_manager, user, &[coin(1000u128, "uusdc")]).unwrap();

    // open a position
    let size = SignedDecimal::from_str("50").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    mock.open_position(&credit_manager, "1", "uatom", size, &[atom_opening_fee]).unwrap();

    // increase uatom price to make the position profitable
    mock.set_price(&owner, "uatom", Decimal::from_str("50").unwrap()).unwrap();

    let deposit = mock.query_deposit(user);
    let res = mock.unlock_from_vault(&credit_manager, user, deposit.shares);
    assert_err(res, ContractError::ZeroWithdrawalBalance {});
}

#[test]
fn calculate_shares_correctly_after_zero_withdrawal_balance() {
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let depositor_1 = "bob";
    let depositor_2 = "dane";
    let depositor_3 = "mark";

    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000u128, &["uatom", "uusdc"]);

    // init denoms
    mock.init_denom(
        &owner,
        "uatom",
        Decimal::from_str("3").unwrap(),
        Decimal::from_str("1000000").unwrap(),
    )
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

    // set prices
    mock.set_price(&owner, "uusdc", Decimal::from_str("0.9").unwrap()).unwrap();
    mock.set_price(&owner, "uatom", Decimal::from_str("10").unwrap()).unwrap();

    // deposit uusdc to vault
    mock.deposit_to_vault(&credit_manager, depositor_1, &[coin(1000u128, "uusdc")]).unwrap();
    mock.deposit_to_vault(&credit_manager, depositor_2, &[coin(4000u128, "uusdc")]).unwrap();

    // check deposits
    let deposit_1_before = mock.query_deposit(depositor_1);
    let deposit_2_before = mock.query_deposit(depositor_2);
    assert_eq!(deposit_1_before.amount, Uint128::new(1000));
    assert_eq!(deposit_2_before.amount, Uint128::new(4000));
    assert_eq!(deposit_2_before.shares, deposit_1_before.shares.multiply_ratio(4u128, 1u128)); // 4 times more than depositor_1

    // open a position
    let size = SignedDecimal::from_str("100").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    mock.open_position(&credit_manager, "1", "uatom", size, &[atom_opening_fee]).unwrap();

    // increase uatom price to make the position profitable
    mock.set_price(&owner, "uatom", Decimal::from_str("100").unwrap()).unwrap();

    // make sure that there is no withdrawal balance
    let deposit = mock.query_deposit(depositor_1);
    let res = mock.unlock_from_vault(&credit_manager, depositor_1, deposit.shares);
    assert_err(res, ContractError::ZeroWithdrawalBalance {});

    // deposit uusdc to vault when zero withdrawal balance
    mock.deposit_to_vault(&credit_manager, depositor_3, &[coin(2500u128, "uusdc")]).unwrap();

    // Check deposits. There should be zero amounts because of zero withdrawal balance.
    let deposit_1 = mock.query_deposit(depositor_1);
    assert_eq!(deposit_1.amount, Uint128::zero());
    assert_eq!(deposit_1.shares, deposit_1_before.shares);
    let deposit_2 = mock.query_deposit(depositor_2);
    assert_eq!(deposit_2.amount, Uint128::zero());
    assert_eq!(deposit_2.shares, deposit_2_before.shares);
    let deposit_3 = mock.query_deposit(depositor_3);
    assert_eq!(deposit_3.amount, Uint128::zero());
    assert_eq!(deposit_3.shares, deposit_1_before.shares.multiply_ratio(5u128, 2u128)); // 2.5 times more than depositor_1

    // change price to previous value
    mock.set_price(&owner, "uatom", Decimal::from_str("10").unwrap()).unwrap();

    // Amounts and shares should be caclulated proportionally.
    let deposit_1 = mock.query_deposit(depositor_1);
    assert_eq!(deposit_1.amount, Uint128::new(1003));
    assert_eq!(deposit_1.shares, deposit_1_before.shares);
    let deposit_2 = mock.query_deposit(depositor_2);
    assert_eq!(deposit_2.amount, deposit_1.amount.multiply_ratio(4u128, 1u128)); // 4 times more than depositor_1
    assert_eq!(deposit_2.shares, deposit_2_before.shares);
    let deposit_3 = mock.query_deposit(depositor_3);
    assert_eq!(deposit_3.amount, deposit_1.amount.multiply_ratio(5u128, 2u128)); // 2.5 times more than depositor_1
    assert_eq!(deposit_3.shares, deposit_1_before.shares.multiply_ratio(5u128, 2u128));
}

#[test]
fn query_vault_position() {
    let account_id = "depositor";
    let cooldown_period = 86400u64;
    let mut mock = MockEnv::new().cooldown_period(cooldown_period).build().unwrap();
    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();

    // set usdc price
    mock.set_price(&owner, "uusdc", Decimal::one()).unwrap();

    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000u128, &["uusdc"]);

    // vault position should be empty
    let vault_position = mock.query_vault_position(account_id, ActionKind::Default);
    assert!(vault_position.is_none());

    let deposit_amt = Uint128::new(1_200_000_000u128);
    mock.deposit_to_vault(&credit_manager, account_id, &[coin(deposit_amt.u128(), "uusdc")])
        .unwrap();
    mock.deposit_to_vault(&credit_manager, "random-user", &[coin(2_400_000_000u128, "uusdc")])
        .unwrap();

    // vault position should contain only deposit for account_id
    let deposit_shares = Uint128::new(1_200_000_000_000_000u128);
    let vault_position = mock.query_vault_position(account_id, ActionKind::Default);
    assert_eq!(
        vault_position.unwrap(),
        PerpVaultPosition {
            denom: "uusdc".to_string(),
            deposit: PerpVaultDeposit {
                shares: deposit_shares,
                amount: deposit_amt
            },
            unlocks: vec![]
        }
    );

    // amounts to unlock
    let shares_to_unlock = deposit_shares.multiply_ratio(1u128, 2u128); // 50%
    let amt_to_unlock = deposit_amt.multiply_ratio(1u128, 2u128);

    // first unlock
    let unlock_1_current_time = mock.query_block_time();
    mock.unlock_from_vault(&credit_manager, account_id, shares_to_unlock).unwrap();

    // vault position should contain deposit and one unlock
    let deposit_shares_after_1_unlock = deposit_shares - shares_to_unlock;
    let deposit_amt_after_1_unlock = deposit_amt - amt_to_unlock;
    let vault_position = mock.query_vault_position(account_id, ActionKind::Default);
    assert_eq!(
        vault_position.unwrap(),
        PerpVaultPosition {
            denom: "uusdc".to_string(),
            deposit: PerpVaultDeposit {
                shares: deposit_shares_after_1_unlock,
                amount: deposit_amt_after_1_unlock
            },
            unlocks: vec![UnlockState {
                created_at: unlock_1_current_time,
                cooldown_end: unlock_1_current_time + cooldown_period,
                amount: amt_to_unlock,
            }]
        }
    );

    // move time forward
    mock.increment_by_time(3600);

    // second unlock
    let unlock_2_current_time = mock.query_block_time();
    mock.unlock_from_vault(&credit_manager, account_id, shares_to_unlock).unwrap();

    // vault position should have zero deposit and two unlocks
    let vault_position = mock.query_vault_position(account_id, ActionKind::Default);
    assert_eq!(
        vault_position.unwrap(),
        PerpVaultPosition {
            denom: "uusdc".to_string(),
            deposit: PerpVaultDeposit {
                shares: Uint128::zero(),
                amount: Uint128::zero()
            },
            unlocks: vec![
                UnlockState {
                    created_at: unlock_1_current_time,
                    cooldown_end: unlock_1_current_time + cooldown_period,
                    amount: amt_to_unlock,
                },
                UnlockState {
                    created_at: unlock_2_current_time,
                    cooldown_end: unlock_2_current_time + cooldown_period,
                    amount: amt_to_unlock,
                }
            ]
        }
    );

    // move time forward to pass cooldown period for first and second unlock
    mock.set_block_time(unlock_2_current_time + cooldown_period);

    // withdraw from vault should succeed for two unlocks
    mock.withdraw_from_vault(&credit_manager, account_id).unwrap();

    // vault position should be empty after withdraw
    let vault_position = mock.query_vault_position(account_id, ActionKind::Default);
    assert!(vault_position.is_none());
}
