use cosmwasm_std::{
    ensure, ensure_eq, BankMsg, Coin, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Order,
    Response, Storage,
};
use mars_types::{
    credit_manager::{
        Action, Condition, CreateTriggerOrderType, ExecutePerpOrderType, TriggerOrder,
    },
    oracle::ActionKind,
};

use crate::{
    error::ContractError,
    execute::dispatch_actions,
    health::query_health_values,
    state::{
        EXECUTED_TRIGGER_ORDERS, KEEPER_FEE_CONFIG, MAX_TRIGGER_ORDERS, NEXT_TRIGGER_ID, ORACLE,
        TRIGGER_ORDERS, TRIGGER_ORDER_RELATED_IDS,
    },
    utils::{decrement_coin_balance, increment_coin_balance},
};

pub fn create_trigger_order(
    deps: DepsMut,
    account_id: &str,
    actions: Vec<Action>,
    conditions: Vec<Condition>,
    keeper_fee: Coin,
) -> Result<Response, ContractError> {
    let current_trigger_order_amount =
        TRIGGER_ORDERS.prefix(account_id).keys(deps.storage, None, None, Order::Ascending).count();
    let max_trigger_orders = MAX_TRIGGER_ORDERS.load(deps.storage).unwrap_or(0);

    ensure!(
        current_trigger_order_amount < max_trigger_orders as usize,
        ContractError::MaxTriggerOrdersReached {
            max_trigger_orders
        }
    );

    // Ensure that the trigger order does not contain any illegal actions
    // Initially, this is limited to just execute_perp_order and lend
    let contains_legal_actions = actions
        .iter()
        .all(|action| matches!(action, Action::ExecutePerpOrder { .. } | Action::Lend(..)));
    ensure!(contains_legal_actions, ContractError::IllegalTriggerAction);

    // Generate & increment id
    let order_id = NEXT_TRIGGER_ID.load(deps.storage)?;
    NEXT_TRIGGER_ID.save(deps.storage, &(order_id + 1))?;

    // Ensure keeper fees are valid according to configuration
    let cfg = KEEPER_FEE_CONFIG.load(deps.storage)?;
    ensure!(
        keeper_fee.amount >= cfg.min_fee.amount,
        ContractError::KeeperFeeTooSmall {
            expected_min_amount: cfg.min_fee.amount,
            received_amount: keeper_fee.amount,
        }
    );
    ensure_eq!(
        keeper_fee.denom,
        cfg.min_fee.denom,
        ContractError::InvalidKeeperFeeDenom {
            expected_denom: cfg.min_fee.denom,
            received_denom: keeper_fee.denom
        }
    );

    for condition in &conditions {
        if let Condition::TriggerOrderExecuted {
            trigger_order_id,
        } = condition
        {
            TRIGGER_ORDER_RELATED_IDS.save(
                deps.storage,
                (account_id, trigger_order_id, &order_id.to_string()),
                &order_id.to_string(),
            )?;
        }
    }

    // Deduct keeper_fee from account
    decrement_coin_balance(deps.storage, account_id, &keeper_fee)?;

    // Store trigger in state
    TRIGGER_ORDERS.save(
        deps.storage,
        (account_id, &order_id.to_string()),
        &TriggerOrder {
            order_id: order_id.to_string(),
            actions,
            conditions,
            keeper_fee,
        },
    )?;

    Ok(Response::new()
        .add_attribute("action", "create_trigger_order")
        .add_attribute("order_id", order_id.to_string()))
}

pub fn delete_trigger_order(
    deps: DepsMut,
    account_id: &str,
    trigger_order_id: &str,
) -> Result<Response, ContractError> {
    let order_to_remove = get_trigger_order(deps.storage, account_id, trigger_order_id)?;

    // Refund keeper fee
    increment_coin_balance(deps.storage, account_id, &order_to_remove.keeper_fee)?;

    // Remove order
    TRIGGER_ORDERS.remove(deps.storage, (account_id, trigger_order_id));

    // Remove any related trigger orders
    remove_related_trigger_orders(deps.storage, account_id, trigger_order_id)?;

    // Remove it from the executed orders
    EXECUTED_TRIGGER_ORDERS.remove(deps.storage, (account_id, trigger_order_id));

    Ok(Response::new()
        .add_attribute("action", "cancel_trigger_order")
        .add_attribute("order_id", trigger_order_id))
}

