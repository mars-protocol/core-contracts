use cosmwasm_std::{attr, testing::mock_env, Addr, Decimal, Uint128};
use cw2::{ContractVersion, VersionError};
use mars_credit_manager::{
    contract::migrate,
    error::ContractError,
    state::{FEE_TIER_CONFIG, GOVERNANCE},
};
use mars_testing::mock_dependencies;
use mars_types::{
    credit_manager::MigrateMsg,
    fee_tiers::{FeeTier, FeeTierConfig},
};

const CONTRACT_NAME: &str = "crates.io:mars-credit-manager";
const CONTRACT_VERSION: &str = "2.4.0";

fn create_test_fee_tier_config() -> FeeTierConfig {
    FeeTierConfig {
        tiers: vec![
            FeeTier {
                id: "tier_3".to_string(),
                min_voting_power: Uint128::new(100000000000), // 100,000 MARS
                discount_pct: Decimal::percent(30),
            },
            FeeTier {
                id: "tier_2".to_string(),
                min_voting_power: Uint128::new(10000000000), // 10,000 MARS
                discount_pct: Decimal::percent(10),
            },
            FeeTier {
                id: "tier_1".to_string(),
                min_voting_power: Uint128::zero(),
                discount_pct: Decimal::percent(0),
            },
        ],
    }
}

#[test]
fn wrong_contract_name() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "contract_xyz", "2.3.0").unwrap();

    let fee_tier_config = create_test_fee_tier_config();
    let governance_address = Addr::unchecked("governance");

    let err = migrate(
        deps.as_mut(),
        mock_env(),
        MigrateMsg::V2_3_0ToV2_4_0 {
            fee_tier_config,
            governance_address,
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
fn wrong_contract_version() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, CONTRACT_NAME, "2.1.0").unwrap();

    let fee_tier_config = create_test_fee_tier_config();
    let governance_address = Addr::unchecked("governance");

    let err = migrate(
        deps.as_mut(),
        mock_env(),
        MigrateMsg::V2_3_0ToV2_4_0 {
            fee_tier_config,
            governance_address,
        },
    )
    .unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongVersion {
            expected: "2.3.0".to_string(),
            found: "2.1.0".to_string()
        })
    );
}

#[test]
fn successful_migration() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, CONTRACT_NAME, "2.3.0").unwrap();

    let fee_tier_config = create_test_fee_tier_config();
    let governance_address = Addr::unchecked("governance");

    let res = migrate(
        deps.as_mut(),
        mock_env(),
        MigrateMsg::V2_3_0ToV2_4_0 {
            fee_tier_config: fee_tier_config.clone(),
            governance_address: governance_address.clone(),
        },
    )
    .unwrap();

    // Verify that the state was set correctly
    let stored_fee_tier_config = FEE_TIER_CONFIG.load(deps.as_ref().storage).unwrap();
    let stored_governance = GOVERNANCE.load(deps.as_ref().storage).unwrap();

    assert_eq!(stored_fee_tier_config, fee_tier_config);
    assert_eq!(stored_governance, governance_address);

    // Verify response attributes
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "migrate"),
            attr("from_version", "2.3.0"),
            attr("to_version", "2.4.0"),
            attr("fee_tier_config", "set"),
            attr("governance", governance_address)
        ]
    );

    // Verify contract version was updated
    let new_contract_version = ContractVersion {
        contract: CONTRACT_NAME.to_string(),
        version: CONTRACT_VERSION.to_string(),
    };
    assert_eq!(cw2::get_contract_version(deps.as_ref().storage).unwrap(), new_contract_version);
}

#[test]
fn migration_with_invalid_fee_tier_config() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, CONTRACT_NAME, "2.3.0").unwrap();

    // Create invalid fee tier config (tiers not sorted by min_voting_power descending)
    let invalid_fee_tier_config = FeeTierConfig {
        tiers: vec![
            FeeTier {
                id: "tier_1".to_string(),
                min_voting_power: Uint128::new(10000000000), // Lower threshold first
                discount_pct: Decimal::percent(10),
            },
            FeeTier {
                id: "tier_2".to_string(),
                min_voting_power: Uint128::new(100000000000), // Higher threshold second
                discount_pct: Decimal::percent(30),
            },
        ],
    };
    let governance_address = Addr::unchecked("governance");

    let err = migrate(
        deps.as_mut(),
        mock_env(),
        MigrateMsg::V2_3_0ToV2_4_0 {
            fee_tier_config: invalid_fee_tier_config,
            governance_address,
        },
    )
    .unwrap_err();

    // Should fail validation due to incorrect tier ordering
    assert!(matches!(err, ContractError::TiersNotSortedDescending));
}
