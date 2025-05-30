use std::str::FromStr;

use cosmwasm_std::{coin, Addr, Decimal, Int128, Uint128};
use cw_multi_test::{BankSudo, SudoMsg};
use mars_mock_oracle::msg::CoinPrice;
use mars_testing::multitest::helpers::{
    coin_info, default_perp_params, deploy_managed_vault_with_performance_fee, uatom_info, CoinInfo,
};
use mars_types::{
    credit_manager::Action,
    oracle::ActionKind,
    params::{ManagedVaultConfigUpdate, PerpParamsUpdate},
};
use mars_vault::{
    error::ContractError,
    performance_fee::{PerformanceFeeConfig, PerformanceFeeState},
};

use super::{
    helpers::{AccountToFund, MockEnv},
    vault_helpers::{assert_vault_err, execute_withdraw_performance_fee},
};
use crate::tests::{
    helpers::deploy_managed_vault,
    vault_helpers::{
        execute_deposit, execute_redeem, execute_unlock, instantiate_vault, open_perp_position,
        query_account_positions, query_performance_fee, query_vault_info, VaultSetup,
    },
};

#[test]
fn deposit_if_credit_manager_account_not_binded() {
    let fund_manager = Addr::unchecked("fund-manager");
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: fund_manager.clone(),
            funds: vec![
                coin(1_000_000_000, "untrn"),
                coin(mars_testing::MIN_VAULT_FEE_CREATION_IN_UUSD, "uusdc"),
            ],
        })
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![coin(1_000_000_000, "uusdc")],
        })
        .build()
        .unwrap();
    let credit_manager = mock.rover.clone();

    mock.update_managed_vault_config(ManagedVaultConfigUpdate::SetMinCreationFeeInUusd(
        mars_testing::MIN_VAULT_FEE_CREATION_IN_UUSD,
    ));

    let managed_vault_addr = deploy_managed_vault(
        &mut mock.app,
        &fund_manager,
        &credit_manager,
        Some(coin(mars_testing::MIN_VAULT_FEE_CREATION_IN_UUSD, "uusdc")),
    );

    let res = execute_withdraw_performance_fee(&mut mock, &user, &managed_vault_addr, None);
    assert_vault_err(res, ContractError::VaultAccountNotFound {});
}

#[test]
fn unauthorized_performance_fee_withdraw() {
    let fund_manager = Addr::unchecked("fund-manager");
    let user = Addr::unchecked("user");
    let user_funded_amt = Uint128::new(1_000_000_000);
    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: fund_manager.clone(),
            funds: vec![
                coin(1_000_000_000, "untrn"),
                coin(mars_testing::MIN_VAULT_FEE_CREATION_IN_UUSD, "uusdc"),
            ],
        })
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![coin(user_funded_amt.u128(), "uusdc")],
        })
        .build()
        .unwrap();
    let credit_manager = mock.rover.clone();

    mock.update_managed_vault_config(ManagedVaultConfigUpdate::SetMinCreationFeeInUusd(
        mars_testing::MIN_VAULT_FEE_CREATION_IN_UUSD,
    ));

    let managed_vault_addr = deploy_managed_vault(
        &mut mock.app,
        &fund_manager,
        &credit_manager,
        Some(coin(mars_testing::MIN_VAULT_FEE_CREATION_IN_UUSD, "uusdc")),
    );
    let code_id = mock.query_code_id(&managed_vault_addr);
    mock.update_managed_vault_config(ManagedVaultConfigUpdate::AddCodeId(code_id));

    let vault_acc_id = mock.create_fund_manager_account(&fund_manager, &managed_vault_addr);

    // vault user can't withdraw performance fee
    let res = execute_withdraw_performance_fee(&mut mock, &user, &managed_vault_addr, None);
    assert_vault_err(
        res,
        ContractError::NotTokenOwner {
            user: user.to_string(),
            account_id: vault_acc_id.clone(),
        },
    );

    // random user can't withdraw performance fee
    let random_user = Addr::unchecked("random-user");
    let res = execute_withdraw_performance_fee(&mut mock, &random_user, &managed_vault_addr, None);
    assert_vault_err(
        res,
        ContractError::NotTokenOwner {
            user: random_user.to_string(),
            account_id: vault_acc_id,
        },
    );
}

