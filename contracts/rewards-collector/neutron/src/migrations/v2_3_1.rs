use cosmwasm_std::{DepsMut, Response};
use cw2::set_contract_version;

use crate::entry::{CONTRACT_NAME, CONTRACT_VERSION};
use cw2::assert_contract_version;
use mars_rewards_collector_base::ContractError;

const FROM_VERSION: &str = "2.2.2";

pub fn migrate(deps: DepsMut) -> Result<Response, ContractError> {
    assert_contract_version(deps.storage, &format!("crates.io:{CONTRACT_NAME}"), FROM_VERSION)?;
    set_contract_version(deps.storage, &format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;
    Ok(Response::new().add_attribute("action", "migrate"))
}
