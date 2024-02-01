use cosmwasm_std::{coin, Addr, Decimal, Uint128};
use mars_perps::error::ContractError;
use mars_types::perps::UnlockState;

use super::helpers::MockEnv;
use crate::tests::helpers::assert_err;

#[test]
fn unlock_few_times() {
    let depositor = Addr::unchecked("depositor");
    let cooldown_period = 1225u64;
    let mut mock = MockEnv::new().cooldown_period(cooldown_period).build().unwrap();
    let owner = mock.owner.clone();

    // set usdc price
    mock.set_price(&owner, "uusdc", Decimal::one()).unwrap();

    mock.fund_accounts(&[&depositor], 1_000_000_000_000u128, &["uusdc"]);

    mock.deposit_to_vault(&depositor, &[coin(1_000_000_000u128, "uusdc")]).unwrap();

    // unlocks should be empty
    let unlocks = mock.query_unlocks(depositor.as_str());
    assert!(unlocks.is_empty());

    // amounts to unlock
    let deposit = mock.query_deposit(depositor.as_str());
    let shares_1 = deposit.shares.multiply_ratio(1u128, 2u128); // 50%
    let amt_1 = deposit.amount.multiply_ratio(1u128, 2u128);
    let shares_2 = deposit.shares.multiply_ratio(1u128, 4u128); // 25%
    let amt_2 = deposit.amount.multiply_ratio(1u128, 4u128);
    let shares_3 = deposit.shares.multiply_ratio(1u128, 4u128); // 25%
    let amt_3 = deposit.amount.multiply_ratio(1u128, 4u128);

    // first unlock
    mock.unlock_from_vault(&depositor, shares_1).unwrap();
    let unlocks = mock.query_unlocks(depositor.as_str());
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
    mock.unlock_from_vault(&depositor, shares_2).unwrap();
    let unlocks = mock.query_unlocks(depositor.as_str());
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
    mock.unlock_from_vault(&depositor, shares_3).unwrap();
    let unlocks = mock.query_unlocks(depositor.as_str());
    let current_time = mock.query_block_time();
    let unlock_3_expected = UnlockState {
        created_at: current_time,
        cooldown_end: current_time + cooldown_period,
        amount: amt_3,
    };
    assert_eq!(unlocks, vec![unlock_1_expected, unlock_2_expected, unlock_3_expected]);

    // deposit should be empty after all unlocks
    let deposit = mock.query_deposit(depositor.as_str());
    assert!(deposit.amount.is_zero());
    assert!(deposit.shares.is_zero());
}

#[test]
fn withdraw_not_possible_if_cooldown_not_ended() {
    let depositor = Addr::unchecked("depositor");
    let cooldown_period = 86400u64;
    let mut mock = MockEnv::new().cooldown_period(cooldown_period).build().unwrap();
    let owner = mock.owner.clone();

    // set usdc price
    mock.set_price(&owner, "uusdc", Decimal::one()).unwrap();

    mock.fund_accounts(&[&depositor], 1_000_000_000_000u128, &["uusdc"]);

    mock.deposit_to_vault(&depositor, &[coin(1_000_000_000u128, "uusdc")]).unwrap();

    mock.unlock_from_vault(&depositor, Uint128::new(1_000_000)).unwrap();

    let unlocks = mock.query_unlocks(depositor.as_str());
    assert!(!unlocks.is_empty());

    // cooldown period should be passed for at least one unlock
    let res = mock.withdraw_from_vault(&depositor);
    assert_err(res, ContractError::UnlockedPositionsNotFound {});
}

#[test]
fn withdraw_unlocked_shares() {
    let depositor = Addr::unchecked("depositor");
    let cooldown_period = 86400u64;
    let mut mock = MockEnv::new().cooldown_period(cooldown_period).build().unwrap();
    let owner = mock.owner.clone();

    // set usdc price
    mock.set_price(&owner, "uusdc", Decimal::one()).unwrap();

    mock.fund_accounts(&[&depositor], 1_000_000_000_000u128, &["uusdc"]);

    mock.deposit_to_vault(&depositor, &[coin(1_000_000_000u128, "uusdc")]).unwrap();

    // unlocks should be empty
    let unlocks = mock.query_unlocks(depositor.as_str());
    assert!(unlocks.is_empty());

    // amounts to unlock
    let deposit = mock.query_deposit(depositor.as_str());
    let shares_1 = deposit.shares.multiply_ratio(1u128, 2u128); // 50%
    let amt_1 = deposit.amount.multiply_ratio(1u128, 2u128);
    let shares_2 = deposit.shares.multiply_ratio(1u128, 4u128); // 25%
    let amt_2 = deposit.amount.multiply_ratio(1u128, 4u128);
    let shares_3 = deposit.shares.multiply_ratio(1u128, 4u128); // 25%
    let amt_3 = deposit.amount.multiply_ratio(1u128, 4u128);

    // first unlock
    mock.unlock_from_vault(&depositor, shares_1).unwrap();

    // move time forward
    mock.increment_by_time(3600);

    // second unlock
    let unlock_2_current_time = mock.query_block_time();
    mock.unlock_from_vault(&depositor, shares_2).unwrap();

    // move time forward
    mock.increment_by_time(3600);

    // second unlock
    let unlock_3_current_time = mock.query_block_time();
    mock.unlock_from_vault(&depositor, shares_3).unwrap();

    // move time forward to pass cooldown period for first and second unlock
    mock.set_block_time(unlock_2_current_time + cooldown_period);

    // check balances before withdraw
    let balance_1 = mock.query_balance(&depositor, "uusdc");

    // withdraw from vault should succeed for two unlocks
    mock.withdraw_from_vault(&depositor).unwrap();

    // check balances after withdraw, it should be increased by amount of two unlocks
    let balance_2 = mock.query_balance(&depositor, "uusdc");
    assert_eq!(balance_2.amount, balance_1.amount + amt_1 + amt_2);

    // check unlocks after withdraw, it should be one unlock left
    let unlocks = mock.query_unlocks(depositor.as_str());
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
    mock.withdraw_from_vault(&depositor).unwrap();

    // check balances after withdraw, it should be increased by amount of last unlock
    let balance_3 = mock.query_balance(&depositor, "uusdc");
    assert_eq!(balance_3.amount, balance_2.amount + amt_3);

    // check unlocks after withdraw, it should be empty
    let unlocks = mock.query_unlocks(depositor.as_str());
    assert!(unlocks.is_empty());
}
