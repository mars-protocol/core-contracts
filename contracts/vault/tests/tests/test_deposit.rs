use std::str::FromStr;

use cosmwasm_std::{coin, Addr, Decimal, Int128, Uint128};
use cw_utils::PaymentError;
use mars_mock_oracle::msg::CoinPrice;
use mars_testing::multitest::helpers::default_perp_params;
use mars_types::{oracle::ActionKind, params::PerpParamsUpdate};
use mars_vault::{error::ContractError, vault_token::calculate_vault_tokens};

use super::{
    helpers::{AccountToFund, MockEnv},
    vault_helpers::{assert_vault_err, execute_deposit},
};
use crate::tests::{
    helpers::deploy_managed_vault,
    test_redeem::uusdc_info,
    vault_helpers::{
        open_perp_position, query_account_positions, query_convert_to_assets,
        query_performance_fee, query_total_assets, query_total_vault_token_supply,
        query_vault_info,
    },
};

#[test]
fn deposit_invalid_funds() {
    let fund_manager = Addr::unchecked("fund-manager");
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: fund_manager.clone(),
            funds: vec![coin(1_000_000_000, "untrn")],
        })
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![coin(1_000_000_000, "untrn"), coin(1_000_000_000, "uusdc")],
        })
        .build()
        .unwrap();
    let credit_manager = mock.rover.clone();

    let managed_vault_addr = deploy_managed_vault(&mut mock.app, &fund_manager, &credit_manager);

    mock.create_fund_manager_account(&fund_manager, &managed_vault_addr);

    let res = execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[],
    );
    assert_vault_err(res, ContractError::Payment(PaymentError::NoFunds {}));

    let res = execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(1_001, "untrn"), coin(1_002, "uusdc")],
    );
    assert_vault_err(res, ContractError::Payment(PaymentError::MultipleDenoms {}));

    let res = execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(1_001, "untrn")],
    );
    assert_vault_err(res, ContractError::Payment(PaymentError::MissingDenom("uusdc".to_string())));
}

#[test]
fn deposit_if_credit_manager_account_not_binded() {
    let fund_manager = Addr::unchecked("fund-manager");
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: fund_manager.clone(),
            funds: vec![coin(1_000_000_000, "untrn")],
        })
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![coin(1_000_000_000, "uusdc")],
        })
        .build()
        .unwrap();
    let credit_manager = mock.rover.clone();

    let managed_vault_addr = deploy_managed_vault(&mut mock.app, &fund_manager, &credit_manager);

    let deposited_amt = Uint128::new(123_000_000);
    let res = execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(deposited_amt.u128(), "uusdc")],
    );
    assert_vault_err(res, ContractError::VaultAccountNotFound {});
}

