use std::str::FromStr;

use cosmwasm_std::{coin, Addr, Decimal, Uint128};
use mars_perps::{error::ContractError, vault::DEFAULT_SHARES_PER_AMOUNT};
use mars_types::{
    params::{PerpParams, PerpParamsUpdate},
    perps::{PerpVaultDeposit, PerpVaultPosition, PerpVaultUnlock, VaultResponse},
    signed_uint::SignedUint,
};

use super::helpers::MockEnv;
use crate::tests::helpers::{assert_err, default_perp_params};

#[test]
fn random_user_cannot_deposit_to_vault() {
    let mut mock = MockEnv::new().build().unwrap();
    let random_sender = Addr::unchecked("random-user-123");
    mock.fund_accounts(&[&random_sender], 1_000_000_000_000u128, &["uusdc"]);

    let res = mock.deposit_to_vault(&random_sender, Some("2"), &[coin(1_000_000_000u128, "uusdc")]);
    assert_err(res, ContractError::SenderIsNotCreditManager);

    let res = mock.deposit_to_vault(&random_sender, Some(""), &[coin(1_000_000_000u128, "uusdc")]);
    assert_err(res, ContractError::SenderIsNotCreditManager);
}

#[test]
fn random_user_cannot_unlock_from_vault() {
    let mut mock = MockEnv::new().build().unwrap();
    let random_sender = Addr::unchecked("random-user-123");
    mock.fund_accounts(&[&random_sender], 1_000_000_000_000u128, &["uusdc"]);

    let res = mock.unlock_from_vault(&random_sender, Some("2"), Uint128::new(100));
    assert_err(res, ContractError::SenderIsNotCreditManager);

    let res = mock.unlock_from_vault(&random_sender, Some(""), Uint128::new(100));
    assert_err(res, ContractError::SenderIsNotCreditManager);
}

#[test]
fn random_user_cannot_withdraw_from_vault() {
    let mut mock = MockEnv::new().build().unwrap();
    let random_sender = Addr::unchecked("random-user-123");
    mock.fund_accounts(&[&random_sender], 1_000_000_000_000u128, &["uusdc"]);

    let res = mock.withdraw_from_vault(&random_sender, Some("2"));
    assert_err(res, ContractError::SenderIsNotCreditManager);

    let res = mock.withdraw_from_vault(&random_sender, Some(""));
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

    mock.deposit_to_vault(&credit_manager, Some(depositor), &[coin(1_000_000_000u128, "uusdc")])
        .unwrap();

    // unlocks should be empty
    let unlocks = mock.query_cm_unlocks(depositor);
    assert!(unlocks.is_empty());

    // amounts to unlock
    let deposit = mock.query_cm_deposit(depositor);
    let shares_1 = deposit.shares.multiply_ratio(1u128, 2u128); // 50%
    let amt_1 = deposit.amount.multiply_ratio(1u128, 2u128);
    let shares_2 = deposit.shares.multiply_ratio(1u128, 4u128); // 25%
    let amt_2 = deposit.amount.multiply_ratio(1u128, 4u128);
    let shares_3 = deposit.shares.multiply_ratio(1u128, 4u128); // 25%
    let amt_3 = deposit.amount.multiply_ratio(1u128, 4u128);

    // vault state before unlocks
    let vault_state_before_unlocks = mock.query_vault();
    assert_eq!(
        vault_state_before_unlocks,
        VaultResponse {
            total_balance: deposit.amount.into(),
            total_shares: deposit.shares,
            total_withdrawal_balance: deposit.amount,
            share_price: Some(Decimal::from_ratio(deposit.amount, deposit.shares)),
            total_liquidity: deposit.amount,
            total_debt: Uint128::zero(),
            collateralization_ratio: None
        }
    );

    // first unlock
    mock.unlock_from_vault(&credit_manager, Some(depositor), shares_1).unwrap();
    let unlocks = mock.query_cm_unlocks(depositor);
    let current_time = mock.query_block_time();
    let unlock_1_expected = PerpVaultUnlock {
        created_at: current_time,
        cooldown_end: current_time + cooldown_period,
        shares: shares_1,
        amount: amt_1,
    };
    assert_eq!(unlocks, vec![unlock_1_expected.clone()]);
    let vault_state = mock.query_vault();
    assert_eq!(vault_state, vault_state_before_unlocks);

    // move time forward
    mock.increment_by_time(3600);

    // second unlock
    mock.unlock_from_vault(&credit_manager, Some(depositor), shares_2).unwrap();
    let unlocks = mock.query_cm_unlocks(depositor);
    let current_time = mock.query_block_time();
    let unlock_2_expected = PerpVaultUnlock {
        created_at: current_time,
        cooldown_end: current_time + cooldown_period,
        shares: shares_2,
        amount: amt_2,
    };
    assert_eq!(unlocks, vec![unlock_1_expected.clone(), unlock_2_expected.clone()]);
    let vault_state = mock.query_vault();
    assert_eq!(vault_state, vault_state_before_unlocks);

    // move time forward
    mock.increment_by_time(3600);

    // third unlock
    mock.unlock_from_vault(&credit_manager, Some(depositor), shares_3).unwrap();
    let unlocks = mock.query_cm_unlocks(depositor);
    let current_time = mock.query_block_time();
    let unlock_3_expected = PerpVaultUnlock {
        created_at: current_time,
        cooldown_end: current_time + cooldown_period,
        shares: shares_3,
        amount: amt_3,
    };
    assert_eq!(unlocks, vec![unlock_1_expected, unlock_2_expected, unlock_3_expected]);
    let vault_state = mock.query_vault();
    assert_eq!(vault_state, vault_state_before_unlocks);

    // deposit should be empty after all unlocks
    let deposit = mock.query_cm_deposit(depositor);
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

    mock.deposit_to_vault(&credit_manager, Some(depositor), &[coin(1_000_000_000u128, "uusdc")])
        .unwrap();

    mock.unlock_from_vault(&credit_manager, Some(depositor), Uint128::new(1_000_000)).unwrap();

    let unlocks = mock.query_cm_unlocks(depositor);
    assert!(!unlocks.is_empty());

    // cooldown period should be passed for at least one unlock
    let res = mock.withdraw_from_vault(&credit_manager, Some(depositor));
    assert_err(res, ContractError::UnlockedPositionsNotFound {});
}

