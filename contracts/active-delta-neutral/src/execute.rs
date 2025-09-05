use std::ops::Neg;

use cosmwasm_std::{
    to_json_binary, CosmosMsg, Decimal, DepsMut, Env, Int128, MessageInfo, Response, Uint128,
    WasmMsg,
};
use mars_delta_neutral_position::types::Position;
use mars_perps_common::pricing::{closing_execution_price, opening_execution_price};
use mars_types::{
    active_delta_neutral::{
        execute::ExecuteMsg,
        query::{Config, MarketConfig},
    },
    adapters::{credit_manager::CreditManager, oracle::Oracle, params::Params, perps::Perps},
    credit_manager::{self, Action, ActionAmount, ActionCoin},
    oracle::ActionKind,
    swapper::SwapperRoute,
};

use super::{error::ContractError, helpers::validate_swapper_route, state::MARKET_CONFIG};
use crate::{
    error::ContractResult,
    helpers::{
        self, assert_deposit_funds_valid, assert_no_funds, combined_balance, PositionDeltas,
    },
    order_creation::build_trade_actions,
    order_validation::{self, DynamicValidator},
    state::{CONFIG, OWNER, POSITION},
    traits::Validator,
};
/// # Execute Increase Position
///
/// Increases the delta-neutral position by the specified amount using the provided swapper route.
/// This function implements the first part of a two-phase trade execution pattern:
///
/// 1. Execute the spot market operation (buy the spot asset)
/// 2. Trigger the CompleteHedge operation to execute the corresponding perp trade
///
/// The delta-neutral position is maintained by keeping equal but opposite exposure in spot and perp markets.
///
/// ## Parameters
/// * `deps` - Mutable dependencies including storage access
/// * `env` - Environment information (contract address, block height, etc.)
/// * `info` - Message information including sender address
/// * `amount` - Amount of the USDC to sell for the volatile asset
/// * `swapper_route` - Detailed path for executing the swap via the Mars credit account
///
/// ## Returns
/// * `Response` on success with messages to execute the spot trade and trigger the hedge completion
/// * `ContractError` if validation fails or if any operation cannot be completed
pub fn buy(
    deps: DepsMut,
    env: Env,
    market_id: &str,
    amount: Uint128,
    swapper_route: &SwapperRoute,
) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;
    let market_config: MarketConfig = MARKET_CONFIG.load(deps.storage, market_id)?;
    let credit_account_id =
        config.credit_account_id.as_ref().ok_or(ContractError::CreditAccountNotInitialized {})?;
    validate_swapper_route(swapper_route, &market_config.usdc_denom, &market_config.spot_denom)?;

    let credit_manager = CreditManager::new(config.credit_manager_addr);

    let stable_balance = combined_balance(
        &credit_manager.query_positions(&deps.querier, credit_account_id)?,
        &market_config.usdc_denom,
    )?;

    // TODO
    // validate config (not more than max size)
    // not more than max leverage
    // If these are true, make it reduce only

    let actions = build_trade_actions(
        amount,
        stable_balance,
        &market_config.usdc_denom,
        &market_config.spot_denom,
        swapper_route,
    );

    let execute_spot_swap = credit_manager.execute_actions_msg(credit_account_id, actions, &[])?;

    let complete_hedge = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_json_binary(&ExecuteMsg::Hedge {
            swap_exact_in_amount: amount,
            market_id: market_config.market_id.clone(),
            increasing: true,
        })?,
        funds: vec![],
    });

    Ok(Response::new()
        .add_message(execute_spot_swap)
        .add_message(complete_hedge)
        .add_attribute("action", "buy"))
}

