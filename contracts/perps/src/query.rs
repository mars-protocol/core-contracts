use std::{cmp::max, collections::HashMap};

use cosmwasm_std::{coin, Addr, Decimal, Deps, Order, StdResult, Storage, Uint128};
use cw_paginate::{paginate_map_query, PaginationResponse};
use cw_storage_plus::Bound;
use mars_types::{
    oracle::ActionKind,
    params::PerpParams,
    perps::{
        Accounting, Config, DenomState, DenomStateResponse, PerpDenomState, PerpPosition,
        PerpVaultDeposit, PerpVaultPosition, PerpVaultUnlock, PnlAmounts, PnlValues,
        PositionFeesResponse, PositionResponse, PositionsByAccountResponse, TradingFee,
        VaultResponse,
    },
    signed_uint::SignedUint,
};

use crate::{
    denom::{compute_total_accounting_data, compute_total_pnl, DenomStateExt},
    error::ContractResult,
    position::{PositionExt, PositionModification},
    pricing::{closing_execution_price, opening_execution_price},
    state::{CONFIG, DENOM_STATES, DEPOSIT_SHARES, POSITIONS, REALIZED_PNL, UNLOCKS, VAULT_STATE},
    utils::create_user_id_key,
    vault::shares_to_amount,
};

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 30;

pub fn config(store: &dyn Storage) -> StdResult<Config<String>> {
    CONFIG.load(store).map(Into::into).map_err(Into::into)
}

pub fn vault(deps: Deps, current_time: u64, action: ActionKind) -> ContractResult<VaultResponse> {
    // Load configuration and vault state from storage
    let cfg = CONFIG.load(deps.storage)?;
    let vault_state = VAULT_STATE.load(deps.storage)?;

    // Query the base denomination price from the oracle
    let base_denom_price =
        cfg.oracle.query_price(&deps.querier, &cfg.base_denom, action.clone())?.price;

    // Compute total accounting data and unrealized PnL amount
    let (acc_data, unrealized_pnl_amt) =
        compute_total_accounting_data(&deps, &cfg.oracle, current_time, base_denom_price, action)?;

    // Calculate total withdrawal balance
    let total_withdrawal_balance =
        acc_data.withdrawal_balance.total.checked_add(vault_state.total_balance)?;
    let total_withdrawal_balance = max(total_withdrawal_balance, SignedUint::zero()).abs;

    // Calculate share price if total shares are non-zero
    let share_price = if vault_state.total_shares.is_zero() {
        None
    } else {
        Some(Decimal::checked_from_ratio(total_withdrawal_balance, vault_state.total_shares)?)
    };

    // Calculate total cash flow
    let total_cash_flow = acc_data.cash_flow.total()?.checked_add(vault_state.total_balance)?;
    let total_cash_flow = max(total_cash_flow, SignedUint::zero()).abs;

    // Calculate total debt
    let total_debt = max(unrealized_pnl_amt.pnl, SignedUint::zero()).abs;

    // Calculate collateralization ratio if total debt is non-zero
    let collateralization_ratio = if total_debt.is_zero() {
        None
    } else {
        Some(Decimal::checked_from_ratio(total_cash_flow, total_debt)?)
    };

    // Construct and return the VaultResponse
    Ok(VaultResponse {
        total_balance: vault_state.total_balance,
        total_shares: vault_state.total_shares,
        total_withdrawal_balance,
        share_price,
        total_liquidity: total_cash_flow,
        total_debt,
        collateralization_ratio,
    })
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
    let base_denom_price =
        cfg.oracle.query_price(&deps.querier, &cfg.base_denom, ActionKind::Default)?.price;

    get_perp_denom_state(deps, &cfg, current_time, denom, ds, base_denom_price)
}

pub fn perp_denom_states(
    deps: Deps,
    current_time: u64,
    start_after: Option<String>,
    limit: Option<u32>,
) -> ContractResult<PaginationResponse<PerpDenomState>> {
    let cfg = CONFIG.load(deps.storage)?;
    let base_denom_price =
        cfg.oracle.query_price(&deps.querier, &cfg.base_denom, ActionKind::Default)?.price;

    let start = start_after.as_ref().map(|start_after| Bound::exclusive(start_after.as_str()));
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT);

    paginate_map_query(&DENOM_STATES, deps.storage, start, Some(limit), |denom, ds| {
        get_perp_denom_state(deps, &cfg, current_time, denom, ds, base_denom_price)
    })
}