#[test]
fn cannot_withdraw_zero_performance_fee() {
    let uusdc_info = coin_info("uusdc");
    let uatom_info = uatom_info();

    let fund_manager = Addr::unchecked("fund-manager");
    let user = Addr::unchecked("user");
    let user_funded_amt = Uint128::new(100_000_000_000);
    let mut mock = MockEnv::new()
        .set_params(&[uusdc_info.clone(), uatom_info.clone()])
        .fund_account(AccountToFund {
            addr: fund_manager.clone(),
            funds: vec![
                coin(1_000_000_000, "untrn"),
                coin(mars_testing::MIN_VAULT_FEE_CREATION_IN_UUSD * 4, "uusdc"),
            ], // uusdc price is 0.25 uusd
        })
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![coin(user_funded_amt.u128(), "uusdc")],
        })
        .build()
        .unwrap();
    let credit_manager = mock.rover.clone();

    mock.update_managed_vault_config(ManagedVaultConfigUpdate::SetMinCreationFeeInUusd(
        mars_testing::MIN_VAULT_FEE_CREATION_IN_UUSD,
    ));

    let managed_vault_addr = deploy_managed_vault_with_performance_fee(
        &mut mock.app,
        &fund_manager,
        &credit_manager,
        1,
        PerformanceFeeConfig {
            fee_rate: Decimal::from_str("0.0000208").unwrap(),
            withdrawal_interval: 60,
        },
        &uusdc_info.denom,
        Some(coin(mars_testing::MIN_VAULT_FEE_CREATION_IN_UUSD * 4, "uusdc")),
    );
    let code_id = mock.query_code_id(&managed_vault_addr);
    mock.update_managed_vault_config(ManagedVaultConfigUpdate::AddCodeId(code_id));

    mock.create_fund_manager_account(&fund_manager, &managed_vault_addr);

    let res = execute_withdraw_performance_fee(&mut mock, &fund_manager, &managed_vault_addr, None);
    assert_vault_err(res, ContractError::ZeroPerformanceFee {});
}

#[test]
fn cannot_withdraw_if_withdrawal_interval_not_passed() {
    let uusdc_info = coin_info("uusdc");
    let uatom_info = uatom_info();

    let fund_manager = Addr::unchecked("fund-manager");
    let user = Addr::unchecked("user");
    let user_funded_amt = Uint128::new(100_000_000_000);
    let mut mock = MockEnv::new()
        .set_params(&[uusdc_info.clone(), uatom_info.clone()])
        .fund_account(AccountToFund {
            addr: fund_manager.clone(),
            funds: vec![
                coin(1_000_000_000, "untrn"),
                coin(mars_testing::MIN_VAULT_FEE_CREATION_IN_UUSD * 4, "uusdc"),
            ], // uusdc price is 0.25 uusd
        })
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![coin(user_funded_amt.u128(), "uusdc")],
        })
        .build()
        .unwrap();
    let credit_manager = mock.rover.clone();

    mock.update_managed_vault_config(ManagedVaultConfigUpdate::SetMinCreationFeeInUusd(
        mars_testing::MIN_VAULT_FEE_CREATION_IN_UUSD,
    ));

    let performance_fee_interval = 7200u64; // 2 hours
    let managed_vault_addr = deploy_managed_vault_with_performance_fee(
        &mut mock.app,
        &fund_manager,
        &credit_manager,
        1,
        PerformanceFeeConfig {
            fee_rate: Decimal::from_str("0.0000208").unwrap(),
            withdrawal_interval: performance_fee_interval,
        },
        &uusdc_info.denom,
        Some(coin(mars_testing::MIN_VAULT_FEE_CREATION_IN_UUSD * 4, "uusdc")),
    );
    let code_id = mock.query_code_id(&managed_vault_addr);
    mock.update_managed_vault_config(ManagedVaultConfigUpdate::AddCodeId(code_id));

    let fund_acc_id = mock.create_fund_manager_account(&fund_manager, &managed_vault_addr);

    // simulate base token price = 1 USD
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: uusdc_info.denom.clone(),
        price: Decimal::one(),
    });

    let deposited_amt = Uint128::new(100_000_000);
    execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(deposited_amt.u128(), "uusdc")],
    )
    .unwrap();

    // swap USDC to ATOM to tune PnL value based on different ATOM price
    swap_usdc_to_atom(&mut mock, &fund_acc_id, &fund_manager, &uusdc_info, &uatom_info);

    let pnl = calculate_pnl(&mut mock, &fund_acc_id, Decimal::from_str("1.25").unwrap());
    assert_eq!(pnl, Uint128::new(120_000_000));

    // check performance fee fund manager wallet balance
    let base_token_balance = mock.query_balance(&fund_manager, &uusdc_info.denom.clone()).amount;
    assert!(base_token_balance.is_zero());

    // move by interval - 1
    mock.increment_by_time(performance_fee_interval - 1);

    let res = execute_withdraw_performance_fee(&mut mock, &fund_manager, &managed_vault_addr, None);
    assert_vault_err(res, ContractError::WithdrawalIntervalNotPassed {});

    // move by another 1 second
    mock.increment_by_time(1);

    // try to pass invalid performance fee config
    let res = execute_withdraw_performance_fee(
        &mut mock,
        &fund_manager,
        &managed_vault_addr,
        Some(PerformanceFeeConfig {
            fee_rate: Decimal::from_str("0.000046287042457350").unwrap(),
            withdrawal_interval: 1563,
        }),
    );
    assert_vault_err(
        res,
        ContractError::InvalidPerformanceFee {
            expected: Decimal::from_str("0.000046287042457349").unwrap(),
            actual: Decimal::from_str("0.000046287042457350").unwrap(),
        },
    );

    execute_withdraw_performance_fee(&mut mock, &fund_manager, &managed_vault_addr, None).unwrap();

    let base_token_balance = mock.query_balance(&fund_manager, &uusdc_info.denom.clone()).amount;
    assert_eq!(base_token_balance, Uint128::new(832));
}

