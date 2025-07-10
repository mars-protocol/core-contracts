use cosmwasm_std::{attr, testing::mock_env, Addr, Event, Uint128};
use cw2::{ContractVersion, VersionError};
use mars_account_nft::{
    contract::migrate, error::ContractError, migrations::v2_3_0::v2_2_0_state, query,
};
use mars_testing::mock_dependencies;
use mars_types::account_nft::MigrateMsg;

#[test]
fn wrong_contract_name() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "contract_xyz", "2.2.0").unwrap();

    let err = migrate(
        deps.as_mut(),
        mock_env(),
        MigrateMsg {
            address_provider: "ap_addr".to_string(),
        },
    )
    .unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongContract {
            expected: "crates.io:mars-account-nft".to_string(),
            found: "contract_xyz".to_string()
        })
    );
}

#[test]
fn wrong_contract_version() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-account-nft", "4.1.0")
        .unwrap();

    let err = migrate(
        deps.as_mut(),
        mock_env(),
        MigrateMsg {
            address_provider: "ap_addr".to_string(),
        },
    )
    .unwrap_err();

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
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-account-nft", "2.2.0")
        .unwrap();

    v2_2_0_state::CONFIG
        .save(
            deps.as_mut().storage,
            &v2_2_0_state::NftConfig {
                max_value_for_burn: Uint128::new(100),
                health_contract_addr: Addr::unchecked("health_addr"),
                credit_manager_contract_addr: Addr::unchecked("credit_manager_addr"),
            },
        )
        .unwrap();

    let res = migrate(
        deps.as_mut(),
        mock_env(),
        MigrateMsg {
            address_provider: "ap_addr_migrated".to_string(),
        },
    )
    .unwrap();

    let config = query::query_config(deps.as_ref()).unwrap();
    assert_eq!(config.max_value_for_burn, Uint128::new(100));
    assert_eq!(config.address_provider_contract_addr, "ap_addr_migrated");

    assert_eq!(res.messages, vec![]);
    assert_eq!(res.events, vec![] as Vec<Event>);
    assert!(res.data.is_none());
    assert_eq!(
        res.attributes,
        vec![attr("action", "migrate"), attr("from_version", "2.2.0"), attr("to_version", "2.2.1")]
    );

    let new_contract_version = ContractVersion {
        contract: "crates.io:mars-account-nft".to_string(),
        version: "2.2.1".to_string(),
    };
    assert_eq!(cw2::get_contract_version(deps.as_ref().storage).unwrap(), new_contract_version);
}
