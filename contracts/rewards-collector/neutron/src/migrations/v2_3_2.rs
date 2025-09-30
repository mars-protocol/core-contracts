use cosmwasm_std::{DepsMut, Response};
use cw2::{assert_contract_version, set_contract_version};
use mars_rewards_collector_base::ContractError;

use crate::CONTRACT_NAME;

const FROM_VERSION: &str = "2.3.1";
const TO_VERSION: &str = "2.3.2";

pub fn migrate(deps: DepsMut) -> Result<Response, ContractError> {
    assert_contract_version(deps.storage, &format!("crates.io:{CONTRACT_NAME}"), FROM_VERSION)?;

    // This is a standard migration with no state changes
    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), TO_VERSION)?;

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("from_version", FROM_VERSION)
        .add_attribute("to_version", TO_VERSION))
}
