use std::str::FromStr;

use anyhow::Error;
use cosmwasm_std::{testing::mock_dependencies, Addr, Coin, Decimal, Int128, Uint128};
use cw_multi_test::AppResponse;
use mars_credit_manager::{
    error::ContractError,
    state::{EXECUTED_TRIGGER_ORDERS, NEXT_TRIGGER_ID},
    trigger::check_order_relations_and_set_parent_id,
};
use mars_mock_oracle::msg::CoinPrice;
use mars_testing::multitest::helpers::{
    coin_info, default_perp_params, uatom_info, uosmo_info, MockEnv,
};
use mars_types::{
    credit_manager::{
        Action,
        Action::{
            Borrow, CreateTriggerOrder, DeleteTriggerOrder, Deposit, ExecutePerpOrder, Lend,
            Liquidate, Repay, Withdraw,
        },
        ActionAmount, ActionCoin, Comparison,
        Condition::{OraclePrice, TriggerOrderExecuted},
        CreateTriggerOrderType, ExecutePerpOrderType, LiquidateRequest, TriggerOrder,
        TriggerOrderResponse,
    },
    oracle::ActionKind,
    params::PerpParamsUpdate,
};
use test_case::test_case;

#[test_case(
    &mut [],
    None;
    "No actions"
)]
#[test_case(
    &mut [
        parent_order(),
        child_order_no_parent_id(),
        child_order_no_parent_id(),
        child_order_no_parent_id(),
        child_order_no_parent_id()
    ],
    None;
    "Parent with multiple child orders"
)]
#[test_case(
    &mut [
        parent_order(),
        child_order_no_parent_id(),
        child_order_no_parent_id(),
        child_order_with_parent_id(1),
        child_order_with_parent_id(2),
    ],
    Some(ContractError::InvalidOrderConditions { reason: "Child order cannot provide a trigger_order_id in TriggerOrderExecuted conditions when earlier action contains a parent.".to_string()});
    "Mixing child orders with and without parent id"
)]
#[test_case(
    &mut [
        parent_order(),
    ],
    Some(ContractError::NoChildOrdersFound);
    "Parent order without child order"
)]
#[test_case(
    &mut [
        parent_market_order(),
    ],
    Some(ContractError::NoChildOrdersFound);
    "Parent market order without child order"
)]
#[test_case(
    &mut [
        default_market_order(),
        parent_market_order(),
        child_order_no_parent_id(),
    ],
    None;
    "Market order before parent order"
)]
#[test_case(
    &mut [
        child_order_no_parent_id(),
    ],
    Some(ContractError::InvalidOrderConditions { reason: "No trigger_order_id in TriggerOrderExecuted conditions.".to_string() });
    "No parent"
)]
#[test_case(
    &mut [
        default_order(),
        parent_market_order(),
        child_order_no_parent_id(),
    ],
    Some(ContractError::InvalidParentOrderPosition);
    "Adding parent order after default order"
)]
#[test_case(
    &mut [
        parent_order_invalid_conditions()
    ],
    Some(ContractError::InvalidOrderConditions { reason: "Parent orders cannot contain a TriggerOrderExecuted condition".to_string()});
    "Invalid conditions. No TriggerOrderExecuted allowed for parent order"
)]
#[test_case(
    &mut [
        default_order_invalid_conditions(),
    ],
    Some(ContractError::InvalidOrderConditions { reason: "Default orders cannot contain a TriggerOrderExecuted condition".to_string()});
    "Invalid conditions. No TriggerOrderExecuted allowed for default order"
)]
#[test_case(
    &mut [
        parent_order_parent_actions(),
        child_order_no_parent_id(),
    ],
    None;
    "Parent order in actions ignored"
)]
#[test_case(
    &mut [
        child_order_invalid_single_condition(),
    ],
    Some(ContractError::InvalidOrderConditions { reason: "Child order needs at least 1 other condition next to TriggerOrderExecuted".to_string() });
    "Invalid child order conditions. Needs 2+ conditions"
)]
#[test_case(
    &mut [
        child_order_invalid_double_trigger_condition(),
    ],
    Some(ContractError::InvalidOrderConditions { reason: "Child order needs exactly 1 TriggerOrderExecuted condition".to_string() });
    "Invalid child order conditions. Can not have 2 TriggerOrderExecuted"
)]
#[test_case(
    &mut [
        parent_market_order(),
        child_order_with_parent_id(2),
    ],
    Some(ContractError::InvalidOrderConditions { reason: "Child order cannot provide a trigger_order_id in TriggerOrderExecuted conditions when earlier action contains a parent.".to_string()});
    "Invalid child order conditions. No parent_order_id allowed when parent_order is provided in transaction"
)]
#[test_case(
    &mut [
        default_order_invalid_conditions(),
    ],
    Some(ContractError::InvalidOrderConditions { reason: "Default orders cannot contain a TriggerOrderExecuted condition".to_string()});
    "Invalid order conditions. Parent/default orders cannot contain a TriggerOrderExecuted condition"
)]
#[test_case(
    &mut [
        parent_order(),
        parent_order(),
    ],
    Some(ContractError::InvalidParentOrderPosition);
    "Multiple parent orders"
)]
#[test_case(
    &mut [
        parent_order(),
        child_order_with_parent_id(1),
    ],
    Some(ContractError::InvalidOrderConditions { reason: "Child order cannot provide a trigger_order_id in TriggerOrderExecuted conditions when earlier action contains a parent.".to_string()});
    "No child orders for parent"
)]
#[test_case(
    &mut [
        default_order(),
    ],
    None;
    "No parent or child orders"
)]
#[test_case(
    &mut [
        default_market_order(),
    ],
    None;
    "Market order: No parent or child orders"
)]
#[test_case(
    &mut vec![
        Deposit (
            Coin {
                denom: "ubtc".to_string(),
                amount: Uint128::new(1000000),
            },
        ),
        Borrow (
            Coin {
                denom: "ubtc".to_string(),
                amount: Uint128::new(1000000),
            },
        ),
        Withdraw (
            ActionCoin {
                denom: "ubtc".to_string(),
                amount: ActionAmount::AccountBalance,
            },
        ),
        Repay {
            recipient_account_id: None,
            coin: ActionCoin {
                denom: "ubtc".to_string(),
                amount: ActionAmount::AccountBalance,
            },
        },
        Liquidate {
            debt_coin: Coin {
                denom: "ubtc".to_string(),
                amount: Uint128::new(1000000),
            },
            liquidatee_account_id: "1".to_string(),
            request: LiquidateRequest::Deposit("ubtc".to_string()),
        },
        Lend (
            ActionCoin {
                denom: "ubtc".to_string(),
                amount: ActionAmount::AccountBalance,
            },
        ),
        ExecutePerpOrder {
            denom: "ubtc".to_string(),
            order_size: Int128::from_str("1").unwrap(),
            reduce_only: None,
            order_type: Some(ExecutePerpOrderType::Default),
        },
    ],
    None;
    "Non-trigger order actions"
)]
fn order_relations(actions: &mut [Action], maybe_expected_error: Option<ContractError>) {
    let mut deps = mock_dependencies();

    NEXT_TRIGGER_ID.save(&mut deps.storage, &0).unwrap();

    let result = check_order_relations_and_set_parent_id(deps.as_mut().storage, "1", actions);

    if maybe_expected_error.is_some() {
        assert_eq!(result.unwrap_err(), maybe_expected_error.unwrap());
    } else {
        assert!(result.is_ok())
    }
}

