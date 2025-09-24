use cosmwasm_std::{Coin, Decimal, Deps, Env, Order, StdResult};
use cw_paginate::{
    paginate_map, paginate_map_query, paginate_prefix_query, PaginationResponse, DEFAULT_LIMIT,
    MAX_LIMIT,
};
use cw_storage_plus::Bound;
use mars_types::{
    adapters::vault::{Vault, VaultBase, VaultPosition, VaultPositionValue, VaultUnchecked},
    credit_manager::{
        Account, AccountTierAndDiscountResponse, CoinBalanceResponseItem, ConfigResponse,
        DebtAmount, DebtShares, FeeTierConfigResponse, PerpTradingFeeResponse, Positions,
        SharesResponseItem, SpotTradingFeeResponse, TradingFeeResponse, TriggerOrderResponse,
        VaultBinding, VaultPositionResponseItem, VaultUtilizationResponse,
    },
    health::AccountKind,
    oracle::ActionKind,
    perps::MarketType,
};

use crate::{
    error::ContractResult,
    staking::get_account_tier_and_discount,
    state::{
        ACCOUNT_KINDS, ACCOUNT_NFT, COIN_BALANCES, DEBT_SHARES, FEE_TIER_CONFIG, GOVERNANCE,
        HEALTH_CONTRACT, INCENTIVES, KEEPER_FEE_CONFIG, MAX_SLIPPAGE, MAX_UNLOCKING_POSITIONS,
        ORACLE, OWNER, PARAMS, PERPS, PERPS_LB_RATIO, RED_BANK, REWARDS_COLLECTOR, SWAPPER,
        SWAP_FEE, TOTAL_DEBT_SHARES, TRIGGER_ORDERS, VAULTS, VAULT_POSITIONS, ZAPPER,
    },
    utils::debt_shares_to_amount,
    vault::vault_utilization_in_deposit_cap_denom,
};

pub fn query_accounts(
    deps: Deps,
    owner: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> ContractResult<Vec<Account>> {
    let account_nft = ACCOUNT_NFT.load(deps.storage)?;

    let tokens = account_nft.query_tokens(&deps.querier, owner, start_after, limit)?;
    tokens
        .tokens
        .iter()
        .map(|acc_id| {
            let acc_kind =
                ACCOUNT_KINDS.may_load(deps.storage, acc_id)?.unwrap_or(AccountKind::Default);
            Ok(Account {
                id: acc_id.clone(),
                kind: acc_kind,
            })
        })
        .collect()
}

pub fn query_config(deps: Deps) -> ContractResult<ConfigResponse> {
    Ok(ConfigResponse {
        ownership: OWNER.query(deps.storage)?,
        account_nft: ACCOUNT_NFT.may_load(deps.storage)?.map(|a| a.address().into()),
        red_bank: RED_BANK.load(deps.storage)?.addr.into(),
        incentives: INCENTIVES.load(deps.storage)?.addr.into(),
        oracle: ORACLE.load(deps.storage)?.address().into(),
        params: PARAMS.load(deps.storage)?.address().into(),
        perps: PERPS.load(deps.storage)?.address().into(),
        max_unlocking_positions: MAX_UNLOCKING_POSITIONS.load(deps.storage)?,
        max_slippage: MAX_SLIPPAGE.load(deps.storage)?,
        swapper: SWAPPER.load(deps.storage)?.address().into(),
        zapper: ZAPPER.load(deps.storage)?.address().into(),
        health_contract: HEALTH_CONTRACT.load(deps.storage)?.address().into(),
        rewards_collector: REWARDS_COLLECTOR.may_load(deps.storage)?,
        keeper_fee_config: KEEPER_FEE_CONFIG.load(deps.storage)?,
        perps_liquidation_bonus_ratio: PERPS_LB_RATIO.load(deps.storage)?,
        governance: GOVERNANCE.load(deps.storage)?.into(),
    })
}

pub fn query_swap_fee(deps: Deps) -> ContractResult<Decimal> {
    Ok(SWAP_FEE.load(deps.storage)?)
}

pub fn query_positions(
    deps: Deps,
    account_id: &str,
    action: ActionKind,
) -> ContractResult<Positions> {
    Ok(Positions {
        account_id: account_id.to_string(),
        account_kind: ACCOUNT_KINDS.load(deps.storage, account_id).unwrap_or(AccountKind::Default),
        deposits: query_coin_balances(deps, account_id)?,
        debts: query_debt_amounts(deps, account_id)?,
        lends: RED_BANK.load(deps.storage)?.query_all_lent(&deps.querier, account_id)?,
        vaults: query_vault_positions(deps, account_id)?,
        staked_astro_lps: INCENTIVES
            .load(deps.storage)?
            .query_all_staked_astro_lp_coins(&deps.querier, account_id)?,
        perps: PERPS.load(deps.storage)?.query_positions_by_account(
            &deps.querier,
            account_id,
            action,
        )?,
    })
}

pub fn query_all_coin_balances(
    deps: Deps,
    start_after: Option<(String, String)>,
    limit: Option<u32>,
) -> StdResult<Vec<CoinBalanceResponseItem>> {
    let start = start_after
        .as_ref()
        .map(|(account_id, denom)| Bound::exclusive((account_id.as_str(), denom.as_str())));
    paginate_map(&COIN_BALANCES, deps.storage, start, limit, |(account_id, denom), amount| {
        Ok(CoinBalanceResponseItem {
            account_id,
            denom,
            amount,
        })
    })
}

fn query_debt_amounts(deps: Deps, account_id: &str) -> ContractResult<Vec<DebtAmount>> {
    DEBT_SHARES
        .prefix(account_id)
        .range(deps.storage, None, None, Order::Ascending)
        .map(|res| {
            let (denom, shares) = res?;
            let coin = debt_shares_to_amount(deps, &denom, shares)?;
            Ok(DebtAmount {
                denom,
                shares,
                amount: coin.amount,
            })
        })
        .collect()
}

pub fn query_coin_balances(deps: Deps, account_id: &str) -> ContractResult<Vec<Coin>> {
    COIN_BALANCES
        .prefix(account_id)
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| {
            let (denom, amount) = item?;
            Ok(Coin {
                denom,
                amount,
            })
        })
        .collect()
}

