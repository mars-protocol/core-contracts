use cosmwasm_std::{attr, testing::mock_env, Empty, Event};
use cw2::{ContractVersion, VersionError};
use mars_oracle_base::ContractError;
use mars_oracle_wasm::{contract::entry::migrate, migrations};
use mars_testing::mock_dependencies;

#[test]
fn wrong_contract_name() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "contract_xyz", "2.1.0").unwrap();

    let err = migrations::v2_2_0::migrate(deps.as_mut()).unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongContract {
            expected: "crates.io:mars-oracle-wasm".to_string(),
            found: "contract_xyz".to_string()
        })
    );
}

#[test]
fn wrong_contract_version() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-oracle-wasm", "4.1.0")
        .unwrap();

    let err = migrate(deps.as_mut(), mock_env(), Empty {}).unwrap_err();

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
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-oracle-wasm", "2.2.0")
        .unwrap();

    let res = migrations::v2_2_0::migrate(deps.as_mut()).unwrap();

    assert_eq!(res.messages, vec![]);
    assert_eq!(res.events, vec![] as Vec<Event>);
    assert!(res.data.is_none());
    assert_eq!(
        res.attributes,
        vec![attr("action", "migrate"), attr("from_version", "2.2.0"), attr("to_version", "2.2.3")]
    );

    let new_contract_version = ContractVersion {
        contract: "crates.io:mars-oracle-wasm".to_string(),
        version: "2.2.3".to_string(),
    };
    assert_eq!(cw2::get_contract_version(deps.as_ref().storage).unwrap(), new_contract_version);
}