#[test]
fn set_parent_id() {
    let mut deps = mock_dependencies();

    let order_id = 0;

    NEXT_TRIGGER_ID.save(&mut deps.storage, &order_id).unwrap();

    let actions = &mut vec![parent_order(), child_order_no_parent_id(), child_order_no_parent_id()];

    let result = check_order_relations_and_set_parent_id(deps.as_mut().storage, "1", actions);

    assert!(result.is_ok());

    let next_trigger_id = NEXT_TRIGGER_ID.load(&deps.storage).unwrap();
    let executed_trigger_order =
        EXECUTED_TRIGGER_ORDERS.may_load(&deps.storage, ("1", &order_id.to_string())).unwrap();

    assert_eq!(next_trigger_id, order_id);
    assert_eq!(actions[1], child_order_with_parent_id(order_id));
    assert_eq!(actions[2], child_order_with_parent_id(order_id));
    assert_eq!(executed_trigger_order, None);
}

#[test]
fn set_parent_id_market_order() {
    let mut deps = mock_dependencies();

    let order_id = 0;

    NEXT_TRIGGER_ID.save(&mut deps.storage, &order_id).unwrap();

    let actions =
        &mut vec![parent_market_order(), child_order_no_parent_id(), child_order_no_parent_id()];

    let result = check_order_relations_and_set_parent_id(deps.as_mut().storage, "1", actions);

    assert!(result.is_ok());

    let next_trigger_id = NEXT_TRIGGER_ID.load(&deps.storage).unwrap();
    let executed_trigger_order =
        EXECUTED_TRIGGER_ORDERS.load(&deps.storage, ("1", &order_id.to_string())).unwrap();

    assert_eq!(next_trigger_id, order_id + 1);
    assert_eq!(actions[1], child_order_with_parent_id(order_id));
    assert_eq!(actions[2], child_order_with_parent_id(order_id));
    assert_eq!(executed_trigger_order, order_id.to_string())
}

#[test]
fn error_when_parent_does_not_exist() {
    let mut deps = mock_dependencies();

    let order_id = 99;

    NEXT_TRIGGER_ID.save(&mut deps.storage, &order_id).unwrap();

    let actions = &mut [child_order_with_parent_id(order_id)];

    let result = check_order_relations_and_set_parent_id(deps.as_mut().storage, "1", actions);

    assert!(result.is_err());

    assert_eq!(
        result.unwrap_err(),
        ContractError::TriggerOrderNotFound {
            order_id: order_id.to_string(),
            account_id: "1".to_string(),
        }
    );
}

/// Test scenario summary:
/// 1. Create TriggerOrder with 2 child orders (TP + SL)
/// 2. Drop price to SL, without parent being executed. No orders should be executed.
/// 3. Increase price to execute parent
/// 4. Increase price to execute TP. No orders should remain.
#[test]
fn parent_and_child_orders() {
    let atom_info = uatom_info();
    let usdc_info = coin_info("uusdc");

    // Addresses
    let contract_owner = Addr::unchecked("owner");
    let cm_user = Addr::unchecked("user");
    let keeper_bot = Addr::unchecked("keeper");
    let vault_depositor = Addr::unchecked("vault_depositor");

    // Funds given to each address
    let usdc_coin = usdc_info.to_coin(100000000000);

    // Create mock env
    let mut mock = MockEnv::new()
        .owner(contract_owner.as_str())
        .set_params(&[atom_info.clone(), usdc_info.clone()])
        .fund_accounts(vec![cm_user.clone(), vault_depositor.clone()], vec![usdc_coin.clone()])
        .build()
        .unwrap();

    mock.update_perp_params(PerpParamsUpdate::AddOrUpdate {
        params: default_perp_params(&atom_info.denom),
    });

    let account_id = mock.create_credit_account(&cm_user).unwrap();
    let account_vault_depositor_id = mock.create_credit_account(&cm_user).unwrap();

    mock.update_credit_account(
        &account_vault_depositor_id,
        &cm_user,
        vec![
            Deposit(usdc_info.to_coin(1_000_000_000)),
            Action::DepositToPerpVault {
                coin: ActionCoin {
                    denom: usdc_info.denom.clone(),
                    amount: ActionAmount::AccountBalance,
                },
                max_receivable_shares: None,
            },
        ],
        &[usdc_info.to_coin(1_000_000_000)],
    )
    .unwrap();

    // Parent order
    let create_parent = create_parent_order(
        "uatom",
        Int128::from_str("10").unwrap(),
        Decimal::from_str("100").unwrap(),
        Comparison::GreaterThan,
    );

    // TP order that executes when price = 120
    let create_tp = create_child_order(
        "uatom",
        Int128::from_str("-10").unwrap(),
        Some(true),
        Decimal::from_str("120").unwrap(),
        Comparison::GreaterThan,
        "",
    );

    // SL order that executes when price = 80
    let create_sl = create_child_order(
        "uatom",
        Int128::from_str("-10").unwrap(),
        Some(true),
        Decimal::from_str("80").unwrap(),
        Comparison::LessThan,
        "",
    );

    let parent = trigger_order_response(&account_id, "1", create_parent.clone()).unwrap();
    let tp = child_trigger_order_response(&account_id, "1", "2", create_tp.clone()).unwrap();
    let sl = child_trigger_order_response(&account_id, "1", "3", create_sl.clone()).unwrap();

    ///////////////////////////
    // 1. Create trigger orders
    ///////////////////////////
    mock.update_credit_account(
        &account_id,
        &cm_user,
        vec![
            Deposit(usdc_info.to_coin(1_000_000_000)),
            create_parent.clone(),
            create_tp.clone(),
            create_sl.clone(),
        ],
        &[usdc_info.to_coin(1_000_000_000)],
    )
    .unwrap();

    let orders = mock.query_trigger_orders_for_account(account_id.clone(), None, None);

    // Check if the orders are created correctly, and the ids are assigned
    assert_eq!(orders.data, vec![parent.clone(), tp.clone(), sl.clone()]);

    ////////////////////////////////////////////////
    // 2. Drop price to SL, without parent executed
    ////////////////////////////////////////////////

    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: "uatom".to_string(),
        price: Decimal::from_str("70").unwrap(),
    });

    let orders = mock.query_trigger_orders_for_account(account_id.clone(), None, None);

    // All orders should remain
    assert_eq!(orders.data, vec![parent.clone(), tp.clone(), sl.clone(),]);

    ///////////////////////////
    // 3. Execute parent order
    ///////////////////////////

    // Increase price to 101 (parent becomes valid)
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: "uatom".to_string(),
        price: Decimal::from_str("101").unwrap(),
    });

    // Trigger execution of the parent order
    mock.execute_trigger_order(&keeper_bot, &account_id, &parent.order.order_id.to_string())
        .unwrap();

    // Check orders
    let orders = mock.query_trigger_orders_for_account(account_id.clone(), None, None);
    assert_eq!(orders.data, vec![tp.clone(), sl.clone(),]);

    // Check position
    let perp_position = mock.query_perp_position(&account_id, "uatom");
    assert_eq!(perp_position.position.unwrap().size, Int128::from_str("10").unwrap());

    //////////////////
    // 4. Execute TP
    //////////////////

    // Increase price to 121 (TP1 becomes valid)
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: "uatom".to_string(),
        price: Decimal::from_str("121").unwrap(),
    });

    // Execute TP1
    mock.execute_trigger_order(&keeper_bot, &account_id, &tp.order.order_id.to_string()).unwrap();

    // Check orders
    let orders = mock.query_trigger_orders_for_account(account_id.clone(), None, None);
    assert_eq!(orders.data, vec![]);

    // Check position size
    let perp_position = mock.query_perp_position(&account_id, "uatom");
    assert_eq!(perp_position.position, None);
}

