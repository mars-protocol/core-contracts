use std::collections::HashMap;

use cosmwasm_std::{coin, Decimal, Deps, Order, StdResult, Storage, Uint128};
use cw_storage_plus::Bound;
use mars_types::{
    math::SignedDecimal,
    oracle::ActionKind,
    perps::{
        Accounting, Config, DenomState, DenomStateResponse, DepositResponse, PerpDenomState,
        PerpPosition, PerpVaultDeposit, PerpVaultPosition, PnlAmounts, PnlValues,
        PositionFeesResponse, PositionPnl, PositionResponse, PositionsByAccountResponse,
        TradingFee, UnlockState, VaultState,
    },
};

use crate::{
    denom::{compute_total_accounting_data, compute_total_pnl, DenomStateExt},
    error::ContractResult,
    position::{PositionExt, PositionModification},
    pricing::{closing_execution_price, opening_execution_price},
    state::{CONFIG, DENOM_STATES, DEPOSIT_SHARES, POSITIONS, REALIZED_PNL, UNLOCKS, VAULT_STATE},
    utils::ensure_position_not_flipped,
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
        long_oi: ds.long_oi,
        short_oi: ds.short_oi,
        total_entry_cost: ds.total_entry_cost,
        total_entry_funding: ds.total_entry_funding,
        rate: curr_funding.last_funding_rate,
        pnl_values,
        funding: ds.funding,
    })
}

pub fn perp_vault_position(
    deps: Deps,
    account_id: String,
    current_time: u64,
) -> ContractResult<Option<PerpVaultPosition>> {
    let cfg = CONFIG.load(deps.storage)?;

    let vs = VAULT_STATE.load(deps.storage)?;
    let shares = DEPOSIT_SHARES.may_load(deps.storage, &account_id)?;
    let unlocks = UNLOCKS.may_load(deps.storage, &account_id)?;

    if shares.is_none() && unlocks.is_none() {
        return Ok(None);
    }

    let shares = shares.unwrap_or_default();
    let perp_vault_deposit = PerpVaultDeposit {
        shares,
        amount: shares_to_amount(&deps, &vs, &cfg.oracle, current_time, &cfg.base_denom, shares)
            .unwrap_or_default(),
    };

    let unlocks = unlocks.unwrap_or_default();

    Ok(Some(PerpVaultPosition {
        denom: cfg.base_denom.clone(),
        deposit: perp_vault_deposit,
        unlocks,
    }))
}