#[test]
fn withdraw_not_possible_if_withdraw_not_enabled() {
    let depositor = "depositor";
    let cooldown_period = 86400u64;
    let mut mock =
        MockEnv::new().cooldown_period(cooldown_period).withdraw_enabled(false).build().unwrap();
    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();

    // set usdc price
    mock.set_price(&owner, "uusdc", Decimal::one()).unwrap();

    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000u128, &["uusdc"]);

    mock.deposit_to_vault(&credit_manager, Some(depositor), &[coin(1_000_000_000u128, "uusdc")])
        .unwrap();

    mock.unlock_from_vault(&credit_manager, Some(depositor), Uint128::new(1_000_000)).unwrap();

    let unlocks = mock.query_cm_unlocks(depositor);
    assert!(!unlocks.is_empty());

    // move time forward
    mock.increment_by_time(86401u64);

    let res = mock.withdraw_from_vault(&credit_manager, Some(depositor));
    assert_err(res, ContractError::VaultWithdrawDisabled {});
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

    mock.deposit_to_vault(&credit_manager, Some(depositor), &[coin(1_000_000_000u128, "uusdc")])
        .unwrap();

    // vault state before unlocks
    let vault_state_before_unlocks = mock.query_vault();

    // unlocks should be empty
    let unlocks = mock.query_cm_unlocks(depositor);
    assert!(unlocks.is_empty());

    // amounts to unlock
    let deposit = mock.query_cm_deposit(depositor);
    let shares_1 = deposit.shares.multiply_ratio(1u128, 2u128); // 50%
    let amt_1 = deposit.amount.multiply_ratio(1u128, 2u128);
    let shares_2 = deposit.shares.multiply_ratio(1u128, 4u128); // 25%
    let amt_2 = deposit.amount.multiply_ratio(1u128, 4u128);
    let shares_3 = deposit.shares.multiply_ratio(1u128, 4u128); // 25%
    let amt_3 = deposit.amount.multiply_ratio(1u128, 4u128);

    // first unlock
    mock.unlock_from_vault(&credit_manager, Some(depositor), shares_1).unwrap();
    let vault_state = mock.query_vault();
    assert_eq!(vault_state, vault_state_before_unlocks);

    // move time forward
    mock.increment_by_time(3600);

    // second unlock
    let unlock_2_current_time = mock.query_block_time();
    mock.unlock_from_vault(&credit_manager, Some(depositor), shares_2).unwrap();
    let vault_state = mock.query_vault();
    assert_eq!(vault_state, vault_state_before_unlocks);

    // move time forward
    mock.increment_by_time(3600);

    // third unlock
    let unlock_3_current_time = mock.query_block_time();
    mock.unlock_from_vault(&credit_manager, Some(depositor), shares_3).unwrap();
    let vault_state = mock.query_vault();
    assert_eq!(vault_state, vault_state_before_unlocks);

    // move time forward to pass cooldown period for first and second unlock
    mock.set_block_time(unlock_2_current_time + cooldown_period);

    // check balances before withdraw
    let balance_1 = mock.query_balance(&credit_manager, "uusdc");

    // withdraw from vault should succeed for two unlocks
    mock.withdraw_from_vault(&credit_manager, Some(depositor)).unwrap();

    // check balances after withdraw, it should be increased by amount of two unlocks
    let balance_2 = mock.query_balance(&credit_manager, "uusdc");
    assert_eq!(balance_2.amount, balance_1.amount + amt_1 + amt_2);

    // check vault state after withdraw, it should be decreased by amount of two unlocks
    let vault_state_after_two_unlocks = mock.query_vault();
    let total_deposits = vault_state_before_unlocks.total_balance.abs - amt_1 - amt_2;
    let total_shares = vault_state_before_unlocks.total_shares - shares_1 - shares_2;
    assert_eq!(
        vault_state_after_two_unlocks,
        VaultResponse {
            total_balance: total_deposits.into(),
            total_shares,
            total_withdrawal_balance: total_deposits,
            share_price: Some(Decimal::from_ratio(total_deposits, total_shares)),
            total_liquidity: total_deposits,
            total_debt: Uint128::zero(),
            collateralization_ratio: None
        }
    );

    // check unlocks after withdraw, it should be one unlock left
    let unlocks = mock.query_cm_unlocks(depositor);
    assert_eq!(
        unlocks,
        vec![PerpVaultUnlock {
            created_at: unlock_3_current_time,
            cooldown_end: unlock_3_current_time + cooldown_period,
            shares: shares_3,
            amount: amt_3,
        }]
    );

    // move time forward to pass cooldown period for last unlock
    mock.set_block_time(unlock_3_current_time + cooldown_period);

    // withdraw from vault should succeed for last unlock
    mock.withdraw_from_vault(&credit_manager, Some(depositor)).unwrap();

    // check balances after withdraw, it should be increased by amount of last unlock
    let balance_3 = mock.query_balance(&credit_manager, "uusdc");
    assert_eq!(balance_3.amount, balance_2.amount + amt_3);

    // check vault state after withdraw
    let vault_state = mock.query_vault();
    assert_eq!(
        vault_state,
        VaultResponse {
            total_balance: vault_state_after_two_unlocks
                .total_balance
                .checked_sub(amt_3.into())
                .unwrap(),
            total_shares: vault_state_after_two_unlocks.total_shares - shares_3,
            ..Default::default()
        }
    );

    // check unlocks after withdraw, it should be empty
    let unlocks = mock.query_cm_unlocks(depositor);
    assert!(unlocks.is_empty());
}

