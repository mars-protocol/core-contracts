use cosmwasm_std::Deps;
use cw_paginate::{paginate_map_query, PaginationResponse};
use cw_storage_plus::Bound;
use mars_types::active_delta_neutral::query::{Config, MarketConfig};

use crate::{
    error::{ContractError, ContractResult},
    state::{CONFIG, MARKET_CONFIG},
};

pub const DEFAULT_LIMIT: u32 = 10;
pub const MAX_LIMIT: u32 = 30;

pub fn query_config(deps: Deps) -> ContractResult<Config> {
    let config: Config = CONFIG.load(deps.storage)?;
    Ok(config)
}

pub fn query_market_config(deps: Deps, market_id: String) -> ContractResult<MarketConfig> {
    let market_config: MarketConfig = MARKET_CONFIG.load(deps.storage, &market_id)?;
    Ok(market_config)
}

pub fn query_all_market_configs(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<PaginationResponse<MarketConfig>, ContractError> {
    let start = start_after.as_ref().map(|denom| Bound::exclusive(denom.as_str()));
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT);
    paginate_map_query(&MARKET_CONFIG, deps.storage, start, Some(limit), |_res, params| {
        Ok::<MarketConfig, ContractError>(params)
    })
}
