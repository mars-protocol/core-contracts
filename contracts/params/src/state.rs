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
