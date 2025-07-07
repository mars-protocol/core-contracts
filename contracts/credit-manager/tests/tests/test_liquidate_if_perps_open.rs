use std::str::FromStr;

use cosmwasm_std::{coin, coins, Addr, Coin, Decimal, Int128, Uint128};
use mars_mock_oracle::msg::CoinPrice;
use mars_types::{
    adapters::vault::VaultUnchecked,
    credit_manager::{
        Action::{Borrow, Deposit, ExecutePerpOrder, Lend, Liquidate, StakeAstroLp},
        ExecutePerpOrderType, LiquidateRequest,
    },
    oracle::ActionKind,
    params::PerpParamsUpdate,
    perps::{PnL, PnlAmounts},
};
use test_case::test_case;

use super::helpers::{get_coin, get_debt, uatom_info, uosmo_info, AccountToFund, MockEnv};
use crate::tests::helpers::{coin_info, default_perp_params, uusdc_info};

/// Tests liquidation of a position with perps open when the liquidatee has enough usdc to cover the perps loss
#[test]
fn close_perps_when_enough_usdc_in_account_to_cover_loss() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();
    let uusdc_info = uusdc_info();

    let contract_owner = Addr::unchecked("owner");

    let liquidator = Addr::unchecked("liquidator");
    let liquidatee = Addr::unchecked("liquidatee");
    let vault_depositor = Addr::unchecked("vault_depositor");

    let vault_coin_deposited = coin(100000, uusdc_info.denom.clone());
    let uosmo_coin_deposited = uosmo_info.to_coin(3000);
    let uusdc_coin_deposited = uusdc_info.to_coin(650);

    let mut mock = MockEnv::new()
        .owner(contract_owner.as_str())
        .perps_liquidation_bonus_ratio(Decimal::zero()) // disable liquidation bonus for closing perps
        .set_params(&[uosmo_info.clone(), uatom_info.clone(), uusdc_info.clone()])
        .fund_account(AccountToFund {
            addr: liquidatee.clone(),
            funds: vec![uosmo_coin_deposited.clone(), uusdc_coin_deposited.clone()],
        })
        .fund_account(AccountToFund {
            addr: liquidator.clone(),
            funds: coins(3000, uatom_info.denom.clone()),
        })
        .fund_account(AccountToFund {
            addr: vault_depositor.clone(),
            funds: vec![vault_coin_deposited.clone()],
        })
        .build()
        .unwrap();
    let liquidatee_account_id = mock.create_credit_account(&liquidatee).unwrap();
    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();
    let vault_depositor_account_id = mock.create_credit_account(&vault_depositor).unwrap();

    // setup perps
    mock.update_perp_params(PerpParamsUpdate::AddOrUpdate {
        params: default_perp_params(&uosmo_info.denom),
    });
    mock.update_perp_params(PerpParamsUpdate::AddOrUpdate {
        params: default_perp_params(&uatom_info.denom),
    });

    // deposit to vault
    mock.update_credit_account(
        &vault_depositor_account_id,
        &vault_depositor,
        vec![Deposit(vault_coin_deposited.clone())],
        &[vault_coin_deposited.clone()],
    )
    .unwrap();
    mock.deposit_to_perp_vault(&vault_depositor_account_id, &vault_coin_deposited, None).unwrap();

    // setup liquidatee's position
    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![
            Deposit(uosmo_coin_deposited.clone()),
            Deposit(uusdc_coin_deposited.clone()),
            Borrow(uatom_info.to_coin(2400)),
            ExecutePerpOrder {
                denom: uosmo_info.denom.clone(),
                order_size: Int128::from_str("200").unwrap(),
                reduce_only: None,
                order_type: Some(ExecutePerpOrderType::Default),
            },
            ExecutePerpOrder {
                denom: uatom_info.denom.clone(),
                order_size: Int128::from_str("-400").unwrap(),
                reduce_only: None,
                order_type: Some(ExecutePerpOrderType::Default),
            },
        ],
        &[uosmo_coin_deposited.clone(), uusdc_coin_deposited.clone()],
    )
    .unwrap();

    // Change the price for Default and Liquidation pricing.
    // Liquidation pricing is used during liquidation.
    // Default pricing is used before and after liquidation to validate perp positions in the test.
    mock.price_change(CoinPrice {
        pricing: ActionKind::Liquidation,
        denom: uatom_info.denom.clone(),
        price: Decimal::from_atomics(26u128, 1).unwrap(),
    });
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: uatom_info.denom.clone(),
        price: Decimal::from_atomics(26u128, 1).unwrap(),
    });

    let prev_health = mock.query_health(&liquidatee_account_id, ActionKind::Liquidation);

    // usdc balance before liquidation
    let usdc_perps_balance_before_liq = mock.query_balance(mock.perps.address(), &uusdc_info.denom);
    let usdc_cm_balance_before_liq = mock.query_balance(&mock.rover, &uusdc_info.denom);

    // usdc position before liquidation
    let position = mock.query_positions(&liquidatee_account_id);
    let usdc_deposit_before_liq = get_coin("uusdc", &position.deposits);

    // perps pnl before liquidation
    let uosmo_perp_position =
        mock.query_perp_position(&liquidatee_account_id, &uosmo_info.denom).position.unwrap();
    let uatom_perp_position =
        mock.query_perp_position(&liquidatee_account_id, &uatom_info.denom).position.unwrap();
    let mut pnl_amounts_acc = PnlAmounts::default();
    pnl_amounts_acc.add(&uosmo_perp_position.unrealized_pnl).unwrap();
    pnl_amounts_acc.add(&uatom_perp_position.unrealized_pnl).unwrap();
    let pnl_before_liq = pnl_amounts_acc.to_coins(&uusdc_info.denom).pnl;
    let loss_amt = pnl_loss(pnl_before_liq);

    mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Deposit(uatom_info.to_coin(1000)),
            Liquidate {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: uatom_info.to_coin(100),
                request: LiquidateRequest::Deposit(uosmo_info.denom.clone()),
            },
        ],
        &[uatom_info.to_coin(1000)],
    )
    .unwrap();

    // Check usdc balance after liquidation
    let usdc_perps_balance = mock.query_balance(mock.perps.address(), &uusdc_info.denom);
    let usdc_cm_balance = mock.query_balance(&mock.rover, &uusdc_info.denom);
    assert_eq!(usdc_perps_balance.amount, usdc_perps_balance_before_liq.amount + loss_amt);
    assert_eq!(usdc_cm_balance.amount, usdc_cm_balance_before_liq.amount - loss_amt);

    // Assert liquidatee's new position
    let position = mock.query_positions(&liquidatee_account_id);
    assert_eq!(position.deposits.len(), 3);
    let usdc_balance = get_coin("uusdc", &position.deposits);
    assert_eq!(usdc_balance.amount, Uint128::new(22)); // initial usdc deposit - perps opening fees - perps loss
    assert_eq!(usdc_balance.amount, usdc_deposit_before_liq.amount - loss_amt);
    let osmo_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(osmo_balance.amount, Uint128::new(1912));
    let atom_balance = get_coin("uatom", &position.deposits);
    assert_eq!(atom_balance.amount, Uint128::new(2400));

    assert_eq!(position.debts.len(), 1);
    let atom_debt = get_debt("uatom", &position.debts);
    assert_eq!(atom_debt.amount, Uint128::new(2301));

    assert!(position.perps.is_empty());

    // Assert liquidator's new position
    let position = mock.query_positions(&liquidator_account_id);
    assert_eq!(position.deposits.len(), 2);
    assert_eq!(position.debts.len(), 0);
    let osmo_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(osmo_balance.amount, Uint128::new(1084));
    let atom_balance = get_coin("uatom", &position.deposits);
    assert_eq!(atom_balance.amount, Uint128::new(900));

    // Assert rewards-collector's new position
    let rewards_collector_acc_id = mock.query_rewards_collector_account();
    let position = mock.query_positions(&rewards_collector_acc_id);
    assert_eq!(position.deposits.len(), 1);
    assert_eq!(position.debts.len(), 0);
    let osmo_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(osmo_balance.amount, Uint128::new(4));

    // Liq HF should improve
    let health = mock.query_health(&liquidatee_account_id, ActionKind::Liquidation);
    assert!(!health.liquidatable);
    assert!(
        prev_health.liquidation_health_factor.unwrap() < health.liquidation_health_factor.unwrap()
    );
}

