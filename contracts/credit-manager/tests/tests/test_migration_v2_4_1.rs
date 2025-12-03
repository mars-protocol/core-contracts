use cosmwasm_std::{attr, testing::mock_env};
use cw2::{ContractVersion, VersionError};
use mars_credit_manager::{contract::migrate, error::ContractError};
use mars_testing::mock_dependencies;
use mars_types::credit_manager::MigrateMsg;

const CONTRACT_NAME: &str = "crates.io:mars-credit-manager";
const TO_VERSION: &str = "2.4.1";

#[test]
fn wrong_contract_name() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "contract_xyz", "2.4.0").unwrap();

    let err = migrate(deps.as_mut(), mock_env(), MigrateMsg::V2_4_0ToV2_4_1 {}).unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongContract {
            expected: CONTRACT_NAME.to_string(),
            found: "contract_xyz".to_string()
        })
    );
}

#[test]
fn wrong_contract_version() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, CONTRACT_NAME, "2.3.0").unwrap();

    let err = migrate(deps.as_mut(), mock_env(), MigrateMsg::V2_4_0ToV2_4_1 {}).unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongVersion {
            expected: "2.4.0".to_string(),
            found: "2.3.0".to_string()
        })
    );
}

#[test]
fn successful_migration() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, CONTRACT_NAME, "2.4.0").unwrap();

    let res = migrate(deps.as_mut(), mock_env(), MigrateMsg::V2_4_0ToV2_4_1 {}).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "migrate"),
            attr("from_version", "2.4.0"),
            attr("to_version", TO_VERSION)
        ]
    );

    let new_contract_version = ContractVersion {
        contract: CONTRACT_NAME.to_string(),
        version: TO_VERSION.to_string(),
    };
    assert_eq!(cw2::get_contract_version(deps.as_ref().storage).unwrap(), new_contract_version);
}