/// Test scenario summary:
/// 1. Execute market order (ExecutePerpOrder) with 2 child orders (TP + SL)
/// 2. Drop price and trigger SL
#[test]
fn market_order_with_child_orders() {
    let atom_info = uatom_info();
    let usdc_info = coin_info("uusdc");

    // Addresses
    let contract_owner = Addr::unchecked("owner");
    let cm_user = Addr::unchecked("user");
    let keeper_bot = Addr::unchecked("keeper");
    let vault_depositor = Addr::unchecked("vault_depositor");

    // Funds given to each address
    let usdc_coin = usdc_info.to_coin(100000000000);

    // Create mock env
    let mut mock = MockEnv::new()
        .owner(contract_owner.as_str())
        .set_params(&[atom_info.clone(), usdc_info.clone()])
        .fund_accounts(vec![cm_user.clone(), vault_depositor.clone()], vec![usdc_coin.clone()])
        .build()
        .unwrap();

    mock.update_perp_params(PerpParamsUpdate::AddOrUpdate {
        params: default_perp_params(&atom_info.denom),
    });

    let account_id = mock.create_credit_account(&cm_user).unwrap();
    let account_vault_depositor_id = mock.create_credit_account(&cm_user).unwrap();

    mock.update_credit_account(
        &account_vault_depositor_id,
        &cm_user,
        vec![
            Deposit(usdc_info.to_coin(1_000_000_000)),
            Action::DepositToPerpVault {
                coin: ActionCoin {
                    denom: usdc_info.denom.clone(),
                    amount: ActionAmount::AccountBalance,
                },
                max_receivable_shares: None,
            },
        ],
        &[usdc_info.to_coin(1_000_000_000)],
    )
    .unwrap();

    // TP order that executes when price = 120
    let create_tp = create_child_order(
        "uatom",
        Int128::from_str("-10").unwrap(),
        Some(true),
        Decimal::from_str("120").unwrap(),
        Comparison::GreaterThan,
        "",
    );

    // SL order that executes when price = 80
    let create_sl = create_child_order(
        "uatom",
        Int128::from_str("-10").unwrap(),
        Some(true),
        Decimal::from_str("80").unwrap(),
        Comparison::LessThan,
        "",
    );

    let tp = child_trigger_order_response(&account_id, "1", "2", create_tp.clone()).unwrap();
    let sl = child_trigger_order_response(&account_id, "1", "3", create_sl.clone()).unwrap();

    //////////////////////////
    // 1. Execute market order
    //////////////////////////
    mock.update_credit_account(
        &account_id,
        &cm_user,
        vec![
            Deposit(usdc_info.to_coin(1_000_000_000)),
            ExecutePerpOrder {
                denom: "uatom".to_string(),
                order_type: Some(ExecutePerpOrderType::Parent),
                order_size: Int128::from_str("10").unwrap(),
                reduce_only: None,
            },
            create_tp.clone(),
            create_sl.clone(),
        ],
        &[usdc_info.to_coin(1_000_000_000)],
    )
    .unwrap();

    let orders = mock.query_trigger_orders_for_account(account_id.clone(), None, None);

    // Check if the order is created correctly, and the ids are assigned
    assert_eq!(orders.data, vec![tp.clone(), sl.clone(),]);

    //////////////////
    // 2. Execute SL
    //////////////////

    // Drop price below SL trigger price
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: "uatom".to_string(),
        price: Decimal::from_str("78").unwrap(),
    });

    // Execute SL
    mock.execute_trigger_order(&keeper_bot, &account_id, &sl.order.order_id.to_string()).unwrap();

    // Check orders
    let orders = mock.query_trigger_orders_for_account(account_id.clone(), None, None);
    assert_eq!(orders.data, vec![]);

    // Check position size
    let perp_position = mock.query_perp_position(&account_id, "uatom");
    assert_eq!(perp_position.position, None);
}