#[test]
fn unlock_and_withdraw_if_zero_withdrawal_balance() {
    let cooldown_period = 86400u64;
    let mut mock = MockEnv::new().cooldown_period(cooldown_period).build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let user = "jake";

    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000u128, &["uatom", "uusdc"]);

    // init denoms
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
    mock.deposit_to_vault(&credit_manager, Some(user), &[coin(1000u128, "uusdc")]).unwrap();

    // open a position
    let size = SignedUint::from_str("50").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    mock.execute_perp_order(&credit_manager, "1", "uatom", size, None, &[atom_opening_fee])
        .unwrap();

    // increase uatom price to make the position profitable
    mock.set_price(&owner, "uatom", Decimal::from_str("50").unwrap()).unwrap();

    // Unlock is possible even if there is zero withdrawal balance. Vault balance can change after unlock during cooldown period.
    let block_time = mock.query_block_time();
    let deposit = mock.query_cm_deposit(user);
    mock.unlock_from_vault(&credit_manager, Some(user), deposit.shares).unwrap();
    let perp_vault_pos = mock.query_cm_vault_position(user).unwrap();
    assert_eq!(
        perp_vault_pos,
        PerpVaultPosition {
            denom: "uusdc".to_string(),
            deposit: PerpVaultDeposit {
                shares: Uint128::zero(),
                amount: Uint128::zero()
            },
            unlocks: vec![PerpVaultUnlock {
                created_at: block_time,
                cooldown_end: block_time + cooldown_period,
                shares: deposit.shares,
                amount: Uint128::zero(), // zero withdrawal balance
            }],
        }
    );

    // move time forward
    mock.increment_by_time(cooldown_period + 1);

    // withdraw from vault fails because of zero withdrawal balance
    let res = mock.withdraw_from_vault(&credit_manager, Some(user));
    assert_err(res, ContractError::ZeroWithdrawalBalance {});
}