/// Scenarios based on spreadsheet:
/// ../files/Mars - 3rd party Vault - Performance Fee - test cases v1.0.xlsx
#[test]
fn performance_fee_correctly_accumulated() {
    let uusdc_info = coin_info("uusdc");
    let uatom_info = uatom_info();
    let user = Addr::unchecked("user");

    let VaultSetup {
        mut mock,
        fund_manager,
        managed_vault_addr,
        fund_acc_id,
    } = instantiate_vault(&uusdc_info, &uatom_info, &uusdc_info.denom);

    // simulate base token price = 1 USD
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: uusdc_info.denom.clone(),
        price: Decimal::one(),
    });

    let vault_info_res = query_vault_info(&mock, &managed_vault_addr);
    let vault_token = vault_info_res.vault_token;

    // there shouldn't be any base tokens in Fund Manager wallet
    let base_token_balance = mock.query_balance(&fund_manager, &uusdc_info.denom.clone()).amount;
    assert!(base_token_balance.is_zero());

    // -- FIRST ACTION --

    let first_deposit_time = mock.query_block_time();
    let deposited_amt = Uint128::new(100_000_000);
    execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(deposited_amt.u128(), "uusdc")],
    )
    .unwrap();

    let performance_fee = query_performance_fee(&mock, &managed_vault_addr);
    assert_eq!(
        performance_fee,
        PerformanceFeeState {
            last_withdrawal: first_deposit_time,
            base_tokens_amt: deposited_amt,
            accumulated_pnl: Int128::zero(),
            accumulated_fee: Uint128::zero()
        }
    );

    // swap USDC to ATOM to tune PnL value based on different ATOM price
    swap_usdc_to_atom(&mut mock, &fund_acc_id, &fund_manager, &uusdc_info, &uatom_info);

    // -- SECOND ACTION --

    // move by 97 hours and 20 minutes
    // fee is applier per 1 hour so 20 minutes should be ignored during fee calculation
    mock.increment_by_time(97 * 60 * 60 + 20 * 60);

    let pnl = calculate_pnl(&mut mock, &fund_acc_id, Decimal::from_str("1.25").unwrap());
    assert_eq!(pnl, Uint128::new(120_000_000));

    let deposited_amt = Uint128::new(20_000_000);
    execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(deposited_amt.u128(), "uusdc")],
    )
    .unwrap();

    let performance_fee = query_performance_fee(&mock, &managed_vault_addr);
    assert_eq!(
        performance_fee,
        PerformanceFeeState {
            last_withdrawal: first_deposit_time,
            base_tokens_amt: Uint128::new(140000000),
            accumulated_pnl: Int128::new(20000000),
            accumulated_fee: Uint128::new(40352)
        }
    );

    // -- THIRD ACTION --

    // move by 72 hours reduced by 20 min (applied in previous step)
    mock.increment_by_time(72 * 60 * 60 - 20 * 60);

    let pnl = calculate_pnl(&mut mock, &fund_acc_id, Decimal::from_str("0.25").unwrap());
    assert_eq!(pnl, Uint128::new(60_000_000));

    let deposited_amt = Uint128::new(15_000_000);
    execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(deposited_amt.u128(), "uusdc")],
    )
    .unwrap();

    let performance_fee = query_performance_fee(&mock, &managed_vault_addr);
    assert_eq!(
        performance_fee,
        PerformanceFeeState {
            last_withdrawal: first_deposit_time,
            base_tokens_amt: Uint128::new(75000000),
            accumulated_pnl: Int128::new(-60000000),
            accumulated_fee: Uint128::zero()
        }
    );

    // -- FOURTH ACTION --

    let unlock_vault_tokens = Uint128::new(10_000_000_000_000);
    execute_unlock(&mut mock, &user, &managed_vault_addr, unlock_vault_tokens, &[]).unwrap();

    // move by 144 hours
    mock.increment_by_time(144 * 60 * 60);

    // we have 55_000_000 uusdc + 80_000_000 uatom
    // we want to have pnl = 450_000_000 uusdc so uatom has to be worth 450_000_000 - 55_000_000 = 395_000_000
    // so the price of uatom has to be 395_000_000 / 80_000_000 = 4.9375
    let pnl = calculate_pnl(&mut mock, &fund_acc_id, Decimal::from_str("4.9375").unwrap());
    assert_eq!(pnl, Uint128::new(450_000_000));

    execute_redeem(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(unlock_vault_tokens.u128(), vault_token.clone())],
    )
    .unwrap();

    let performance_fee = query_performance_fee(&mock, &managed_vault_addr);
    assert_eq!(
        performance_fee,
        PerformanceFeeState {
            last_withdrawal: first_deposit_time,
            base_tokens_amt: Uint128::new(419284958),
            accumulated_pnl: Int128::new(315000000),
            accumulated_fee: Uint128::new(2050776)
        }
    );

    // -- FIFTH ACTION --

    // move by 744 hours
    mock.increment_by_time(744 * 60 * 60);

    let pnl = calculate_pnl(&mut mock, &fund_acc_id, Decimal::from_str("10").unwrap());
    assert_eq!(pnl, Uint128::new(824284958));

    execute_withdraw_performance_fee(
        &mut mock,
        &fund_manager,
        &managed_vault_addr,
        Some(PerformanceFeeConfig {
            fee_rate: Decimal::from_str("0.0000408").unwrap(),
            withdrawal_interval: 60,
        }),
    )
    .unwrap();

    let fee_withdraw_time = mock.query_block_time();
    let performance_fee = query_performance_fee(&mock, &managed_vault_addr);
    assert_eq!(
        performance_fee,
        PerformanceFeeState {
            last_withdrawal: fee_withdraw_time,
            base_tokens_amt: Uint128::new(808455326),
            accumulated_pnl: Int128::zero(),
            accumulated_fee: Uint128::zero()
        }
    );

    let base_token_balance = mock.query_balance(&fund_manager, &uusdc_info.denom.clone()).amount;
    assert_eq!(base_token_balance, Uint128::new(15829632));

    // -- SIXTH ACTION --

    // move by 48 hours
    mock.increment_by_time(48 * 60 * 60);

    let pnl = calculate_pnl(&mut mock, &fund_acc_id, Decimal::from_str("10.5").unwrap());
    assert_eq!(pnl, Uint128::new(848455326));

    let deposited_amt = Uint128::new(55_000_000);
    execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(deposited_amt.u128(), "uusdc")],
    )
    .unwrap();

    // new performance fee percentage should be used
    let performance_fee = query_performance_fee(&mock, &managed_vault_addr);
    assert_eq!(
        performance_fee,
        PerformanceFeeState {
            last_withdrawal: fee_withdraw_time,
            base_tokens_amt: Uint128::new(903455326),
            accumulated_pnl: Int128::new(40000000),
            accumulated_fee: Uint128::new(78336)
        }
    );
}