/// Test scenario summary:
/// 1. Create TriggerOrder
/// 2. Add a child TriggerOrder (SL)
/// 3. Update child (remove + create)
/// 4. Increase price to execute parent
/// 5. Try to add another child order (should fail, as parent is executed)
#[test]
fn limit_order_changing_child_orders() {
    let atom_info = uatom_info();
    let usdc_info = coin_info("uusdc");

    // Addresses
    let contract_owner = Addr::unchecked("owner");
    let cm_user = Addr::unchecked("user");
    let keeper_bot = Addr::unchecked("keeper");
    let vault_depositor = Addr::unchecked("vault_depositor");

    // Funds given to each address
    let usdc_coin = usdc_info.to_coin(100000000000);

    // Create mock env
    let mut mock = MockEnv::new()
        .owner(contract_owner.as_str())
        .set_params(&[atom_info.clone(), usdc_info.clone()])
        .fund_accounts(vec![cm_user.clone(), vault_depositor.clone()], vec![usdc_coin.clone()])
        .build()
        .unwrap();

    mock.update_perp_params(PerpParamsUpdate::AddOrUpdate {
        params: default_perp_params(&atom_info.denom),
    });

    let account_id = mock.create_credit_account(&cm_user).unwrap();
    let account_vault_depositor_id = mock.create_credit_account(&cm_user).unwrap();

    mock.update_credit_account(
        &account_vault_depositor_id,
        &cm_user,
        vec![
            Deposit(usdc_info.to_coin(1_000_000_000)),
            Action::DepositToPerpVault {
                coin: ActionCoin {
                    denom: usdc_info.denom.clone(),
                    amount: ActionAmount::AccountBalance,
                },
                max_receivable_shares: None,
            },
        ],
        &[usdc_info.to_coin(1_000_000_000)],
    )
    .unwrap();

    /////////////////////////////////////////
    // 1. Create normal limit trigger order
    /////////////////////////////////////////

    let create_order = create_trigger_order(
        "uatom",
        Int128::from_str("10").unwrap(),
        Decimal::from_str("100").unwrap(),
        Comparison::GreaterThan,
    );

    let order = trigger_order_response(&account_id, "1", create_order.clone()).unwrap();

    mock.update_credit_account(
        &account_id,
        &cm_user,
        vec![Deposit(usdc_info.to_coin(1_000_000_000)), create_order.clone()],
        &[usdc_info.to_coin(1_000_000_000)],
    )
    .unwrap();

    let orders = mock.query_trigger_orders_for_account(account_id.clone(), None, None);

    // Check if the orders are created correctly, and the ids are assigned
    assert_eq!(orders.data, vec![order.clone(),]);

    ////////////////////////////////////////////////
    // 2. Add a take profit child order
    ////////////////////////////////////////////////

    let create_tp = create_child_order(
        "uatom",
        Int128::from_str("-10").unwrap(),
        Some(true),
        Decimal::from_str("120").unwrap(),
        Comparison::GreaterThan,
        "1",
    );

    let tp = child_trigger_order_response(&account_id, "1", "2", create_tp.clone()).unwrap();

    mock.update_credit_account(&account_id, &cm_user, vec![create_tp.clone()], &[]).unwrap();

    let orders = mock.query_trigger_orders_for_account(account_id.clone(), None, None);

    // Check if the orders are created correctly, and the ids are assigned
    assert_eq!(orders.data, vec![order.clone(), tp.clone()]);

    ////////////////////////////////////////////////
    // 3. Update child order
    ////////////////////////////////////////////////

    let create_tp2 = create_child_order(
        "uatom",
        Int128::from_str("-10").unwrap(),
        Some(true),
        Decimal::from_str("110").unwrap(),
        Comparison::GreaterThan,
        "1",
    );

    let tp2 = child_trigger_order_response(&account_id, "1", "3", create_tp2.clone()).unwrap();

    mock.update_credit_account(
        &account_id,
        &cm_user,
        vec![
            DeleteTriggerOrder {
                trigger_order_id: tp.order.order_id,
            },
            create_tp2.clone(),
        ],
        &[],
    )
    .unwrap();

    let orders = mock.query_trigger_orders_for_account(account_id.clone(), None, None);

    // Check if the orders are created correctly, and the ids are assigned
    assert_eq!(orders.data, vec![order.clone(), tp2.clone()]);

    ////////////////////////////////////////////////
    // 4. Execute parent order
    ////////////////////////////////////////////////

    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: "uatom".to_string(),
        price: Decimal::from_str("101").unwrap(),
    });

    mock.execute_trigger_order(&keeper_bot, &account_id, &order.order.order_id.to_string())
        .unwrap();

    // Check orders
    let orders = mock.query_trigger_orders_for_account(account_id.clone(), None, None);
    assert_eq!(orders.data, vec![tp2.clone()]);

    // Check position size
    let perp_position = mock.query_perp_position(&account_id, "uatom");
    assert_eq!(perp_position.position.unwrap().size, Int128::from_str("10").unwrap());

    ////////////////////////////////////////////////
    // 4. Try (and fail) to create another child
    ////////////////////////////////////////////////

    let create_sl = create_child_order(
        "uatom",
        Int128::from_str("-10").unwrap(),
        Some(true),
        Decimal::from_str("90").unwrap(),
        Comparison::GreaterThan,
        "1",
    );

    let res = mock.update_credit_account(&account_id, &cm_user, vec![create_sl.clone()], &[]);

    check_result_for_expected_error(
        res,
        Some(ContractError::TriggerOrderNotFound {
            order_id: 1.to_string(),
            account_id: 2.to_string(),
        }),
    );
}

