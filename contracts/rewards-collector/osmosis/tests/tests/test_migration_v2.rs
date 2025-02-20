use cosmwasm_std::{attr, testing::mock_env, Event};
use cw2::{ContractVersion, VersionError};
use mars_rewards_collector_base::ContractError;
use mars_rewards_collector_osmosis::entry::migrate;
use mars_testing::mock_dependencies;
use mars_types::rewards_collector::OsmosisMigrateMsg;

#[test]
fn wrong_contract_name() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "contract_xyz", "1.0.0").unwrap();

    let err = migrate(deps.as_mut(), mock_env(), OsmosisMigrateMsg::V1_0_0ToV2_0_0 {}).unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongContract {
            expected: "crates.io:mars-rewards-collector-osmosis".to_string(),
            found: "contract_xyz".to_string()
        })
    );
}

#[test]
fn wrong_contract_version() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(
        deps.as_mut().storage,
        "crates.io:mars-rewards-collector-osmosis",
        "4.1.0",
    )
    .unwrap();

    let err = migrate(deps.as_mut(), mock_env(), OsmosisMigrateMsg::V1_0_0ToV2_0_0 {}).unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongVersion {
            expected: "1.0.0".to_string(),
            found: "4.1.0".to_string()
        })
    );
}

#[test]
fn successful_migration_to_v2_1_0() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(
        deps.as_mut().storage,
        "crates.io:mars-rewards-collector-osmosis",
        "2.0.0",
    )
    .unwrap();

    let res = migrate(deps.as_mut(), mock_env(), OsmosisMigrateMsg::V2_0_0ToV2_0_1 {}).unwrap();

    assert_eq!(res.messages, vec![]);
    assert_eq!(res.events, vec![] as Vec<Event>);
    assert!(res.data.is_none());
    assert_eq!(
        res.attributes,
        vec![attr("action", "migrate"), attr("from_version", "2.0.0"), attr("to_version", "2.1.1")]
    );

    let new_contract_version = ContractVersion {
        contract: "crates.io:mars-rewards-collector-osmosis".to_string(),
        version: "2.1.1".to_string(),
    };
    assert_eq!(cw2::get_contract_version(deps.as_ref().storage).unwrap(), new_contract_version);
}
