use cosmwasm_std::{attr, testing::mock_env, Event};
use cw2::{ContractVersion, VersionError};
use mars_red_bank::{contract::migrate, error::ContractError};
use mars_testing::mock_dependencies;
use mars_types::red_bank::MigrateMsg;

#[test]
fn v2_2_0_to_v2_3_0_wrong_contract_name() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "contract_xyz", "2.2.0").unwrap();

    let err = migrate(deps.as_mut(), mock_env(), MigrateMsg::V2_2_0ToV2_3_0 {}).unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongContract {
            expected: "crates.io:mars-red-bank".to_string(),
            found: "contract_xyz".to_string()
        })
    );
}

#[test]
fn v2_2_0_to_v2_3_0_wrong_contract_version() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-red-bank", "4.1.0").unwrap();

    let err = migrate(deps.as_mut(), mock_env(), MigrateMsg::V2_2_0ToV2_3_0 {}).unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongVersion {
            expected: "2.2.0".to_string(),
            found: "4.1.0".to_string()
        })
    );
}

#[test]
fn v2_2_0_to_v2_3_0_successful_migration() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-red-bank", "2.2.0").unwrap();

    let res = migrate(deps.as_mut(), mock_env(), MigrateMsg::V2_2_0ToV2_3_0 {}).unwrap();

    assert_eq!(res.messages, vec![]);
    assert_eq!(res.events, vec![] as Vec<Event>);
    assert!(res.data.is_none());
    assert_eq!(
        res.attributes,
        vec![attr("action", "migrate"), attr("from_version", "2.2.0"), attr("to_version", "2.3.0")]
    );

    let new_contract_version = ContractVersion {
        contract: "crates.io:mars-red-bank".to_string(),
        version: "2.3.0".to_string(),
    };
    assert_eq!(cw2::get_contract_version(deps.as_ref().storage).unwrap(), new_contract_version);
}

#[test]
fn v2_3_0_to_v2_3_1_wrong_contract_name() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "contract_xyz", "2.3.0").unwrap();

    let err = migrate(deps.as_mut(), mock_env(), MigrateMsg::V2_3_0ToV2_3_1 {}).unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongContract {
            expected: "crates.io:mars-red-bank".to_string(),
            found: "contract_xyz".to_string()
        })
    );
}

#[test]
fn v2_3_0_to_v2_3_1_wrong_contract_version() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-red-bank", "2.2.0").unwrap();

    let err = migrate(deps.as_mut(), mock_env(), MigrateMsg::V2_3_0ToV2_3_1 {}).unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongVersion {
            expected: "2.3.0".to_string(),
            found: "2.2.0".to_string()
        })
    );
}

#[test]
fn v2_3_0_to_v2_3_1_successful_migration() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-red-bank", "2.3.0").unwrap();

    let res = migrate(deps.as_mut(), mock_env(), MigrateMsg::V2_3_0ToV2_3_1 {}).unwrap();

    assert_eq!(res.messages, vec![]);
    assert_eq!(res.events, vec![] as Vec<Event>);
    assert!(res.data.is_none());
    assert_eq!(
        res.attributes,
        vec![attr("action", "migrate"), attr("from_version", "2.3.0"), attr("to_version", "2.3.1")]
    );

    let new_contract_version = ContractVersion {
        contract: "crates.io:mars-red-bank".to_string(),
        version: "2.3.1".to_string(),
    };
    assert_eq!(cw2::get_contract_version(deps.as_ref().storage).unwrap(), new_contract_version);
}