/// Test summary:
/// 0. Create and activate some trigger orders for another account for OSMO and ATOM
/// 1. Create 2 OSMO limit orders, each with 2 child orders
/// 2. Create 1 ATOM (short) limit order with 2 child orders
/// 3. Initiate OSMO position with market order
/// 4. Close OSMO position with market order. All trigger orders should remain
/// 5. Execute ATOM limit order (decrease price). Child orders become active
/// 6. Execute both OSMO orders. All 4 child orders become active
/// 7. Execute market order to flip BTC position. All OSMO trigger orders should be removed, ATOM remains.
/// 8. Verify that the other account has all trigger_orders intact.
#[test]
fn multiple_limit_orders() {
    let atom_info = uatom_info();
    let osmo_info = uosmo_info();
    let usdc_info = coin_info("uusdc");

    // Addresses
    let contract_owner = Addr::unchecked("owner");
    let user1 = Addr::unchecked("user1");
    let user2 = Addr::unchecked("user2");
    let keeper_bot = Addr::unchecked("keeper");
    let vault_depositor = Addr::unchecked("vault_depositor");

    // Funds given to each address
    let usdc_coin = usdc_info.to_coin(100000000000);

    // Create mock env
    let mut mock = MockEnv::new()
        .owner(contract_owner.as_str())
        .set_params(&[atom_info.clone(), usdc_info.clone(), osmo_info.clone()])
        .fund_accounts(
            vec![user1.clone(), user2.clone(), vault_depositor.clone()],
            vec![usdc_coin.clone()],
        )
        .build()
        .unwrap();

    // Setup both perp markets
    mock.update_perp_params(PerpParamsUpdate::AddOrUpdate {
        params: default_perp_params(&atom_info.denom),
    });

    mock.update_perp_params(PerpParamsUpdate::AddOrUpdate {
        params: default_perp_params(&osmo_info.denom),
    });

    let account_id_1 = mock.create_credit_account(&user1).unwrap();
    let account_id_2 = mock.create_credit_account(&user2).unwrap();

    let account_vault_depositor_id = mock.create_credit_account(&user1).unwrap();

    mock.update_credit_account(
        &account_vault_depositor_id,
        &user1,
        vec![
            Deposit(usdc_info.to_coin(1_000_000_000)),
            Action::DepositToPerpVault {
                coin: ActionCoin {
                    denom: usdc_info.denom.clone(),
                    amount: ActionAmount::AccountBalance,
                },
                max_receivable_shares: None,
            },
        ],
        &[usdc_info.to_coin(1_000_000_000)],
    )
    .unwrap();

    ////////////////////////////////////////////////////////////////////////////////////
    // 0. Create and activate some trigger orders for another account for OSMO and ATOM
    ////////////////////////////////////////////////////////////////////////////////////
    let create_acc2_osmo_tp = create_child_order(
        "uosmo",
        Int128::from_str("-5").unwrap(),
        Some(true),
        Decimal::from_str("0.3").unwrap(),
        Comparison::GreaterThan,
        "",
    );

    let create_acc2_osmo_sl = create_child_order(
        "uosmo",
        Int128::from_str("-5").unwrap(),
        Some(true),
        Decimal::from_str("0.2").unwrap(),
        Comparison::LessThan,
        "",
    );

    let acc2_osmo_tp =
        child_trigger_order_response(&account_id_2, "1", "2", create_acc2_osmo_tp.clone()).unwrap();
    let acc2_osmo_sl =
        child_trigger_order_response(&account_id_2, "1", "3", create_acc2_osmo_sl.clone()).unwrap();

    mock.update_credit_account(
        &account_id_2,
        &user2,
        vec![
            Deposit(usdc_info.to_coin(1_000_000_000)),
            ExecutePerpOrder {
                denom: "uosmo".to_string(),
                order_size: Int128::from_str("5").unwrap(),
                reduce_only: None,
                order_type: Some(ExecutePerpOrderType::Parent),
            },
            create_acc2_osmo_tp.clone(),
            create_acc2_osmo_sl.clone(),
        ],
        &[usdc_info.to_coin(1_000_000_000)],
    )
    .unwrap();

    let create_acc2_atom_tp = create_child_order(
        "uatom",
        Int128::from_str("5").unwrap(),
        Some(true),
        Decimal::from_str("0.22").unwrap(),
        Comparison::LessThan,
        "",
    );

    let create_acc2_atom_sl = create_child_order(
        "uatom",
        Int128::from_str("5").unwrap(),
        Some(true),
        Decimal::from_str("0.28").unwrap(),
        Comparison::GreaterThan,
        "",
    );

    let acc2_atom_tp =
        child_trigger_order_response(&account_id_2, "4", "5", create_acc2_atom_tp.clone()).unwrap();
    let acc2_atom_sl =
        child_trigger_order_response(&account_id_2, "4", "6", create_acc2_atom_sl.clone()).unwrap();

    mock.update_credit_account(
        &account_id_2,
        &user2,
        vec![
            ExecutePerpOrder {
                denom: "uatom".to_string(),
                order_size: Int128::from_str("-5").unwrap(),
                reduce_only: None,
                order_type: Some(ExecutePerpOrderType::Parent),
            },
            create_acc2_atom_tp.clone(),
            create_acc2_atom_sl.clone(),
        ],
        &[],
    )
    .unwrap();

    let orders = mock.query_trigger_orders_for_account(account_id_2.clone(), None, None);

    // Check if the orders are created correctly, and the ids are assigned
    assert_eq!(
        orders.data,
        vec![
            acc2_osmo_tp.clone(),
            acc2_osmo_sl.clone(),
            acc2_atom_tp.clone(),
            acc2_atom_sl.clone(),
        ]
    );

    ///////////////////////////////////////////////////////////
    // 1. Create 2 OSMO limit orders, each with 2 child orders
    //////////////////////////////////////////////////////////

    let create_acc1_osmo1 = create_parent_order(
        "uosmo",
        Int128::from_str("5").unwrap(),
        Decimal::from_str("0.26").unwrap(),
        Comparison::GreaterThan,
    );

    let create_acc1_osmo1_tp = create_child_order(
        "uosmo",
        Int128::from_str("-5").unwrap(),
        Some(true),
        Decimal::from_str("0.3").unwrap(),
        Comparison::GreaterThan,
        "",
    );

    let create_acc1_osmo1_sl = create_child_order(
        "uosmo",
        Int128::from_str("-5").unwrap(),
        Some(true),
        Decimal::from_str("0.2").unwrap(),
        Comparison::LessThan,
        "",
    );

    let acc1_osmo1 = trigger_order_response(&account_id_1, "7", create_acc1_osmo1.clone()).unwrap();
    let acc1_osmo1_tp =
        child_trigger_order_response(&account_id_1, "7", "8", create_acc1_osmo1_tp.clone())
            .unwrap();
    let acc1_osmo1_sl =
        child_trigger_order_response(&account_id_1, "7", "9", create_acc1_osmo1_sl.clone())
            .unwrap();

    mock.update_credit_account(
        &account_id_1,
        &user1,
        vec![
            Deposit(usdc_info.to_coin(1_000_000_000)),
            create_acc1_osmo1.clone(),
            create_acc1_osmo1_tp.clone(),
            create_acc1_osmo1_sl.clone(),
        ],
        &[usdc_info.to_coin(1_000_000_000)],
    )
    .unwrap();

    let create_acc1_osmo2 = create_parent_order(
        "uosmo",
        Int128::from_str("15").unwrap(),
        Decimal::from_str("0.255").unwrap(),
        Comparison::GreaterThan,
    );

    let create_acc1_osmo2_tp = create_child_order(
        "uosmo",
        Int128::from_str("-5").unwrap(),
        Some(true),
        Decimal::from_str("0.4").unwrap(),
        Comparison::GreaterThan,
        "",
    );

    let create_acc1_osmo2_sl = create_child_order(
        "uosmo",
        Int128::from_str("-5").unwrap(),
        Some(true),
        Decimal::from_str("0.1").unwrap(),
        Comparison::LessThan,
        "",
    );

    let acc1_osmo2 =
        trigger_order_response(&account_id_1, "10", create_acc1_osmo2.clone()).unwrap();
    let acc1_osmo2_tp =
        child_trigger_order_response(&account_id_1, "10", "11", create_acc1_osmo2_tp.clone())
            .unwrap();
    let acc1_osmo2_sl =
        child_trigger_order_response(&account_id_1, "10", "12", create_acc1_osmo2_sl.clone())
            .unwrap();

    mock.update_credit_account(
        &account_id_1,
        &user1,
        vec![create_acc1_osmo2.clone(), create_acc1_osmo2_tp.clone(), create_acc1_osmo2_sl.clone()],
        &[],
    )
    .unwrap();

    let orders = mock.query_trigger_orders_for_account(account_id_1.clone(), None, None);

    // Check if the orders are created correctly, and the ids are assigned
    assert_eq!(
        orders.data,
        vec![
            acc1_osmo2.clone(),
            acc1_osmo2_tp.clone(),
            acc1_osmo2_sl.clone(),
            acc1_osmo1.clone(),
            acc1_osmo1_tp.clone(),
            acc1_osmo1_sl.clone(),
        ]
    );

    ///////////////////////////////////////////////////////////
    // 2. Create 1 ATOM (short) limit order with 2 child orders
    ///////////////////////////////////////////////////////////

    let create_acc1_atom = create_parent_order(
        "uatom",
        Int128::from_str("-15").unwrap(),
        Decimal::from_str("80").unwrap(),
        Comparison::LessThan,
    );

    let create_acc1_atom_tp = create_child_order(
        "uatom",
        Int128::from_str("15").unwrap(),
        Some(true),
        Decimal::from_str("70").unwrap(),
        Comparison::LessThan,
        "",
    );

    let create_acc1_atom_sl = create_child_order(
        "uatom",
        Int128::from_str("15").unwrap(),
        Some(true),
        Decimal::from_str("90").unwrap(),
        Comparison::GreaterThan,
        "",
    );

    let acc1_atom = trigger_order_response(&account_id_1, "13", create_acc1_atom.clone()).unwrap();
    let acc1_atom_tp =
        child_trigger_order_response(&account_id_1, "13", "14", create_acc1_atom_tp.clone())
            .unwrap();
    let acc1_atom_sl =
        child_trigger_order_response(&account_id_1, "13", "15", create_acc1_atom_sl.clone())
            .unwrap();

    mock.update_credit_account(
        &account_id_1,
        &user1,
        vec![create_acc1_atom.clone(), create_acc1_atom_tp.clone(), create_acc1_atom_sl.clone()],
        &[],
    )
    .unwrap();

    let orders = mock.query_trigger_orders_for_account(account_id_1.clone(), None, None);

    // Check if the orders are created correctly, and the ids are assigned
    assert_eq!(
        orders.data,
        vec![
            acc1_osmo2.clone(),
            acc1_osmo2_tp.clone(),
            acc1_osmo2_sl.clone(),
            acc1_atom.clone(),
            acc1_atom_tp.clone(),
            acc1_atom_sl.clone(),
            acc1_osmo1.clone(),
            acc1_osmo1_tp.clone(),
            acc1_osmo1_sl.clone(),
        ]
    );

    ///////////////////////////////////////////////////////////
    // 3. Initiate OSMO position with market order
    ///////////////////////////////////////////////////////////

    mock.update_credit_account(
        &account_id_1,
        &user1,
        vec![ExecutePerpOrder {
            denom: "uosmo".to_string(),
            order_size: Int128::from_str("10").unwrap(),
            reduce_only: None,
            order_type: Some(ExecutePerpOrderType::Default),
        }],
        &[],
    )
    .unwrap();

    // Check position size
    let perp_position = mock.query_perp_position(&account_id_1, "uosmo");
    assert_eq!(perp_position.position.unwrap().size, Int128::from_str("10").unwrap());

    /////////////////////////////////////////////////////////////////////////////
    // 4. Close BTC position with market order. All trigger orders should remain
    /////////////////////////////////////////////////////////////////////////////

    mock.update_credit_account(
        &account_id_1,
        &user1,
        vec![ExecutePerpOrder {
            denom: "uosmo".to_string(),
            order_size: Int128::from_str("-10").unwrap(),
            reduce_only: None,
            order_type: Some(ExecutePerpOrderType::Default),
        }],
        &[],
    )
    .unwrap();

    // Check position size
    let perp_position = mock.query_perp_position(&account_id_1, "uosmo");
    assert_eq!(perp_position.position, None);

    let orders = mock.query_trigger_orders_for_account(account_id_1.clone(), None, None);

    // Check if the orders are created correctly, and the ids are assigned
    assert_eq!(
        orders.data,
        vec![
            acc1_osmo2.clone(),
            acc1_osmo2_tp.clone(),
            acc1_osmo2_sl.clone(),
            acc1_atom.clone(),
            acc1_atom_tp.clone(),
            acc1_atom_sl.clone(),
            acc1_osmo1.clone(),
            acc1_osmo1_tp.clone(),
            acc1_osmo1_sl.clone(),
        ]
    );

    ////////////////////////////////////////////////////////////////////////////
    // 5. Execute ATOM limit order (decrease price). Child orders become active
    ////////////////////////////////////////////////////////////////////////////

    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: "uatom".to_string(),
        price: Decimal::from_str("79").unwrap(),
    });

    mock.execute_trigger_order(&keeper_bot, &account_id_1, &acc1_atom.order.order_id.to_string())
        .unwrap();

    let orders = mock.query_trigger_orders_for_account(account_id_1.clone(), None, None);

    // Check if the orders are created correctly, and the ids are assigned
    assert_eq!(
        orders.data,
        vec![
            acc1_osmo2.clone(),
            acc1_osmo2_tp.clone(),
            acc1_osmo2_sl.clone(),
            acc1_atom_tp.clone(),
            acc1_atom_sl.clone(),
            acc1_osmo1.clone(),
            acc1_osmo1_tp.clone(),
            acc1_osmo1_sl.clone(),
        ]
    );

    ////////////////////////////////////////////////////////////////
    // 6. Execute both OSMO orders. All 4 child orders become active
    ////////////////////////////////////////////////////////////////

    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: "uosmo".to_string(),
        price: Decimal::from_str("0.261").unwrap(),
    });

    mock.execute_trigger_order(&keeper_bot, &account_id_1, &acc1_osmo1.order.order_id.to_string())
        .unwrap();
    mock.execute_trigger_order(&keeper_bot, &account_id_1, &acc1_osmo2.order.order_id.to_string())
        .unwrap();

    let orders = mock.query_trigger_orders_for_account(account_id_1.clone(), None, None);

    // Check if the orders are created correctly, and the ids are assigned
    assert_eq!(
        orders.data,
        vec![
            acc1_osmo2_tp.clone(),
            acc1_osmo2_sl.clone(),
            acc1_atom_tp.clone(),
            acc1_atom_sl.clone(),
            acc1_osmo1_tp.clone(),
            acc1_osmo1_sl.clone(),
        ]
    );

    ///////////////////////////////////////////////////////////////////////////////////////////////////////
    // 7. Execute market order to flip OSMO position. All OSMO trigger orders should be removed, ATOM remains.
    ///////////////////////////////////////////////////////////////////////////////////////////////////////

    mock.update_credit_account(
        &account_id_1,
        &user1,
        vec![ExecutePerpOrder {
            denom: "uosmo".to_string(),
            order_size: Int128::from_str("-40").unwrap(),
            reduce_only: None,
            order_type: Some(ExecutePerpOrderType::Default),
        }],
        &[],
    )
    .unwrap();

    // Check position size
    let perp_position = mock.query_perp_position(&account_id_1, "uosmo");
    assert_eq!(perp_position.position.unwrap().size, Int128::from_str("-20").unwrap());

    let orders = mock.query_trigger_orders_for_account(account_id_1.clone(), None, None);

    // Check if the orders are created correctly, and the ids are assigned
    assert_eq!(orders.data, vec![acc1_atom_tp.clone(), acc1_atom_sl.clone(),]);

    ////////////////////////////////////////////////////////////////////
    // 8. Verify that the other account has all trigger_orders intact.
    ////////////////////////////////////////////////////////////////////

    let orders = mock.query_trigger_orders_for_account(account_id_2.clone(), None, None);

    // Check if the orders are created correctly, and the ids are assigned
    assert_eq!(
        orders.data,
        vec![
            acc2_osmo_tp.clone(),
            acc2_osmo_sl.clone(),
            acc2_atom_tp.clone(),
            acc2_atom_sl.clone(),
        ]
    );
}