#[test]
fn deposit_succeded() {
    let fund_manager = Addr::unchecked("fund-manager");
    let user = Addr::unchecked("user");
    let user_funded_amt = Uint128::new(1_000_000_000);
    let mut mock = MockEnv::new()
        .set_params(&[uusdc_info()])
        .fund_account(AccountToFund {
            addr: fund_manager.clone(),
            funds: vec![coin(1_000_000_000, "untrn")],
        })
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![coin(user_funded_amt.u128(), "uusdc")],
        })
        .build()
        .unwrap();
    let credit_manager = mock.rover.clone();

    let managed_vault_addr = deploy_managed_vault(&mut mock.app, &fund_manager, &credit_manager);
    let vault_info_res = query_vault_info(&mock, &managed_vault_addr);
    let vault_token = vault_info_res.vault_token;

    // there shouldn't be any vault tokens
    let vault_token_balance = mock.query_balance(&managed_vault_addr, &vault_token).amount;
    assert!(vault_token_balance.is_zero());
    let vault_token_balance = mock.query_balance(&user, &vault_token).amount;
    assert!(vault_token_balance.is_zero());

    let account_id = mock.create_fund_manager_account(&fund_manager, &managed_vault_addr);

    let deposited_amt = Uint128::new(123_000_000);
    execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(deposited_amt.u128(), "uusdc")],
    )
    .unwrap();

    // check base token balance after deposit
    let user_base_token_balance = mock.query_balance(&user, "uusdc").amount;
    assert_eq!(user_base_token_balance, user_funded_amt - deposited_amt);

    // there should be vault tokens for the user now
    let vault_token_balance = mock.query_balance(&managed_vault_addr, &vault_token).amount;
    assert!(vault_token_balance.is_zero());
    let user_vault_token_balance = mock.query_balance(&user, &vault_token).amount;
    assert!(!user_vault_token_balance.is_zero());
    assert_eq!(user_vault_token_balance, deposited_amt * Uint128::new(1_000_000));

    // there should be a deposit in Fund Manager's account
    let res = mock.query_positions(&account_id);
    assert_eq!(res.deposits.len(), 1);
    let assets_res = res.deposits.first().unwrap();
    assert_eq!(assets_res.amount, deposited_amt);
    assert_eq!(assets_res.denom, "uusdc".to_string());

    // check total base/vault tokens and share price
    let vault_info_res = query_vault_info(&mock, &managed_vault_addr);
    let total_base_tokens = query_total_assets(&mock, &managed_vault_addr);
    let total_vault_tokens = query_total_vault_token_supply(&mock, &managed_vault_addr);
    assert_eq!(total_base_tokens, deposited_amt);
    assert_eq!(total_vault_tokens, user_vault_token_balance);
    assert_eq!(vault_info_res.total_base_tokens, total_base_tokens);
    assert_eq!(vault_info_res.total_vault_tokens, total_vault_tokens);
    assert_eq!(
        vault_info_res.share_price,
        Some(Decimal::from_ratio(total_base_tokens, total_vault_tokens))
    );
}

#[test]
fn deposit_with_perp_position_unrealized_pnl() {
    let fund_manager = Addr::unchecked("fund-manager");
    let user = Addr::unchecked("user");
    let user_funded_amt = Uint128::new(1_000_000_000);
    let mut mock = MockEnv::new()
        .set_params(&[uusdc_info()])
        .fund_account(AccountToFund {
            addr: fund_manager.clone(),
            funds: vec![coin(1_000_000_000, "untrn")],
        })
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![coin(user_funded_amt.u128(), "uusdc")],
        })
        .build()
        .unwrap();
    let credit_manager = mock.rover.clone();

    let btc_perp_denom = "perp/btc";
    let uusdc_info = uusdc_info();
    let perp_params = default_perp_params(btc_perp_denom);
    mock.update_perp_params(PerpParamsUpdate::AddOrUpdate {
        params: perp_params,
    });

    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: btc_perp_denom.to_string(),
        price: Decimal::from_str("100").unwrap(),
    });

    // default price of uusdc is 1.02, lets change it back to 1
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: uusdc_info.denom.to_string(),
        price: Decimal::from_str("1.000").unwrap(),
    });

    let managed_vault_addr = deploy_managed_vault(&mut mock.app, &fund_manager, &credit_manager);
    let vault_info_res = query_vault_info(&mock, &managed_vault_addr);
    let vault_token = vault_info_res.vault_token;

    let account_id = mock.create_fund_manager_account(&fund_manager, &managed_vault_addr);

    let first_deposit_amt = Uint128::new(100_000_000);
    execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(first_deposit_amt.u128(), "uusdc")],
    )
    .unwrap();

    let performance_fee = query_performance_fee(&mock, &managed_vault_addr);

    // open perp position @ 100 price
    open_perp_position(
        &mut mock,
        &account_id,
        &fund_manager,
        btc_perp_denom,
        Int128::from_str("-1000000").unwrap(),
    );

    // change price so we have unrealized positive pnl
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: btc_perp_denom.to_string(),
        price: Decimal::from_str("90").unwrap(),
    });

    // query position pnl of vault credit account, verify that it's positive
    let positions = query_account_positions(&mock, &credit_manager, &account_id);
    assert_eq!(positions.perps.len(), 1);
    let position = positions.perps.first().unwrap();
    assert_eq!(position.unrealized_pnl.pnl, Int128::from_str("9099999").unwrap());

    let user_vault_token_balance_after_first_deposit =
        mock.query_balance(&user, &vault_token).amount;
    let vault_token_supply_after_first_deposit =
        query_total_vault_token_supply(&mock, &managed_vault_addr);
    let underlying_base_tokens = query_convert_to_assets(
        &mock,
        &managed_vault_addr,
        user_vault_token_balance_after_first_deposit,
    );

    // we are up 10% on our $100 position, however both opening fee and closing fee are taken out of the position.
    // so:
    // initial amount = 100_000_000
    // opening fee = -1_000_000
    // closing fee = -900_000
    // unrealized_price_pnl = 9_999_999
    // unrealized_pnl = 9_999_999 - 1_000_000 - 900_000 = 8_099_999
    // so our base tokens should be 108099999
    let expected_base_tokens_pre_deposit = Uint128::new(108099999);
    assert_eq!(underlying_base_tokens, expected_base_tokens_pre_deposit);

    // deposit again
    execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(first_deposit_amt.u128(), "uusdc")],
    )
    .unwrap();

    // for the second deposit, we should receive less tokens than the first deposit
    // amount of tokens received when depositing is governed by the formula:
    // vault_token_supply.multiply_ratio(base_tokens, total_base_tokens)
    // initial amount = 100_000_000
    // opening fee = -1_000_000
    // closing fee = -900_000
    // unrealized_price_pnl = 9_999_999
    // unrealized_pnl = 9_999_999 - 1_000_000 - 900_000 = 8_099_999
    // so our base tokens before deposit should be 108099999
    // query performance fee amount

    let vault_tokens_expected_from_deposit = calculate_vault_tokens(
        first_deposit_amt,
        expected_base_tokens_pre_deposit - performance_fee.accumulated_fee,
        vault_token_supply_after_first_deposit,
    )
    .unwrap();

    let user_vault_token_balance_after_second_deposit =
        mock.query_balance(&user, &vault_token).amount;
    let vault_tokens_from_second_deposit = user_vault_token_balance_after_second_deposit
        - user_vault_token_balance_after_first_deposit;

    assert_eq!(vault_tokens_from_second_deposit, vault_tokens_expected_from_deposit);
}