/// # Execute selling of our volatile asset
///
/// Short in this context refers to selling spot and buying perp.
///
/// Sells the delta-neutral position by the specified amount using the provided swapper route.
/// Similar to the increase operation, this function also implements a two-phase trade execution:
///
/// 1. Execute the spot market operation (sell the spot asset)
/// 2. Trigger the CompleteHedge operation to execute the corresponding perp trade
///
/// When decreasing a position, the contract also calculates realized PnL based on the prorated
/// entry value of the position being closed, accounting for funding and borrowing costs.
///
/// ## Parameters
/// * `deps` - Mutable dependencies including storage access
/// * `env` - Environment information (contract address, block height, etc.)
/// * `info` - Message information including sender address
/// * `amount` - Amount of the volatile asset to sell
/// * `swapper_route` - Detailed path for executing the swap through Astroport
///
/// ## Returns
/// * `Response` on success with messages to execute the spot trade and trigger the hedge completion
/// * `ContractError` if validation fails, if the position is too small, or if any operation cannot be completed
pub fn sell(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    amount: Uint128,
    market_id: &str,
    swapper_route: &SwapperRoute,
) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;
    let market_config: MarketConfig = MARKET_CONFIG.load(deps.storage, market_id)?;
    let credit_account_id =
        config.credit_account_id.as_ref().ok_or(ContractError::CreditAccountNotInitialized {})?;
    validate_swapper_route(swapper_route, &market_config.spot_denom, &market_config.perp_denom)?;

    let credit_manager = CreditManager::new(config.credit_manager_addr);

    let spot_balance = combined_balance(
        &credit_manager.query_positions(&deps.querier, credit_account_id)?,
        &market_config.spot_denom,
    )?;

    // TODO
    // validate config (not more than max size)
    // not more than max leverage
    // If these are true, make it reduce only

    let actions = build_trade_actions(
        amount,
        spot_balance,
        &market_config.spot_denom,
        &market_config.usdc_denom,
        swapper_route,
    );

    let execute_spot_swap = credit_manager.execute_actions_msg(credit_account_id, actions, &[])?;

    // Complete the hedge by calling an internal hedge function
    let complete_hedge = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_json_binary(&ExecuteMsg::Hedge {
            swap_exact_in_amount: amount,
            market_id: market_config.spot_denom.clone(),
            increasing: false,
        })?,
        funds: vec![],
    });

    Ok(Response::new()
        .add_message(execute_spot_swap)
        .add_message(complete_hedge)
        .add_attribute("action", "sell"))
}

/// # Execute Complete Hedge
///
/// Completes the hedging operation after a spot trade by executing the corresponding perpendicular trade.
/// This function is the second phase of both increase and decrease operations and is called automatically
/// after the spot trade completes.
///
/// The function:
/// 1. Verifies it's being called by the contract itself (security check)
/// 2. Determines how much the spot position changed by comparing balances
/// 3. Queries current funding and borrow rates to ensure profitability
/// 4. Executes the opposite trade in the perp market to maintain delta neutrality
///
/// This design allows atomic execution of both legs of the delta-neutral trade, ensuring
/// that positions remain properly hedged even in volatile market conditions.
///
/// ## Parameters
/// * `deps` - Mutable dependencies including storage access
/// * `env` - Environment information (contract address, block height, etc.)
/// * `info` - Message information including sender address (must be the contract itself)
/// * `swap_in_amount` - The amount of the spot asset that was swapped in
/// * `denom` - The denomination of the spot asset
/// * `increasing` - Whether the position is increasing or decreasing
///
/// ## Returns
/// * `Response` on success with attributes detailing the hedge operation
/// * `ContractError::Unauthorized` if called by anyone other than the contract itself
/// * Other errors if token balance queries fail or if the trade would be unprofitable
pub fn hedge(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    swap_in_amount: Uint128,
    market_id: &str,
    increasing: bool,
) -> ContractResult<Response> {
    // Internal method only
    if info.sender != env.contract.address {
        return Err(ContractError::Unauthorized {});
    }

    // State variables
    let config: Config = CONFIG.load(deps.storage)?;
    let market_config: MarketConfig = MARKET_CONFIG.load(deps.storage, market_id)?;
    let mut position_state: Position =
        POSITION.may_load(deps.storage, market_id)?.unwrap_or_default();
    let credit_account_id =
        config.credit_account_id.as_ref().ok_or(ContractError::CreditAccountNotInitialized {})?;

    // Contract adapters
    let credit_manager = CreditManager::new(config.credit_manager_addr);
    let params = Params::new(config.params_addr.clone());
    let perps = Perps::new(config.perps_addr.clone());
    let oracle = Oracle::new(config.oracle_addr.clone());

    // Fresh state info
    let mars_positions = credit_manager.query_positions(&deps.querier, credit_account_id)?;
    let perps_market = perps.query_perp_market_state(&deps.querier, &market_config.perp_denom)?;

    let PositionDeltas {
        funding_delta,
        borrow_delta,
        spot_delta,
    } = helpers::calculate_deltas(&mars_positions, &market_config, &position_state)?;

    let perp_params = params.query_perp_params(&deps.querier, &market_config.perp_denom)?;

    // We need to hedge the opposite of the spot we just bought.
    let required_hedge_size = Int128::neg(spot_delta);
    let required_hedge_size_unsigned = required_hedge_size.unsigned_abs();
    let spot_execution_price = Decimal::from_ratio(swap_in_amount, spot_delta.unsigned_abs());
    let oracle_price =
        oracle.query_price(&deps.querier, &market_config.perp_denom, ActionKind::Default)?.price;
    let skew =
        Int128::try_from(perps_market.long_oi)?.checked_sub(perps_market.short_oi.try_into()?)?;

    // Validate position entry
    // validate risk conditions

    // Update Position
    let position_state = match increasing {
        true => {
            let perp_execution_price = opening_execution_price(
                skew,
                perp_params.skew_scale,
                required_hedge_size,
                oracle_price,
            )?;
            position_state.increase(
                required_hedge_size.unsigned_abs(),
                spot_execution_price,
                perp_execution_price,
                Int128::try_from(
                    required_hedge_size_unsigned.checked_mul_floor(perp_params.opening_fee_rate)?,
                )?,
                env.block.time.nanos(),
                funding_delta,
                Int128::try_from(borrow_delta)?,
            )
        }
        false => {
            let perp_execution_price = closing_execution_price(
                skew,
                perp_params.skew_scale,
                required_hedge_size,
                oracle_price,
            )?;
            position_state.decrease(
                required_hedge_size.unsigned_abs(),
                spot_execution_price,
                perp_execution_price,
                // TODO can probably make this nicer with little helpers rather than litter this through the codebase
                Int128::try_from(
                    required_hedge_size_unsigned.checked_mul_floor(perp_params.closing_fee_rate)?,
                )?, // todo add fees
                env.block.time.nanos(),
                funding_delta,
                // TODO tidy
                Int128::try_from(borrow_delta)?,
            )
        }
    }?;

    POSITION.save(deps.storage, market_id, &position_state)?;

    // Create the perp order
    let action = Action::ExecutePerpOrder {
        denom: market_config.perp_denom.clone(),
        order_size: required_hedge_size,
        reduce_only: None,
        order_type: None,
    };
    let actions = vec![action];
    let execute_credit_account =
        credit_manager.execute_actions_msg(credit_account_id, actions, &[])?;

    Ok(Response::new()
        .add_message(execute_credit_account)
        .add_attribute("action", "complete_hedge")
        .add_attribute("spot_delta", spot_delta.to_string())
        .add_attribute("funding_delta", funding_delta.to_string())
        .add_attribute("borrow_delta", borrow_delta.to_string()))
}

