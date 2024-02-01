use std::collections::HashMap;

use cosmwasm_std::{coin, Decimal, Deps, Order, StdResult, Storage, Uint128};
use cw_storage_plus::Bound;
use mars_types::{
    math::SignedDecimal,
    oracle::ActionKind,
    perps::{
        Accounting, Config, DenomPnlValues, DenomState, DenomStateResponse, DepositResponse,
        PerpDenomState, PerpPosition, PositionResponse, PositionsByAccountResponse,
        RealizedPnlAmounts, TradingFee, UnlockState, VaultState,
    },
};

use crate::{
    denom::{compute_total_accounting_data, compute_total_pnl, DenomStateExt},
    error::ContractResult,
    position::PositionExt,
    pricing::opening_execution_price,
    state::{CONFIG, DENOM_STATES, DEPOSIT_SHARES, POSITIONS, REALIZED_PNL, UNLOCKS, VAULT_STATE},
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
        total_cost_base: ds.total_entry_cost,
        funding: ds.funding,
        last_updated: ds.last_updated,
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
                total_cost_base: ds.total_entry_cost,
                funding: ds.funding,
                last_updated: ds.last_updated,
            })
        })
        .collect()
}

pub fn perp_denom_state(
    deps: Deps,
    current_time: u64,
    denom: String,
) -> ContractResult<PerpDenomState> {
    let ds = DENOM_STATES.load(deps.storage, &denom)?;
    let cfg = CONFIG.load(deps.storage)?;
    let denom_price = cfg.oracle.query_price(&deps.querier, &denom, ActionKind::Default)?.price;
    let base_denom_price =
        cfg.oracle.query_price(&deps.querier, &cfg.base_denom, ActionKind::Default)?.price;
    let (pnl_values, curr_funding) =
        ds.compute_pnl(current_time, denom_price, base_denom_price, cfg.closing_fee_rate)?;
    Ok(PerpDenomState {
        denom,
        enabled: ds.enabled,
        total_entry_cost: ds.total_entry_cost,
        total_entry_funding: ds.total_entry_funding,
        rate: curr_funding.last_funding_rate,
        pnl_values,
    })
}

pub fn deposit(
    deps: Deps,
    depositor: String,
    current_time: u64,
) -> ContractResult<DepositResponse> {
    let cfg = CONFIG.load(deps.storage)?;

    let depositor_addr = deps.api.addr_validate(&depositor)?;
    let vs = VAULT_STATE.load(deps.storage)?;
    let shares =
        DEPOSIT_SHARES.may_load(deps.storage, &depositor_addr)?.unwrap_or_else(Uint128::zero);

    Ok(DepositResponse {
        depositor,
        shares,
        amount: shares_to_amount(&deps, &vs, &cfg.oracle, current_time, &cfg.base_denom, shares)
            .unwrap_or_default(),
    })
}

pub fn deposits(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
    current_time: u64,
) -> ContractResult<Vec<DepositResponse>> {
    let cfg = CONFIG.load(deps.storage)?;

    let vs = VAULT_STATE.load(deps.storage)?;
    let start = start_after.map(|addr| Bound::ExclusiveRaw(addr.into_bytes()));
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    DEPOSIT_SHARES
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (depositor_addr, shares) = item?;
            Ok(DepositResponse {
                depositor: depositor_addr.into(),
                shares,
                amount: shares_to_amount(
                    &deps,
                    &vs,
                    &cfg.oracle,
                    current_time,
                    &cfg.base_denom,
                    shares,
                )
                .unwrap_or_default(),
            })
        })
        .collect()
}

pub fn unlocks(deps: Deps, depositor: String) -> ContractResult<Vec<UnlockState>> {
    let depositor_addr = deps.api.addr_validate(&depositor)?;
    let unlocks = UNLOCKS.may_load(deps.storage, &depositor_addr)?.unwrap_or_default();
    Ok(unlocks)
}

pub fn position(
    deps: Deps,
    current_time: u64,
    account_id: String,
    denom: String,
) -> ContractResult<PositionResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    let denom_price = cfg.oracle.query_price(&deps.querier, &denom, ActionKind::Default)?.price;
    let base_denom_price =
        cfg.oracle.query_price(&deps.querier, &cfg.base_denom, ActionKind::Default)?.price;

    let ds = DENOM_STATES.load(deps.storage, &denom)?;
    let curr_funding = ds.current_funding(current_time, denom_price, base_denom_price)?;
    let position = POSITIONS.load(deps.storage, (&account_id, &denom))?;
    let (pnl, _pnl_amounts) = position.compute_pnl(
        &curr_funding,
        ds.skew()?,
        denom_price,
        base_denom_price,
        &cfg.base_denom,
        cfg.closing_fee_rate,
    )?;

    Ok(PositionResponse {
        account_id,
        position: PerpPosition {
            denom,
            base_denom: cfg.base_denom,
            size: position.size,
            entry_price: position.entry_price,
            current_price: denom_price,
            pnl,
            closing_fee_rate: cfg.closing_fee_rate,
        },
    })
}

