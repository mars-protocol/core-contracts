use cosmwasm_std::{attr, testing::mock_env, Empty};
use cw2::{ContractVersion, VersionError};
use mars_perps::{contract::migrate, error::ContractError};
use mars_testing::mock_dependencies;

const CONTRACT_NAME: &str = "mars-perps";
const CONTRACT_VERSION: &str = "2.2.3";

#[test]
fn wrong_contract_name() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "contract_xyz", "2.2.1").unwrap();

    let err = migrate(deps.as_mut(), mock_env(), Empty {}).unwrap_err();

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
    cw2::set_contract_version(deps.as_mut().storage, CONTRACT_NAME, "1.0.0").unwrap();

    let err = migrate(deps.as_mut(), mock_env(), Empty {}).unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongVersion {
            expected: "2.2.1".to_string(),
            found: "1.0.0".to_string()
        })
    );
}

#[test]
fn successful_migration_from_2_2_1() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, CONTRACT_NAME, "2.2.1").unwrap();

    let res = migrate(deps.as_mut(), mock_env(), Empty {}).unwrap();

    assert_eq!(res.messages, vec![]);
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "migrate"),
            attr("from_version", "2.2.1"),
            attr("to_version", CONTRACT_VERSION),
        ]
    );
    assert!(res.data.is_none());

    // Verify the contract version was updated
    let new_contract_version = cw2::get_contract_version(deps.as_ref().storage).unwrap();
    assert_eq!(
        new_contract_version,
        ContractVersion {
            contract: format!("crates.io:{CONTRACT_NAME}"),
            version: CONTRACT_VERSION.to_string(),
        }
    );
}
