use std::{cmp::max, collections::HashMap};

use cosmwasm_std::{coin, Addr, Decimal, Deps, Int128, Order, StdResult, Storage};
use cw_paginate::{paginate_map_query, PaginationResponse};
use cw_storage_plus::Bound;
use mars_perps_common::pricing::{closing_execution_price, opening_execution_price};
use mars_types::{
    address_provider::{
        helpers::{query_contract_addr, query_contract_addrs},
        MarsAddressType,
    },
    oracle::ActionKind,
    params::PerpParams,
    perps::{
        AccountingResponse, Config, MarketResponse, MarketState, MarketStateResponse, PerpPosition,
        PnlAmounts, PositionFeesResponse, PositionResponse, PositionsByAccountResponse, TradingFee,
        VaultDeposit, VaultPositionResponse, VaultResponse, VaultUnlock,
    },
};

use crate::{
    accounting::AccountingExt,
    error::ContractResult,
    market::{compute_total_accounting_data, MarketStateExt},
    position::{PositionExt, PositionModification},
    state::{
        CONFIG, DEPOSIT_SHARES, MARKET_STATES, POSITIONS, REALIZED_PNL,
        TOTAL_UNLOCKING_OR_UNLOCKED_SHARES, UNLOCKS, VAULT_STATE,
    },
    utils::{create_user_id_key, get_oracle_adapter, get_params_adapter},
    vault::shares_to_amount,
};

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 30;

/// Queries the configuration data from storage.
pub fn query_config(store: &dyn Storage) -> StdResult<Config<String>> {
    CONFIG.load(store).map(Into::into).map_err(Into::into)
}

/// Retrieves and calculates the current state of the vault.
/// This includes querying the base denomination price, computing total accounting data,
/// and calculating metrics like share price and collateralization ratio.
/// Returns a `VaultResponse` with the vault's key financial metrics.
pub fn query_vault(
    deps: Deps,
    current_time: u64,
    action: ActionKind,
) -> ContractResult<VaultResponse> {
    // Load configuration and vault state from storage
    let cfg = CONFIG.load(deps.storage)?;
    let vault_state = VAULT_STATE.load(deps.storage)?;

    let addresses = query_contract_addrs(
        deps,
        &cfg.address_provider,
        vec![MarsAddressType::Oracle, MarsAddressType::Params],
    )?;

    let oracle = get_oracle_adapter(&addresses[&MarsAddressType::Oracle]);
    let params = get_params_adapter(&addresses[&MarsAddressType::Params]);

    // Compute total accounting data and unrealized PnL amount
    let (acc_data, unrealized_pnl_amt) = compute_total_accounting_data(
        &deps,
        &oracle,
        &params,
        current_time,
        &cfg.base_denom,
        action,
    )?;

    // Calculate total withdrawal balance
    let total_withdrawal_balance = acc_data.total_withdrawal_balance(&vault_state)?;

    // Calculate share price if total shares are non-zero
    let share_price = if vault_state.total_shares.is_zero() {
        None
    } else {
        Some(Decimal::checked_from_ratio(total_withdrawal_balance, vault_state.total_shares)?)
    };

    // Calculate total cash flow
    let total_cash_flow = acc_data.cash_flow.total()?.checked_add(vault_state.total_balance)?;
    let total_cash_flow = max(total_cash_flow, Int128::zero()).unsigned_abs();

    // A positive total unrealized PnL indicates profit for traders, which is treated as a liability for the vault.
    // Thus, the vault's debt is equal to the positive unrealized PnL amount, or zero if there is no profit.
    let total_debt = max(unrealized_pnl_amt.pnl, Int128::zero()).unsigned_abs();

    // Calculate collateralization ratio if total debt is non-zero
    let collateralization_ratio = if total_debt.is_zero() {
        None
    } else {
        Some(Decimal::checked_from_ratio(total_cash_flow, total_debt)?)
    };

    // Calculate total unlocking/unlocked shares and amount
    let total_unlocking_or_unlocked_shares =
        TOTAL_UNLOCKING_OR_UNLOCKED_SHARES.may_load(deps.storage)?.unwrap_or_default();
    let total_unlocking_or_unlocked_amount = shares_to_amount(
        &vault_state,
        total_unlocking_or_unlocked_shares,
        total_withdrawal_balance,
    )
    .unwrap_or_default();

    // Construct and return the VaultResponse
    Ok(VaultResponse {
        total_balance: vault_state.total_balance,
        total_shares: vault_state.total_shares,
        total_unlocking_or_unlocked_shares,
        total_unlocking_or_unlocked_amount,
        total_withdrawal_balance,
        share_price,
        total_liquidity: total_cash_flow,
        total_debt,
        collateralization_ratio,
    })
}

