use cosmwasm_std::{attr, Event};
use cw2::{ContractVersion, VersionError};
use mars_credit_manager::{error::ContractError, migrations, state::NEXT_TRIGGER_ID};
use mars_testing::mock_dependencies;

#[test]
fn wrong_contract_name() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "contract_xyz", "2.2.0").unwrap();

    let err = migrations::v2_2_0::migrate(deps.as_mut()).unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongContract {
            expected: "crates.io:mars-credit-manager".to_string(),
            found: "contract_xyz".to_string()
        })
    );
}

#[test]
fn wrong_contract_version() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-credit-manager", "4.1.0")
        .unwrap();

    let err = migrations::v2_2_0::migrate(deps.as_mut()).unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongVersion {
            expected: "2.1.0".to_string(),
            found: "4.1.0".to_string()
        })
    );
}

#[test]
fn successful_migration() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-credit-manager", "2.1.0")
        .unwrap();

    let res = migrations::v2_2_0::migrate(deps.as_mut()).unwrap();

    let order_id = NEXT_TRIGGER_ID.load(deps.as_ref().storage).unwrap();
    assert_eq!(order_id, 1);

    assert_eq!(res.messages, vec![]);
    assert_eq!(res.events, vec![] as Vec<Event>);
    assert!(res.data.is_none());
    assert_eq!(
        res.attributes,
        vec![attr("action", "migrate"), attr("from_version", "2.1.0"), attr("to_version", "2.2.0")]
    );

    let new_contract_version = ContractVersion {
        contract: "crates.io:mars-credit-manager".to_string(),
        version: "2.2.0".to_string(),
    };
    assert_eq!(cw2::get_contract_version(deps.as_ref().storage).unwrap(), new_contract_version);
}