pub fn execute_trigger_order(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    account_id: &str,
    trigger_order_id: &str,
) -> Result<Response, ContractError> {
    let order = get_trigger_order(deps.storage, account_id, trigger_order_id)?;

    let oracle = ORACLE.load(deps.storage)?;
    let mut used_conditional_types = vec![];
    let mut parent_order_id: Option<String> = None;

    // Iterate all conditions in the order
    for condition in order.conditions {
        let conditions_met = match condition {
            Condition::OraclePrice {
                denom,
                price,
                comparison,
            } => {
                used_conditional_types.push("price");
                let oracle_price =
                    oracle.query_price(&deps.querier, &denom, ActionKind::Default)?;
                comparison.is_met(oracle_price.price, price)
            }
            Condition::HealthFactor {
                threshold,
                comparison,
            } => {
                used_conditional_types.push("health_factor");
                let health_factor = query_health_values(
                    deps.as_ref(),
                    env.clone(),
                    account_id,
                    ActionKind::Default,
                )?
                .max_ltv_health_factor
                .unwrap_or(Decimal::MAX);
                comparison.is_met(health_factor, threshold)
            }
            Condition::RelativePrice {
                base_price_denom,
                quote_price_denom,
                price,
                comparison,
            } => {
                used_conditional_types.push("relative_price");
                let base_price = oracle
                    .query_price(&deps.querier, &base_price_denom, ActionKind::Default)?
                    .price;
                let quote_price = oracle
                    .query_price(&deps.querier, &quote_price_denom, ActionKind::Default)?
                    .price;

                let relative_price = base_price.checked_div(quote_price)?;
                comparison.is_met(relative_price, price)
            }
            Condition::TriggerOrderExecuted {
                trigger_order_id,
            } => {
                parent_order_id = Some(trigger_order_id.clone());
                EXECUTED_TRIGGER_ORDERS
                    .may_load(deps.storage, (account_id, &trigger_order_id))?
                    .is_some()
            }
        };

        if !conditions_met {
            return Err(ContractError::IllegalExecuteTriggerOrder);
        }
    }

    // Remove the current trigger_order
    TRIGGER_ORDERS.remove(deps.storage, (account_id, trigger_order_id));

    match parent_order_id {
        Some(parent_order_id) => {
            // Any related orders can be removed, as execution of one renders other child orders redundant (by design).
            remove_related_trigger_orders(deps.storage, account_id, &parent_order_id)?;
        }
        None => {
            // Store this order in executed orders, but only if there are sub_orders
            let orders = TRIGGER_ORDER_RELATED_IDS.prefix((account_id, trigger_order_id));

            if !orders.is_empty(deps.storage) {
                EXECUTED_TRIGGER_ORDERS.save(
                    deps.storage,
                    (account_id, trigger_order_id),
                    &trigger_order_id.to_string(),
                )?;
            }
        }
    }

    let keeper_address = info.sender.to_string();

    // Execute actions on behalf of user
    let mut res = dispatch_actions(
        deps,
        env,
        info,
        Some(account_id.to_string()),
        None,
        order.actions,
        false,
    )?;

    // Add relevant attributes
    res = res
        .add_attribute("action", "execute_trigger_order")
        .add_attribute("order_id", trigger_order_id)
        .add_attribute("conditionals", used_conditional_types.join(","));

    // Send keeper fee to method caller
    let transfer_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: keeper_address,
        amount: vec![order.keeper_fee],
    });

    Ok(res.add_message(transfer_msg))
}

fn get_trigger_order(
    storage: &mut dyn Storage,
    account_id: &str,
    order_id: &str,
) -> Result<TriggerOrder, ContractError> {
    TRIGGER_ORDERS.may_load(storage, (account_id, order_id))?.ok_or(
        ContractError::TriggerOrderNotFound {
            order_id: order_id.to_string(),
            account_id: account_id.to_string(),
        },
    )
}

fn remove_related_trigger_orders(
    storage: &mut dyn Storage,
    account_id: &str,
    parent_order_id: &str,
) -> Result<(), ContractError> {
    // Because it is not allowed to mutate the storage in a loop, order ids are stored here
    // and removed at the end of the function.
    let mut child_orders_to_remove: Vec<String> = vec![];

    let related_order_ids = TRIGGER_ORDER_RELATED_IDS.prefix((account_id, parent_order_id)).range(
        storage,
        None,
        None,
        Order::Ascending,
    );

    for child in related_order_ids {
        let (_, other_child_id) = child?;
        child_orders_to_remove.push(other_child_id.clone());
    }

    for order_id in child_orders_to_remove {
        TRIGGER_ORDERS.remove(storage, (account_id, &order_id));
    }

    Ok(())
}

