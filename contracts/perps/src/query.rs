use std::collections::HashMap;

use cosmwasm_std::{Decimal, Deps, Order, StdResult, Storage, Uint128};
use cw_storage_plus::Bound;
use mars_types::{
    math::SignedDecimal,
    oracle::ActionKind,
    perps::{
        Config, DenomStateResponse, DepositResponse, PerpPosition, PositionResponse,
        PositionsByAccountResponse, VaultState,
    },
};

use crate::{
    error::ContractResult,
    pnl::{compute_pnl, compute_total_unrealized_pnl},
    state::{CONFIG, DENOM_STATES, DEPOSIT_SHARES, POSITIONS, VAULT_STATE},
    vault::shares_to_amount,
};

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 30;

pub fn config(store: &dyn Storage) -> StdResult<Config<String>> {
    CONFIG.load(store).map(Into::into).map_err(Into::into)
}

pub fn vault_state(store: &dyn Storage) -> StdResult<VaultState> {
    VAULT_STATE.load(store)
}

pub fn denom_state(store: &dyn Storage, denom: String) -> StdResult<DenomStateResponse> {
    let ds = DENOM_STATES.load(store, &denom)?;
    Ok(DenomStateResponse {
        denom,
        enabled: ds.enabled,
        total_size: ds.total_size,
        total_cost_base: ds.total_cost_base,
    })
}

pub fn denom_states(
    store: &dyn Storage,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<DenomStateResponse>> {
    let start = start_after.as_ref().map(|denom| Bound::exclusive(denom.as_str()));
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    DENOM_STATES
        .range(store, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (denom, ds) = item?;
            Ok(DenomStateResponse {
                denom,
                enabled: ds.enabled,
                total_size: ds.total_size,
                total_cost_base: ds.total_cost_base,
            })
        })
        .collect()
}

pub fn deposit(deps: Deps, depositor: String) -> ContractResult<DepositResponse> {
    let depositor_addr = deps.api.addr_validate(&depositor)?;
    let vs = VAULT_STATE.load(deps.storage)?;
    let shares =
        DEPOSIT_SHARES.may_load(deps.storage, &depositor_addr)?.unwrap_or_else(Uint128::zero);

    Ok(DepositResponse {
        depositor,
        shares,
        amount: shares_to_amount(&vs, shares)?,
    })
}

pub fn deposits(
    store: &dyn Storage,
    start_after: Option<String>,
    limit: Option<u32>,
) -> ContractResult<Vec<DepositResponse>> {
    let vs = VAULT_STATE.load(store)?;
    let start = start_after.map(|addr| Bound::ExclusiveRaw(addr.into_bytes()));
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    DEPOSIT_SHARES
        .range(store, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (depositor_addr, shares) = item?;
            Ok(DepositResponse {
                depositor: depositor_addr.into(),
                shares,
                amount: shares_to_amount(&vs, shares)?,
            })
        })
        .collect()
}

pub fn position(deps: Deps, account_id: String, denom: String) -> ContractResult<PositionResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    let current_price = cfg.oracle.query_price(&deps.querier, &denom, ActionKind::Default)?.price;

    let position = POSITIONS.load(deps.storage, (&account_id, &denom))?;
    let pnl = compute_pnl(&position, current_price, &cfg.base_denom)?;

    Ok(PositionResponse {
        account_id,
        position: PerpPosition {
            denom,
            size: position.size,
            entry_price: position.entry_price,
            current_price,
            pnl,
        },
    })
}

pub fn positions(
    deps: Deps,
    start_after: Option<(String, String)>,
    limit: Option<u32>,
) -> ContractResult<Vec<PositionResponse>> {
    let cfg = CONFIG.load(deps.storage)?;

    let start = start_after
        .as_ref()
        .map(|(account_id, denom)| Bound::exclusive((account_id.as_str(), denom.as_str())));
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    // cache the prices here so that we don't repetitively query them
    let mut prices: HashMap<String, Decimal> = HashMap::new();

    POSITIONS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let ((account_id, denom), position) = item?;

            // if price is already in the cache, simply read it
            // otherwise, query it, and insert into the cache
            let current_price = if let Some(price) = prices.get(&denom) {
                *price
            } else {
                let price =
                    cfg.oracle.query_price(&deps.querier, &denom, ActionKind::Default)?.price;
                prices.insert(denom.clone(), price);
                price
            };

            let pnl = compute_pnl(&position, current_price, &cfg.base_denom)?;

            Ok(PositionResponse {
                account_id,
                position: PerpPosition {
                    denom,
                    size: position.size,
                    entry_price: position.entry_price,
                    current_price,
                    pnl,
                },
            })
        })
        .collect()
}

pub fn positions_by_account(
    deps: Deps,
    account_id: String,
) -> ContractResult<PositionsByAccountResponse> {
    let cfg = CONFIG.load(deps.storage)?;

    let positions = POSITIONS
        .prefix(&account_id)
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| {
            let (denom, position) = item?;

            let current_price =
                cfg.oracle.query_price(&deps.querier, &denom, ActionKind::Default)?.price;
            let pnl = compute_pnl(&position, current_price, &cfg.base_denom)?;

            Ok(PerpPosition {
                denom,
                size: position.size,
                entry_price: position.entry_price,
                current_price,
                pnl,
            })
        })
        .collect::<ContractResult<Vec<_>>>()?;

    Ok(PositionsByAccountResponse {
        account_id,
        positions,
    })
}

pub fn total_unrealized_pnl(deps: Deps) -> ContractResult<SignedDecimal> {
    let cfg = CONFIG.load(deps.storage)?;
    compute_total_unrealized_pnl(deps, &cfg.oracle)
}
