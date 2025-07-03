use cosmwasm_std::{DepsMut, Response};
use cw2::{assert_contract_version, set_contract_version};

use crate::{contract::CONTRACT_NAME, error::ContractError, state::MAX_TRIGGER_ORDERS};

const FROM_VERSION: &str = "2.2.3";
const TO_VERSION: &str = "2.3.0";

pub fn migrate(deps: DepsMut, max_trigger_orders: u8) -> Result<Response, ContractError> {
    // make sure we're migrating the correct contract and from the correct version
    assert_contract_version(deps.storage, &format!("crates.io:{CONTRACT_NAME}"), FROM_VERSION)?;

    MAX_TRIGGER_ORDERS.save(deps.storage, &max_trigger_orders)?;

    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), TO_VERSION)?;

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("from_version", FROM_VERSION)
        .add_attribute("to_version", TO_VERSION))
}
