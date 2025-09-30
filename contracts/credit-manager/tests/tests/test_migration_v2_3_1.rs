use cosmwasm_std::{attr, testing::mock_env, Decimal};
use cw2::{ContractVersion, VersionError};
use mars_credit_manager::{contract::migrate, error::ContractError, state::SWAP_FEE};
use mars_testing::mock_dependencies;
use mars_types::credit_manager::MigrateMsg;

const CONTRACT_NAME: &str = "crates.io:mars-credit-manager";
const FROM_VERSION: &str = "2.3.0";
const TO_VERSION: &str = "2.3.1";

#[test]
fn wrong_contract_name() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "wrong-name", FROM_VERSION).unwrap();

    let err = migrate(
        deps.as_mut(),
        mock_env(),
        MigrateMsg::V2_3_0ToV2_3_1 {
            swap_fee: Decimal::percent(1),
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
    cw2::set_contract_version(deps.as_mut().storage, CONTRACT_NAME, "2.2.3").unwrap();

    let err = migrate(
        deps.as_mut(),
        mock_env(),
        MigrateMsg::V2_3_0ToV2_3_1 {
            swap_fee: Decimal::percent(1),
        },
    )
    .unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongVersion {
            expected: FROM_VERSION.to_string(),
            found: "2.2.3".to_string()
        })
    );
}

#[test]
fn successful_migration() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, CONTRACT_NAME, FROM_VERSION).unwrap();

    let swap_fee = Decimal::percent(1);
    let res = migrate(
        deps.as_mut(),
        mock_env(),
        MigrateMsg::V2_3_0ToV2_3_1 {
            swap_fee,
        },
    )
    .unwrap();

    assert_eq!(res.messages, vec![]);
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "migrate"),
            attr("from_version", FROM_VERSION),
            attr("to_version", TO_VERSION),
        ]
    );

    let version = cw2::get_contract_version(&deps.storage).unwrap();
    assert_eq!(
        version,
        ContractVersion {
            contract: CONTRACT_NAME.to_string(),
            version: TO_VERSION.to_string()
        }
    );

    let stored_swap_fee = SWAP_FEE.load(&deps.storage).unwrap();
    assert_eq!(stored_swap_fee, swap_fee);
}