#[test]
fn performance_fee_correctly_accumulated_with_perp_position() {
    let uusdc_info = coin_info("uusdc");
    let uatom_info = uatom_info();
    let base_denom = uusdc_info.denom.clone();
    let btc_perp_denom = "perp/btc";
    let user = Addr::unchecked("user");
    let VaultSetup {
        mut mock,
        fund_manager,
        managed_vault_addr,
        fund_acc_id,
    } = instantiate_vault(&uusdc_info, &uatom_info, &base_denom);

    // set usdc price to 1 USD
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: uusdc_info.denom.clone(),
        price: Decimal::from_str("1").unwrap(),
    });

    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: btc_perp_denom.to_string(),
        price: Decimal::from_str("10").unwrap(),
    });

    // add perp params
    mock.update_perp_params(PerpParamsUpdate::AddOrUpdate {
        params: default_perp_params(btc_perp_denom),
    });

    // perform deposit
    let deposited_amt = Uint128::new(100_000_000);
    execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(deposited_amt.u128(), "uusdc")],
    )
    .unwrap();

    // open perp position
    open_perp_position(
        &mut mock,
        &fund_acc_id,
        &fund_manager,
        btc_perp_denom,
        Int128::from_str("-1000000").unwrap(),
    );

    // update price to reflect profit
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: btc_perp_denom.to_string(),
        price: Decimal::from_str("9").unwrap(),
    });

    // query position pnl of vault, verify that it's positive
    let positions = query_account_positions(&mock, &mock.rover, &fund_acc_id);
    assert_eq!(positions.perps.len(), 1);
    let position = positions.perps.first().unwrap();
    assert_eq!(position.unrealized_pnl.pnl, Int128::from_str("909999").unwrap());

    // move by 48 hours
    mock.increment_by_time(48 * 60 * 60);

    // get our performance of the vault
    let performance_fee = query_performance_fee(&mock, &managed_vault_addr);

    // non PNL has actually accumulated yet, so we expect performance fee to not incorporate pnl or fees.
    assert_eq!(
        performance_fee,
        PerformanceFeeState {
            last_withdrawal: 1571797419_u64,
            base_tokens_amt: Uint128::new(100_000_000),
            accumulated_pnl: Int128::zero(),
            accumulated_fee: Uint128::zero(),
        }
    );

    // trigger a performance fee update using a deposit.
    let deposited_amt = Uint128::new(100_000_000);
    execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(deposited_amt.u128(), "uusdc")],
    )
    .unwrap();

    let performance_fee: PerformanceFeeState = query_performance_fee(&mock, &managed_vault_addr);

    // ensure that performance fee is updated.
    assert_eq!(
        performance_fee,
        PerformanceFeeState {
            last_withdrawal: 1571797419_u64,
            base_tokens_amt: Uint128::new(200809999),
            accumulated_pnl: Int128::new(809999),
            accumulated_fee: Uint128::new(808),
        }
    );

    // move by 48 hours
    mock.increment_by_time(48 * 60 * 60);

    // trigger a performance fee update using a deposit
    let deposited_amt = Uint128::new(100_000_000);
    execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(deposited_amt.u128(), uusdc_info.denom.clone())],
    )
    .unwrap();

    // get our pnl of the vault
    let performance_fee = query_performance_fee(&mock, &managed_vault_addr);

    // ensure that performance fee is updated.
    assert_eq!(
        performance_fee,
        PerformanceFeeState {
            last_withdrawal: 1571797419_u64,
            base_tokens_amt: Uint128::new(300809997),
            accumulated_pnl: Int128::new(809997),
            accumulated_fee: Uint128::new(1617),
        }
    );
}