fn create_parent_order(
    denom: &str,
    order_size: Int128,
    price: Decimal,
    comparison: Comparison,
) -> Action {
    CreateTriggerOrder {
        order_type: Some(CreateTriggerOrderType::Parent),
        actions: vec![ExecutePerpOrder {
            denom: denom.to_string(),
            order_size,
            reduce_only: None,
            order_type: Some(ExecutePerpOrderType::Default),
        }],
        conditions: vec![OraclePrice {
            denom: denom.to_string(),
            price,
            comparison,
        }],
        keeper_fee: Coin {
            denom: "uusdc".to_string(),
            amount: Uint128::new(10000000),
        },
    }
}

fn trigger_order_response(
    account_id: &str,
    order_id: &str,
    action: Action,
) -> Option<TriggerOrderResponse> {
    match action {
        CreateTriggerOrder {
            actions,
            conditions,
            keeper_fee,
            ..
        } => Some(TriggerOrderResponse {
            account_id: account_id.to_string(),
            order: TriggerOrder {
                order_id: order_id.to_string(),
                actions,
                conditions,
                keeper_fee,
            },
        }),
        _ => None,
    }
}

fn create_child_order(
    denom: &str,
    order_size: Int128,
    reduce_only: Option<bool>,
    price: Decimal,
    comparison: Comparison,
    parent_id: &str,
) -> Action {
    CreateTriggerOrder {
        order_type: Some(CreateTriggerOrderType::Child),
        actions: vec![ExecutePerpOrder {
            denom: denom.to_string(),
            order_size,
            reduce_only,
            order_type: Some(ExecutePerpOrderType::Default),
        }],
        conditions: vec![
            OraclePrice {
                denom: denom.to_string(),
                price,
                comparison,
            },
            TriggerOrderExecuted {
                trigger_order_id: parent_id.to_string(),
            },
        ],
        keeper_fee: Coin {
            denom: "uusdc".to_string(),
            amount: Uint128::new(10000000),
        },
    }
}

