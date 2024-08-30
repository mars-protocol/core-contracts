use std::str::FromStr;

use cosmwasm_std::{coin, coins, Addr, Decimal, Uint128};
use mars_mock_oracle::msg::CoinPrice;
use mars_types::{
    credit_manager::{
        Action::{Borrow, Deposit, ExecutePerpOrder, Liquidate},
        LiquidateRequest,
    },
    oracle::ActionKind,
    params::PerpParamsUpdate,
    perps::{PnL, PnlAmounts},
    signed_uint::SignedUint,
};

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
    mock.deposit_to_perp_vault(&vault_depositor_account_id, &vault_coin_deposited).unwrap();

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
                order_size: SignedUint::from_str("200").unwrap(),
                reduce_only: None,
            },
            ExecutePerpOrder {
                denom: uatom_info.denom.clone(),
                order_size: SignedUint::from_str("-400").unwrap(),
                reduce_only: None,
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
    pnl_amounts_acc.add(&uosmo_perp_position.unrealised_pnl).unwrap();
    pnl_amounts_acc.add(&uatom_perp_position.unrealised_pnl).unwrap();
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
    assert_eq!(usdc_balance.amount, Uint128::new(21)); // initial usdc deposit - perps opening fees - perps loss
    assert_eq!(usdc_balance.amount, usdc_deposit_before_liq.amount - loss_amt);
    let osmo_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(osmo_balance.amount, Uint128::new(1944));
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
    assert_eq!(osmo_balance.amount, Uint128::new(1052));
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
    mock.deposit_to_perp_vault(&vault_depositor_account_id, &vault_coin_deposited).unwrap();

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
                order_size: SignedUint::from_str("200").unwrap(),
                reduce_only: None,
            },
            ExecutePerpOrder {
                denom: uatom_info.denom.clone(),
                order_size: SignedUint::from_str("-400").unwrap(),
                reduce_only: None,
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
    pnl_amounts_acc.add(&uosmo_perp_position.unrealised_pnl).unwrap();
    pnl_amounts_acc.add(&uatom_perp_position.unrealised_pnl).unwrap();
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
    assert_eq!(usdc_debt.amount, Uint128::new(130));
    let atom_debt = get_debt("uatom", &position.debts);
    assert_eq!(atom_debt.amount, Uint128::new(2301));

    assert_eq!(position.deposits.len(), 2);
    let osmo_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(osmo_balance.amount, Uint128::new(1892));
    let atom_balance = get_coin("uatom", &position.deposits);
    assert_eq!(atom_balance.amount, Uint128::new(2400));

    assert!(position.perps.is_empty());

    // Assert liquidator's new position
    let position = mock.query_positions(&liquidator_account_id);
    assert_eq!(position.deposits.len(), 2);
    assert_eq!(position.debts.len(), 0);
    let osmo_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(osmo_balance.amount, Uint128::new(1104));
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
    mock.deposit_to_perp_vault(&vault_depositor_account_id, &vault_coin_deposited).unwrap();

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
                order_size: SignedUint::from_str("200").unwrap(),
                reduce_only: None,
            },
            ExecutePerpOrder {
                denom: utia_info.denom.clone(),
                order_size: SignedUint::from_str("-400").unwrap(),
                reduce_only: None,
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
    pnl_amounts_acc.add(&uosmo_perp_position.unrealised_pnl).unwrap();
    pnl_amounts_acc.add(&utia_perp_position.unrealised_pnl).unwrap();
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
    assert_eq!(usdc_balance.amount, Uint128::new(93)); // initial usdc deposit - perps opening fees + perps profit
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
    mock.deposit_to_perp_vault(&vault_depositor_account_id, &vault_coin_deposited).unwrap();

    // setup liquidatee's position
    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![
            Deposit(uosmo_coin_deposited.clone()),
            Borrow(uatom_info.to_coin(2650)),
            ExecutePerpOrder {
                denom: utia_info.denom.clone(),
                order_size: SignedUint::from_str("-400").unwrap(),
                reduce_only: None,
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
