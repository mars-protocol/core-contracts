use cosmwasm_std::{attr, testing::mock_env, Event};
use cw2::{ContractVersion, VersionError};
use mars_credit_manager::{
    contract::migrate,
    error::ContractError,
    state::MAX_TRIGGER_ORDERS,
};
use mars_testing::mock_dependencies;
use mars_types::credit_manager::MigrateMsg;

const CONTRACT_NAME: &str = "crates.io:mars-credit-manager";
const FROM_VERSION: &str = "2.2.3";
const TO_VERSION: &str = "2.3.0";

#[test]
fn wrong_contract_name() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "wrong-name", FROM_VERSION).unwrap();

    let err = migrate(
        deps.as_mut(),
        mock_env(),
        MigrateMsg::V2_2_3ToV2_3_0 {
            max_trigger_orders: 50,
        },
    )
    .unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongContract {
            expected: CONTRACT_NAME.to_string(),
            found: "wrong-name".to_string()
        })
    );
}

#[test]
fn wrong_contract_version() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, CONTRACT_NAME, "2.2.0").unwrap();

    let err = migrate(
        deps.as_mut(),
        mock_env(),
        MigrateMsg::V2_2_3ToV2_3_0 {
            max_trigger_orders: 50,
        },
    )
    .unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongVersion {
            expected: FROM_VERSION.to_string(),
            found: "2.2.0".to_string()
        })
    );
}

#[test]
fn successful_migration() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, CONTRACT_NAME, FROM_VERSION).unwrap();

    let res = migrate(
        deps.as_mut(),
        mock_env(),
        MigrateMsg::V2_2_3ToV2_3_0 {
            max_trigger_orders: 50,
        },
    )
    .unwrap();

    let max_trigger_orders = MAX_TRIGGER_ORDERS.load(deps.as_ref().storage).unwrap();

    assert_eq!(max_trigger_orders, 50);

    assert_eq!(res.messages, vec![]);
    assert_eq!(res.events, vec![] as Vec<Event>);
    assert!(res.data.is_none());
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "migrate"),
            attr("from_version", FROM_VERSION),
            attr("to_version", TO_VERSION)
        ]
    );

    let new_contract_version = ContractVersion {
        contract: CONTRACT_NAME.to_string(),
        version: TO_VERSION.to_string(),
    };
    assert_eq!(
        cw2::get_contract_version(deps.as_ref().storage).unwrap(),
        new_contract_version
    );
}