pub fn remove_invalid_trigger_orders(
    storage: &mut dyn Storage,
    account_id: &str,
    market: &str,
) -> Result<(), ContractError> {
    // Because it is not allowed to mutate the storage in a loop, order ids are stored here
    // and removed at the end of the function.
    let mut order_ids_to_remove: Vec<String> = vec![];

    let trigger_orders =
        TRIGGER_ORDERS.prefix(account_id).range(storage, None, None, Order::Ascending);

    for item in trigger_orders {
        let (_, trigger_order) = item?;

        for action in trigger_order.actions {
            if let Action::ExecutePerpOrder {
                denom,
                reduce_only,
                ..
            } = action
            {
                // Only check orders with `reduce_only` and for the same market
                if reduce_only.unwrap_or(false) && denom == *market.to_string() {
                    let mut is_child = false;

                    for condition in &trigger_order.conditions {
                        // It's a child order
                        if let Condition::TriggerOrderExecuted {
                            trigger_order_id,
                        } = condition
                        {
                            is_child = true;
                            let is_parent_executed = EXECUTED_TRIGGER_ORDERS
                                .may_load(storage, (account_id, trigger_order_id))?
                                .is_some();

                            // Remove this order
                            if is_parent_executed {
                                order_ids_to_remove.push(trigger_order.order_id.clone());
                            }
                        }
                    }

                    // Any parent_orders
                    if !is_child {
                        order_ids_to_remove.push(trigger_order.order_id.clone());
                    }
                }
            }
        }
    }

    // Remove invalid orders
    for order_id in order_ids_to_remove {
        TRIGGER_ORDERS.remove(storage, (account_id, &order_id));
    }

    Ok(())
}

/// Verifies order relationships and assigns a parent ID to child actions where needed.
///
/// # Rules Enforced:
/// 1. At most one parent order per transaction.
/// 2. Parent order must not have a `TriggerOrderExecuted` condition.
/// 3. Parent must appear first if present.
/// 4. A child order must contain exactly 1 TriggerOrderExecuted condition + at least 1 other condition.
/// 5. The TriggerOrderExecuted must refer to either a previously created order or (if None) a parent order in the same transaction.
pub fn check_order_relations_and_set_parent_id(
    storage: &mut dyn Storage,
    account_id: &str,
    actions: &mut [Action],
) -> Result<(), ContractError> {
    let mut parent_order_id: Option<String> = None;
    let mut num_child_orders: u32 = 0;
    // Ensures the parent order is the first to be assigned a `trigger_order_id`:
    // - TriggerOrders get their ID during execution.
    // - Creating orders before the parent can mis-align the ID.
    let mut is_trigger_order_initialized = false;

    for action in actions.iter_mut() {
        match action {
            Action::CreateTriggerOrder {
                order_type,
                conditions,
                ..
            } => {
                match order_type.as_ref().unwrap_or(&CreateTriggerOrderType::Default) {
                    CreateTriggerOrderType::Parent => {
                        ensure_parent_order_conditions(conditions, is_trigger_order_initialized)?;

                        let order_id = NEXT_TRIGGER_ID.load(storage)?;
                        parent_order_id = Some(order_id.to_string());
                    }
                    CreateTriggerOrderType::Child => {
                        ensure_child_order_conditions_and_parent(
                            storage,
                            account_id,
                            conditions,
                            &parent_order_id,
                        )?;
                        num_child_orders += 1;
                    }
                    CreateTriggerOrderType::Default => {
                        ensure_default_order_conditions(conditions)?;
                    }
                }

                is_trigger_order_initialized = true
            }
            Action::ExecutePerpOrder {
                order_type,
                ..
            } => {
                match order_type.as_ref().unwrap_or(&ExecutePerpOrderType::Default) {
                    ExecutePerpOrderType::Parent => {
                        ensure_parent_order_conditions(&[], is_trigger_order_initialized)?;

                        // If the parent order is executed directly (i.e. it is not a CreateTriggerOrder), we still
                        // assign it a trigger_order_id, which can be used for reference in the child orders.
                        let order_id = NEXT_TRIGGER_ID.load(storage)?;

                        parent_order_id = Some(order_id.to_string());

                        // Increment the NEXT_TRIGGER_ID as we have used it
                        NEXT_TRIGGER_ID.save(storage, &(order_id + 1))?;

                        // Store the order_id in the executed orders
                        EXECUTED_TRIGGER_ORDERS.save(
                            storage,
                            (account_id, &order_id.to_string()),
                            &order_id.to_string(),
                        )?;

                        is_trigger_order_initialized = true
                    }
                    // Ignore default orders. These are not relevant for the order relations
                    ExecutePerpOrderType::Default => {}
                }
            }

            // Ignore any actions that have nothing to do with the order relations
            _ => {}
        }
    }

    if parent_order_id.is_some() {
        // Make sure that there are child orders for the parent order
        ensure!(num_child_orders > 0, ContractError::NoChildOrdersFound);
    }

    Ok(())
}

