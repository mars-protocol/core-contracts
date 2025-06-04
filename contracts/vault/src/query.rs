use cosmwasm_std::{Addr, Decimal, Deps, Int256, Order, Uint128};
use cw_paginate::{paginate_map_query, PaginationResponse, DEFAULT_LIMIT, MAX_LIMIT};
use cw_storage_plus::Bound;

use crate::{
    error::ContractResult,
    execute::total_base_tokens_in_account,
    msg::{UserPnlResponse, VaultInfoResponseExt, VaultUnlock},
    pnl,
    state::{
        BASE_TOKEN, COOLDOWN_PERIOD, CREDIT_MANAGER, DESCRIPTION, PERFORMANCE_FEE_CONFIG, SUBTITLE,
        TITLE, UNLOCKS, VAULT_ACC_ID, VAULT_TOKEN,
    },
    vault_token::{calculate_base_tokens, calculate_vault_tokens},
};

pub fn query_vault_info(deps: Deps) -> ContractResult<VaultInfoResponseExt> {
    let vault_token = VAULT_TOKEN.load(deps.storage)?;
    let total_vault_tokens = vault_token.query_total_supply(deps)?;

    // If vault account is not set, we don't calculate share price.
    // It means that the vault is not binded to any account yet.
    let vault_account_id_opt = VAULT_ACC_ID.may_load(deps.storage)?;
    let mut total_base_tokens = Uint128::zero();
    let mut share_price = None;
    if vault_account_id_opt.is_some() {
        total_base_tokens = total_base_tokens_in_account(deps)?;
        share_price = if total_vault_tokens.is_zero() {
            None
        } else {
            Some(Decimal::checked_from_ratio(total_base_tokens, total_vault_tokens)?)
        };
    }
    Ok(VaultInfoResponseExt {
        base_token: BASE_TOKEN.load(deps.storage)?,
        vault_token: vault_token.to_string(),
        title: TITLE.may_load(deps.storage)?,
        subtitle: SUBTITLE.may_load(deps.storage)?,
        description: DESCRIPTION.may_load(deps.storage)?,
        credit_manager: CREDIT_MANAGER.load(deps.storage)?,
        vault_account_id: vault_account_id_opt,
        cooldown_period: COOLDOWN_PERIOD.load(deps.storage)?,
        performance_fee_config: PERFORMANCE_FEE_CONFIG.load(deps.storage)?,
        total_base_tokens,
        total_vault_tokens,
        share_price,
    })
}

pub fn query_user_unlocks(deps: Deps, user_addr: Addr) -> ContractResult<Vec<VaultUnlock>> {
    let vault_token_supply = VAULT_TOKEN.load(deps.storage)?.query_total_supply(deps)?;
    let total_base_tokens = total_base_tokens_in_account(deps)?;

    UNLOCKS
        .prefix(user_addr.as_str())
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| {
            let (_created_at, unlock) = item?;
            let base_tokens =
                calculate_base_tokens(unlock.vault_tokens, total_base_tokens, vault_token_supply)?;
            Ok(VaultUnlock {
                user_address: user_addr.to_string(),
                created_at: unlock.created_at,
                cooldown_end: unlock.cooldown_end,
                vault_tokens: unlock.vault_tokens,
                base_tokens,
            })
        })
        .collect()
}

pub fn query_all_unlocks(
    deps: Deps,
    start_after: Option<(String, u64)>,
    limit: Option<u32>,
) -> ContractResult<PaginationResponse<VaultUnlock>> {
    let start = start_after
        .as_ref()
        .map(|(user_addr, created_at)| Bound::exclusive((user_addr.as_str(), *created_at)));
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT);

    let vault_token_supply = VAULT_TOKEN.load(deps.storage)?.query_total_supply(deps)?;
    let total_base_tokens = total_base_tokens_in_account(deps)?;

    paginate_map_query(
        &UNLOCKS,
        deps.storage,
        start,
        Some(limit),
        |(user_addr, _created_at), unlock| {
            let base_tokens =
                calculate_base_tokens(unlock.vault_tokens, total_base_tokens, vault_token_supply)?;
            Ok(VaultUnlock {
                user_address: user_addr,
                created_at: unlock.created_at,
                cooldown_end: unlock.cooldown_end,
                vault_tokens: unlock.vault_tokens,
                base_tokens,
            })
        },
    )
}

pub fn convert_to_vault_tokens(deps: Deps, amount: Uint128) -> ContractResult<Uint128> {
    let vault_token_supply = VAULT_TOKEN.load(deps.storage)?.query_total_supply(deps)?;
    let total_base_tokens = total_base_tokens_in_account(deps)?;
    Ok(calculate_vault_tokens(amount, total_base_tokens, vault_token_supply)?)
}

pub fn convert_to_base_tokens(deps: Deps, amount: Uint128) -> ContractResult<Uint128> {
    let vault_token_supply = VAULT_TOKEN.load(deps.storage)?.query_total_supply(deps)?;
    let total_base_tokens = total_base_tokens_in_account(deps)?;
    Ok(calculate_base_tokens(amount, total_base_tokens, vault_token_supply)?)
}

/// Query the PNL for a specific user
pub fn query_user_pnl(deps: Deps, user_address: String) -> ContractResult<UserPnlResponse> {
    let user_addr = deps.api.addr_validate(&user_address)?;

    let net_worth_now = total_base_tokens_in_account(deps)?;

    // get user's vault token balance
    let vault_token = VAULT_TOKEN.load(deps.storage)?;
    let vault_token_supply = vault_token.query_total_supply(deps)?;
    let user_shares = vault_token.query_balance(deps, &user_addr)?;

    // if user has no shares, return zero PNL
    if user_shares.is_zero() {
        return Ok(crate::msg::UserPnlResponse {
            pnl: Int256::zero(),
            shares: Uint128::zero(),
        });
    }

    // calculate user's PNL
    let (vault_pnl_index, _) =
        pnl::query_current_vault_pnl_index(deps.storage, net_worth_now, vault_token_supply)?;
    let user_pnl = pnl::query_user_pnl(deps.storage, &user_addr, user_shares, vault_pnl_index)?;

    Ok(crate::msg::UserPnlResponse {
        pnl: user_pnl,
        shares: user_shares,
    })
}

/// Query the total PNL for the vault
pub fn query_vault_pnl(deps: Deps) -> ContractResult<crate::msg::VaultPnlResponse> {
    let total_base_tokens = total_base_tokens_in_account(deps)?;
    let vault_token = VAULT_TOKEN.load(deps.storage)?;
    let total_shares = vault_token.query_total_supply(deps)?;

    // If there are no shares, return zero PNL
    if total_shares.is_zero() {
        return Ok(crate::msg::VaultPnlResponse {
            total_pnl: Int256::zero(),
            total_shares: Uint128::zero(),
        });
    }

    // Calculate total vault PNL
    let vault_pnl = pnl::query_vault_pnl(deps.storage, total_base_tokens)?;

    Ok(crate::msg::VaultPnlResponse {
        total_pnl: vault_pnl,
        total_shares,
    })
}
