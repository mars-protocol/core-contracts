use cosmwasm_std::{attr, testing::mock_env};
use cw2::{ContractVersion, VersionError};
use mars_params::{
    contract::migrate,
    error::ContractError,
    state::{OWNER, RISK_MANAGER},
};
use mars_testing::mock_dependencies;
use mars_types::params::MigrateMsg;

#[test]
fn wrong_contract_name() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "contract_xyz", "2.2.0").unwrap();

    let err = migrate(deps.as_mut(), mock_env(), MigrateMsg::V2_2_3 {}).unwrap_err();

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
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-params", "4.1.0").unwrap();

    let err = migrate(deps.as_mut(), mock_env(), MigrateMsg::V2_2_3 {}).unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongVersion {
            expected: "2.2.0".to_string(),
            found: "4.1.0".to_string()
        })
    );
}

#[test]
fn successful_migration() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-params", "2.2.0").unwrap();

    // Set up the owner (required for the migration)
    let owner = "owner";
    let deps_mut = deps.as_mut();
    OWNER
        .initialize(
            deps_mut.storage,
            deps_mut.api,
            mars_owner::OwnerInit::SetInitialOwner {
                owner: owner.to_string(),
            },
        )
        .unwrap();

    // Initialize risk manager (required for the migration)
    RISK_MANAGER
        .initialize(
            deps_mut.storage,
            deps_mut.api,
            mars_owner::OwnerInit::SetInitialOwner {
                owner: owner.to_string(),
            },
        )
        .unwrap();

    let res = migrate(deps.as_mut(), mock_env(), MigrateMsg::V2_2_3 {}).unwrap();

    // Verify the response
    assert_eq!(res.messages, vec![]);
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "migrate"),
            attr("from_version", "2.2.0"),
            attr("to_version", env!("CARGO_PKG_VERSION")),
        ]
    );
    assert!(res.data.is_none());

    // Verify the version was updated
    let version = cw2::get_contract_version(deps.as_ref().storage).unwrap();
    assert_eq!(
        version,
        ContractVersion {
            contract: "crates.io:mars-params".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string()
        }
    );
}