/// Queries the current state of a specific market based on its denomination.
/// This function returns key details such as the market's enabled status, open interest,
/// and the current funding rate.
pub fn query_market(
    deps: Deps,
    current_time: u64,
    denom: String,
) -> ContractResult<MarketResponse> {
    let ms = MARKET_STATES.load(deps.storage, &denom)?;
    let cfg = CONFIG.load(deps.storage)?;

    let oracle_address = query_contract_addr(deps, &cfg.address_provider, MarsAddressType::Oracle)?;
    let oracle = get_oracle_adapter(&oracle_address);

    let base_denom_price =
        oracle.query_price(&deps.querier, &cfg.base_denom, ActionKind::Default)?.price;
    let denom_price = oracle.query_price(&deps.querier, &denom, ActionKind::Default)?.price;
    let curr_funding = ms.current_funding(current_time, denom_price, base_denom_price)?;

    Ok(MarketResponse {
        denom: denom.clone(),
        enabled: ms.enabled,
        long_oi: ms.long_oi,
        short_oi: ms.short_oi,
        current_funding_rate: curr_funding.last_funding_rate,
    })
}

/// Retrieves a paginated list of markets.
/// This function queries multiple markets, providing a response with details for each market
/// including its current funding rate, open interest, and enabled status.
pub fn query_markets(
    deps: Deps,
    current_time: u64,
    start_after: Option<String>,
    limit: Option<u32>,
) -> ContractResult<PaginationResponse<MarketResponse>> {
    let cfg = CONFIG.load(deps.storage)?;

    let oracle_address = query_contract_addr(deps, &cfg.address_provider, MarsAddressType::Oracle)?;
    let oracle = get_oracle_adapter(&oracle_address);

    let base_denom_price =
        oracle.query_price(&deps.querier, &cfg.base_denom, ActionKind::Default)?.price;

    let start = start_after.as_ref().map(|start_after| Bound::exclusive(start_after.as_str()));
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT);

    paginate_map_query(&MARKET_STATES, deps.storage, start, Some(limit), |denom, ms| {
        let denom_price = oracle.query_price(&deps.querier, &denom, ActionKind::Default)?.price;
        let curr_funding = ms.current_funding(current_time, denom_price, base_denom_price)?;

        Ok(MarketResponse {
            denom: denom.clone(),
            enabled: ms.enabled,
            long_oi: ms.long_oi,
            short_oi: ms.short_oi,
            current_funding_rate: curr_funding.last_funding_rate,
        })
    })
}

