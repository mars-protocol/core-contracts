use cosmwasm_std::Event;
use cw2::{ContractVersion, VersionError};
use mars_oracle_base::ContractError;
use mars_oracle_wasm::migrations::v2_2_0;
use mars_testing::mock_dependencies;

const FROM_VERSION: &str = "2.2.0";
const TO_VERSION: &str = "2.2.3";

#[test]
fn wrong_contract_name() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "contract_xyz", FROM_VERSION).unwrap();

    let err = v2_2_0::migrate(deps.as_mut()).unwrap_err();

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

    let err = v2_2_0::migrate(deps.as_mut()).unwrap_err();

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
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-oracle-wasm", FROM_VERSION)
        .unwrap();

    let res = v2_2_0::migrate(deps.as_mut()).unwrap();

    // Verify the response
    assert_eq!(res.messages, vec![]);
    assert_eq!(res.events, vec![] as Vec<Event>);
    assert!(res.data.is_none());

    // Verify the version was updated
    let version = cw2::get_contract_version(deps.as_ref().storage).unwrap();
    assert_eq!(
        version,
        ContractVersion {
            contract: "crates.io:mars-oracle-wasm".to_string(),
            version: TO_VERSION.to_string(),
        }
    );
}