/// Tests liquidation of a position with perps open.
/// If the liquidatee's account has not enough usdc to cover the perps loss, the liquidatee's debt position is increased (usdc is borrowed from the Red Bank).
#[test]
fn close_perps_when_not_enough_usdc_in_account_to_cover_loss() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();
    let uusdc_info = uusdc_info();

    let contract_owner = Addr::unchecked("owner");

    let liquidator = Addr::unchecked("liquidator");
    let liquidatee = Addr::unchecked("liquidatee");
    let vault_depositor = Addr::unchecked("vault_depositor");

    let vault_coin_deposited = coin(100000, uusdc_info.denom.clone());
    let uosmo_coin_deposited = uosmo_info.to_coin(3000);
    let uusdc_coin_deposited = uusdc_info.to_coin(500);

    let mut mock = MockEnv::new()
        .owner(contract_owner.as_str())
        .perps_liquidation_bonus_ratio(Decimal::zero()) // disable liquidation bonus for closing perps
        .set_params(&[uosmo_info.clone(), uatom_info.clone(), uusdc_info.clone()])
        .fund_account(AccountToFund {
            addr: liquidatee.clone(),
            funds: vec![uosmo_coin_deposited.clone(), uusdc_coin_deposited.clone()],
        })
        .fund_account(AccountToFund {
            addr: liquidator.clone(),
            funds: coins(3000, uatom_info.denom.clone()),
        })
        .fund_account(AccountToFund {
            addr: vault_depositor.clone(),
            funds: vec![vault_coin_deposited.clone()],
        })
        .build()
        .unwrap();
    let liquidatee_account_id = mock.create_credit_account(&liquidatee).unwrap();
    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();
    let vault_depositor_account_id = mock.create_credit_account(&vault_depositor).unwrap();

    // setup perps
    mock.update_perp_params(PerpParamsUpdate::AddOrUpdate {
        params: default_perp_params(&uosmo_info.denom),
    });
    mock.update_perp_params(PerpParamsUpdate::AddOrUpdate {
        params: default_perp_params(&uatom_info.denom),
    });

    // deposit to vault
    mock.update_credit_account(
        &vault_depositor_account_id,
        &vault_depositor,
        vec![Deposit(vault_coin_deposited.clone())],
        &[vault_coin_deposited.clone()],
    )
    .unwrap();
    mock.deposit_to_perp_vault(&vault_depositor_account_id, &vault_coin_deposited, None).unwrap();

    // setup liquidatee's position
    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![
            Deposit(uosmo_coin_deposited.clone()),
            Deposit(uusdc_coin_deposited.clone()),
            Borrow(uatom_info.to_coin(2400)),
            ExecutePerpOrder {
                denom: uosmo_info.denom.clone(),
                order_size: Int128::from_str("200").unwrap(),
                reduce_only: None,
                order_type: Some(ExecutePerpOrderType::Default),
            },
            ExecutePerpOrder {
                denom: uatom_info.denom.clone(),
                order_size: Int128::from_str("-400").unwrap(),
                reduce_only: None,
                order_type: Some(ExecutePerpOrderType::Default),
            },
        ],
        &[uosmo_coin_deposited.clone(), uusdc_coin_deposited.clone()],
    )
    .unwrap();

    // Change the price for Default and Liquidation pricing.
    // Liquidation pricing is used during liquidation.
    // Default pricing is used before and after liquidation to validate perp positions in the test.
    mock.price_change(CoinPrice {
        pricing: ActionKind::Liquidation,
        denom: uatom_info.denom.clone(),
        price: Decimal::from_atomics(26u128, 1).unwrap(),
    });
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: uatom_info.denom.clone(),
        price: Decimal::from_atomics(26u128, 1).unwrap(),
    });

    let prev_health = mock.query_health(&liquidatee_account_id, ActionKind::Liquidation);

    // usdc balance before liquidation
    let usdc_perps_balance_before_liq = mock.query_balance(mock.perps.address(), &uusdc_info.denom);
    let usdc_cm_balance_before_liq = mock.query_balance(&mock.rover, &uusdc_info.denom);
    assert!(!usdc_cm_balance_before_liq.amount.is_zero());

    // usdc position before liquidation
    let position = mock.query_positions(&liquidatee_account_id);
    let usdc_deposit_before_liq = get_coin("uusdc", &position.deposits);
    assert!(!usdc_deposit_before_liq.amount.is_zero());

    // perps pnl before liquidation
    let uosmo_perp_position =
        mock.query_perp_position(&liquidatee_account_id, &uosmo_info.denom).position.unwrap();
    let uatom_perp_position =
        mock.query_perp_position(&liquidatee_account_id, &uatom_info.denom).position.unwrap();
    let mut pnl_amounts_acc = PnlAmounts::default();
    pnl_amounts_acc.add(&uosmo_perp_position.unrealized_pnl).unwrap();
    pnl_amounts_acc.add(&uatom_perp_position.unrealized_pnl).unwrap();
    let pnl_before_liq = pnl_amounts_acc.to_coins(&uusdc_info.denom).pnl;
    let loss_amt = pnl_loss(pnl_before_liq);

    mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Deposit(uatom_info.to_coin(1000)),
            Liquidate {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: uatom_info.to_coin(100),
                request: LiquidateRequest::Deposit(uosmo_info.denom.clone()),
            },
        ],
        &[uatom_info.to_coin(1000)],
    )
    .unwrap();

    // Check usdc balance after liquidation
    let usdc_perps_balance = mock.query_balance(mock.perps.address(), &uusdc_info.denom);
    let usdc_cm_balance = mock.query_balance(&mock.rover, &uusdc_info.denom);
    assert_eq!(usdc_perps_balance.amount, usdc_perps_balance_before_liq.amount + loss_amt);
    assert!(usdc_cm_balance.amount.is_zero());

    // Assert liquidatee's new position
    let position = mock.query_positions(&liquidatee_account_id);
    assert_eq!(position.debts.len(), 2);
    let usdc_debt = get_debt("uusdc", &position.debts);
    assert_eq!(usdc_debt.amount, Uint128::new(129));
    let atom_debt = get_debt("uatom", &position.debts);
    assert_eq!(atom_debt.amount, Uint128::new(2301));

    assert_eq!(position.deposits.len(), 2);
    let osmo_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(osmo_balance.amount, Uint128::new(1868));
    let atom_balance = get_coin("uatom", &position.deposits);
    assert_eq!(atom_balance.amount, Uint128::new(2400));

    assert!(position.perps.is_empty());

    // Assert liquidator's new position
    let position = mock.query_positions(&liquidator_account_id);
    assert_eq!(position.deposits.len(), 2);
    assert_eq!(position.debts.len(), 0);
    let osmo_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(osmo_balance.amount, Uint128::new(1128));
    let atom_balance = get_coin("uatom", &position.deposits);
    assert_eq!(atom_balance.amount, Uint128::new(900));

    // Assert rewards-collector's new position
    let rewards_collector_acc_id = mock.query_rewards_collector_account();
    let position = mock.query_positions(&rewards_collector_acc_id);
    assert_eq!(position.deposits.len(), 1);
    assert_eq!(position.debts.len(), 0);
    let osmo_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(osmo_balance.amount, Uint128::new(4));

    // Liq HF should improve
    let health = mock.query_health(&liquidatee_account_id, ActionKind::Liquidation);
    assert!(health.liquidatable);
    assert!(
        prev_health.liquidation_health_factor.unwrap() < health.liquidation_health_factor.unwrap()
    );
}