/// Queries the current vault position of a specific user, including both active deposits and pending unlocks.
/// The function returns details such as the amount of shares the user holds, their corresponding value,
/// and any pending unlocks. If the user has no active shares or unlocks, the function returns `None`.
pub fn query_vault_position(
    deps: Deps,
    user_addr: Addr,
    account_id: Option<String>,
    current_time: u64,
) -> ContractResult<Option<VaultPositionResponse>> {
    let cfg = CONFIG.load(deps.storage)?;

    let addresses = query_contract_addrs(
        deps,
        &cfg.address_provider,
        vec![MarsAddressType::Oracle, MarsAddressType::Params],
    )?;

    let oracle = get_oracle_adapter(&addresses[&MarsAddressType::Oracle]);
    let params = get_params_adapter(&addresses[&MarsAddressType::Params]);

    let user_id_key = create_user_id_key(&user_addr, account_id)?;

    let vs = VAULT_STATE.load(deps.storage)?;
    let shares = DEPOSIT_SHARES.may_load(deps.storage, &user_id_key)?;
    let unlocks = UNLOCKS.may_load(deps.storage, &user_id_key)?;

    if shares.is_none() && unlocks.is_none() {
        return Ok(None);
    }

    let (global_acc_data, _) = compute_total_accounting_data(
        &deps,
        &oracle,
        &params,
        current_time,
        &cfg.base_denom,
        ActionKind::Default,
    )?;
    let total_withdrawal_balance = global_acc_data.total_withdrawal_balance(&vs)?;

    let shares = shares.unwrap_or_default();
    let perp_vault_deposit = VaultDeposit {
        shares,
        amount: shares_to_amount(&vs, shares, total_withdrawal_balance).unwrap_or_default(),
    };

    let unlocks = unlocks.unwrap_or_default();
    let unlocks: ContractResult<Vec<_>> = unlocks
        .into_iter()
        .map(|unlock| {
            Ok(VaultUnlock {
                created_at: unlock.created_at,
                cooldown_end: unlock.cooldown_end,
                shares: unlock.shares,
                amount: shares_to_amount(&vs, unlock.shares, total_withdrawal_balance)
                    .unwrap_or_default(),
            })
        })
        .collect();

    Ok(Some(VaultPositionResponse {
        denom: cfg.base_denom.clone(),
        deposit: perp_vault_deposit,
        unlocks: unlocks?,
    }))
}

/// Queries the current position for a given account and market (denom).
/// It calculates the position's current state, including unrealized and realized PnL,
/// based on the latest market data, funding rates, and any potential order size modification.
/// Returns a `PositionResponse` containing the position details or `None` if no position exists.
pub fn query_position(
    deps: Deps,
    current_time: u64,
    account_id: String,
    denom: String,
    order_size: Option<Int128>,
    reduce_only: Option<bool>,
) -> ContractResult<PositionResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    let addresses = query_contract_addrs(
        deps,
        &cfg.address_provider,
        vec![MarsAddressType::Oracle, MarsAddressType::Params],
    )?;

    let oracle = get_oracle_adapter(&addresses[&MarsAddressType::Oracle]);
    let params = get_params_adapter(&addresses[&MarsAddressType::Params]);

    let denom_price = oracle.query_price(&deps.querier, &denom, ActionKind::Default)?.price;
    let base_denom_price =
        oracle.query_price(&deps.querier, &cfg.base_denom, ActionKind::Default)?.price;
    let perp_params = params.query_perp_params(&deps.querier, &denom)?;

    let ms = MARKET_STATES.load(deps.storage, &denom)?;
    let curr_funding = ms.current_funding(current_time, denom_price, base_denom_price)?;
    let position_opt = POSITIONS.may_load(deps.storage, (&account_id, &denom))?;

    let Some(position) = position_opt else {
        return Ok(PositionResponse {
            account_id,
            position: None,
        });
    };

    let modification = match order_size {
        Some(order_size_checked) => {
            PositionModification::from_order_size(position.size, order_size_checked, reduce_only)?
        }
        None => PositionModification::Decrease(position.size),
    };

    let pnl_amounts = position.compute_pnl(
        &curr_funding,
        ms.skew()?,
        denom_price,
        base_denom_price,
        perp_params.opening_fee_rate,
        perp_params.closing_fee_rate,
        modification,
    )?;

    let exit_exec_price =
        closing_execution_price(ms.skew()?, curr_funding.skew_scale, position.size, denom_price)?;

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
            unrealized_pnl: pnl_amounts,
            realized_pnl: position.realized_pnl,
        }),
    })
}

