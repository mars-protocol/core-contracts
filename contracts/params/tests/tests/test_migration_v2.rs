use cosmwasm_std::{attr, testing::mock_env, Addr, Empty, Event};
use cw2::{ContractVersion, VersionError};
use mars_params::{
    contract::migrate,
    error::ContractError,
    state::{OWNER, RISK_MANAGER},
};
use mars_testing::mock_dependencies;

#[test]
fn wrong_contract_name() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "contract_xyz", "2.1.0").unwrap();

    let err = migrate(deps.as_mut(), mock_env(), Empty {}).unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongContract {
            expected: "crates.io:mars-params".to_string(),
            found: "contract_xyz".to_string()
        })
    );
}

#[test]
fn wrong_contract_version() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-params", "2.0.0").unwrap();

    let err = migrate(deps.as_mut(), mock_env(), Empty {}).unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongVersion {
            expected: "2.1.0".to_string(),
            found: "2.0.0".to_string()
        })
    );
}

#[test]
fn successful_migration() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-params", "2.1.0").unwrap();

    // Initialize the OWNER storage item so we can later verify the migration has set the risk manager to the owner as default
    let owner = Addr::unchecked("owner");
    let deps_muted = deps.as_mut();
    OWNER
        .initialize(
            deps_muted.storage,
            deps_muted.api,
            mars_owner::OwnerInit::SetInitialOwner {
                owner: owner.to_string(),
            },
        )
        .unwrap();

    let res = migrate(deps.as_mut(), mock_env(), Empty {}).unwrap();

    assert_eq!(res.messages, vec![]);
    assert_eq!(res.events, vec![] as Vec<Event>);
    assert!(res.data.is_none());
    assert_eq!(
        res.attributes,
        vec![attr("action", "migrate"), attr("from_version", "2.1.0"), attr("to_version", "2.2.0")]
    );

    let new_contract_version = ContractVersion {
        contract: "crates.io:mars-params".to_string(),
        version: "2.2.0".to_string(),
    };
    assert_eq!(cw2::get_contract_version(deps.as_ref().storage).unwrap(), new_contract_version);

    assert!(OWNER.is_owner(deps.as_ref().storage, &owner).unwrap());
    // Check that the risk manager has been set to the owner as default
    assert!(RISK_MANAGER.is_owner(deps.as_ref().storage, &owner).unwrap());
}