/// Tests liquidation of a position with perps open when the liquidatee has a perps profit
#[test]
fn close_perps_with_profit() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();
    let utia_info = coin_info("utia");
    let uusdc_info = uusdc_info();

    let contract_owner = Addr::unchecked("owner");

    let liquidator = Addr::unchecked("liquidator");
    let liquidatee = Addr::unchecked("liquidatee");
    let vault_depositor = Addr::unchecked("vault_depositor");

    let vault_coin_deposited = coin(100000, uusdc_info.denom.clone());
    let uosmo_coin_deposited = uosmo_info.to_coin(3000);
    let uusdc_coin_deposited = uusdc_info.to_coin(50);

    let mut mock = MockEnv::new()
        .owner(contract_owner.as_str())
        .set_params(&[
            uosmo_info.clone(),
            uatom_info.clone(),
            uusdc_info.clone(),
            utia_info.clone(),
        ])
        .fund_account(AccountToFund {
            addr: liquidatee.clone(),
            funds: vec![uosmo_coin_deposited.clone(), uusdc_coin_deposited.clone()],
        })
        .fund_account(AccountToFund {
            addr: liquidator.clone(),
            funds: coins(3000, uatom_info.denom.clone()),
        })
        .fund_account(AccountToFund {
            addr: vault_depositor.clone(),
            funds: vec![vault_coin_deposited.clone()],
        })
        .build()
        .unwrap();
    let liquidatee_account_id = mock.create_credit_account(&liquidatee).unwrap();
    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();
    let vault_depositor_account_id = mock.create_credit_account(&vault_depositor).unwrap();

    // setup perps
    mock.update_perp_params(PerpParamsUpdate::AddOrUpdate {
        params: default_perp_params(&uosmo_info.denom),
    });
    mock.update_perp_params(PerpParamsUpdate::AddOrUpdate {
        params: default_perp_params(&utia_info.denom),
    });

    // deposit to vault
    mock.update_credit_account(
        &vault_depositor_account_id,
        &vault_depositor,
        vec![Deposit(vault_coin_deposited.clone())],
        &[vault_coin_deposited.clone()],
    )
    .unwrap();
    mock.deposit_to_perp_vault(&vault_depositor_account_id, &vault_coin_deposited, None).unwrap();

    // setup liquidatee's position
    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![
            Deposit(uosmo_coin_deposited.clone()),
            Deposit(uusdc_coin_deposited.clone()),
            Borrow(uatom_info.to_coin(2400)),
            ExecutePerpOrder {
                denom: uosmo_info.denom.clone(),
                order_size: Int128::from_str("200").unwrap(),
                reduce_only: None,
                order_type: Some(ExecutePerpOrderType::Default),
            },
            ExecutePerpOrder {
                denom: utia_info.denom.clone(),
                order_size: Int128::from_str("-400").unwrap(),
                reduce_only: None,
                order_type: Some(ExecutePerpOrderType::Default),
            },
        ],
        &[uosmo_coin_deposited.clone(), uusdc_coin_deposited.clone()],
    )
    .unwrap();

    // Change the price for Default and Liquidation pricing.
    // Liquidation pricing is used during liquidation.
    // Default pricing is used before and after liquidation to validate perp positions in the test.
    mock.price_change(CoinPrice {
        pricing: ActionKind::Liquidation,
        denom: uosmo_info.denom.clone(),
        price: Decimal::from_atomics(5u128, 2).unwrap(),
    });
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: uosmo_info.denom.clone(),
        price: Decimal::from_atomics(5u128, 2).unwrap(),
    });
    mock.price_change(CoinPrice {
        pricing: ActionKind::Liquidation,
        denom: utia_info.denom.clone(),
        price: Decimal::from_atomics(25u128, 3).unwrap(),
    });
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: utia_info.denom.clone(),
        price: Decimal::from_atomics(25u128, 3).unwrap(),
    });

    let prev_health = mock.query_health(&liquidatee_account_id, ActionKind::Liquidation);

    // usdc balance before liquidation
    let usdc_perps_balance_before_liq = mock.query_balance(mock.perps.address(), &uusdc_info.denom);
    let usdc_cm_balance_before_liq = mock.query_balance(&mock.rover, &uusdc_info.denom);

    // usdc position before liquidation
    let position = mock.query_positions(&liquidatee_account_id);
    let usdc_deposit_before_liq = get_coin("uusdc", &position.deposits);

    // perps pnl before liquidation
    let uosmo_perp_position =
        mock.query_perp_position(&liquidatee_account_id, &uosmo_info.denom).position.unwrap();
    let utia_perp_position =
        mock.query_perp_position(&liquidatee_account_id, &utia_info.denom).position.unwrap();
    let mut pnl_amounts_acc = PnlAmounts::default();
    pnl_amounts_acc.add(&uosmo_perp_position.unrealized_pnl).unwrap();
    pnl_amounts_acc.add(&utia_perp_position.unrealized_pnl).unwrap();
    let pnl_before_liq = pnl_amounts_acc.to_coins(&uusdc_info.denom).pnl;
    let profit_amt = pnl_profit(pnl_before_liq);

    mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Deposit(uatom_info.to_coin(1000)),
            Liquidate {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: uatom_info.to_coin(100),
                request: LiquidateRequest::Deposit(uosmo_info.denom.clone()),
            },
        ],
        &[uatom_info.to_coin(1000)],
    )
    .unwrap();

    // Check usdc balance after liquidation
    let usdc_perps_balance = mock.query_balance(mock.perps.address(), &uusdc_info.denom);
    let usdc_cm_balance = mock.query_balance(&mock.rover, &uusdc_info.denom);
    assert_eq!(usdc_perps_balance.amount, usdc_perps_balance_before_liq.amount - profit_amt);
    assert_eq!(usdc_cm_balance.amount, usdc_cm_balance_before_liq.amount + profit_amt);

    // Assert liquidatee's new position
    let position = mock.query_positions(&liquidatee_account_id);
    assert_eq!(position.deposits.len(), 3);
    let usdc_balance = get_coin("uusdc", &position.deposits);
    assert_eq!(usdc_balance.amount, Uint128::new(94)); // initial usdc deposit - perps opening fees + perps profit
    assert_eq!(usdc_balance.amount, usdc_deposit_before_liq.amount + profit_amt);
    let osmo_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(osmo_balance.amount, Uint128::new(940));
    let atom_balance = get_coin("uatom", &position.deposits);
    assert_eq!(atom_balance.amount, Uint128::new(2400));

    assert_eq!(position.debts.len(), 1);
    let atom_debt = get_debt("uatom", &position.debts);
    assert_eq!(atom_debt.amount, Uint128::new(2301));

    assert!(position.perps.is_empty());

    // Assert liquidator's new position
    let position = mock.query_positions(&liquidator_account_id);
    assert_eq!(position.deposits.len(), 2);
    assert_eq!(position.debts.len(), 0);
    let osmo_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(osmo_balance.amount, Uint128::new(2040));
    let atom_balance = get_coin("uatom", &position.deposits);
    assert_eq!(atom_balance.amount, Uint128::new(900));

    // Assert rewards-collector's new position
    let rewards_collector_acc_id = mock.query_rewards_collector_account();
    let position = mock.query_positions(&rewards_collector_acc_id);
    assert_eq!(position.deposits.len(), 1);
    assert_eq!(position.debts.len(), 0);
    let osmo_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(osmo_balance.amount, Uint128::new(20));

    // Liq HF should improve
    let health = mock.query_health(&liquidatee_account_id, ActionKind::Liquidation);
    assert!(health.liquidatable);
    assert!(
        prev_health.liquidation_health_factor.unwrap() < health.liquidation_health_factor.unwrap()
    );
}