#[test]
fn calculate_shares_correctly_after_zero_withdrawal_balance() {
    let cooldown_period = 86400u64;
    let mut mock = MockEnv::new().cooldown_period(cooldown_period).build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let depositor_1 = "bob";
    let depositor_2 = "dane";
    let depositor_3 = "mark";

    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000u128, &["uatom", "uusdc"]);

    // init denoms
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
    mock.deposit_to_vault(&credit_manager, Some(depositor_1), &[coin(1000u128, "uusdc")]).unwrap();
    mock.deposit_to_vault(&credit_manager, Some(depositor_2), &[coin(4000u128, "uusdc")]).unwrap();

    // check deposits
    let deposit_1_before = mock.query_cm_deposit(depositor_1);
    let deposit_2_before = mock.query_cm_deposit(depositor_2);
    assert_eq!(deposit_1_before.amount, Uint128::new(1000));
    assert_eq!(deposit_2_before.amount, Uint128::new(4000));
    assert_eq!(deposit_2_before.shares, deposit_1_before.shares.multiply_ratio(4u128, 1u128)); // 4 times more than depositor_1

    // open a position
    let size = SignedUint::from_str("100").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    mock.execute_perp_order(&credit_manager, "1", "uatom", size, None, &[atom_opening_fee])
        .unwrap();

    // increase uatom price to make the position profitable
    mock.set_price(&owner, "uatom", Decimal::from_str("100").unwrap()).unwrap();

    // make sure that there is no withdrawal balance
    let vault_state = mock.query_vault();
    let accounting = mock.query_total_accounting();
    let available_liquidity =
        accounting.withdrawal_balance.total.checked_add(vault_state.total_balance).unwrap();
    assert!(available_liquidity < SignedUint::zero());

    // deposit uusdc to vault when zero withdrawal balance
    mock.deposit_to_vault(&credit_manager, Some(depositor_3), &[coin(2500u128, "uusdc")]).unwrap();

    // Check deposits. There should be zero amounts because of zero withdrawal balance.
    let deposit_1 = mock.query_cm_deposit(depositor_1);
    assert_eq!(deposit_1.amount, Uint128::zero());
    assert_eq!(deposit_1.shares, deposit_1_before.shares);
    let deposit_2 = mock.query_cm_deposit(depositor_2);
    assert_eq!(deposit_2.amount, Uint128::zero());
    assert_eq!(deposit_2.shares, deposit_2_before.shares);
    let deposit_3 = mock.query_cm_deposit(depositor_3);
    assert_eq!(deposit_3.amount, Uint128::zero());
    assert_eq!(deposit_3.shares, deposit_1_before.shares.multiply_ratio(5u128, 2u128)); // 2.5 times more than depositor_1

    // change price to previous value
    mock.set_price(&owner, "uatom", Decimal::from_str("10").unwrap()).unwrap();

    // Amounts and shares should be caclulated proportionally.
    let deposit_1 = mock.query_cm_deposit(depositor_1);
    assert_eq!(deposit_1.amount, Uint128::new(1003));
    assert_eq!(deposit_1.shares, deposit_1_before.shares);
    let deposit_2 = mock.query_cm_deposit(depositor_2);
    assert_eq!(deposit_2.amount, deposit_1.amount.multiply_ratio(4u128, 1u128)); // 4 times more than depositor_1
    assert_eq!(deposit_2.shares, deposit_2_before.shares);
    let deposit_3 = mock.query_cm_deposit(depositor_3);
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
    let vault_position = mock.query_cm_vault_position(account_id);
    assert!(vault_position.is_none());

    let deposit_amt = Uint128::new(1_200_000_000u128);
    mock.deposit_to_vault(&credit_manager, Some(account_id), &[coin(deposit_amt.u128(), "uusdc")])
        .unwrap();
    mock.deposit_to_vault(
        &credit_manager,
        Some("random-user"),
        &[coin(2_400_000_000u128, "uusdc")],
    )
    .unwrap();

    // vault position should contain only deposit for account_id
    let deposit_shares = Uint128::new(1_200_000_000_000_000u128);
    let vault_position = mock.query_cm_vault_position(account_id);
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
    mock.unlock_from_vault(&credit_manager, Some(account_id), shares_to_unlock).unwrap();

    // vault position should contain deposit and one unlock
    let deposit_shares_after_1_unlock = deposit_shares - shares_to_unlock;
    let deposit_amt_after_1_unlock = deposit_amt - amt_to_unlock;
    let vault_position = mock.query_cm_vault_position(account_id);
    assert_eq!(
        vault_position.unwrap(),
        PerpVaultPosition {
            denom: "uusdc".to_string(),
            deposit: PerpVaultDeposit {
                shares: deposit_shares_after_1_unlock,
                amount: deposit_amt_after_1_unlock
            },
            unlocks: vec![PerpVaultUnlock {
                created_at: unlock_1_current_time,
                cooldown_end: unlock_1_current_time + cooldown_period,
                shares: shares_to_unlock,
                amount: amt_to_unlock,
            }]
        }
    );

    // move time forward
    mock.increment_by_time(3600);

    // second unlock
    let unlock_2_current_time = mock.query_block_time();
    mock.unlock_from_vault(&credit_manager, Some(account_id), shares_to_unlock).unwrap();

    // vault position should have zero deposit and two unlocks
    let vault_position = mock.query_cm_vault_position(account_id);
    assert_eq!(
        vault_position.unwrap(),
        PerpVaultPosition {
            denom: "uusdc".to_string(),
            deposit: PerpVaultDeposit {
                shares: Uint128::zero(),
                amount: Uint128::zero()
            },
            unlocks: vec![
                PerpVaultUnlock {
                    created_at: unlock_1_current_time,
                    cooldown_end: unlock_1_current_time + cooldown_period,
                    shares: shares_to_unlock,
                    amount: amt_to_unlock,
                },
                PerpVaultUnlock {
                    created_at: unlock_2_current_time,
                    cooldown_end: unlock_2_current_time + cooldown_period,
                    shares: shares_to_unlock,
                    amount: amt_to_unlock,
                }
            ]
        }
    );

    // move time forward to pass cooldown period for first and second unlock
    mock.set_block_time(unlock_2_current_time + cooldown_period);

    // withdraw from vault should succeed for two unlocks
    mock.withdraw_from_vault(&credit_manager, Some(account_id)).unwrap();

    // vault position should be empty after withdraw
    let vault_position = mock.query_cm_vault_position(account_id);
    assert!(vault_position.is_none());
}