pub fn query_all_debt_shares(
    deps: Deps,
    start_after: Option<(String, String)>,
    limit: Option<u32>,
) -> StdResult<Vec<SharesResponseItem>> {
    let start: Option<Bound<'_, (&str, &str)>> = start_after
        .as_ref()
        .map(|(account_id, denom)| Bound::exclusive((account_id.as_str(), denom.as_str())));
    paginate_map(&DEBT_SHARES, deps.storage, start, limit, |(account_id, denom), shares| {
        Ok(SharesResponseItem {
            account_id,
            denom,
            shares,
        })
    })
}

pub fn query_all_trigger_orders_for_account(
    deps: Deps,
    account_id: String,
    start_after_order_id: Option<String>,
    limit: Option<u32>,
) -> StdResult<PaginationResponse<TriggerOrderResponse>> {
    let start = start_after_order_id.as_ref().map(|order_id| Bound::exclusive(order_id.as_str()));
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT);

    paginate_prefix_query(
        &TRIGGER_ORDERS,
        deps.storage,
        &account_id,
        start,
        Some(limit),
        |_order_id, order| {
            Ok(TriggerOrderResponse {
                account_id: account_id.clone(),
                order,
            })
        },
    )
}

pub fn query_all_trigger_orders(
    deps: Deps,
    start_after: Option<(String, String)>,
    limit: Option<u32>,
) -> StdResult<PaginationResponse<TriggerOrderResponse>> {
    let start: Option<Bound<'_, (&str, &str)>> = start_after
        .as_ref()
        .map(|(account_id, order_id)| Bound::exclusive((account_id.as_str(), order_id.as_str())));
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT);

    paginate_map_query(
        &TRIGGER_ORDERS,
        deps.storage,
        start,
        Some(limit),
        |(account_id, _order_id), order| {
            Ok(TriggerOrderResponse {
                account_id: account_id.to_string(),
                order,
            })
        },
    )
}

pub fn query_vault_utilization(
    deps: Deps,
    env: Env,
    unchecked: VaultUnchecked,
) -> ContractResult<VaultUtilizationResponse> {
    let vault = unchecked.check(deps.api)?;
    let params = PARAMS.load(deps.storage)?;
    let oracle = ORACLE.load(deps.storage)?;
    let vault_config = params.query_vault_config(&deps.querier, &vault.address)?;

    Ok(VaultUtilizationResponse {
        vault: vault.into(),
        utilization: vault_utilization_in_deposit_cap_denom(
            &deps,
            &oracle,
            &vault_config,
            &env.contract.address,
        )?,
    })
}

pub fn query_all_vault_utilizations(
    deps: Deps,
    env: Env,
    start_after: Option<String>,
    limit: Option<u32>,
) -> ContractResult<PaginationResponse<VaultUtilizationResponse>> {
    let params = PARAMS.load(deps.storage)?;
    let oracle = ORACLE.load(deps.storage)?;
    let vault_configs_response =
        params.query_all_vault_configs_v2(&deps.querier, start_after, limit)?;

    let vault_utilizations: ContractResult<Vec<VaultUtilizationResponse>> = vault_configs_response
        .data
        .iter()
        .map(|vault_config| {
            Ok(VaultUtilizationResponse {
                vault: Vault {
                    address: vault_config.addr.clone(),
                }
                .into(),
                utilization: vault_utilization_in_deposit_cap_denom(
                    &deps,
                    &oracle,
                    vault_config,
                    &env.contract.address,
                )?,
            })
        })
        .collect();

    Ok(PaginationResponse {
        data: vault_utilizations?,
        metadata: vault_configs_response.metadata,
    })
}