#[test]
fn liquidation_uses_correct_price_kind_if_perps_open() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();
    let utia_info = coin_info("utia");
    let uusdc_info = uusdc_info();

    let contract_owner = Addr::unchecked("owner");

    let liquidator = Addr::unchecked("liquidator");
    let liquidatee = Addr::unchecked("liquidatee");
    let vault_depositor = Addr::unchecked("vault_depositor");

    let vault_coin_deposited = coin(100000, uusdc_info.denom.clone());
    let uosmo_coin_deposited = uosmo_info.to_coin(3000);

    let mut mock = MockEnv::new()
        .owner(contract_owner.as_str())
        .set_params(&[
            uosmo_info.clone(),
            uatom_info.clone(),
            uusdc_info.clone(),
            utia_info.clone(),
        ])
        .fund_account(AccountToFund {
            addr: liquidatee.clone(),
            funds: vec![uosmo_coin_deposited.clone()],
        })
        .fund_account(AccountToFund {
            addr: liquidator.clone(),
            funds: coins(3000, uatom_info.denom.clone()),
        })
        .fund_account(AccountToFund {
            addr: vault_depositor.clone(),
            funds: vec![vault_coin_deposited.clone()],
        })
        .build()
        .unwrap();
    let liquidatee_account_id = mock.create_credit_account(&liquidatee).unwrap();
    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();
    let vault_depositor_account_id = mock.create_credit_account(&vault_depositor).unwrap();

    // setup perps
    mock.update_perp_params(PerpParamsUpdate::AddOrUpdate {
        params: default_perp_params(&utia_info.denom),
    });

    // deposit to vault
    mock.update_credit_account(
        &vault_depositor_account_id,
        &vault_depositor,
        vec![Deposit(vault_coin_deposited.clone())],
        &[vault_coin_deposited.clone()],
    )
    .unwrap();
    mock.deposit_to_perp_vault(&vault_depositor_account_id, &vault_coin_deposited, None).unwrap();

    // setup liquidatee's position
    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![
            Deposit(uosmo_coin_deposited.clone()),
            Borrow(uatom_info.to_coin(2650)),
            ExecutePerpOrder {
                denom: utia_info.denom.clone(),
                order_size: Int128::from_str("-400").unwrap(),
                reduce_only: None,
                order_type: Some(ExecutePerpOrderType::Default),
            },
        ],
        &[uosmo_coin_deposited.clone()],
    )
    .unwrap();

    let set_price = |mock: &mut MockEnv, pricing: ActionKind| {
        mock.price_change(CoinPrice {
            pricing: pricing.clone(),
            denom: uusdc_info.denom.clone(),
            price: uusdc_info.price,
        });
        mock.price_change(CoinPrice {
            pricing: pricing.clone(),
            denom: uosmo_info.denom.clone(),
            price: uosmo_info.price,
        });
        mock.price_change(CoinPrice {
            pricing: pricing.clone(),
            denom: utia_info.denom.clone(),
            price: Decimal::from_atomics(4u128, 0).unwrap(),
        });
        mock.price_change(CoinPrice {
            pricing: pricing.clone(),
            denom: uatom_info.denom.clone(),
            price: Decimal::from_atomics(38u128, 1).unwrap(),
        });
    };

    set_price(&mut mock, ActionKind::Default);

    let health = mock.query_health(&liquidatee_account_id, ActionKind::Default);
    assert!(health.liquidatable);

    // The liquidation should fail if Default pricing is used
    mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Deposit(uatom_info.to_coin(1000)),
            Liquidate {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: uatom_info.to_coin(100),
                request: LiquidateRequest::Deposit(uosmo_info.denom.clone()),
            },
        ],
        &[uatom_info.to_coin(1000)],
    )
    .unwrap_err();

    mock.remove_price(&uusdc_info.denom, ActionKind::Default);
    mock.remove_price(&uosmo_info.denom, ActionKind::Default);
    mock.remove_price(&utia_info.denom, ActionKind::Default);
    mock.remove_price(&uatom_info.denom, ActionKind::Default);

    set_price(&mut mock, ActionKind::Liquidation);

    // Query the liquidatee's position with Default pricing should fail
    let res = mock.query_positions_with_action(&liquidatee_account_id, Some(ActionKind::Default));
    assert!(res.is_err());

    // Query the liquidatee's position with Liquidation pricing should succeed
    let position = mock
        .query_positions_with_action(&liquidatee_account_id, Some(ActionKind::Liquidation))
        .unwrap();
    let usdc_debt_before = get_debt(&uusdc_info.denom, &position.debts);

    // The liquidation should acknowledge LIQUIDATION pricing changes and go through fine
    mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Deposit(uatom_info.to_coin(1000)),
            Liquidate {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: uatom_info.to_coin(100),
                request: LiquidateRequest::Deposit(uosmo_info.denom.clone()),
            },
        ],
        &[uatom_info.to_coin(1000)],
    )
    .unwrap();

    // Query the liquidatee's position with Default pricing should succeed now because perps are closed and no need to use pricing
    let position = mock
        .query_positions_with_action(&liquidatee_account_id, Some(ActionKind::Default))
        .unwrap();
    let usdc_debt = get_debt(&uusdc_info.denom, &position.debts);
    // USDC debt should increase.
    // It means that borrowing from the Red Bank was successful with only Liquidation pricing.
    assert!(usdc_debt.amount > usdc_debt_before.amount);
}

