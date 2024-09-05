use cosmwasm_std::{ensure_eq, Addr, Deps, DepsMut, Env, Response};
use mars_types::{
    address_provider::{self, MarsAddressType},
    error::MarsError,
    oracle::ActionKind,
    params::PerpParams,
    perps::{Config, Funding, MarketState},
};

use crate::{
    error::{ContractError, ContractResult},
    market::MarketStateExt,
    state::{CONFIG, MARKET_STATES},
};

/// Updates the perp parameters for a given market
pub fn update_market(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    params: PerpParams,
) -> ContractResult<Response> {
    // Current block time
    let current_time = env.block.time.seconds();

    // Load the contract's configuration
    let cfg = CONFIG.load(deps.storage)?;

    // Ensure that the sender is authorized to update the parameters
    assert_is_authorized(&deps, &sender, &cfg.address_provider)?;

    // Try to load the existing state for the given market
    let market_state_opt = MARKET_STATES.may_load(deps.storage, &params.denom)?;

    // Determine the appropriate action based on whether the market state exists
    let market_state = match market_state_opt {
        // If the market exists, update its parameters
        Some(ms) => update_market_state(ms, deps.as_ref(), &cfg, &params, current_time)?,

        // If the market does not exist, initialize a new state
        None => initialize_market_state(&params, current_time),
    };

    // Save the updated market state to storage
    MARKET_STATES.save(deps.storage, &params.denom, &market_state)?;

    // Return a response indicating the success of the update, with relevant attributes
    Ok(Response::new()
        .add_attribute("action", "update_market")
        .add_attribute("denom", params.denom)
        .add_attribute("enabled", params.enabled.to_string())
        .add_attribute("max_funding_velocity", params.max_funding_velocity.to_string())
        .add_attribute("skew_scale", params.skew_scale.to_string()))
}

/// Asserts that the sender is authorized to update the parameters
fn assert_is_authorized(deps: &DepsMut, sender: &Addr, ap_addr: &Addr) -> ContractResult<()> {
    // Get the address of the contract responsible for managing parameters
    let params_addr = address_provider::helpers::query_contract_addr(
        deps.as_ref(),
        ap_addr,
        MarsAddressType::Params,
    )?;

    // Ensure that only the authorized params contract can update the parameters
    ensure_eq!(sender, &params_addr, ContractError::Mars(MarsError::Unauthorized {}));

    Ok(())
}

/// Initializes a new state for a market that does not yet exist
fn initialize_market_state(params: &PerpParams, current_time: u64) -> MarketState {
    MarketState {
        enabled: params.enabled,
        funding: Funding {
            max_funding_velocity: params.max_funding_velocity,
            skew_scale: params.skew_scale,
            ..Default::default()
        },
        last_updated: current_time,
        ..Default::default() // Use default values for other fields
    }
}

/// Updates the state of a given market with new parameters and funding information
fn update_market_state(
    mut market_state: MarketState,
    deps: Deps,
    cfg: &Config<Addr>,
    params: &PerpParams,
    current_time: u64,
) -> ContractResult<MarketState> {
    // If the market is enabled and hasn't been updated in the current block,
    // refresh the funding rate and update its parameters.
    if market_state.enabled && market_state.last_updated != current_time {
        // Query the current price of the market and the base market
        let denom_price =
            cfg.oracle.query_price(&deps.querier, &params.denom, ActionKind::Default)?.price;
        let base_denom_price =
            cfg.oracle.query_price(&deps.querier, &cfg.base_denom, ActionKind::Default)?.price;

        // Refresh the funding rate and index before updating the parameters
        let current_funding =
            market_state.current_funding(current_time, denom_price, base_denom_price)?;
        market_state.funding = current_funding;
    }

    // Update the funding parameters and enable/disable the market
    market_state.funding.max_funding_velocity = params.max_funding_velocity;
    market_state.funding.skew_scale = params.skew_scale;
    market_state.enabled = params.enabled;
    market_state.last_updated = current_time;

    Ok(market_state)
}