pub fn perp_vault_position(
    deps: Deps,
    user_addr: Addr,
    account_id: Option<String>,
    current_time: u64,
) -> ContractResult<Option<PerpVaultPosition>> {
    let cfg = CONFIG.load(deps.storage)?;

    let user_id_key = create_user_id_key(&user_addr, account_id)?;

    let vs = VAULT_STATE.load(deps.storage)?;
    let shares = DEPOSIT_SHARES.may_load(deps.storage, &user_id_key)?;
    let unlocks = UNLOCKS.may_load(deps.storage, &user_id_key)?;

    if shares.is_none() && unlocks.is_none() {
        return Ok(None);
    }

    let shares = shares.unwrap_or_default();
    let perp_vault_deposit = PerpVaultDeposit {
        shares,
        amount: shares_to_amount(
            &deps,
            &vs,
            &cfg.oracle,
            current_time,
            &cfg.base_denom,
            shares,
            ActionKind::Default,
        )
        .unwrap_or_default(),
    };

    let unlocks = unlocks.unwrap_or_default();
    let unlocks: ContractResult<Vec<_>> = unlocks
        .into_iter()
        .map(|unlock| {
            Ok(PerpVaultUnlock {
                created_at: unlock.created_at,
                cooldown_end: unlock.cooldown_end,
                shares: unlock.shares,
                amount: shares_to_amount(
                    &deps,
                    &vs,
                    &cfg.oracle,
                    current_time,
                    &cfg.base_denom,
                    unlock.shares,
                    ActionKind::Default,
                )
                .unwrap_or_default(),
            })
        })
        .collect();

    Ok(Some(PerpVaultPosition {
        denom: cfg.base_denom.clone(),
        deposit: perp_vault_deposit,
        unlocks: unlocks?,
    }))
}

pub fn deposit(
    deps: Deps,
    user_addr: Addr,
    account_id: Option<String>,
    current_time: u64,
) -> ContractResult<PerpVaultDeposit> {
    let cfg = CONFIG.load(deps.storage)?;

    let user_id_key = create_user_id_key(&user_addr, account_id)?;

    let vs = VAULT_STATE.load(deps.storage)?;
    let shares = DEPOSIT_SHARES.may_load(deps.storage, &user_id_key)?.unwrap_or_else(Uint128::zero);

    Ok(PerpVaultDeposit {
        shares,
        amount: shares_to_amount(
            &deps,
            &vs,
            &cfg.oracle,
            current_time,
            &cfg.base_denom,
            shares,
            ActionKind::Default,
        )
        .unwrap_or_default(),
    })
}

pub fn unlocks(
    deps: Deps,
    user_addr: Addr,
    account_id: Option<String>,
    current_time: u64,
) -> ContractResult<Vec<PerpVaultUnlock>> {
    let cfg = CONFIG.load(deps.storage)?;

    let user_id_key = create_user_id_key(&user_addr, account_id)?;

    let vs = VAULT_STATE.load(deps.storage)?;

    let unlocks = UNLOCKS.may_load(deps.storage, &user_id_key)?.unwrap_or_default();
    unlocks
        .into_iter()
        .map(|unlock| {
            Ok(PerpVaultUnlock {
                created_at: unlock.created_at,
                cooldown_end: unlock.cooldown_end,
                shares: unlock.shares,
                amount: shares_to_amount(
                    &deps,
                    &vs,
                    &cfg.oracle,
                    current_time,
                    &cfg.base_denom,
                    unlock.shares,
                    ActionKind::Default,
                )
                .unwrap_or_default(),
            })
        })
        .collect()
}

