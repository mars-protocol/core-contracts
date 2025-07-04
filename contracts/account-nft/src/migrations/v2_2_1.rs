use cosmwasm_std::{DepsMut, Response};
use cw2::{assert_contract_version, set_contract_version};
use mars_types::account_nft::{MigrateMsg, NftConfig};

use crate::{
    contract::{CONTRACT_NAME, CONTRACT_VERSION},
    error::ContractError,
    state::CONFIG,
};

const FROM_VERSION: &str = "2.2.0";

pub mod v2_2_0_state {
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::{Addr, Uint128};
    use cw_storage_plus::Item;

    #[cw_serde]
    pub struct NftConfig {
        pub max_value_for_burn: Uint128,
        pub health_contract_addr: Addr,
        pub credit_manager_contract_addr: Addr,
    }

    pub const CONFIG: Item<NftConfig> = Item::new("config");
}

pub fn migrate(deps: DepsMut, msg: MigrateMsg) -> Result<Response, ContractError> {
    // make sure we're migrating the correct contract and from the correct version
    assert_contract_version(deps.storage, &format!("crates.io:{CONTRACT_NAME}"), FROM_VERSION)?;

    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;

    let old_config = v2_2_0_state::CONFIG.load(deps.storage)?;
    v2_2_0_state::CONFIG.remove(deps.storage);
    let new_config = NftConfig {
        max_value_for_burn: old_config.max_value_for_burn,
        address_provider_contract_addr: deps.api.addr_validate(&msg.address_provider)?,
    };
    CONFIG.save(deps.storage, &new_config)?;

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("from_version", FROM_VERSION)
        .add_attribute("to_version", CONTRACT_VERSION))
}
