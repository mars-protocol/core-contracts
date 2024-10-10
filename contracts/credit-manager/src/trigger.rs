use cosmwasm_std::{
    ensure, ensure_eq, BankMsg, Coin, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Response,
};
use mars_types::{
    credit_manager::{Action, Condition, TriggerOrder},
    oracle::ActionKind,
};

use crate::{
    error::ContractError,
    execute::dispatch_actions,
    health::query_health_values,
    state::{KEEPER_FEE_CONFIG, NEXT_TRIGGER_ID, ORACLE, TRIGGER_ORDERS},
    utils::{decrement_coin_balance, increment_coin_balance},
};

pub fn create_trigger_order(
    deps: DepsMut,
    account_id: &str,
    actions: Vec<Action>,
    conditions: Vec<Condition>,
    keeper_fee: Coin,
) -> Result<Response, ContractError> {
    // Ensure that the trigger order does not contain any illegal actions
    // Initially, this is limited to just execute_perp_order
    let contains_legal_actions =
        actions.iter().all(|action| matches!(action, Action::ExecutePerpOrder { .. }));
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
    // Use may_load so we can give a better error message
    let order_to_remove = TRIGGER_ORDERS
        .may_load(deps.storage, (account_id, trigger_order_id))?
        .ok_or(ContractError::TriggerOrderNotFound {
            order_id: trigger_order_id.to_string(),
            account_id: account_id.to_string(),
        })?;

    // Refund keeper fee
    increment_coin_balance(deps.storage, account_id, &order_to_remove.keeper_fee)?;

    // Remove order
    TRIGGER_ORDERS.remove(deps.storage, (account_id, trigger_order_id));

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
    // Use may_load so we can give a better error message
    let order = TRIGGER_ORDERS.may_load(deps.storage, (account_id, trigger_order_id))?.ok_or(
        ContractError::TriggerOrderNotFound {
            order_id: trigger_order_id.to_string(),
            account_id: account_id.to_string(),
        },
    )?;

    let oracle = ORACLE.load(deps.storage)?;
    let mut used_conditional_types = vec![];

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
        };

        if !conditions_met {
            return Err(ContractError::IllegalExecuteTriggerOrder);
        }
    }

    TRIGGER_ORDERS.remove(deps.storage, (account_id, trigger_order_id));
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