pub fn position(
    deps: Deps,
    current_time: u64,
    account_id: String,
    denom: String,
    order_size: Option<SignedUint>,
) -> ContractResult<PositionResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    let denom_price = cfg.oracle.query_price(&deps.querier, &denom, ActionKind::Default)?.price;
    let base_denom_price =
        cfg.oracle.query_price(&deps.querier, &cfg.base_denom, ActionKind::Default)?.price;
    let perp_params = cfg.params.query_perp_params(&deps.querier, &denom)?;
    let ds = DENOM_STATES.load(deps.storage, &denom)?;
    let curr_funding = ds.current_funding(current_time, denom_price, base_denom_price)?;
    let position_opt = POSITIONS.may_load(deps.storage, (&account_id, &denom))?;

    let Some(position) = position_opt else {
        return Ok(PositionResponse {
            account_id,
            position: None,
        });
    };

    let modification = match order_size {
        Some(order_size_checked) => {
            PositionModification::from_order_size(position.size, order_size_checked)?
        }
        None => PositionModification::Decrease(position.size),
    };

    let pnl_amounts = position.compute_pnl(
        &curr_funding,
        ds.skew()?,
        denom_price,
        base_denom_price,
        perp_params.opening_fee_rate,
        perp_params.closing_fee_rate,
        modification,
    )?;

    let exit_exec_price =
        closing_execution_price(ds.skew()?, curr_funding.skew_scale, position.size, denom_price)?;

    Ok(PositionResponse {
        account_id,
        position: Some(PerpPosition {
            denom,
            base_denom: cfg.base_denom,
            size: position.size,
            entry_price: position.entry_price,
            current_price: denom_price,
            entry_exec_price: position.entry_exec_price,
            current_exec_price: exit_exec_price,
            unrealised_pnl: pnl_amounts,
            realised_pnl: position.realized_pnl,
        }),
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

    // cache the price, params, denom state here so that we don't repetitively query/recalculate them
    let mut cache: HashMap<String, (Decimal, PerpParams, DenomState)> = HashMap::new();

    let base_denom_price =
        cfg.oracle.query_price(&deps.querier, &cfg.base_denom, ActionKind::Default)?.price;

    POSITIONS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let ((account_id, denom), position) = item?;

            // if price, params, denom state are already in the cache, simply read it
            // otherwise, query/recalculate it, and insert into the cache
            let (current_price, perp_params, funding, skew) =
                if let Some((price, params, ds)) = cache.get(&denom) {
                    (*price, params.clone(), ds.funding.clone(), ds.skew()?)
                } else {
                    let price =
                        cfg.oracle.query_price(&deps.querier, &denom, ActionKind::Default)?.price;
                    let params = cfg.params.query_perp_params(&deps.querier, &denom)?;

                    let mut ds = DENOM_STATES.load(deps.storage, &denom)?;
                    let curr_funding = ds.current_funding(current_time, price, base_denom_price)?;
                    let skew = ds.skew()?;
                    ds.funding = curr_funding.clone();

                    cache.insert(denom.clone(), (price, params.clone(), ds));

                    (price, params, curr_funding, skew)
                };

            let pnl_amounts = position.compute_pnl(
                &funding,
                skew,
                current_price,
                base_denom_price,
                perp_params.opening_fee_rate,
                perp_params.closing_fee_rate,
                PositionModification::Decrease(position.size),
            )?;

            let exit_exec_price =
                closing_execution_price(skew, funding.skew_scale, position.size, current_price)?;

            Ok(PositionResponse {
                account_id,
                position: Some(PerpPosition {
                    denom,
                    base_denom: cfg.base_denom.clone(),
                    size: position.size,
                    entry_price: position.entry_price,
                    current_price,
                    entry_exec_price: position.entry_exec_price,
                    current_exec_price: exit_exec_price,
                    unrealised_pnl: pnl_amounts,
                    realised_pnl: position.realized_pnl,
                }),
            })
        })
        .collect()
}