/// Tests liquidation of a position when the liquidatee after closing perps has no debt.
/// 800 uusdc deposited to liquidatee's account.
#[test_case(
    0,
    Coin {
        denom: "uatom".to_string(),
        amount: Uint128::new(100),
    },
    LiquidateRequest::Deposit("uusdc".to_string()),
    27,
    1;
    "liquidator repays non-existent debt coin and requests deposit"
)]
#[test_case(
    0,
    Coin {
        denom: "uusdc".to_string(),
        amount: Uint128::zero(),
    },
    LiquidateRequest::Deposit("uusdc".to_string()),
    27,
    1;
    "liquidator repays debt coin with zero amount and requests deposit"
)]
#[test_case(
    800, // lend full deposit
    Coin {
        denom: "uatom".to_string(),
        amount: Uint128::new(100),
    },
    LiquidateRequest::Lend("uusdc".to_string()),
    27,
    1;
    "liquidator repays non-existent debt coin and requests lend"
)]
#[test_case(
    800, // lend full deposit
    Coin {
        denom: "uusdc".to_string(),
        amount: Uint128::zero(),
    },
    LiquidateRequest::Lend("uusdc".to_string()),
    27,
    1;
    "liquidator repays debt coin with zero amount and requests lend"
)]
#[test_case(
    0,
    Coin {
        denom: "uatom".to_string(),
        amount: Uint128::new(100),
    },
    LiquidateRequest::StakedAstroLp("uosmo".to_string()),
    10, // full staked astro lp liquidated
    4;
    "liquidator repays non-existent debt coin and requests staked astro lp"
)]
#[test_case(
    0,
    Coin {
        denom: "uusdc".to_string(),
        amount: Uint128::zero(),
    },
    LiquidateRequest::StakedAstroLp("uosmo".to_string()),
    10, // full staked astro lp liquidated
    4;
    "liquidator repays debt coin with zero amount and requests staked astro lp"
)]
fn liquidate_if_no_debt_after_closing_perps(
    lend_amt: u128,
    debt_repayed: Coin,
    collateral_requested: LiquidateRequest<VaultUnchecked>,
    expected_collateral_amount_to_liquidate: u128, // it is perps closing bonus
    expected_protocol_fee: u128,
) {
    let exptected_protocol_fee = Uint128::new(expected_protocol_fee);

    // uosmo is used as LP token
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();
    let mut uusdc_info = uusdc_info();
    uusdc_info.price = Decimal::one();

    let contract_owner = Addr::unchecked("owner");

    let liquidator = Addr::unchecked("liquidator");
    let liquidatee = Addr::unchecked("liquidatee");
    let vault_depositor = Addr::unchecked("vault_depositor");

    let uosmo_coin_deposited = uosmo_info.to_coin(10);
    let vault_coin_deposited = coin(100000, uusdc_info.denom.clone());
    let uusdc_coin_deposited = uusdc_info.to_coin(800);

    let mut mock = MockEnv::new()
        .owner(contract_owner.as_str())
        .perps_liquidation_bonus_ratio(Decimal::percent(80))
        .set_params(&[uosmo_info.clone(), uatom_info.clone(), uusdc_info.clone()])
        .fund_account(AccountToFund {
            addr: liquidatee.clone(),
            funds: vec![uosmo_coin_deposited.clone(), uusdc_coin_deposited.clone()],
        })
        .fund_account(AccountToFund {
            addr: liquidator.clone(),
            funds: coins(3000, uatom_info.denom.clone()),
        })
        .fund_account(AccountToFund {
            addr: vault_depositor.clone(),
            funds: vec![vault_coin_deposited.clone()],
        })
        .build()
        .unwrap();
    let liquidatee_account_id = mock.create_credit_account(&liquidatee).unwrap();
    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();
    let vault_depositor_account_id = mock.create_credit_account(&vault_depositor).unwrap();

    // Setup perps
    mock.update_perp_params(PerpParamsUpdate::AddOrUpdate {
        params: default_perp_params(&uatom_info.denom),
    });

    // Deposit to vault
    mock.update_credit_account(
        &vault_depositor_account_id,
        &vault_depositor,
        vec![Deposit(vault_coin_deposited.clone())],
        &[vault_coin_deposited.clone()],
    )
    .unwrap();
    mock.deposit_to_perp_vault(&vault_depositor_account_id, &vault_coin_deposited, None).unwrap();

    // Setup liquidatee's position
    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![
            Deposit(uusdc_coin_deposited.clone()),
            Lend(uusdc_info.to_action_coin(lend_amt)),
            Deposit(uosmo_coin_deposited.clone()),
            StakeAstroLp {
                lp_token: uosmo_info.to_action_coin(uosmo_coin_deposited.amount.u128()),
            },
            ExecutePerpOrder {
                denom: uatom_info.denom.clone(),
                order_size: Int128::from_str("-500").unwrap(),
                reduce_only: None,
                order_type: Some(ExecutePerpOrderType::Default),
            },
        ],
        &[uusdc_coin_deposited.clone(), uosmo_coin_deposited.clone()],
    )
    .unwrap();

    // Change the price for Default and Liquidation pricing.
    // Liquidation pricing is used during liquidation.
    // Default pricing is used before and after liquidation to validate perp positions in the test.
    mock.price_change(CoinPrice {
        pricing: ActionKind::Liquidation,
        denom: uatom_info.denom.clone(),
        price: Decimal::from_atomics(25u128, 1).unwrap(),
    });
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: uatom_info.denom.clone(),
        price: Decimal::from_atomics(25u128, 1).unwrap(),
    });

    let prev_health = mock.query_health(&liquidatee_account_id, ActionKind::Liquidation);
    assert!(prev_health.liquidatable);

    // usdc balance before liquidation
    let usdc_perps_balance_before_liq = mock.query_balance(mock.perps.address(), &uusdc_info.denom);
    let usdc_cm_balance_before_liq = mock.query_balance(&mock.rover, &uusdc_info.denom);

    // usdc position before liquidation
    let position = mock.query_positions(&liquidatee_account_id);
    let usdc_deposit_before_liq = if !position.lends.is_empty() {
        get_coin("uusdc", &position.lends)
    } else {
        get_coin("uusdc", &position.deposits)
    };

    // Perps pnl before liquidation
    let uatom_perp_position =
        mock.query_perp_position(&liquidatee_account_id, &uatom_info.denom).position.unwrap();
    let pnl_before_liq = uatom_perp_position.unrealized_pnl.to_coins(&uusdc_info.denom).pnl;
    let loss_amt = pnl_loss(pnl_before_liq);
    assert_eq!(loss_amt, Uint128::new(762));

    let liquidator_atom_deposit = 1000;
    mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Deposit(uatom_info.to_coin(liquidator_atom_deposit)),
            Liquidate {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: debt_repayed,
                request: collateral_requested.clone(),
            },
        ],
        &[uatom_info.to_coin(liquidator_atom_deposit)],
    )
    .unwrap();

    let mut collateral_amount_to_liquidate = Uint128::new(expected_collateral_amount_to_liquidate);

    // Check usdc balance after liquidation
    let usdc_perps_balance = mock.query_balance(mock.perps.address(), &uusdc_info.denom);
    let usdc_cm_balance = mock.query_balance(&mock.rover, &uusdc_info.denom);
    assert_eq!(usdc_perps_balance.amount, usdc_perps_balance_before_liq.amount + loss_amt);

    // Common assertions for requested collateral types
    let liquidatee_position = mock.query_positions(&liquidatee_account_id);
    assert!(liquidatee_position.debts.is_empty());
    assert!(liquidatee_position.perps.is_empty());
    let liquidator_position = mock.query_positions(&liquidator_account_id);
    assert!(liquidator_position.debts.is_empty());
    assert!(liquidator_position.perps.is_empty());
    assert!(liquidator_position.lends.is_empty());
    assert!(liquidator_position.staked_astro_lps.is_empty());
    let rewards_collector_acc_id = mock.query_rewards_collector_account();
    let rc_position = mock.query_positions(&rewards_collector_acc_id);
    assert!(rc_position.debts.is_empty());
    assert!(rc_position.perps.is_empty());
    assert!(rc_position.lends.is_empty());
    assert!(rc_position.staked_astro_lps.is_empty());

    match collateral_requested {
        LiquidateRequest::Deposit(_) => {
            // Assert that balance transfer between accounts is accurate post-liquidation
            assert_eq!(usdc_cm_balance.amount, usdc_cm_balance_before_liq.amount - loss_amt);

            // Assert liquidatee's new position
            assert_eq!(liquidatee_position.deposits.len(), 1);
            let usdc_balance = get_coin("uusdc", &liquidatee_position.deposits);
            assert_eq!(
                usdc_balance.amount,
                usdc_deposit_before_liq.amount - loss_amt - collateral_amount_to_liquidate
            );
            assert!(liquidatee_position.lends.is_empty());
            assert!(!liquidatee_position.staked_astro_lps.is_empty());

            // Assert liquidator's new position
            assert_eq!(liquidator_position.deposits.len(), 2);
            let usdc_balance = get_coin("uusdc", &liquidator_position.deposits);
            assert_eq!(
                usdc_balance.amount,
                collateral_amount_to_liquidate - exptected_protocol_fee
            );
            let atom_balance = get_coin("uatom", &liquidator_position.deposits);
            assert_eq!(atom_balance.amount, Uint128::new(liquidator_atom_deposit));

            // Assert rewards-collector's new position
            assert_eq!(rc_position.deposits.len(), 1);
            let usdc_balance = get_coin("uusdc", &rc_position.deposits);
            assert_eq!(usdc_balance.amount, exptected_protocol_fee);
        }
        LiquidateRequest::Lend(_) => {
            collateral_amount_to_liquidate += Uint128::one(); // +1 interest

            // Reclaimed from RB during liquidation
            assert_eq!(usdc_cm_balance.amount, collateral_amount_to_liquidate);

            // Assert liquidatee's new position
            assert!(liquidatee_position.deposits.is_empty());
            assert_eq!(liquidatee_position.lends.len(), 1);
            let usdc_balance = get_coin("uusdc", &liquidatee_position.lends);
            assert_eq!(
                usdc_balance.amount,
                usdc_deposit_before_liq.amount - loss_amt - collateral_amount_to_liquidate
            );
            assert!(!liquidatee_position.staked_astro_lps.is_empty());

            // Assert liquidator's new position
            assert_eq!(liquidator_position.deposits.len(), 2);
            let usdc_balance = get_coin("uusdc", &liquidator_position.deposits);
            assert_eq!(
                usdc_balance.amount,
                collateral_amount_to_liquidate - exptected_protocol_fee
            );
            let atom_balance = get_coin("uatom", &liquidator_position.deposits);
            assert_eq!(atom_balance.amount, Uint128::new(liquidator_atom_deposit));

            // Assert rewards-collector's new position
            assert_eq!(rc_position.deposits.len(), 1);
            let usdc_balance = get_coin("uusdc", &rc_position.deposits);
            assert_eq!(usdc_balance.amount, exptected_protocol_fee);
        }
        LiquidateRequest::StakedAstroLp(_) => {
            assert_eq!(usdc_cm_balance.amount, usdc_cm_balance_before_liq.amount - loss_amt);

            // Assert liquidatee's new position
            assert_eq!(liquidatee_position.deposits.len(), 1);
            let usdc_balance = get_coin("uusdc", &liquidatee_position.deposits);
            assert_eq!(usdc_balance.amount, usdc_deposit_before_liq.amount - loss_amt);
            assert!(liquidatee_position.lends.is_empty());
            assert!(liquidatee_position.staked_astro_lps.is_empty());

            // Assert liquidator's new position
            assert_eq!(liquidator_position.deposits.len(), 2);
            let staked_balance = get_coin("uosmo", &liquidator_position.deposits);
            assert_eq!(
                staked_balance.amount,
                collateral_amount_to_liquidate - exptected_protocol_fee
            );
            let atom_balance = get_coin("uatom", &liquidator_position.deposits);
            assert_eq!(atom_balance.amount, Uint128::new(liquidator_atom_deposit));

            // Assert rewards-collector's new position
            assert_eq!(rc_position.deposits.len(), 1);
            let usdc_balance = get_coin("uosmo", &rc_position.deposits);
            assert_eq!(usdc_balance.amount, exptected_protocol_fee);
        }
        LiquidateRequest::Vault {
            ..
        } => panic!("unexpected request"),
    }

    // Liq HF should improve
    let health = mock.query_health(&liquidatee_account_id, ActionKind::Liquidation);
    assert!(!health.liquidatable);
    assert!(
        prev_health.liquidation_health_factor.unwrap()
            < health.liquidation_health_factor.unwrap_or(Decimal::MAX)
    );
}

