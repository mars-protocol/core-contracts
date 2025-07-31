use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response,
};
use mars_types::active_delta_neutral::{
    execute::ExecuteMsg, instantiate::InstantiateMsg, query::QueryMsg,
};

use crate::{
    error::ContractResult,
    execute, instantiate,
    query::{query_all_market_configs, query_config, query_market_config},
    reply,
};

/// Handles execution of contract messages for the delta-neutral strategy.
///
/// Routes incoming `ExecuteMsg` variants to the appropriate handler:
/// - `Increase`: Opens or increases a delta-neutral position.
/// - `Decrease`: Reduces an existing position.
/// - `CompleteHedge`: Internal operation to rebalance and maintain delta neutrality.
///
/// # Parameters
/// - `deps`: Mutable dependencies for storage and queries.
/// - `env`: Current blockchain environment.
/// - `info`: Message sender and attached funds.
/// - `msg`: The execution message to process.
///
/// # Returns
/// - `ContractResult<Response>`: Standard CosmWasm contract response or error.
///
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::Increase {
            amount,
            denom,
            swapper_route,
        } => execute::buy(deps, env, &denom, amount, &swapper_route),

        ExecuteMsg::Decrease {
            amount,
            denom,
            swapper_route,
        } => execute::sell(deps, env, info, amount, &denom, &swapper_route),

        ExecuteMsg::AddMarket {
            config,
        } => execute::add_market(deps, env, config),

        // For internal operations
        ExecuteMsg::CompleteHedge {
            swap_exact_in_amount,
            denom,
            increasing,
        } => execute::hedge(deps, env, info, swap_exact_in_amount, &denom, increasing), // Add additional routes like Withdraw etc. here
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _: Env, reply: Reply) -> ContractResult<Response> {
    reply::reply(deps, reply)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    instantiate::instantiate(deps, env, info, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    let res = match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::MarketConfig {
            market_id,
        } => to_json_binary(&query_market_config(deps, market_id)?),
        QueryMsg::MarketConfigs {
            start_after,
            limit,
        } => to_json_binary(&query_all_market_configs(deps, start_after, limit)?),
    };
    res.map_err(Into::into)
}