pub fn query_vault_positions(deps: Deps, account_id: &str) -> ContractResult<Vec<VaultPosition>> {
    VAULT_POSITIONS
        .prefix(account_id)
        .range(deps.storage, None, None, Order::Ascending)
        .map(|res| {
            let (addr, position) = res?;
            Ok(VaultPosition {
                vault: VaultBase::new(addr),
                amount: position,
            })
        })
        .collect()
}

pub fn query_all_vault_positions(
    deps: Deps,
    start_after: Option<(String, String)>,
    limit: Option<u32>,
) -> StdResult<Vec<VaultPositionResponseItem>> {
    let start = match &start_after {
        Some((account_id, unchecked)) => {
            let addr = deps.api.addr_validate(unchecked)?;
            Some(Bound::exclusive((account_id.as_str(), addr)))
        }
        None => None,
    };
    paginate_map(&VAULT_POSITIONS, deps.storage, start, limit, |(account_id, addr), amount| {
        Ok(VaultPositionResponseItem {
            account_id,
            position: VaultPosition {
                vault: VaultBase::new(addr),
                amount,
            },
        })
    })
}

pub fn query_total_debt_shares(deps: Deps, denom: &str) -> StdResult<DebtShares> {
    let shares = TOTAL_DEBT_SHARES.load(deps.storage, denom)?;
    Ok(DebtShares {
        denom: denom.to_string(),
        shares,
    })
}

pub fn query_all_total_debt_shares(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<DebtShares>> {
    let start = start_after.as_ref().map(|denom| Bound::exclusive(denom.as_str()));
    paginate_map(&TOTAL_DEBT_SHARES, deps.storage, start, limit, |denom, shares| {
        Ok(DebtShares {
            denom,
            shares,
        })
    })
}

pub fn query_vault_position_value(
    deps: Deps,
    vault_position: VaultPosition,
) -> StdResult<VaultPositionValue> {
    let oracle = ORACLE.load(deps.storage)?;
    vault_position.query_values(&deps.querier, &oracle, ActionKind::Default)
}

pub fn query_vault_bindings(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> ContractResult<PaginationResponse<VaultBinding>> {
    let start = start_after.map(|acc_id| Bound::ExclusiveRaw(acc_id.into_bytes()));
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT);

    paginate_map_query(&VAULTS, deps.storage, start, Some(limit), |account_id, vault_addr| {
        Ok(VaultBinding {
            account_id,
            vault_address: vault_addr.to_string(),
        })
    })
}

pub fn query_account_tier_and_discount(
    deps: Deps,
    account_id: &str,
) -> ContractResult<AccountTierAndDiscountResponse> {
    use crate::staking::get_account_tier_and_discount;

    let (tier, discount_pct, voting_power) = get_account_tier_and_discount(deps, account_id)?;

    Ok(AccountTierAndDiscountResponse {
        tier_id: tier.id,
        discount_pct,
        voting_power,
    })
}

pub fn query_trading_fee(
    deps: Deps,
    account_id: &str,
    market_type: &MarketType,
) -> ContractResult<TradingFeeResponse> {
    let (tier, discount_pct, _) = get_account_tier_and_discount(deps, account_id)?;

    match market_type {
        MarketType::Spot => {
            let base_fee_pct = SWAP_FEE.load(deps.storage)?;
            let effective_fee_pct =
                base_fee_pct.checked_mul(cosmwasm_std::Decimal::one() - discount_pct)?;

            Ok(TradingFeeResponse::Spot(SpotTradingFeeResponse {
                base_fee_pct,
                discount_pct,
                effective_fee_pct,
                tier_id: tier.id,
            }))
        }
        MarketType::Perp {
            denom,
        } => {
            let params = PARAMS.load(deps.storage)?;
            let perp_params = params.query_perp_params(&deps.querier, denom)?;

            let opening_fee_pct = perp_params.opening_fee_rate;
            let closing_fee_pct = perp_params.closing_fee_rate;

            let effective_opening_fee_pct =
                opening_fee_pct.checked_mul(cosmwasm_std::Decimal::one() - discount_pct)?;
            let effective_closing_fee_pct =
                closing_fee_pct.checked_mul(cosmwasm_std::Decimal::one() - discount_pct)?;

            Ok(TradingFeeResponse::Perp(PerpTradingFeeResponse {
                opening_fee_pct,
                closing_fee_pct,
                discount_pct,
                effective_opening_fee_pct,
                effective_closing_fee_pct,
                tier_id: tier.id,
            }))
        }
    }
}

pub fn query_fee_tier_config(deps: Deps) -> ContractResult<FeeTierConfigResponse> {
    Ok(FeeTierConfigResponse {
        fee_tier_config: FEE_TIER_CONFIG.load(deps.storage)?,
    })
}
