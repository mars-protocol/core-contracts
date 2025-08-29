use cosmwasm_std::{attr, testing::mock_env};
use cw2::VersionError;
use mars_rewards_collector_base::ContractError;
use mars_rewards_collector_neutron::entry::{migrate, CONTRACT_NAME, CONTRACT_VERSION};
use mars_testing::mock_dependencies;
use mars_types::rewards_collector::NeutronMigrateMsg;

#[test]
fn test_successful_migration() {
    let mut deps = mock_dependencies(&[]);

    // Set the contract version to the old version
    cw2::set_contract_version(&mut deps.storage, format!("crates.io:{}", CONTRACT_NAME), "2.2.2")
        .unwrap();

    // Perform the migration
    let msg = NeutronMigrateMsg::V2_2_2ToV2_3_1 {};
    let res = migrate(deps.as_mut(), mock_env(), msg).unwrap();

    // Check the response attributes
    assert_eq!(res.attributes, vec![attr("action", "migrate"),]);

    // After migration, check that the contract version is updated
    let version = cw2::get_contract_version(&deps.storage).unwrap();
    assert_eq!(version.version, CONTRACT_VERSION);
    assert_eq!(version.contract, format!("crates.io:{}", CONTRACT_NAME));
}

#[test]
fn test_unsuccessful_migration_from_wrong_version() {
    let mut deps = mock_dependencies(&[]);

    // Set the contract version to a wrong version
    cw2::set_contract_version(&mut deps.storage, format!("crates.io:{}", CONTRACT_NAME), "2.0.0")
        .unwrap();

    // Perform the migration and expect an error
    let msg = NeutronMigrateMsg::V2_2_2ToV2_3_1 {};
    let err = migrate(deps.as_mut(), mock_env(), msg).unwrap_err();

    // Check that the error is a VersionError
    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongVersion {
            expected: "2.2.2".to_string(),
            found: "2.0.0".to_string(),
        })
    );
}

#[test]
fn test_unsuccessful_migration_from_wrong_contract_name() {
    let mut deps = mock_dependencies(&[]);

    // Set the contract version with a wrong contract name
    let wrong_contract_name = "wrong-contract-name";
    cw2::set_contract_version(&mut deps.storage, wrong_contract_name, "2.2.2").unwrap();

    // Perform the migration and expect an error
    let msg = NeutronMigrateMsg::V2_2_2ToV2_3_1 {};
    let err = migrate(deps.as_mut(), mock_env(), msg).unwrap_err();

    // Check that the error is a VersionError for wrong contract
    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongContract {
            expected: format!("crates.io:{}", CONTRACT_NAME),
            found: wrong_contract_name.to_string(),
        })
    );
}