#[test]
fn deposit_into_bankrupt_vault() {
    let fund_manager = Addr::unchecked("fund-manager");
    let user = Addr::unchecked("user");
    let user_funded_amt = Uint128::new(1_000_000_000);
    let mut mock = MockEnv::new()
        .set_params(&[uusdc_info()])
        .fund_account(AccountToFund {
            addr: fund_manager.clone(),
            funds: vec![coin(1_000_000_000, "untrn")],
        })
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![coin(user_funded_amt.u128(), "uusdc")],
        })
        .build()
        .unwrap();
    let credit_manager = mock.rover.clone();

    let managed_vault_addr = deploy_managed_vault(&mut mock.app, &fund_manager, &credit_manager);

    let account_id = mock.create_fund_manager_account(&fund_manager, &managed_vault_addr);

    let first_deposit_amt = Uint128::new(100_000_000);
    execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(first_deposit_amt.u128(), "uusdc")],
    )
    .unwrap();

    let btc_perp_denom = "perp/btc";
    let perp_params = default_perp_params(btc_perp_denom);
    mock.update_perp_params(PerpParamsUpdate::AddOrUpdate {
        params: perp_params,
    });

    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: btc_perp_denom.to_string(),
        price: Decimal::from_str("100").unwrap(),
    });

    // open perp position @ 100 price
    open_perp_position(
        &mut mock,
        &account_id,
        &fund_manager,
        btc_perp_denom,
        Int128::from_str("-3000000").unwrap(),
    );

    // change price so we have unrealized negative pnl beyond our collateral
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: btc_perp_denom.to_string(),
        price: Decimal::from_str("145").unwrap(),
    });

    // query position pnl of vault credit account, verify that it's negative
    let positions = query_account_positions(&mock, &credit_manager, &account_id);
    assert_eq!(positions.perps.len(), 1);
    let position = positions.perps.first().unwrap();
    assert_eq!(position.unrealized_pnl.pnl, Int128::from_str("-136617645").unwrap());

    // try to deposit again
    let res = execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(first_deposit_amt.u128(), "uusdc")],
    );
    assert_vault_err(
        res,
        ContractError::VaultBankrupt {
            vault_account_id: account_id.to_string(),
        },
    );
}