/// Queries a list of positions across multiple accounts and markets (denoms).
/// It retrieves the current state of each position, including unrealized and realized PnL,
/// by using the latest market data and funding rates. The function supports pagination
/// through the `start_after` parameter and allows limiting the number of returned positions.
/// To optimize performance, market data and parameters are cached to avoid redundant queries.
pub fn query_positions(
    deps: Deps,
    current_time: u64,
    start_after: Option<(String, String)>,
    limit: Option<u32>,
) -> ContractResult<Vec<PositionResponse>> {
    let cfg = CONFIG.load(deps.storage)?;

    let addresses = query_contract_addrs(
        deps,
        &cfg.address_provider,
        vec![MarsAddressType::Oracle, MarsAddressType::Params],
    )?;

    let oracle = get_oracle_adapter(&addresses[&MarsAddressType::Oracle]);
    let params = get_params_adapter(&addresses[&MarsAddressType::Params]);

    let start = start_after
        .as_ref()
        .map(|(account_id, denom)| Bound::exclusive((account_id.as_str(), denom.as_str())));
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    // Cache the price, params, market state here so that we don't repetitively query/recalculate them
    let mut cache: HashMap<String, (Decimal, PerpParams, MarketState)> = HashMap::new();

    let base_denom_price =
        oracle.query_price(&deps.querier, &cfg.base_denom, ActionKind::Default)?.price;

    POSITIONS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let ((account_id, denom), position) = item?;

            // If price, params, market state are already in the cache, simply read it
            // otherwise, query/recalculate it, and insert into the cache
            let (current_price, perp_params, funding, skew) = if let Some((price, params, ms)) =
                cache.get(&denom)
            {
                (*price, params.clone(), ms.funding.clone(), ms.skew()?)
            } else {
                let price = oracle.query_price(&deps.querier, &denom, ActionKind::Default)?.price;
                let params = params.query_perp_params(&deps.querier, &denom)?;

                let mut ms = MARKET_STATES.load(deps.storage, &denom)?;
                let curr_funding = ms.current_funding(current_time, price, base_denom_price)?;
                let skew = ms.skew()?;
                ms.funding = curr_funding.clone();

                cache.insert(denom.clone(), (price, params.clone(), ms));

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
                    unrealized_pnl: pnl_amounts,
                    realized_pnl: position.realized_pnl,
                }),
            })
        })
        .collect()
}

/// Queries all positions associated with a specific account across various markets (denoms).
/// For each position, the function calculates the current state, including unrealized and realized PnL,
/// using the latest market data, funding rates, and execution prices. The function also optimizes price queries
/// by avoiding unnecessary queries when no positions exist, which is especially important during liquidation scenarios.
pub fn query_positions_by_account(
    deps: Deps,
    current_time: u64,
    account_id: String,
    action: ActionKind,
) -> ContractResult<PositionsByAccountResponse> {
    let cfg = CONFIG.load(deps.storage)?;

    let addresses = query_contract_addrs(
        deps,
        &cfg.address_provider,
        vec![MarsAddressType::Oracle, MarsAddressType::Params],
    )?;

    let oracle = get_oracle_adapter(&addresses[&MarsAddressType::Oracle]);
    let params = get_params_adapter(&addresses[&MarsAddressType::Params]);

    // Don't query the price if there are no positions. This is important during liquidation as
    // the price query might fail (if Default pricing is pased in).
    let mut base_denom_price: Option<Decimal> = None;

    let positions = POSITIONS
        .prefix(&account_id)
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| {
            let (denom, position) = item?;
            let perp_params = params.query_perp_params(&deps.querier, &denom)?;

            let base_denom_price = if let Some(price) = base_denom_price {
                price
            } else {
                let price =
                    oracle.query_price(&deps.querier, &cfg.base_denom, action.clone())?.price;
                base_denom_price = Some(price);
                price
            };

            let denom_price = oracle.query_price(&deps.querier, &denom, action.clone())?.price;

            let ms = MARKET_STATES.load(deps.storage, &denom)?;
            let curr_funding = ms.current_funding(current_time, denom_price, base_denom_price)?;

            let pnl_amounts = position.compute_pnl(
                &curr_funding,
                ms.skew()?,
                denom_price,
                base_denom_price,
                perp_params.opening_fee_rate,
                perp_params.closing_fee_rate,
                PositionModification::Decrease(position.size),
            )?;

            let exit_exec_price = closing_execution_price(
                ms.skew()?,
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
                unrealized_pnl: pnl_amounts,
                realized_pnl: position.realized_pnl,
            })
        })
        .collect::<ContractResult<Vec<_>>>()?;

    Ok(PositionsByAccountResponse {
        account_id,
        positions,
    })
}

