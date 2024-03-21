use cosmwasm_std::{Addr, StdError, StdResult, Storage, Uint128};
use cw_storage_plus::{Item, Map};
use mars_owner::Owner;
use mars_types::{
    keys::UserIdKey,
    perps::{CashFlow, Config, DenomState, PnlAmounts, Position, UnlockState, VaultState},
};

pub const OWNER: Owner = Owner::new("owner");

pub const CONFIG: Item<Config<Addr>> = Item::new("cfg");

pub const VAULT_STATE: Item<VaultState> = Item::new("gs");

// denom => denom state
pub const DENOM_STATES: Map<&str, DenomState> = Map::new("ds");

// (user, account id) => shares
pub const DEPOSIT_SHARES: Map<&UserIdKey, Uint128> = Map::new("s");

// (user, account id) => unlocks
pub const UNLOCKS: Map<&UserIdKey, Vec<UnlockState>> = Map::new("ul");

// (account_id, denom) => position
pub const POSITIONS: Map<(&str, &str), Position> = Map::new("p");

// (account_id, denom) => realized PnL amounts
pub const REALIZED_PNL: Map<(&str, &str), PnlAmounts> = Map::new("rpnl");

// denom => denom cash flow
pub const DENOM_CASH_FLOW: Map<&str, CashFlow> = Map::new("dcf");

// total cash flow, accumulated across all denoms
pub const TOTAL_CASH_FLOW: Item<CashFlow> = Item::new("tcf");

/// Increase the deposit shares of a depositor by the given amount.
/// Return the updated deposit shares.
pub fn increase_deposit_shares(
    store: &mut dyn Storage,
    user_id_key: &UserIdKey,
    shares: Uint128,
) -> StdResult<Uint128> {
    DEPOSIT_SHARES.update(store, user_id_key, |old_shares| {
        old_shares.unwrap_or_else(Uint128::zero).checked_add(shares).map_err(StdError::overflow)
    })
}

/// Decrease the deposit shares of a depositor by the given amount.
/// Return the updated deposit shares.
/// If the shares is reduced to zero, delete the entry from contract store.
pub fn decrease_deposit_shares(
    store: &mut dyn Storage,
    user_id_key: &UserIdKey,
    shares: Uint128,
) -> StdResult<Uint128> {
    let shares = DEPOSIT_SHARES
        .may_load(store, user_id_key)?
        .unwrap_or_else(Uint128::zero)
        .checked_sub(shares)?;

    if shares.is_zero() {
        DEPOSIT_SHARES.remove(store, user_id_key);
    } else {
        DEPOSIT_SHARES.save(store, user_id_key, &shares)?;
    }

    Ok(shares)
}