pub fn add_market(deps: DepsMut, market_config: MarketConfig) -> ContractResult<Response> {
    market_config.validate()?;
    MARKET_CONFIG.save(deps.storage, &market_config.market_id, &market_config)?;
    Ok(Response::new().add_attribute("action", "add_market"))
}

// Currently just a simple deposit owned by the owner
pub fn deposit(deps: DepsMut, info: MessageInfo) -> ContractResult<Response> {
    let config: Config = CONFIG.load(deps.storage)?;
    OWNER.assert_owner(deps.storage, &info.sender)?;

    let credit_manager = CreditManager::new(config.credit_manager_addr);
    let funds = info.funds;

    assert_deposit_funds_valid(&funds, &config.base_denom)?;

    let credit_account_id =
        config.credit_account_id.as_ref().ok_or(ContractError::CreditAccountNotInitialized {})?;
    let coin = &funds[0];
    let actions = vec![credit_manager::Action::Deposit(coin.clone())];

    let execute_credit_account: CosmosMsg =
        credit_manager.execute_actions_msg(credit_account_id, actions, &funds)?;

    Ok(Response::new()
        .add_message(execute_credit_account)
        .add_attribute("action", "deposit")
        .add_attribute("amount", coin.amount.to_string())
        .add_attribute("denom", &coin.denom))
}

pub fn withdraw(
    deps: DepsMut,
    info: MessageInfo,
    amount: Uint128,
    recipient: Option<String>,
) -> ContractResult<Response> {
    let config: Config = CONFIG.load(deps.storage)?;
    let sender = info.sender;
    OWNER.assert_owner(deps.storage, &sender)?;

    // Prevent potentially expensive mistakes
    assert_no_funds(&info.funds)?;

    let recipient = recipient.unwrap_or(sender.to_string());

    let credit_account_id =
        config.credit_account_id.as_ref().ok_or(ContractError::CreditAccountNotInitialized {})?;
    let credit_manager = CreditManager::new(config.credit_manager_addr);
    let actions = vec![credit_manager::Action::WithdrawToWallet {
        coin: ActionCoin {
            denom: config.base_denom,
            amount: ActionAmount::Exact(amount),
        },
        recipient: recipient.clone(),
    }];

    let execute_credit_account =
        credit_manager.execute_actions_msg(credit_account_id, actions, &[])?;

    Ok(Response::new()
        .add_message(execute_credit_account)
        .add_attribute("action", "withdraw")
        .add_attribute("amount", amount.to_string())
        .add_attribute("recipient", recipient))
}