#[test]
fn performance_fee_correctly_accumulated_when_base_denom_is_uatom() {
    let uusdc_info = coin_info("uusdc");
    let uatom_info = uatom_info();
    let base_denom = uatom_info.denom.clone();
    let atom_perp_denom = "atom/btc";
    let user = Addr::unchecked("user");

    let VaultSetup {
        mut mock,
        fund_manager,
        managed_vault_addr,
        fund_acc_id,
    } = instantiate_vault(&uusdc_info, &uatom_info, &base_denom);

    // Set usdc price to 1 USD
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: uusdc_info.denom.clone(),
        price: Decimal::from_str("1").unwrap(),
    });

    // Set atom price to 10 USD
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: uatom_info.denom.clone(),
        price: Decimal::from_str("10").unwrap(),
    });

    // Set perp price to 10 USD
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: atom_perp_denom.to_string(),
        price: Decimal::from_str("10").unwrap(),
    });

    mock.update_perp_params(PerpParamsUpdate::AddOrUpdate {
        params: default_perp_params(atom_perp_denom),
    });

    // Step 1 - Perform deposit
    let deposited_amt = Uint128::new(100_000_000);
    execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(deposited_amt.u128(), base_denom.clone())],
    )
    .unwrap();

    // Step 2 - open perp position
    open_perp_position(
        &mut mock,
        &fund_acc_id,
        &fund_manager,
        atom_perp_denom,
        Int128::from_str("-1000000").unwrap(),
    );

    // Step 3 - update price of both spot and perp
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: uatom_info.denom.clone(),
        price: Decimal::from_str("9.0").unwrap(),
    });

    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: atom_perp_denom.to_string(),
        price: Decimal::from_str("9.0").unwrap(),
    });

    // Step 4 - move time by 48 hours
    mock.increment_by_time(48 * 60 * 60);

    // step 5 - deposit to trigger performance fee update
    let deposited_amt = Uint128::new(100_000_000);
    execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(deposited_amt.u128(), base_denom.clone())],
    )
    .unwrap();

    // Step 6 - verify that performance fee is updated
    let performance_fee = query_performance_fee(&mock, &managed_vault_addr);
    assert_eq!(
        performance_fee,
        PerformanceFeeState {
            last_withdrawal: 1571797419_u64,
            base_tokens_amt: Uint128::new(200_089_999),
            accumulated_pnl: Int128::new(89999),
            accumulated_fee: Uint128::new(89),
        }
    );
}

