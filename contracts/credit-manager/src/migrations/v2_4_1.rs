use cosmwasm_std::{DepsMut, Response};
use cw2::{assert_contract_version, set_contract_version};

use crate::{contract::CONTRACT_NAME, error::ContractError};

pub const FROM_VERSION: &str = "2.4.0";
pub const TO_VERSION: &str = "2.4.1";

pub fn migrate(deps: DepsMut) -> Result<Response, ContractError> {
    assert_contract_version(deps.storage, &format!("crates.io:{CONTRACT_NAME}"), FROM_VERSION)?;

    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), TO_VERSION)?;

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("from_version", FROM_VERSION)
        .add_attribute("to_version", TO_VERSION))
}