/// Tests liquidation of a position when the liquidatee after closing perps has bad debt.
#[test_case(
    Coin {
        denom: "uusdc".to_string(),
        amount: Uint128::zero(),
    },
    LiquidateRequest::Deposit("uusdc".to_string());
    "liquidator repays debt coin with zero amount and requests non-existent deposit"
)]
#[test_case(
    Coin {
        denom: "uusdc".to_string(),
        amount: Uint128::new(500),
    },
    LiquidateRequest::Deposit("uatom".to_string());
    "liquidator repays debt coin and requests available deposit"
)]
#[test_case(
    Coin {
        denom: "uusdc".to_string(),
        amount: Uint128::zero(),
    },
    LiquidateRequest::Lend("uusdc".to_string());
    "liquidator repays debt coin with zero amount and requests non-existent lend"
)]
#[test_case(
    Coin {
        denom: "uusdc".to_string(),
        amount: Uint128::zero(),
    },
    LiquidateRequest::StakedAstroLp("uusdc".to_string());
    "liquidator repays debt coin with zero amount and requests non-existent staked astro lp"
)]
fn liquidate_if_bad_debt_created(
    debt_requested_to_repay: Coin,
    collateral_requested: LiquidateRequest<VaultUnchecked>,
) {
    let uatom_info = uatom_info();
    let mut uusdc_info = uusdc_info();
    uusdc_info.price = Decimal::one();

    let contract_owner = Addr::unchecked("owner");

    let liquidator = Addr::unchecked("liquidator");
    let liquidatee = Addr::unchecked("liquidatee");
    let vault_depositor = Addr::unchecked("vault_depositor");

    let vault_coin_deposited = coin(100000, uusdc_info.denom.clone());
    let uusdc_coin_deposited = uusdc_info.to_coin(600);
    let uatom_coin_deposited = uatom_info.to_coin(5);

    let mut mock = MockEnv::new()
        .owner(contract_owner.as_str())
        .perps_liquidation_bonus_ratio(Decimal::percent(60))
        .set_params(&[uatom_info.clone(), uusdc_info.clone()])
        .fund_account(AccountToFund {
            addr: liquidatee.clone(),
            funds: vec![uusdc_coin_deposited.clone(), uatom_coin_deposited.clone()],
        })
        .fund_account(AccountToFund {
            addr: liquidator.clone(),
            funds: coins(3000, uusdc_info.denom.clone()),
        })
        .fund_account(AccountToFund {
            addr: vault_depositor.clone(),
            funds: vec![vault_coin_deposited.clone()],
        })
        .build()
        .unwrap();
    let liquidatee_account_id = mock.create_credit_account(&liquidatee).unwrap();
    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();
    let vault_depositor_account_id = mock.create_credit_account(&vault_depositor).unwrap();

    // Setup perps
    mock.update_perp_params(PerpParamsUpdate::AddOrUpdate {
        params: default_perp_params(&uatom_info.denom),
    });

    // Deposit to vault
    mock.update_credit_account(
        &vault_depositor_account_id,
        &vault_depositor,
        vec![Deposit(vault_coin_deposited.clone())],
        &[vault_coin_deposited.clone()],
    )
    .unwrap();
    mock.deposit_to_perp_vault(&vault_depositor_account_id, &vault_coin_deposited, None).unwrap();

    // Setup liquidatee's position
    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![
            Deposit(uusdc_coin_deposited.clone()),
            ExecutePerpOrder {
                denom: uatom_info.denom.clone(),
                order_size: Int128::from_str("-400").unwrap(),
                reduce_only: None,
                order_type: Some(ExecutePerpOrderType::Default),
            },
            Deposit(uatom_coin_deposited.clone()),
        ],
        &[uusdc_coin_deposited.clone(), uatom_coin_deposited.clone()],
    )
    .unwrap();

    // Change the price for Default and Liquidation pricing.
    // Liquidation pricing is used during liquidation.
    // Default pricing is used before and after liquidation to validate perp positions in the test.
    mock.price_change(CoinPrice {
        pricing: ActionKind::Liquidation,
        denom: uatom_info.denom.clone(),
        price: Decimal::from_atomics(25u128, 1).unwrap(),
    });
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: uatom_info.denom.clone(),
        price: Decimal::from_atomics(25u128, 1).unwrap(),
    });

    let prev_health = mock.query_health(&liquidatee_account_id, ActionKind::Liquidation);
    assert_eq!(
        prev_health.liquidation_health_factor.unwrap().to_string(),
        "0.896209386281588447".to_string()
    );

    // usdc balance before liquidation
    let usdc_perps_balance_before_liq = mock.query_balance(mock.perps.address(), &uusdc_info.denom);

    // usdc position before liquidation
    let position = mock.query_positions(&liquidatee_account_id);
    let usdc_deposit_before_liq = get_coin("uusdc", &position.deposits);

    // Perps pnl before liquidation
    let uatom_perp_position =
        mock.query_perp_position(&liquidatee_account_id, &uatom_info.denom).position.unwrap();
    let pnl_before_liq = uatom_perp_position.unrealized_pnl.to_coins(&uusdc_info.denom).pnl;
    let loss_amt = pnl_loss(pnl_before_liq);
    assert_eq!(loss_amt, Uint128::new(609));
    assert!(loss_amt > usdc_deposit_before_liq.amount);

    // Check liquidator's position before liquidation
    let position = mock.query_positions(&liquidator_account_id);
    assert!(position.deposits.is_empty());
    assert!(position.lends.is_empty());
    assert!(position.staked_astro_lps.is_empty());
    assert!(position.debts.is_empty());

    let liquidator_usdc_deposit = Uint128::new(1000);
    mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Deposit(uusdc_info.to_coin(liquidator_usdc_deposit.u128())),
            Liquidate {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: debt_requested_to_repay.clone(),
                request: collateral_requested.clone(),
            },
        ],
        &[uusdc_info.to_coin(liquidator_usdc_deposit.u128())],
    )
    .unwrap();

    // Check usdc balance after liquidation
    let usdc_perps_balance = mock.query_balance(mock.perps.address(), &uusdc_info.denom);
    let usdc_cm_balance = mock.query_balance(&mock.rover, &uusdc_info.denom);
    assert_eq!(usdc_perps_balance.amount, usdc_perps_balance_before_liq.amount + loss_amt);
    let expected_usdc_debt_repayed = if debt_requested_to_repay.amount.is_zero() {
        // // No debt repaid, no collateral liquidated
        Uint128::zero()
    } else {
        // Collateral value to liquidate: floor(5 uatom * price of 2.5) = 12
        // After applying the liquidation bonus and debt price, final value ≈ 11
        Uint128::new(11)
    };
    assert_eq!(usdc_cm_balance.amount, liquidator_usdc_deposit - expected_usdc_debt_repayed);

    // Assert liquidatee's new position.
    // Should have a bad debt.
    let position = mock.query_positions(&liquidatee_account_id);
    assert_eq!(position.debts.len(), 1);
    let usdc_debt = get_debt("uusdc", &position.debts);
    assert!(usdc_deposit_before_liq.amount < loss_amt);
    let exptected_debt_from_perps = usdc_deposit_before_liq.amount.abs_diff(loss_amt);
    assert_eq!(
        usdc_debt.amount,
        exptected_debt_from_perps - expected_usdc_debt_repayed + Uint128::one()
    ); // +1 for interest rate
    if debt_requested_to_repay.amount.is_zero() {
        assert_eq!(position.deposits.len(), 1);
        let atom_balance = get_coin("uatom", &position.deposits);
        assert_eq!(atom_balance.amount, uatom_coin_deposited.amount);
    } else {
        // Atom is liquidated
        assert!(position.deposits.is_empty());
    }
    assert!(position.lends.is_empty());
    assert!(position.staked_astro_lps.is_empty());
    assert!(position.perps.is_empty());

    // Assert liquidator's new position
    let position = mock.query_positions(&liquidator_account_id);
    assert!(position.debts.is_empty());
    if debt_requested_to_repay.amount.is_zero() {
        assert_eq!(position.deposits.len(), 1);
    } else {
        assert_eq!(position.deposits.len(), 2);
        let atom_balance = get_coin("uatom", &position.deposits);
        assert_eq!(atom_balance.amount, uatom_coin_deposited.amount);
    }
    let usdc_balance = get_coin("uusdc", &position.deposits);
    assert_eq!(usdc_balance.amount, liquidator_usdc_deposit - expected_usdc_debt_repayed);
    assert!(position.lends.is_empty());
    assert!(position.staked_astro_lps.is_empty());

    // Assert rewards-collector's new position.
    // No rewards should be collected.
    let rewards_collector_acc_id = mock.query_rewards_collector_account();
    let position = mock.query_positions(&rewards_collector_acc_id);
    assert!(position.debts.is_empty());
    assert!(position.deposits.is_empty());
    assert!(position.lends.is_empty());
    assert!(position.staked_astro_lps.is_empty());

    // Account is bancrupt
    let health = mock.query_health(&liquidatee_account_id, ActionKind::Liquidation);
    assert!(health.liquidatable);
    assert!(
        prev_health.liquidation_health_factor.unwrap()
            > health.liquidation_health_factor.unwrap_or(Decimal::MAX)
    );
}

fn pnl_profit(pnl: PnL) -> Uint128 {
    match pnl {
        PnL::Profit(coin) => coin.amount,
        _ => panic!("expected profit"),
    }
}

fn pnl_loss(pnl: PnL) -> Uint128 {
    match pnl {
        PnL::Loss(coin) => coin.amount,
        _ => panic!("expected loss"),
    }
}
