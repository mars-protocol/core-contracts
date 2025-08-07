use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};
use mars_owner::Owner;
use mars_types::params::{AssetParams, PerpParams, VaultConfig};

pub const RISK_MANAGER_KEY: &str = "risk_manager";

pub const OWNER: Owner = Owner::new("owner");
pub const RISK_MANAGER: Owner = Owner::new(RISK_MANAGER_KEY);
pub const ADDRESS_PROVIDER: Item<Addr> = Item::new("address_provider");
pub const MAX_PERP_PARAMS: Item<u8> = Item::new("max_perp_params");
pub const ASSET_PARAMS: Map<&str, AssetParams> = Map::new("asset_params");
pub const VAULT_CONFIGS: Map<&Addr, VaultConfig> = Map::new("vault_configs");
pub const PERP_PARAMS: Map<&str, PerpParams> = Map::new("perp_params");

// Managed vault min creation fee in uusd
pub const MANAGED_VAULT_MIN_CREATION_FEE_IN_UUSD: Item<u128> = Item::new("vault_min_creation_fee");

#[cw_serde]
#[derive(Default)]
pub struct ManagedVaultCodeIds {
    pub code_ids: Vec<u64>,
}

#[cw_serde]
#[derive(Default)]
pub struct BlacklistedVaults {
    pub vaults: Vec<Addr>,
}

// Managed vault allowed code ids
pub const MANAGED_VAULT_CODE_IDS: Item<ManagedVaultCodeIds> = Item::new("vault_code_ids");

pub const BLACKLISTED_VAULTS: Item<BlacklistedVaults> = Item::new("blacklisted_vaults");
