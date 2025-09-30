use cosmwasm_std::{Addr, DepsMut, Response};
use cw2::{assert_contract_version, set_contract_version};
use mars_types::fee_tiers::FeeTierConfig;

use crate::{
    contract::CONTRACT_NAME,
    error::ContractError,
    staking::StakingTierManager,
    state::{FEE_TIER_CONFIG, GOVERNANCE},
};

const FROM_VERSION: &str = "2.3.1";
const TO_VERSION: &str = "2.4.0";

pub fn migrate(
    deps: DepsMut,
    fee_tier_config: FeeTierConfig,
    governance_address: Addr,
) -> Result<Response, ContractError> {
    // make sure we're migrating the correct contract and from the correct version
    assert_contract_version(deps.storage, &format!("crates.io:{CONTRACT_NAME}"), FROM_VERSION)?;

    let manager = StakingTierManager::new(fee_tier_config.clone());
    manager.validate()?;
    // Set the new state items
    FEE_TIER_CONFIG.save(deps.storage, &fee_tier_config)?;
    GOVERNANCE.save(deps.storage, &governance_address)?;

    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), TO_VERSION)?;

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("from_version", FROM_VERSION)
        .add_attribute("to_version", TO_VERSION)
        .add_attribute("fee_tier_config", "set")
        .add_attribute("governance", governance_address))
}
