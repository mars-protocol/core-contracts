use cosmwasm_std::{DepsMut, Response};
use cw2::{assert_contract_version, set_contract_version};
use mars_owner::OwnerInit::SetInitialOwner;

use crate::{
    contract::{CONTRACT_NAME, CONTRACT_VERSION},
    error::ContractError,
    state::{OWNER, RISK_MANAGER},
};

const FROM_VERSION: &str = "2.1.0";

pub fn migrate(deps: DepsMut) -> Result<Response, ContractError> {
    // Make sure we're migrating the correct contract and from the correct version.
    assert_contract_version(deps.storage, &format!("crates.io:{CONTRACT_NAME}"), FROM_VERSION)?;

    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;

    // Since version <= 2.1.0 of the contract didn't have the risk manager storage item, that is initialised in the instantiate function.
    // We need to initialise the risk manager to the default owner of the contract here in the migration.
    let owner = OWNER.query(deps.storage)?.owner.unwrap();
    RISK_MANAGER.initialize(
        deps.storage,
        deps.api,
        SetInitialOwner {
            owner,
        },
    )?;

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("from_version", FROM_VERSION)
        .add_attribute("to_version", CONTRACT_VERSION))
}
