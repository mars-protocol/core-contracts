use cosmwasm_std::{Addr, Int256, SignedDecimal256, Uint128};
use cw_storage_plus::{Item, Map};
use mars_owner::Owner;

use crate::{
    msg::UnlockState,
    performance_fee::{PerformanceFeeConfig, PerformanceFeeState},
    token_factory::TokenFactoryDenom,
};

pub const OWNER: Owner = Owner::new("owner");

/// The vault token implementation for this vault
pub const VAULT_TOKEN: Item<TokenFactoryDenom> = Item::new("vault_token");

/// The token that is depositable to the vault
pub const BASE_TOKEN: Item<String> = Item::new("base_token");

pub const CREDIT_MANAGER: Item<String> = Item::new("cm_addr");
pub const VAULT_ACC_ID: Item<String> = Item::new("vault_acc_id");

pub const TITLE: Item<String> = Item::new("title");
pub const SUBTITLE: Item<String> = Item::new("subtitle");
pub const DESCRIPTION: Item<String> = Item::new("desc");

pub const COOLDOWN_PERIOD: Item<u64> = Item::new("cooldown_period");
pub const UNLOCKS: Map<(&str, u64), UnlockState> = Map::new("unlocks");

pub const PERFORMANCE_FEE_CONFIG: Item<PerformanceFeeConfig> = Item::new("performance_fee_config");
pub const PERFORMANCE_FEE_STATE: Item<PerformanceFeeState> = Item::new("performance_fee_state");

/// PnL tracking
pub const VAULT_PNL: Item<Int256> = Item::new("vault_pnl");

/// An index of the vault's PnL, used to track what portion of the vaults pnl a user has accumulated
pub const VAULT_PNL_INDEX: Item<SignedDecimal256> = Item::new("vault_pnl_index");

/// The pnl index a user entered with
pub const USER_ENTRY_PNL_INDEX: Map<&Addr, SignedDecimal256> = Map::new("user_entry_pnl_index");

/// The pnl a user has accumulated (not included by the entry pnl index)
pub const USER_TRACKED_PNL: Map<&Addr, Int256> = Map::new("user_tracked_pnl");

/// The last net worth of the vault, used to calculate the vault's pnl delta since last update
pub const LAST_NET_WORTH: Item<Uint128> = Item::new("last_net_worth");