/// Retrieves the realized profit and loss (PnL) for a specific account and market.
/// This function loads the realized PnL data from storage using the provided account id and market denomination.
/// Returns the PnL amounts associated with the specified account and market.
pub fn query_realized_pnl_by_account_and_market(
    deps: Deps,
    account_id: String,
    denom: String,
) -> ContractResult<PnlAmounts> {
    let realized_pnl = REALIZED_PNL.load(deps.storage, (&account_id, &denom))?;
    Ok(realized_pnl)
}

/// Queries and calculates the accounting data for a specific market.
/// This function retrieves the current market state, denomination price, and calculates both accounting metrics and unrealized PnL.
/// Returns an `AccountingResponse` containing the computed data for the given market.
pub fn query_market_accounting(
    deps: Deps,
    denom: &str,
    current_time: u64,
) -> ContractResult<AccountingResponse> {
    let cfg = CONFIG.load(deps.storage)?;

    let addresses = query_contract_addrs(
        deps,
        &cfg.address_provider,
        vec![MarsAddressType::Oracle, MarsAddressType::Params],
    )?;

    let oracle = get_oracle_adapter(&addresses[&MarsAddressType::Oracle]);
    let params = get_params_adapter(&addresses[&MarsAddressType::Params]);

    let perp_params = params.query_perp_params(&deps.querier, denom)?;
    let denom_price = oracle.query_price(&deps.querier, denom, ActionKind::Default)?.price;
    let base_denom_price =
        oracle.query_price(&deps.querier, &cfg.base_denom, ActionKind::Default)?.price;

    let ms = MARKET_STATES.load(deps.storage, denom)?;
    let (accounting, unrealized_pnl) = ms.compute_accounting_data(
        current_time,
        denom_price,
        base_denom_price,
        perp_params.closing_fee_rate,
    )?;

    Ok(AccountingResponse {
        accounting,
        unrealized_pnl,
    })
}

/// Computes and retrieves the total accounting data across all markets.
/// This function aggregates the accounting data and unrealized PnL across all markets.
/// Returns an `AccountingResponse` summarizing the total accounting metrics.
pub fn query_total_accounting(deps: Deps, current_time: u64) -> ContractResult<AccountingResponse> {
    let cfg = CONFIG.load(deps.storage)?;

    let addresses = query_contract_addrs(
        deps,
        &cfg.address_provider,
        vec![MarsAddressType::Oracle, MarsAddressType::Params],
    )?;

    let oracle = get_oracle_adapter(&addresses[&MarsAddressType::Oracle]);
    let params = get_params_adapter(&addresses[&MarsAddressType::Params]);

    let (accounting, unrealized_pnl) = compute_total_accounting_data(
        &deps,
        &oracle,
        &params,
        current_time,
        &cfg.base_denom,
        ActionKind::Default,
    )?;

    Ok(AccountingResponse {
        accounting,
        unrealized_pnl,
    })
}