#[test]
fn use_wallet_for_vault() {
    let depositor = Addr::unchecked("charles");
    let cooldown_period = 1225u64;
    let mut mock = MockEnv::new().cooldown_period(cooldown_period).build().unwrap();
    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let perps = mock.perps.clone();

    // set usdc price
    mock.set_price(&owner, "uusdc", Decimal::one()).unwrap();

    mock.fund_accounts(&[&credit_manager, &depositor], 1_000_000_000_000u128, &["uusdc"]);

    let deposit_amt = Uint128::new(2_400_000_000u128);
    mock.deposit_to_vault(&depositor, None, &[coin(deposit_amt.u128(), "uusdc")]).unwrap();

    // balances after deposit
    let depositor_balance_after_deposit = mock.query_balance(&depositor, "uusdc");
    let vault_balance_after_deposit = mock.query_balance(&perps, "uusdc");

    let deposit_shares = deposit_amt.checked_mul(Uint128::new(DEFAULT_SHARES_PER_AMOUNT)).unwrap();
    let vault_position = mock.query_vault_position(depositor.as_str(), None);
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

    let unlock_current_time = mock.query_block_time();
    mock.unlock_from_vault(&depositor, None, shares_to_unlock).unwrap();

    // vault position should contain deposit and one unlock
    let deposit_shares_after_unlock = deposit_shares - shares_to_unlock;
    let deposit_amt_after_unlock = deposit_amt - amt_to_unlock;
    let vault_position = mock.query_vault_position(depositor.as_str(), None);
    assert_eq!(
        vault_position.unwrap(),
        PerpVaultPosition {
            denom: "uusdc".to_string(),
            deposit: PerpVaultDeposit {
                shares: deposit_shares_after_unlock,
                amount: deposit_amt_after_unlock
            },
            unlocks: vec![PerpVaultUnlock {
                created_at: unlock_current_time,
                cooldown_end: unlock_current_time + cooldown_period,
                shares: shares_to_unlock,
                amount: amt_to_unlock,
            }]
        }
    );

    // move time forward to pass cooldown period for first and second unlock
    mock.set_block_time(unlock_current_time + cooldown_period + 1);

    // withdraw from vault
    mock.withdraw_from_vault(&depositor, None).unwrap();

    // vault position should contain only deposit
    let vault_position = mock.query_vault_position(depositor.as_str(), None);
    assert_eq!(
        vault_position.unwrap(),
        PerpVaultPosition {
            denom: "uusdc".to_string(),
            deposit: PerpVaultDeposit {
                shares: deposit_shares_after_unlock,
                amount: deposit_amt_after_unlock
            },
            unlocks: vec![]
        }
    );

    // balances after withdraw
    let depositor_balance_after_withdraw = mock.query_balance(&depositor, "uusdc");
    let vault_balance_after_withdraw = mock.query_balance(&perps, "uusdc");
    assert_eq!(
        depositor_balance_after_deposit.amount + amt_to_unlock,
        depositor_balance_after_withdraw.amount
    );
    assert_eq!(
        vault_balance_after_deposit.amount - amt_to_unlock,
        vault_balance_after_withdraw.amount
    );
}