pub fn positions_by_account(
    deps: Deps,
    current_time: u64,
    account_id: String,
    action: ActionKind,
) -> ContractResult<PositionsByAccountResponse> {
    let cfg = CONFIG.load(deps.storage)?;

    // Don't query the price if there are no positions. This is important during liquidation as
    // the price query might fail (if Default pricing is pased in).
    let mut base_denom_price: Option<Decimal> = None;

    let positions = POSITIONS
        .prefix(&account_id)
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| {
            let (denom, position) = item?;
            let perp_params = cfg.params.query_perp_params(&deps.querier, &denom)?;

            let base_denom_price = if let Some(price) = base_denom_price {
                price
            } else {
                let price =
                    cfg.oracle.query_price(&deps.querier, &cfg.base_denom, action.clone())?.price;
                base_denom_price = Some(price);
                price
            };

            let denom_price = cfg.oracle.query_price(&deps.querier, &denom, action.clone())?.price;

            let ds = DENOM_STATES.load(deps.storage, &denom)?;
            let curr_funding = ds.current_funding(current_time, denom_price, base_denom_price)?;

            let pnl_amounts = position.compute_pnl(
                &curr_funding,
                ds.skew()?,
                denom_price,
                base_denom_price,
                perp_params.opening_fee_rate,
                perp_params.closing_fee_rate,
                PositionModification::Decrease(position.size),
            )?;

            let exit_exec_price = closing_execution_price(
                ds.skew()?,
                curr_funding.skew_scale,
                position.size,
                denom_price,
            )?;

            Ok(PerpPosition {
                denom,
                base_denom: cfg.base_denom.clone(),
                size: position.size,
                entry_price: position.entry_price,
                current_price: denom_price,
                entry_exec_price: position.entry_exec_price,
                current_exec_price: exit_exec_price,
                unrealised_pnl: pnl_amounts,
                realised_pnl: position.realized_pnl,
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
    compute_total_pnl(&deps, &cfg.oracle, current_time, ActionKind::Default)
}

// TODO: remove this function when frontend is updated (they should use position_fees instead)
pub fn opening_fee(deps: Deps, denom: &str, size: SignedUint) -> ContractResult<TradingFee> {
    let cfg = CONFIG.load(deps.storage)?;
    let ds = DENOM_STATES.load(deps.storage, denom)?;

    let base_denom_price =
        cfg.oracle.query_price(&deps.querier, &cfg.base_denom, ActionKind::Default)?.price;
    let denom_price = cfg.oracle.query_price(&deps.querier, denom, ActionKind::Default)?.price;
    let perp_params = cfg.params.query_perp_params(&deps.querier, denom)?;

    let fees = PositionModification::Increase(size).compute_fees(
        perp_params.opening_fee_rate,
        perp_params.closing_fee_rate,
        denom_price,
        base_denom_price,
        ds.skew()?,
        perp_params.skew_scale,
    )?;

    Ok(TradingFee {
        rate: perp_params.opening_fee_rate,
        fee: coin(fees.opening_fee.abs.u128(), cfg.base_denom),
    })
}

pub fn denom_accounting(deps: Deps, denom: &str, current_time: u64) -> ContractResult<Accounting> {
    let cfg = CONFIG.load(deps.storage)?;
    let perp_params = cfg.params.query_perp_params(&deps.querier, denom)?;
    let denom_price = cfg.oracle.query_price(&deps.querier, denom, ActionKind::Default)?.price;
    let base_denom_price =
        cfg.oracle.query_price(&deps.querier, &cfg.base_denom, ActionKind::Default)?.price;

    let ds = DENOM_STATES.load(deps.storage, denom)?;
    ds.compute_accounting_data(
        current_time,
        denom_price,
        base_denom_price,
        perp_params.closing_fee_rate,
    )
}

pub fn total_accounting(deps: Deps, current_time: u64) -> ContractResult<Accounting> {
    let cfg = CONFIG.load(deps.storage)?;
    let base_denom_price =
        cfg.oracle.query_price(&deps.querier, &cfg.base_denom, ActionKind::Default)?.price;

    let (accounting, _) = compute_total_accounting_data(
        &deps,
        &cfg.oracle,
        current_time,
        base_denom_price,
        ActionKind::Default,
    )?;
    Ok(accounting)
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
    new_size: SignedUint,
) -> ContractResult<PositionFeesResponse> {
    let cfg = CONFIG.load(deps.storage)?;

    let base_denom_price =
        cfg.oracle.query_price(&deps.querier, &cfg.base_denom, ActionKind::Default)?.price;
    let denom_price = cfg.oracle.query_price(&deps.querier, denom, ActionKind::Default)?.price;
    let perp_params = cfg.params.query_perp_params(&deps.querier, denom)?;
    let ds = DENOM_STATES.load(deps.storage, denom)?;
    let skew_scale = ds.funding.skew_scale;
    let skew = ds.skew()?;

    let mut opening_exec_price = None;
    let mut closing_exec_price = None;
    let position_opt = POSITIONS.may_load(deps.storage, (account_id, denom))?;
    let modification = match position_opt {
        Some(position) => {
            // Calculate the closing price and fee for the `old_size`
            let exec_price = closing_execution_price(skew, skew_scale, position.size, denom_price)?;
            closing_exec_price = Some(exec_price);

            if position.size.negative != new_size.negative && !new_size.is_zero() {
                // Position is being flipped

                // Update the skew to reflect the position flip
                let new_skew = skew.checked_sub(position.size)?;

                // Calculate the opening price and fee for the `new_size`
                let exec_price =
                    opening_execution_price(new_skew, skew_scale, new_size, denom_price)?;
                opening_exec_price = Some(exec_price);
            } else if !new_size.is_zero() {
                let exec_price = opening_execution_price(skew, skew_scale, new_size, denom_price)?;
                opening_exec_price = Some(exec_price);
            }

            PositionModification::from_new_size(position.size, new_size)?
        }
        None => {
            let exec_price = opening_execution_price(skew, skew_scale, new_size, denom_price)?;
            opening_exec_price = Some(exec_price);

            PositionModification::Increase(new_size)
        }
    };
    let fees = modification.compute_fees(
        perp_params.opening_fee_rate,
        perp_params.closing_fee_rate,
        denom_price,
        base_denom_price,
        skew,
        perp_params.skew_scale,
    )?;

    Ok(PositionFeesResponse {
        base_denom: cfg.base_denom,
        opening_fee: fees.opening_fee.abs,
        closing_fee: fees.closing_fee.abs,
        opening_exec_price,
        closing_exec_price,
    })
}

fn get_perp_denom_state(
    deps: Deps,
    cfg: &Config<Addr>,
    current_time: u64,
    denom: String,
    ds: DenomState,
    base_denom_price: Decimal,
) -> ContractResult<PerpDenomState> {
    let denom_price = cfg.oracle.query_price(&deps.querier, &denom, ActionKind::Default)?.price;
    let perp_params = cfg.params.query_perp_params(&deps.querier, &denom)?;
    let (pnl_values, curr_funding) =
        ds.compute_pnl(current_time, denom_price, base_denom_price, perp_params.closing_fee_rate)?;
    Ok(PerpDenomState {
        denom: denom.clone(),
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