/// Calculates the opening fee for a given position size in a specified market.
/// This function retrieves market and configuration data, including the current prices of the base denomination and the market asset.
/// It then computes the opening trading fee based on the provided position size and market parameters.
/// Returns a `TradingFee` structure containing the fee rate and the calculated fee amount.
pub fn query_opening_fee(deps: Deps, denom: &str, size: Int128) -> ContractResult<TradingFee> {
    let cfg = CONFIG.load(deps.storage)?;
    let ms = MARKET_STATES.load(deps.storage, denom)?;

    let addresses = query_contract_addrs(
        deps,
        &cfg.address_provider,
        vec![MarsAddressType::Oracle, MarsAddressType::Params],
    )?;

    let oracle = get_oracle_adapter(&addresses[&MarsAddressType::Oracle]);
    let params = get_params_adapter(&addresses[&MarsAddressType::Params]);

    let base_denom_price =
        oracle.query_price(&deps.querier, &cfg.base_denom, ActionKind::Default)?.price;
    let denom_price = oracle.query_price(&deps.querier, denom, ActionKind::Default)?.price;
    let perp_params = params.query_perp_params(&deps.querier, denom)?;

    let fees = PositionModification::Increase(size).compute_fees(
        perp_params.opening_fee_rate,
        perp_params.closing_fee_rate,
        denom_price,
        base_denom_price,
        ms.skew()?,
        perp_params.skew_scale,
    )?;

    Ok(TradingFee {
        rate: perp_params.opening_fee_rate,
        fee: coin(fees.opening_fee.unsigned_abs().u128(), cfg.base_denom),
    })
}

/// Computes the fees associated with modifying a position in a specific market.
/// Depending on whether the position is being opened, closed, or flipped, this function calculates the relevant execution prices and fees.
/// It retrieves current market conditions and uses them to determine the opening and closing fees for the new position size.
/// Returns a `PositionFeesResponse` containing the calculated fees and execution prices.
pub fn query_position_fees(
    deps: Deps,
    account_id: &str,
    denom: &str,
    new_size: Int128,
) -> ContractResult<PositionFeesResponse> {
    let cfg = CONFIG.load(deps.storage)?;

    let addresses = query_contract_addrs(
        deps,
        &cfg.address_provider,
        vec![MarsAddressType::Oracle, MarsAddressType::Params],
    )?;

    let oracle = get_oracle_adapter(&addresses[&MarsAddressType::Oracle]);
    let params = get_params_adapter(&addresses[&MarsAddressType::Params]);

    let base_denom_price =
        oracle.query_price(&deps.querier, &cfg.base_denom, ActionKind::Default)?.price;
    let denom_price = oracle.query_price(&deps.querier, denom, ActionKind::Default)?.price;
    let perp_params = params.query_perp_params(&deps.querier, denom)?;
    let ms = MARKET_STATES.load(deps.storage, denom)?;
    let skew_scale = ms.funding.skew_scale;
    let skew = ms.skew()?;

    let mut opening_exec_price = None;
    let mut closing_exec_price = None;
    let position_opt = POSITIONS.may_load(deps.storage, (account_id, denom))?;
    let modification = match position_opt {
        Some(position) => {
            // Calculate the closing price and fee for the `old_size`
            let exec_price = closing_execution_price(skew, skew_scale, position.size, denom_price)?;
            closing_exec_price = Some(exec_price);

            if position.size.is_negative() != new_size.is_negative() && !new_size.is_zero() {
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
        opening_fee: fees.opening_fee.unsigned_abs(),
        closing_fee: fees.closing_fee.unsigned_abs(),
        opening_exec_price,
        closing_exec_price,
    })
}

/// Retrieves the current state of a specified market.
pub fn query_market_state(store: &dyn Storage, denom: String) -> StdResult<MarketStateResponse> {
    let ms = MARKET_STATES.load(store, &denom)?;
    Ok(MarketStateResponse {
        denom,
        market_state: ms,
    })
}
