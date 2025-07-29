use crate::error::ContractResult;
use crate::helpers::PositionDeltas;
use crate::state::POSITION;
use crate::{helpers, order_validation};
use cosmwasm_std::{
    to_json_binary, CosmosMsg, Decimal, DepsMut, Env, Int128, MessageInfo, Response, Uint128,
    WasmMsg,
};
use mars_types::adapters::params::Params;
use mars_delta_neutral_position::types::Position;
use mars_types::active_delta_neutral::query::Config;
use mars_utils::helpers::uint128_to_int128;

use super::{
    error::ContractError, helpers::validate_swapper_route, msg::ExecuteMsg, state::CONFIG,
};

use mars_types::credit_manager::{self, ActionAmount, ActionCoin};
use mars_types::swapper::SwapperRoute;
use mars_types::adapters::credit_manager::CreditManager;

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
/// * `amount` - Amount of the base asset to trade in both spot and perp markets
/// * `swapper_route` - Detailed path for executing the swap via the Mars credit account
///
/// ## Returns
/// * `Response` on success with messages to execute the spot trade and trigger the hedge completion
/// * `ContractError` if validation fails or if any operation cannot be completed
pub fn increase(
    deps: DepsMut,
    env: Env,
    denom: &str,
    amount: Uint128,
    swapper_route: &SwapperRoute,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage, denom)?;

    validate_swapper_route(swapper_route, &config.spot_denom, &config.perp_denom)?;

    let credit_manager = CreditManager::new(config.credit_manager_addr);

    // TODO correct this swap
    // Sell the USDC
    let actions = vec![credit_manager::Action::SwapExactIn {
        coin_in: ActionCoin {
            amount: ActionAmount::Exact(amount),
            denom: config.spot_denom.clone(),
        },
        denom_out: config.perp_denom.clone(),
        min_receive: Uint128::zero(),
        route: Some(swapper_route.clone()),
    }];

    let execute_spot_swap =
        credit_manager.execute_actions_msg(&config.credit_account_id, actions)?;

    let complete_hedge = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_json_binary(&ExecuteMsg::CompleteHedge {
            swap_exact_in_amount: amount,
            denom: config.spot_denom.clone(),
            increasing: true,
        })?,
        funds: vec![],
    });

    Ok(Response::new()
        .add_message(execute_spot_swap)
        .add_message(complete_hedge)
        .add_attribute("action", "increase_position"))
}

/// # Execute Decrease Position
///
/// Decreases the delta-neutral position by the specified amount using the provided swapper route.
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
/// * `amount` - Amount of the base asset to reduce from both spot and perp positions
/// * `swapper_route` - Detailed path for executing the swap through Astroport
///
/// ## Returns
/// * `Response` on success with messages to execute the spot trade and trigger the hedge completion
/// * `ContractError` if validation fails, if the position is too small, or if any operation cannot be completed
pub fn decrease(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    amount: Uint128,
    denom: &str,
    swapper_route: &SwapperRoute,
) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage, denom)?;

    validate_swapper_route(swapper_route, &config.spot_denom, &config.perp_denom)?;

    let credit_manager = CreditManager::new(config.credit_manager_addr);

    // Execute the spot decrease operation
    let actions = vec![credit_manager::Action::SwapExactIn {
        coin_in: ActionCoin {
            amount: ActionAmount::Exact(amount),
            denom: config.spot_denom.clone(),
        },
        denom_out: config.perp_denom.clone(),
        min_receive: Uint128::zero(),
        route: Some(swapper_route.clone()),
    }];

    let execute_spot_swap =
        credit_manager.execute_actions_msg(&config.credit_account_id, actions)?;

    // Complete the hedge by calling an internal hedge function
    let complete_hedge = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_json_binary(&ExecuteMsg::CompleteHedge {
            swap_exact_in_amount: amount,
            denom: config.spot_denom.clone(),
            increasing: false,
        })?,
        funds: vec![],
    });

    Ok(Response::new()
        .add_message(execute_spot_swap)
        .add_message(complete_hedge)
        .add_attribute("action", "decrease_position"))
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
/// * `previous_balance` - The token balance before the spot trade was executed
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
    denom: &str,
    increasing: bool,
) -> ContractResult<Response> {
    if info.sender != env.contract.address {
        return Err(ContractError::Unauthorized {});
    }

    // State variables
    let config = CONFIG.load(deps.storage, denom)?;
    let mut position_state: Position = POSITION.load(deps.storage, denom)?;

    // Contract adapters
    let credit_manager = CreditManager::new(config.credit_manager_addr.clone());
    let params = Params::new(deps.api.addr_validate(&config.perp_denom)?);

    // Fresh state info
    let mars_positions = credit_manager.query_positions(&deps.querier, &config.spot_denom)?;

    let PositionDeltas {
        funding_delta,
        borrow_delta,
        spot_delta,
    } = helpers::calculate_deltas(&mars_positions, &config, &position_state)?;

    let perp_params = params.query_perp_params(&deps.querier, &config.perp_denom)?;
    let trading_fee_rate = match increasing {
        true => perp_params.opening_fee_rate,
        false => perp_params.closing_fee_rate,
    };

    let amount = spot_delta.unsigned_abs();
    let spot_execution_price = Decimal::from_ratio(swap_in_amount, amount);
    // TODO calculate perp execution price
    let perp_execution_price = Decimal::zero();

    let perp_trading_fee_amount = amount.checked_mul_floor(trading_fee_rate)?;

    // Validate position entry
    // TODO : return the right data from this
    order_validation::validate_entry()?;

    // Update Position
    let position_state = match increasing {
        true => position_state.increase(
            amount,
            spot_execution_price,
            perp_execution_price,
            Int128::zero(), // todo add fees
            env.block.time.nanos(),
            funding_delta,
            uint128_to_int128(borrow_delta)?,
        ),
        false => position_state.decrease(
            amount,
            spot_execution_price,
            perp_execution_price,
            uint128_to_int128(perp_trading_fee_amount)?,
            env.block.time.nanos(),
            funding_delta,
            uint128_to_int128(borrow_delta)?,
        ),
    }?;

    POSITION.save(deps.storage, denom, &position_state)?;

    Ok(Response::new()
        .add_attribute("action", "complete_hedge")
        .add_attribute("spot_delta", spot_delta.to_string())
        .add_attribute("funding_delta", funding_delta.to_string())
        .add_attribute("borrow_delta", borrow_delta.to_string()))
}