fn child_trigger_order_response(
    account_id: &str,
    parent_order_id: &str,
    order_id: &str,
    action: Action,
) -> Option<TriggerOrderResponse> {
    match action {
        CreateTriggerOrder {
            actions,
            mut conditions,
            keeper_fee,
            ..
        } => {
            for condition in &mut conditions {
                if let TriggerOrderExecuted {
                    ref mut trigger_order_id,
                } = condition
                {
                    *trigger_order_id = parent_order_id.to_string()
                }
            }
            Some(TriggerOrderResponse {
                account_id: account_id.to_string(),
                order: TriggerOrder {
                    order_id: order_id.to_string(),
                    actions,
                    conditions,
                    keeper_fee,
                },
            })
        }
        _ => None,
    }
}

fn create_trigger_order(
    denom: &str,
    order_size: Int128,
    price: Decimal,
    comparison: Comparison,
) -> Action {
    CreateTriggerOrder {
        order_type: Some(CreateTriggerOrderType::Default),
        actions: vec![ExecutePerpOrder {
            denom: denom.to_string(),
            order_size,
            reduce_only: None,
            order_type: Some(ExecutePerpOrderType::Default),
        }],
        conditions: vec![OraclePrice {
            denom: denom.to_string(),
            price,
            comparison,
        }],
        keeper_fee: Coin {
            denom: "uusdc".to_string(),
            amount: Uint128::new(10000000),
        },
    }
}

fn child_order_no_parent_id() -> Action {
    CreateTriggerOrder {
        order_type: Some(CreateTriggerOrderType::Child),
        keeper_fee: Coin {
            denom: "ubtc".to_string(),
            amount: Uint128::new(1000000),
        },
        actions: vec![ExecutePerpOrder {
            denom: "ubtc".to_string(),
            order_size: Int128::from_str("1").unwrap(),
            reduce_only: None,
            order_type: Some(ExecutePerpOrderType::Default),
        }],
        conditions: vec![
            TriggerOrderExecuted {
                trigger_order_id: "".to_string(),
            },
            OraclePrice {
                denom: "perp1".to_string(),
                price: Decimal::from_str("100").unwrap(),
                comparison: Comparison::GreaterThan,
            },
        ],
    }
}