fn swap_usdc_to_atom(
    mock: &mut MockEnv,
    fund_acc_id: &str,
    fund_manager: &Addr,
    uusdc_info: &CoinInfo,
    uatom_info: &CoinInfo,
) {
    let swap_amt = Uint128::new(80_000_000);
    let cm_config = mock.query_config();
    mock.app
        .sudo(SudoMsg::Bank(BankSudo::Mint {
            to_address: cm_config.swapper,
            amount: vec![coin(swap_amt.u128(), uatom_info.denom.clone())],
        }))
        .unwrap();
    let estimate_res = mock.query_swap_estimate_with_optional_route(
        &uusdc_info.to_coin(swap_amt.u128()),
        &uatom_info.denom,
        None,
    );
    let min_receive =
        estimate_res.amount * (Decimal::one() - Decimal::from_atomics(6u128, 1).unwrap());
    mock.update_credit_account(
        fund_acc_id,
        fund_manager,
        vec![Action::SwapExactIn {
            coin_in: uusdc_info.to_action_coin(swap_amt.u128()),
            denom_out: uatom_info.denom.clone(),
            min_receive,
            route: None,
        }],
        &[],
    )
    .unwrap();
}

fn calculate_pnl(mock: &mut MockEnv, fund_acc_id: &str, new_atom_price: Decimal) -> Uint128 {
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: "uatom".to_string(),
        price: new_atom_price,
    });

    let res = mock.query_positions(fund_acc_id);
    assert_eq!(res.deposits.len(), 2);

    let mut pnl = Uint128::zero();
    for deposit in res.deposits.iter() {
        let price = mock.query_price(&deposit.denom).price;
        let value = deposit.amount * price;
        pnl += value;
    }

    pnl
}