#[test]
fn withdraw_profits_for_depositors() {
    let cooldown_period = 86400u64;
    let mut mock = MockEnv::new().cooldown_period(cooldown_period).build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let depositor_1 = "bob";
    let depositor_2 = "dane";

    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000u128, &["uatom", "uusdc"]);

    // init denoms
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
    let depositor_1_amt = Uint128::new(1000u128);
    let depositor_2_amt = Uint128::new(4000u128);
    mock.deposit_to_vault(
        &credit_manager,
        Some(depositor_1),
        &[coin(depositor_1_amt.u128(), "uusdc")],
    )
    .unwrap();
    mock.deposit_to_vault(
        &credit_manager,
        Some(depositor_2),
        &[coin(depositor_2_amt.u128(), "uusdc")],
    )
    .unwrap();

    // check deposits
    let deposit_1_before = mock.query_cm_deposit(depositor_1);
    let deposit_2_before = mock.query_cm_deposit(depositor_2);
    assert_eq!(deposit_1_before.amount, depositor_1_amt);
    assert_eq!(deposit_1_before.shares, depositor_1_amt * Uint128::new(DEFAULT_SHARES_PER_AMOUNT));
    assert_eq!(deposit_2_before.amount, depositor_2_amt);
    assert_eq!(deposit_2_before.shares, deposit_1_before.shares.multiply_ratio(4u128, 1u128)); // 4 times more than depositor_1

    // open a position
    let size = SignedUint::from_str("100").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    assert_eq!(atom_opening_fee, coin(23, "uusdc"));
    mock.execute_perp_order(&credit_manager, "1", "uatom", size, None, &[atom_opening_fee.clone()])
        .unwrap();

    // decrease uatom price to make the position losing
    mock.set_price(&owner, "uatom", Decimal::from_str("5").unwrap()).unwrap();

    // close the position
    let atom_closing_pnl = coin(562, "uusdc");
    mock.execute_perp_order(
        &credit_manager,
        "1",
        "uatom",
        size.neg(),
        None,
        &[atom_closing_pnl.clone()],
    )
    .unwrap();

    // check vault state
    let vault = mock.query_vault();
    let total_deposits = depositor_1_amt + depositor_2_amt;
    let total_shares = total_deposits * Uint128::new(DEFAULT_SHARES_PER_AMOUNT);
    let total_amt_from_perp_pos = atom_opening_fee.amount + atom_closing_pnl.amount;
    let total_liquidity = total_deposits + total_amt_from_perp_pos;
    assert_eq!(
        vault,
        VaultResponse {
            total_balance: total_deposits.into(),
            total_shares,
            total_withdrawal_balance: total_liquidity, // total cash flow is equal to total withdrawal balance when no open positions
            share_price: Some(Decimal::from_ratio(total_liquidity, total_shares)),
            total_liquidity,
            total_debt: Uint128::zero(),
            collateralization_ratio: None
        }
    );

    // unlocks
    let unlock_current_time = mock.query_block_time();
    mock.unlock_from_vault(&credit_manager, Some(depositor_1), deposit_1_before.shares).unwrap();
    mock.unlock_from_vault(&credit_manager, Some(depositor_2), deposit_2_before.shares).unwrap();

    // move time forward to pass cooldown period
    mock.set_block_time(unlock_current_time + cooldown_period + 1);

    // withdraw from the vault
    mock.withdraw_from_vault(&credit_manager, Some(depositor_1)).unwrap();
    mock.withdraw_from_vault(&credit_manager, Some(depositor_2)).unwrap();

    // Check deposits. There should be zero amounts/shares.
    let deposit_1 = mock.query_cm_deposit(depositor_1);
    assert_eq!(deposit_1.amount, Uint128::zero());
    assert_eq!(deposit_1.shares, Uint128::zero());
    let deposit_2 = mock.query_cm_deposit(depositor_2);
    assert_eq!(deposit_2.amount, Uint128::zero());
    assert_eq!(deposit_2.shares, Uint128::zero());

    // check vault state
    let vault = mock.query_vault();
    assert_eq!(
        vault,
        VaultResponse {
            total_balance: SignedUint::zero().checked_sub(total_amt_from_perp_pos.into()).unwrap(), // negative number because of profits from perp positions
            total_shares: Uint128::zero(),
            total_withdrawal_balance: Uint128::zero(),
            share_price: None,
            total_liquidity: Uint128::zero(),
            total_debt: Uint128::zero(),
            collateralization_ratio: None
        }
    );
}