pub fn deposit(
    deps: Deps,
    account_id: String,
    current_time: u64,
) -> ContractResult<DepositResponse> {
    let cfg = CONFIG.load(deps.storage)?;

    let vs = VAULT_STATE.load(deps.storage)?;
    let shares = DEPOSIT_SHARES.may_load(deps.storage, &account_id)?.unwrap_or_else(Uint128::zero);

    Ok(DepositResponse {
        account_id,
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
    let start = start_after.map(|acc_id| Bound::ExclusiveRaw(acc_id.into_bytes()));
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    DEPOSIT_SHARES
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (account_id, shares) = item?;
            Ok(DepositResponse {
                account_id,
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

pub fn unlocks(deps: Deps, account_id: String) -> ContractResult<Vec<UnlockState>> {
    let unlocks = UNLOCKS.may_load(deps.storage, &account_id)?.unwrap_or_default();
    Ok(unlocks)
}

pub fn position(
    deps: Deps,
    current_time: u64,
    account_id: String,
    denom: String,
    new_size: Option<SignedDecimal>,
) -> ContractResult<PositionResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    let denom_price = cfg.oracle.query_price(&deps.querier, &denom, ActionKind::Default)?.price;
    let base_denom_price =
        cfg.oracle.query_price(&deps.querier, &cfg.base_denom, ActionKind::Default)?.price;

    let ds = DENOM_STATES.load(deps.storage, &denom)?;
    let curr_funding = ds.current_funding(current_time, denom_price, base_denom_price)?;
    let position = POSITIONS.load(deps.storage, (&account_id, &denom))?;

    // Update the opening fee amount if the position size is increased
    let modification = match new_size {
        Some(ns) if ns.abs > position.size.abs => {
            let q_change = ns.checked_sub(position.size)?;
            PositionModification::Increase(q_change)
        }
        Some(ns) => {
            let q_change = position.size.checked_sub(ns)?;
            PositionModification::Decrease(q_change)
        }
        _ => PositionModification::None,
    };

    let (pnl_values, pnl_amounts) = position.compute_pnl(
        &curr_funding,
        ds.skew()?,
        denom_price,
        base_denom_price,
        cfg.opening_fee_rate,
        cfg.closing_fee_rate,
        modification,
    )?;

    let pnl_coins = pnl_amounts.to_coins(&cfg.base_denom);

    let exit_exec_price =
        closing_execution_price(ds.skew()?, curr_funding.skew_scale, position.size, denom_price)?;

    Ok(PositionResponse {
        account_id,
        position: PerpPosition {
            denom,
            base_denom: cfg.base_denom,
            size: position.size,
            entry_price: position.entry_price,
            current_price: denom_price,
            entry_exec_price: position.entry_exec_price,
            current_exec_price: exit_exec_price.abs,
            unrealised_pnl: PositionPnl {
                values: pnl_values,
                amounts: pnl_amounts,
                coins: pnl_coins,
            },
            realised_pnl: position.realized_pnl,
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
            let (pnl_values, pnl_amounts, skew, skew_scale) =
                if let Some(curr_ds) = denoms.get(&denom) {
                    let (pnl, pnl_amounts) = position.compute_pnl(
                        &curr_ds.funding,
                        curr_ds.skew()?,
                        current_price,
                        base_denom_price,
                        cfg.opening_fee_rate,
                        cfg.closing_fee_rate,
                        PositionModification::None,
                    )?;
                    (pnl, pnl_amounts, curr_ds.skew()?, curr_ds.funding.skew_scale)
                } else {
                    let mut ds = DENOM_STATES.load(deps.storage, &denom)?;
                    let curr_funding =
                        ds.current_funding(current_time, current_price, base_denom_price)?;
                    let (pnl, pnl_amounts) = position.compute_pnl(
                        &curr_funding,
                        ds.skew()?,
                        current_price,
                        base_denom_price,
                        cfg.opening_fee_rate,
                        cfg.closing_fee_rate,
                        PositionModification::None,
                    )?;
                    let skew = ds.skew()?;
                    let skew_scale = curr_funding.skew_scale;
                    ds.funding = curr_funding;
                    denoms.insert(denom.clone(), ds);
                    (pnl, pnl_amounts, skew, skew_scale)
                };

            let exit_exec_price =
                closing_execution_price(skew, skew_scale, position.size, current_price)?;

            let pnl_coins = pnl_amounts.to_coins(&cfg.base_denom);
            Ok(PositionResponse {
                account_id,
                position: PerpPosition {
                    denom,
                    base_denom: cfg.base_denom.clone(),
                    size: position.size,
                    entry_price: position.entry_price,
                    current_price: position.entry_exec_price,
                    entry_exec_price: position.entry_exec_price,
                    current_exec_price: exit_exec_price.abs,
                    unrealised_pnl: PositionPnl {
                        values: pnl_values,
                        amounts: pnl_amounts,
                        coins: pnl_coins,
                    },
                    realised_pnl: position.realized_pnl,
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

            let (pnl_values, pnl_amounts) = position.compute_pnl(
                &curr_funding,
                ds.skew()?,
                denom_price,
                base_denom_price,
                cfg.opening_fee_rate,
                cfg.closing_fee_rate,
                PositionModification::None,
            )?;

            let exit_exec_price = closing_execution_price(
                ds.skew()?,
                curr_funding.skew_scale,
                position.size,
                denom_price,
            )?;

            let pnl_coins = pnl_amounts.to_coins(&cfg.base_denom);
            Ok(PerpPosition {
                denom,
                base_denom: cfg.base_denom.clone(),
                size: position.size,
                entry_price: position.entry_price,
                current_price: denom_price,
                entry_exec_price: position.entry_exec_price,
                current_exec_price: exit_exec_price.abs,
                unrealised_pnl: PositionPnl {
                    values: pnl_values,
                    amounts: pnl_amounts,
                    coins: pnl_coins,
                },
                realised_pnl: position.realized_pnl,
                closing_fee_rate: cfg.closing_fee_rate,
            })
        })
        .collect::<ContractResult<Vec<_>>>()?;

    Ok(PositionsByAccountResponse {
        account_id,
        positions,
    })
}

pub fn total_pnl(deps: Deps, current_time: u64) -> ContractResult<PnlValues> {
    let cfg = CONFIG.load(deps.storage)?;
    compute_total_pnl(&deps, &cfg.oracle, current_time)
}

// FIXME: remove this function when frontend is updated (they should use position_fees instead)
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
) -> ContractResult<PnlAmounts> {
    let realized_pnl = REALIZED_PNL.load(deps.storage, (&account_id, &denom))?;
    Ok(realized_pnl)
}

pub fn position_fees(
    deps: Deps,
    account_id: &str,
    denom: &str,
    new_size: SignedDecimal,
) -> ContractResult<PositionFeesResponse> {
    let cfg = CONFIG.load(deps.storage)?;

    let base_denom_price =
        cfg.oracle.query_price(&deps.querier, &cfg.base_denom, ActionKind::Default)?.price;
    let denom_price = cfg.oracle.query_price(&deps.querier, denom, ActionKind::Default)?.price;

    let ds = DENOM_STATES.load(deps.storage, denom)?;
    let skew_scale = ds.funding.skew_scale;
    let skew = ds.skew()?;

    let position_opt = POSITIONS.may_load(deps.storage, (account_id, denom))?;

    let mut opening_fee = Uint128::zero();
    let mut closing_fee = Uint128::zero();
    let mut opening_exec_price = None;
    let mut closing_exec_price = None;
    match position_opt {
        Some(position) => {
            ensure_position_not_flipped(position.size, new_size)?;

            if position.size.abs > new_size.abs {
                // decrease position size
                let q_change = position.size.checked_sub(new_size)?;
                let denom_exec_price =
                    closing_execution_price(skew, skew_scale, q_change, denom_price)?;
                let denom_exec_price = denom_exec_price.abs;
                closing_fee = calculate_fee(
                    denom_exec_price,
                    base_denom_price,
                    q_change,
                    cfg.closing_fee_rate,
                )?;
                if new_size.is_zero() {
                    closing_exec_price = Some(denom_exec_price);
                } else {
                    opening_exec_price =
                        Some(opening_execution_price(skew, skew_scale, new_size, denom_price)?.abs);
                    closing_exec_price = Some(
                        closing_execution_price(skew, skew_scale, position.size, denom_price)?.abs,
                    );
                }
            } else {
                // increase position size
                let q_change = new_size.checked_sub(position.size)?;
                let denom_exec_price =
                    opening_execution_price(skew, skew_scale, q_change, denom_price)?;
                let denom_exec_price = denom_exec_price.abs;
                opening_fee = calculate_fee(
                    denom_exec_price,
                    base_denom_price,
                    q_change,
                    cfg.opening_fee_rate,
                )?;
                opening_exec_price =
                    Some(opening_execution_price(skew, skew_scale, new_size, denom_price)?.abs);
                closing_exec_price = Some(
                    closing_execution_price(skew, skew_scale, position.size, denom_price)?.abs,
                );
            }
        }
        None => {
            let denom_exec_price =
                opening_execution_price(skew, skew_scale, new_size, denom_price)?;
            let denom_exec_price = denom_exec_price.abs;
            opening_fee =
                calculate_fee(denom_exec_price, base_denom_price, new_size, cfg.opening_fee_rate)?;
            opening_exec_price = Some(denom_exec_price)
        }
    }

    Ok(PositionFeesResponse {
        base_denom: cfg.base_denom,
        opening_fee,
        closing_fee,
        opening_exec_price,
        closing_exec_price,
    })
}

fn calculate_fee(
    denom_exec_price: Decimal,
    base_denom_price: Decimal,
    size: SignedDecimal,
    rate: Decimal,
) -> ContractResult<Uint128> {
    // fee_in_usd = rate * denom_exec_price * size
    // fee_in_usdc = fee_in_usd / base_denom_price
    //
    // ceil in favor of the contract
    let price = denom_exec_price.checked_div(base_denom_price)?;
    Ok(size.abs.to_uint_floor().checked_mul_ceil(price.checked_mul(rate)?)?)
}