pub fn positions(
    deps: Deps,
    current_time: u64,
    start_after: Option<(String, String)>,
    limit: Option<u32>,
) -> ContractResult<Vec<PositionResponse>> {
    let cfg = CONFIG.load(deps.storage)?;

    let start = start_after
        .as_ref()
        .map(|(account_id, denom)| Bound::exclusive((account_id.as_str(), denom.as_str())));
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    // cache the denom state here so that we don't repetitively recalculate them
    let mut denoms: HashMap<String, DenomState> = HashMap::new();

    // cache the prices here so that we don't repetitively query them
    let mut prices: HashMap<String, Decimal> = HashMap::new();

    let base_denom_price =
        cfg.oracle.query_price(&deps.querier, &cfg.base_denom, ActionKind::Default)?.price;

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

            // if denom state is already in the cache, simply read it
            // otherwise, recalculate it, and insert into the cache
            let (pnl, _pnl_amounts) = if let Some(curr_ds) = denoms.get(&denom) {
                position.compute_pnl(
                    &curr_ds.funding,
                    curr_ds.skew()?,
                    current_price,
                    base_denom_price,
                    &cfg.base_denom,
                    cfg.closing_fee_rate,
                )?
            } else {
                let mut ds = DENOM_STATES.load(deps.storage, &denom)?;
                let curr_funding =
                    ds.current_funding(current_time, current_price, base_denom_price)?;
                let pnl = position.compute_pnl(
                    &curr_funding,
                    ds.skew()?,
                    current_price,
                    base_denom_price,
                    &cfg.base_denom,
                    cfg.closing_fee_rate,
                )?;
                ds.funding = curr_funding;
                denoms.insert(denom.clone(), ds);
                pnl
            };

            Ok(PositionResponse {
                account_id,
                position: PerpPosition {
                    denom,
                    base_denom: cfg.base_denom.clone(),
                    size: position.size,
                    entry_price: position.entry_price,
                    current_price,
                    pnl,
                    closing_fee_rate: cfg.closing_fee_rate,
                },
            })
        })
        .collect()
}

pub fn positions_by_account(
    deps: Deps,
    current_time: u64,
    account_id: String,
) -> ContractResult<PositionsByAccountResponse> {
    let cfg = CONFIG.load(deps.storage)?;

    let base_denom_price =
        cfg.oracle.query_price(&deps.querier, &cfg.base_denom, ActionKind::Default)?.price;

    let positions = POSITIONS
        .prefix(&account_id)
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| {
            let (denom, position) = item?;

            let denom_price =
                cfg.oracle.query_price(&deps.querier, &denom, ActionKind::Default)?.price;

            let ds = DENOM_STATES.load(deps.storage, &denom)?;
            let curr_funding = ds.current_funding(current_time, denom_price, base_denom_price)?;

            let (pnl, _pnl_amounts) = position.compute_pnl(
                &curr_funding,
                ds.skew()?,
                denom_price,
                base_denom_price,
                &cfg.base_denom,
                cfg.closing_fee_rate,
            )?;

            Ok(PerpPosition {
                denom,
                base_denom: cfg.base_denom.clone(),
                size: position.size,
                entry_price: position.entry_price,
                current_price: denom_price,
                pnl,
                closing_fee_rate: cfg.closing_fee_rate,
            })
        })
        .collect::<ContractResult<Vec<_>>>()?;

    Ok(PositionsByAccountResponse {
        account_id,
        positions,
    })
}

pub fn total_pnl(deps: Deps, current_time: u64) -> ContractResult<DenomPnlValues> {
    let cfg = CONFIG.load(deps.storage)?;
    compute_total_pnl(&deps, &cfg.oracle, current_time)
}

pub fn opening_fee(deps: Deps, denom: &str, size: SignedDecimal) -> ContractResult<TradingFee> {
    let cfg = CONFIG.load(deps.storage)?;

    let base_denom_price =
        cfg.oracle.query_price(&deps.querier, &cfg.base_denom, ActionKind::Default)?.price;
    let denom_price = cfg.oracle.query_price(&deps.querier, denom, ActionKind::Default)?.price;

    let ds = DENOM_STATES.load(deps.storage, denom)?;

    // TODO: price should be positive
    let denom_exec_price =
        opening_execution_price(ds.skew()?, ds.funding.skew_scale, size, denom_price)?;
    let denom_exec_price = denom_exec_price.abs;

    // fee_in_usd = cfg.opening_fee_rate * denom_exec_price * size
    // fee_in_usdc = fee_in_usd / base_denom_price
    //
    // ceil in favor of the contract
    let price = denom_exec_price.checked_div(base_denom_price)?;
    let fee =
        size.abs.to_uint_floor().checked_mul_ceil(price.checked_mul(cfg.opening_fee_rate)?)?;

    Ok(TradingFee {
        rate: cfg.opening_fee_rate,
        fee: coin(fee.u128(), cfg.base_denom),
    })
}

pub fn denom_accounting(deps: Deps, denom: &str, current_time: u64) -> ContractResult<Accounting> {
    let cfg = CONFIG.load(deps.storage)?;
    let denom_price = cfg.oracle.query_price(&deps.querier, denom, ActionKind::Default)?.price;
    let base_denom_price =
        cfg.oracle.query_price(&deps.querier, &cfg.base_denom, ActionKind::Default)?.price;

    let ds = DENOM_STATES.load(deps.storage, denom)?;
    ds.compute_accounting_data(current_time, denom_price, base_denom_price, cfg.closing_fee_rate)
}

pub fn total_accounting(deps: Deps, current_time: u64) -> ContractResult<Accounting> {
    let cfg = CONFIG.load(deps.storage)?;
    let base_denom_price =
        cfg.oracle.query_price(&deps.querier, &cfg.base_denom, ActionKind::Default)?.price;

    compute_total_accounting_data(&deps, &cfg.oracle, current_time, base_denom_price)
}

pub fn denom_realized_pnl_for_account(
    deps: Deps,
    account_id: String,
    denom: String,
) -> ContractResult<RealizedPnlAmounts> {
    let realized_pnl = REALIZED_PNL.load(deps.storage, (&account_id, &denom))?;
    Ok(realized_pnl)
}
