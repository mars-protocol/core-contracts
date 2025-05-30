use cosmwasm_std::{DepsMut, Response};
use cw2::{assert_contract_version, set_contract_version};

use crate::{
    contract::{CONTRACT_NAME, CONTRACT_VERSION},
    error::ContractError,
    state::NEXT_TRIGGER_ID,
};

const FROM_VERSION: &str = "2.1.0";

pub fn migrate(deps: DepsMut) -> Result<Response, ContractError> {
    // make sure we're migrating the correct contract and from the correct version
    assert_contract_version(deps.storage, &format!("crates.io:{CONTRACT_NAME}"), FROM_VERSION)?;

    NEXT_TRIGGER_ID.save(deps.storage, &1)?;

    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("from_version", FROM_VERSION)
        .add_attribute("to_version", CONTRACT_VERSION))
}