fn child_order_with_parent_id(parent_id: u64) -> Action {
    CreateTriggerOrder {
        order_type: Some(CreateTriggerOrderType::Child),
        keeper_fee: Coin {
            denom: "ubtc".to_string(),
            amount: Uint128::new(1000000),
        },
        actions: vec![ExecutePerpOrder {
            denom: "ubtc".to_string(),
            order_size: Int128::from_str("1").unwrap(),
            reduce_only: None,
            order_type: Some(ExecutePerpOrderType::Default),
        }],
        conditions: vec![
            TriggerOrderExecuted {
                trigger_order_id: parent_id.to_string(),
            },
            OraclePrice {
                denom: "perp1".to_string(),
                price: Decimal::from_str("100").unwrap(),
                comparison: Comparison::GreaterThan,
            },
        ],
    }
}

fn child_order_invalid_single_condition() -> Action {
    CreateTriggerOrder {
        order_type: Some(CreateTriggerOrderType::Child),
        keeper_fee: Coin {
            denom: "ubtc".to_string(),
            amount: Uint128::new(1000000),
        },
        actions: vec![ExecutePerpOrder {
            denom: "ubtc".to_string(),
            order_size: Int128::from_str("1").unwrap(),
            reduce_only: None,
            order_type: Some(ExecutePerpOrderType::Default),
        }],
        conditions: vec![TriggerOrderExecuted {
            trigger_order_id: "".to_string(),
        }],
    }
}

fn child_order_invalid_double_trigger_condition() -> Action {
    CreateTriggerOrder {
        order_type: Some(CreateTriggerOrderType::Child),
        keeper_fee: Coin {
            denom: "ubtc".to_string(),
            amount: Uint128::new(1000000),
        },
        actions: vec![ExecutePerpOrder {
            denom: "ubtc".to_string(),
            order_size: Int128::from_str("1").unwrap(),
            reduce_only: None,
            order_type: Some(ExecutePerpOrderType::Default),
        }],
        conditions: vec![
            TriggerOrderExecuted {
                trigger_order_id: "".to_string(),
            },
            TriggerOrderExecuted {
                trigger_order_id: "".to_string(),
            },
        ],
    }
}

fn parent_order_invalid_conditions() -> Action {
    CreateTriggerOrder {
        order_type: Some(CreateTriggerOrderType::Parent),
        keeper_fee: Coin {
            denom: "ubtc".to_string(),
            amount: Uint128::new(1000000),
        },
        actions: vec![ExecutePerpOrder {
            denom: "ubtc".to_string(),
            order_size: Int128::from_str("1").unwrap(),
            reduce_only: None,
            order_type: Some(ExecutePerpOrderType::Default),
        }],
        conditions: vec![TriggerOrderExecuted {
            trigger_order_id: "2".to_string(),
        }],
    }
}

fn default_order_invalid_conditions() -> Action {
    CreateTriggerOrder {
        order_type: Some(CreateTriggerOrderType::Default),
        keeper_fee: Coin {
            denom: "ubtc".to_string(),
            amount: Uint128::new(1000000),
        },
        actions: vec![ExecutePerpOrder {
            denom: "ubtc".to_string(),
            order_size: Int128::from_str("1").unwrap(),
            reduce_only: None,
            order_type: Some(ExecutePerpOrderType::Default),
        }],
        conditions: vec![TriggerOrderExecuted {
            trigger_order_id: "2".to_string(),
        }],
    }
}

fn default_order() -> Action {
    CreateTriggerOrder {
        order_type: Some(CreateTriggerOrderType::Default),
        keeper_fee: Coin {
            denom: "ubtc".to_string(),
            amount: Uint128::new(1000000),
        },
        actions: vec![ExecutePerpOrder {
            denom: "ubtc".to_string(),
            order_size: Int128::from_str("1").unwrap(),
            reduce_only: None,
            order_type: Some(ExecutePerpOrderType::Default),
        }],
        conditions: vec![OraclePrice {
            denom: "perp1".to_string(),
            price: Decimal::from_str("100").unwrap(),
            comparison: Comparison::GreaterThan,
        }],
    }
}

fn parent_order() -> Action {
    CreateTriggerOrder {
        order_type: Some(CreateTriggerOrderType::Parent),
        keeper_fee: Coin {
            denom: "ubtc".to_string(),
            amount: Uint128::new(1000000),
        },
        actions: vec![ExecutePerpOrder {
            denom: "ubtc".to_string(),
            order_size: Int128::from_str("1").unwrap(),
            reduce_only: None,
            order_type: Some(ExecutePerpOrderType::Default),
        }],
        conditions: vec![OraclePrice {
            denom: "perp1".to_string(),
            price: Decimal::from_str("100").unwrap(),
            comparison: Comparison::GreaterThan,
        }],
    }
}

fn parent_market_order() -> Action {
    ExecutePerpOrder {
        denom: "ubtc".to_string(),
        order_size: Int128::from_str("1").unwrap(),
        reduce_only: None,
        order_type: Some(ExecutePerpOrderType::Parent),
    }
}

fn default_market_order() -> Action {
    ExecutePerpOrder {
        denom: "ubtc".to_string(),
        order_size: Int128::from_str("1").unwrap(),
        reduce_only: None,
        order_type: Some(ExecutePerpOrderType::Default),
    }
}

fn parent_order_parent_actions() -> Action {
    CreateTriggerOrder {
        order_type: Some(CreateTriggerOrderType::Parent),
        keeper_fee: Coin {
            denom: "ubtc".to_string(),
            amount: Uint128::new(1000000),
        },
        actions: vec![ExecutePerpOrder {
            denom: "ubtc".to_string(),
            order_size: Int128::from_str("1").unwrap(),
            reduce_only: None,
            order_type: Some(ExecutePerpOrderType::Parent),
        }],
        conditions: vec![OraclePrice {
            denom: "perp1".to_string(),
            price: Decimal::from_str("100").unwrap(),
            comparison: Comparison::GreaterThan,
        }],
    }
}

fn check_result_for_expected_error(
    result: Result<AppResponse, Error>,
    expected_error: Option<ContractError>,
) {
    // check result
    match (result, expected_error) {
        (Err(err), Some(exp_err)) => {
            let err: ContractError = err.downcast().unwrap();
            assert_eq!(err, exp_err);
        }
        (Err(err), None) => {
            panic!("unexpected error: {:?}", err);
        }
        (Ok(_), Some(_)) => panic!("expected error, but got success"),
        (Ok(_), None) => {}
    }
}