fn ensure_parent_order_conditions(
    conditions: &[Condition],
    is_trigger_order_initialized: bool,
) -> Result<(), ContractError> {
    // It is not allowed to have a TriggerOrderExecuted condition in a parent order
    // This is because the parent order is not allowed to be dependent on another order.
    // We only support 1 level of dependency.
    let contains_illegal_conditions = conditions
        .iter()
        .any(|condition| matches!(condition, Condition::TriggerOrderExecuted { .. }));
    ensure!(
        !contains_illegal_conditions,
        ContractError::InvalidOrderConditions {
            reason: "Parent orders cannot contain a TriggerOrderExecuted condition".to_string()
        }
    );

    // The parent order has to be the first order that gets assigned a `trigger_order_id`,
    // so it is not allowed to have created any other trigger orders before in the same transaction
    ensure!(!is_trigger_order_initialized, ContractError::InvalidParentOrderPosition);

    Ok(())
}

fn ensure_default_order_conditions(conditions: &[Condition]) -> Result<(), ContractError> {
    // It is not allowed to have a TriggerOrderExecuted condition in a default order
    let contains_illegal_conditions = conditions
        .iter()
        .any(|condition| matches!(condition, Condition::TriggerOrderExecuted { .. }));
    ensure!(
        !contains_illegal_conditions,
        ContractError::InvalidOrderConditions {
            reason: "Default orders cannot contain a TriggerOrderExecuted condition".to_string()
        }
    );

    Ok(())
}

// This functions checks the conditions of a child order and also sets the `trigger_order_id` in case
// the TriggerOrderExecuted condition is empty.
fn ensure_child_order_conditions_and_parent(
    storage: &mut dyn Storage,
    account_id: &str,
    conditions: &mut [Condition],
    parent_order_id: &Option<String>,
) -> Result<(), ContractError> {
    let conditions_len = conditions.len();
    let mut trigger_order_exec_conditions = conditions
        .iter_mut()
        .filter(|condition| matches!(condition, Condition::TriggerOrderExecuted { .. }))
        .collect::<Vec<_>>();

    // Ensure that there is exactly one TriggerOrderExecuted condition.
    // This has to be the case, otherwise it is not a valid child order.
    ensure!(
        trigger_order_exec_conditions.len() == 1,
        ContractError::InvalidOrderConditions {
            reason: "Child order needs exactly 1 TriggerOrderExecuted condition".to_string()
        }
    );

    // There has to be at least 1 condition extra (next to the TriggerOrderExecuted condition),
    // Otherwise the trigger_order could never be executed.
    ensure!(
        conditions_len > 1,
        ContractError::InvalidOrderConditions {
            reason: "Child order needs at least 1 other condition next to TriggerOrderExecuted"
                .to_string()
        }
    );

    // Check the contents of the condition
    // At this point we are sure there is exactly 1 TriggerOrderExecuted conditions, so we can
    // safely unwrap the first element.
    if let Some(Condition::TriggerOrderExecuted {
        trigger_order_id,
    }) = trigger_order_exec_conditions.first_mut()
    {
        validate_trigger_order_condition(storage, account_id, parent_order_id, trigger_order_id)?;
    }

    Ok(())
}

fn validate_trigger_order_condition(
    storage: &mut dyn Storage,
    account_id: &str,
    parent_order_id: &Option<String>,
    trigger_order_id: &mut String,
) -> Result<(), ContractError> {
    match parent_order_id {
        Some(parent_order_id) => {
            // If there is a parent order in the same transaction, any child order's trigger_order_id
            // should be empty.
            ensure!(trigger_order_id.is_empty(), ContractError::InvalidOrderConditions { reason: "Child order cannot provide a trigger_order_id in TriggerOrderExecuted conditions when earlier action contains a parent.".to_string()});

            // Set the trigger_order_id to the parent order id
            *trigger_order_id = parent_order_id.to_string();
        }
        None => {
            // If there is no parent order in the same tx, the condition should refer
            // to an already existing parent order.
            ensure!(
                !trigger_order_id.is_empty(),
                ContractError::InvalidOrderConditions {
                    reason: "No trigger_order_id in TriggerOrderExecuted conditions.".to_string()
                }
            );

            // Check if the parent order actually exists AND is owned by the same account_id
            get_trigger_order(storage, account_id, trigger_order_id)?;
        }
    }
    Ok(())
}
