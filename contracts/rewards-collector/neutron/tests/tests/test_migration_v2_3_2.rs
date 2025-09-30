use cosmwasm_std::testing::mock_env;
use cw2::VersionError;
use mars_rewards_collector_base::ContractError;
use mars_rewards_collector_neutron::{entry::migrate, CONTRACT_NAME};
use mars_testing::mock_dependencies;
use mars_types::rewards_collector::NeutronMigrateMsg;

const FROM_VERSION: &str = "2.3.1";
const TO_VERSION: &str = "2.3.2";

#[test]
fn test_successful_migration() {
    let mut deps = mock_dependencies(&[]);

    // Set the contract version to the correct from version
    cw2::set_contract_version(
        &mut deps.storage,
        format!("crates.io:{}", CONTRACT_NAME),
        FROM_VERSION,
    )
    .unwrap();

    // Perform the migration
    let msg = NeutronMigrateMsg::V2_3_1ToV2_3_2 {};
    let res = migrate(deps.as_mut(), mock_env(), msg).unwrap();

    // Check the response attributes
    assert_eq!(res.attributes[0].key, "action");
    assert_eq!(res.attributes[0].value, "migrate");
    assert_eq!(res.attributes[1].key, "from_version");
    assert_eq!(res.attributes[1].value, FROM_VERSION);
    assert_eq!(res.attributes[2].key, "to_version");
    assert_eq!(res.attributes[2].value, TO_VERSION);

    // Verify that the contract version is updated
    let version = cw2::get_contract_version(&deps.storage).unwrap();
    assert_eq!(version.version, TO_VERSION);
    assert_eq!(version.contract, format!("crates.io:{}", CONTRACT_NAME));
}

#[test]
fn test_unsuccessful_migration_from_wrong_version() {
    let mut deps = mock_dependencies(&[]);

    // Set the contract version to a wrong version
    cw2::set_contract_version(&mut deps.storage, format!("crates.io:{}", CONTRACT_NAME), "2.3.0")
        .unwrap();

    // Perform the migration and expect an error
    let msg = NeutronMigrateMsg::V2_3_1ToV2_3_2 {};
    let err = migrate(deps.as_mut(), mock_env(), msg).unwrap_err();

    // Check that the error is a VersionError
    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongVersion {
            expected: FROM_VERSION.to_string(),
            found: "2.3.0".to_string(),
        })
    );
}

#[test]
fn test_unsuccessful_migration_from_wrong_contract_name() {
    let mut deps = mock_dependencies(&[]);

    // Set the contract version with a wrong contract name
    let wrong_contract_name = "wrong-contract-name";
    cw2::set_contract_version(&mut deps.storage, wrong_contract_name, FROM_VERSION).unwrap();

    // Perform the migration and expect an error
    let msg = NeutronMigrateMsg::V2_3_1ToV2_3_2 {};
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