#[test]
fn cannot_withdraw_if_cr_decreases_below_threshold() {
    let cooldown_period = 86400u64;
    let target_collateralization_ratio = Decimal::percent(130);
    let mut mock = MockEnv::new()
        .cooldown_period(cooldown_period)
        .target_vault_collaterization_ratio(target_collateralization_ratio)
        .build()
        .unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let depositor_1 = "bob";
    let depositor_2 = "dane";

    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000u128, &["uatom", "uusdc"]);

    // init denoms
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
    let depositor_1_amt = Uint128::new(1000u128);
    let depositor_2_amt = Uint128::new(4000u128);
    mock.deposit_to_vault(
        &credit_manager,
        Some(depositor_1),
        &[coin(depositor_1_amt.u128(), "uusdc")],
    )
    .unwrap();
    mock.deposit_to_vault(
        &credit_manager,
        Some(depositor_2),
        &[coin(depositor_2_amt.u128(), "uusdc")],
    )
    .unwrap();

    // check deposits
    let deposit_1_before = mock.query_cm_deposit(depositor_1);
    let deposit_2_before = mock.query_cm_deposit(depositor_2);

    // unlocks
    let unlock_current_time = mock.query_block_time();
    mock.unlock_from_vault(&credit_manager, Some(depositor_1), deposit_1_before.shares).unwrap();
    mock.unlock_from_vault(&credit_manager, Some(depositor_2), deposit_2_before.shares).unwrap();

    // move time forward to pass cooldown period
    mock.set_block_time(unlock_current_time + cooldown_period + 1);

    // open a position
    let size = SignedUint::from_str("100").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", size).fee;
    mock.execute_perp_order(&credit_manager, "1", "uatom", size, None, &[atom_opening_fee.clone()])
        .unwrap();

    // check vault state
    let vault = mock.query_vault();
    assert!(vault.collateralization_ratio.is_none());

    // increase uatom price to make the position profiting
    mock.set_price(&owner, "uatom", Decimal::from_str("45").unwrap()).unwrap();

    // check vault state
    let vault = mock.query_vault();
    assert!(vault.collateralization_ratio.unwrap() > target_collateralization_ratio);

    // should fail because CR decreases below the threshold after withdrawal
    let res = mock.withdraw_from_vault(&credit_manager, Some(depositor_1));
    assert_err(
        res,
        ContractError::VaultUndercollateralized {
            current_cr: Decimal::from_str("1.250260688216892596").unwrap(),
            threshold_cr: target_collateralization_ratio,
        },
    );
}
