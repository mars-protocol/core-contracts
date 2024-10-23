use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, StdError, StdResult, Storage, Uint128};
use cw_storage_plus::{Item, Map};
use mars_owner::Owner;
use mars_types::{
    keys::UserIdKey,
    perps::{CashFlow, Config, MarketState, PnlAmounts, Position, UnlockState, VaultState},
};

#[cw_serde]
pub struct DeleverageRequestTempStorage {
    /// Denom of the requested coin from Credit Manager contract
    pub denom: String,

    /// Contract balance after deleverage in Perps contract
    pub contract_balance: Uint128,

    /// Requested amount of the denom from Credit Manager contract (to cover PnL loss)
    pub requested_amount: Uint128,
}

pub const OWNER: Owner = Owner::new("owner");

pub const CONFIG: Item<Config<Addr>> = Item::new("cfg");

pub const VAULT_STATE: Item<VaultState> = Item::new("vault");

// denom => market state
pub const MARKET_STATES: Map<&str, MarketState> = Map::new("markets");

// (user, account id) => shares
pub const DEPOSIT_SHARES: Map<&UserIdKey, Uint128> = Map::new("deposit_shares");

// (user, account id) => unlocks
pub const UNLOCKS: Map<&UserIdKey, Vec<UnlockState>> = Map::new("unlocks");

// (account_id, denom) => position
pub const POSITIONS: Map<(&str, &str), Position> = Map::new("positions");

// (account_id, denom) => realized PnL amounts
pub const REALIZED_PNL: Map<(&str, &str), PnlAmounts> = Map::new("realized_pnls");

// denom => market cash flow
pub const MARKET_CASH_FLOW: Map<&str, CashFlow> = Map::new("market_cf");

// total cash flow, accumulated across all denoms
pub const TOTAL_CASH_FLOW: Item<CashFlow> = Item::new("total_cf");

// Temporary state to save variables to be used on reply handling
pub const DELEVERAGE_REQUEST_TEMP_STORAGE: Item<DeleverageRequestTempStorage> =
    Item::new("deleverage_req_temp_var");

// Total unlocking shares across all users
pub const TOTAL_UNLOCKING_OR_UNLOCKED_SHARES: Item<Uint128> =
    Item::new("total_unlocking_or_unlocked_shares");

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

/// Increase the total unlocking or unlocked shares by the given amount.
pub fn increase_total_unlocking_or_unlocked_shares(
    store: &mut dyn Storage,
    shares: Uint128,
) -> StdResult<Uint128> {
    let updated_shares = TOTAL_UNLOCKING_OR_UNLOCKED_SHARES
        .may_load(store)?
        .unwrap_or_else(Uint128::zero)
        .checked_add(shares)?;
    TOTAL_UNLOCKING_OR_UNLOCKED_SHARES.save(store, &updated_shares)?;
    Ok(updated_shares)
}

/// Decrease the total unlocking or unlocked shares by the given amount.
pub fn decrease_total_unlocking_or_unlocked_shares(
    store: &mut dyn Storage,
    shares: Uint128,
) -> StdResult<Uint128> {
    let updated_shares = TOTAL_UNLOCKING_OR_UNLOCKED_SHARES.load(store)?.checked_sub(shares)?;
    TOTAL_UNLOCKING_OR_UNLOCKED_SHARES.save(store, &updated_shares)?;
    Ok(updated_shares)
}
