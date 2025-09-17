use cosmwasm_std::{attr, testing::mock_env, Event};
use cw2::{ContractVersion, VersionError};
use mars_address_provider::{
    contract::{migrate, CONTRACT_NAME, CONTRACT_VERSION},
    error::ContractError,
    migrations::v2_3_2::FROM_VERSION,
};
use mars_testing::mock_dependencies;
use mars_types::address_provider::MigrateMsg;

#[test]
fn wrong_contract_name() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "contract_xyz", FROM_VERSION).unwrap();

    let err = migrate(deps.as_mut(), mock_env(), MigrateMsg::V2_2_2ToV2_3_2 {}).unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongContract {
            expected: format!("crates.io:{CONTRACT_NAME}"),
            found: "contract_xyz".to_string()
        })
    );
}

#[test]
fn wrong_contract_version() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(
        deps.as_mut().storage,
        &format!("crates.io:{CONTRACT_NAME}"),
        "4.1.0",
    )
    .unwrap();

    let err = migrate(deps.as_mut(), mock_env(), MigrateMsg::V2_2_2ToV2_3_2 {}).unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongVersion {
            expected: FROM_VERSION.to_string(),
            found: "4.1.0".to_string()
        })
    );
}

#[test]
fn successful_migration() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(
        deps.as_mut().storage,
        &format!("crates.io:{CONTRACT_NAME}"),
        FROM_VERSION,
    )
    .unwrap();

    let res = migrate(deps.as_mut(), mock_env(), MigrateMsg::V2_2_2ToV2_3_2 {}).unwrap();

    assert_eq!(res.messages, vec![]);
    assert_eq!(res.events, vec![] as Vec<Event>);
    assert!(res.data.is_none());
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "migrate"),
            attr("from_version", FROM_VERSION),
            attr("to_version", CONTRACT_VERSION)
        ]
    );

    let new_contract_version = ContractVersion {
        contract: format!("crates.io:{CONTRACT_NAME}"),
        version: CONTRACT_VERSION.to_string(),
    };
    assert_eq!(cw2::get_contract_version(deps.as_ref().storage).unwrap(), new_contract_version);
}
