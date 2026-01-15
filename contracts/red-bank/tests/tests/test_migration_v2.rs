use cosmwasm_std::{attr, testing::mock_env, Addr, Decimal, Event, Uint128};
use cw2::{ContractVersion, VersionError};
use mars_red_bank::{
    contract::{migrate, CONTRACT_VERSION},
    error::ContractError,
    state::{COLLATERALS, MARKETS},
};
use mars_testing::mock_dependencies;
use mars_types::{
    keys::{UserId, UserIdKey},
    red_bank::{Collateral, Market, MigrateMsg},
};

const CONTRACT_NAME: &str = "crates.io:mars-red-bank";

const FROM_VERSION_V2_3_2: &str = "2.3.2";
const FROM_VERSION_V2_3_3: &str = "2.3.3";

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
    cw2::set_contract_version(deps.as_mut().storage, CONTRACT_NAME, "4.1.0").unwrap();

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
    cw2::set_contract_version(deps.as_mut().storage, CONTRACT_NAME, "2.2.0").unwrap();

    let res = migrate(deps.as_mut(), mock_env(), MigrateMsg::V2_2_0ToV2_3_0 {}).unwrap();

    assert_eq!(res.messages, vec![]);
    assert_eq!(res.events, vec![] as Vec<Event>);
    assert!(res.data.is_none());
    assert_eq!(
        res.attributes,
        vec![attr("action", "migrate"), attr("from_version", "2.2.0"), attr("to_version", "2.3.0")]
    );

    let new_contract_version = ContractVersion {
        contract: CONTRACT_NAME.to_string(),
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
    cw2::set_contract_version(deps.as_mut().storage, CONTRACT_NAME, "2.2.0").unwrap();

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
    cw2::set_contract_version(deps.as_mut().storage, CONTRACT_NAME, "2.3.0").unwrap();

    let res = migrate(deps.as_mut(), mock_env(), MigrateMsg::V2_3_0ToV2_3_1 {}).unwrap();

    assert_eq!(res.messages, vec![]);
    assert_eq!(res.events, vec![] as Vec<Event>);
    assert!(res.data.is_none());
    assert_eq!(
        res.attributes,
        vec![attr("action", "migrate"), attr("from_version", "2.3.0"), attr("to_version", "2.3.1")]
    );

    let new_contract_version = ContractVersion {
        contract: CONTRACT_NAME.to_string(),
        version: "2.3.1".to_string(),
    };
    assert_eq!(cw2::get_contract_version(deps.as_ref().storage).unwrap(), new_contract_version);
}

#[test]
fn v2_3_2_to_v2_3_3_wrong_contract_name() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "contract_xyz", FROM_VERSION_V2_3_2).unwrap();

    let err = migrate(
        deps.as_mut(),
        mock_env(),
        MigrateMsg::V2_3_2ToV2_3_3 {
            haircut: Decimal::percent(10),
            market: "umars".to_string(),
        },
    )
    .unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongContract {
            expected: CONTRACT_NAME.to_string(),
            found: "contract_xyz".to_string()
        })
    );
}

#[test]
fn v2_3_2_to_v2_3_3_wrong_contract_version() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, CONTRACT_NAME, "2.3.0").unwrap();

    let err = migrate(
        deps.as_mut(),
        mock_env(),
        MigrateMsg::V2_3_2ToV2_3_3 {
            haircut: Decimal::percent(10),
            market: "umars".to_string(),
        },
    )
    .unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongVersion {
            expected: FROM_VERSION_V2_3_2.to_string(),
            found: "2.3.0".to_string()
        })
    );
}

#[test]
fn v2_3_2_to_v2_3_3_successful_migration() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, CONTRACT_NAME, FROM_VERSION_V2_3_2).unwrap();

    let denom = "umars";
    let market = Market {
        denom: denom.to_string(),
        liquidity_index: Decimal::percent(200),
        ..Market::default()
    };
    MARKETS.save(deps.as_mut().storage, denom, &market).unwrap();

    let user_addr =
        Addr::unchecked("neutron1qdzn3l4kn7gsjna2tfpg3g3mwd6kunx4p50lfya59k02846xas6qslgs3r");
    let user_id = UserId::credit_manager(user_addr, "4954".to_string());
    let user_id_key: UserIdKey = user_id.try_into().unwrap();
    let collateral = Collateral {
        amount_scaled: Uint128::new(1234),
        enabled: true,
    };
    COLLATERALS.save(deps.as_mut().storage, (&user_id_key, denom), &collateral).unwrap();

    let haircut = Decimal::percent(10);
    let res = migrate(
        deps.as_mut(),
        mock_env(),
        MigrateMsg::V2_3_2ToV2_3_3 {
            haircut,
            market: denom.to_string(),
        },
    )
    .unwrap();

    assert_eq!(res.messages, vec![]);
    assert_eq!(res.events, vec![] as Vec<Event>);
    assert!(res.data.is_none());
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "migrate"),
            attr("from_version", FROM_VERSION_V2_3_2),
            attr("to_version", CONTRACT_VERSION),
            attr("to_version", CONTRACT_VERSION),
            attr("haircut_percent", haircut.to_string()),
            attr("haircut_market", denom),
        ]
    );

    let new_market = MARKETS.load(deps.as_ref().storage, denom).unwrap();
    assert_eq!(new_market.liquidity_index, Decimal::percent(180));

    assert!(COLLATERALS.may_load(deps.as_ref().storage, (&user_id_key, denom)).unwrap().is_none());

    let new_contract_version = ContractVersion {
        contract: CONTRACT_NAME.to_string(),
        version: CONTRACT_VERSION.to_string(),
    };
    assert_eq!(cw2::get_contract_version(deps.as_ref().storage).unwrap(), new_contract_version);
}

#[test]
fn v2_3_3_to_v2_3_4_wrong_contract_name() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "contract_xyz", FROM_VERSION_V2_3_3).unwrap();

    let err = migrate(deps.as_mut(), mock_env(), MigrateMsg::V2_3_3ToV2_3_4 {}).unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongContract {
            expected: CONTRACT_NAME.to_string(),
            found: "contract_xyz".to_string()
        })
    );
}

#[test]
fn v2_3_3_to_v2_3_4_wrong_contract_version() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, CONTRACT_NAME, "2.3.2").unwrap();

    let err = migrate(deps.as_mut(), mock_env(), MigrateMsg::V2_3_3ToV2_3_4 {}).unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongVersion {
            expected: FROM_VERSION_V2_3_3.to_string(),
            found: "2.3.2".to_string()
        })
    );
}

#[test]
fn v2_3_3_to_v2_3_4_successful_migration() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, CONTRACT_NAME, FROM_VERSION_V2_3_3).unwrap();

    let res = migrate(deps.as_mut(), mock_env(), MigrateMsg::V2_3_3ToV2_3_4 {}).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "migrate"),
            attr("from_version", FROM_VERSION_V2_3_3),
            attr("to_version", CONTRACT_VERSION),
        ]
    );

    let new_contract_version = ContractVersion {
        contract: CONTRACT_NAME.to_string(),
        version: CONTRACT_VERSION.to_string(),
    };
    assert_eq!(cw2::get_contract_version(deps.as_ref().storage).unwrap(), new_contract_version);
}
