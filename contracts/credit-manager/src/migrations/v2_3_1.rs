use cosmwasm_std::{Decimal, DepsMut, Response};
use cw2::{assert_contract_version, set_contract_version};

use crate::{contract::CONTRACT_NAME, error::ContractError, state::SWAP_FEE};

const FROM_VERSION: &str = "2.3.0";
const TO_VERSION: &str = "2.3.1";

pub fn migrate(deps: DepsMut, swap_fee: Decimal) -> Result<Response, ContractError> {
    // make sure we're migrating the correct contract and from the correct version
    assert_contract_version(deps.storage, &format!("crates.io:{CONTRACT_NAME}"), FROM_VERSION)?;

    SWAP_FEE.save(deps.storage, &swap_fee)?;

    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), TO_VERSION)?;

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("from_version", FROM_VERSION)
        .add_attribute("to_version", TO_VERSION))
}
