use std::{collections::HashMap, str::FromStr};

use cosmwasm_std::{attr, Addr, Decimal, Event, Order, StdResult, Uint128};
use cw2::{ContractVersion, VersionError};
use mars_params::{
    error::ContractError,
    migrations::{self, v2_2_0::v2_1_0_state},
    state::{ASSET_PARAMS, OWNER, RISK_MANAGER},
};
use mars_testing::mock_dependencies;
use mars_types::params::{
    AssetParams, CmSettings, HlsAssetType, HlsParams, LiquidationBonus, RedBankSettings,
};

#[test]
fn wrong_contract_name() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "contract_xyz", "2.1.0").unwrap();

    let err = migrations::v2_2_0::migrate(deps.as_mut(), Decimal::one()).unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongContract {
            expected: "crates.io:mars-params".to_string(),
            found: "contract_xyz".to_string()
        })
    );
}

#[test]
fn wrong_contract_version() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-params", "2.0.0").unwrap();

    let err = migrations::v2_2_0::migrate(deps.as_mut(), Decimal::one()).unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongVersion {
            expected: "2.1.0".to_string(),
            found: "2.0.0".to_string()
        })
    );
}

#[test]
fn successful_migration() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-params", "2.1.0").unwrap();

    // Initialize the OWNER storage item so we can later verify the migration has set the risk manager to the owner as default
    let owner = Addr::unchecked("owner");
    let deps_muted = deps.as_mut();
    OWNER
        .initialize(
            deps_muted.storage,
            deps_muted.api,
            mars_owner::OwnerInit::SetInitialOwner {
                owner: owner.to_string(),
            },
        )
        .unwrap();

    // Initialize the ASSET_PARAMS storage items with the old state
    v2_1_0_state::ASSET_PARAMS.save(deps.as_mut().storage, "asset_2", &asset_2()).unwrap();
    v2_1_0_state::ASSET_PARAMS.save(deps.as_mut().storage, "asset_1", &asset_1()).unwrap();

    let res =
        migrations::v2_2_0::migrate(deps.as_mut(), Decimal::from_str("0.9").unwrap()).unwrap();

    assert_eq!(res.messages, vec![]);
    assert_eq!(res.events, vec![] as Vec<Event>);
    assert!(res.data.is_none());
    assert_eq!(
        res.attributes,
        vec![attr("action", "migrate"), attr("from_version", "2.1.0"), attr("to_version", "2.2.3")]
    );

    let new_contract_version = ContractVersion {
        contract: "crates.io:mars-params".to_string(),
        version: "2.2.3".to_string(),
    };
    assert_eq!(cw2::get_contract_version(deps.as_ref().storage).unwrap(), new_contract_version);

    assert!(OWNER.is_owner(deps.as_ref().storage, &owner).unwrap());
    // Check that the risk manager has been set to the owner as default
    assert!(RISK_MANAGER.is_owner(deps.as_ref().storage, &owner).unwrap());

    // Check that the ASSET_PARAMS storage items have been migrated correctly
    let asset_params = ASSET_PARAMS
        .range(deps.as_ref().storage, None, None, Order::Ascending)
        .collect::<StdResult<HashMap<_, _>>>()
        .unwrap();
    assert_eq!(asset_params.len(), 2);
    assert_eq!(asset_params.get("asset_1").unwrap(), &expected_asset_1());
    assert_eq!(asset_params.get("asset_2").unwrap(), &expected_asset_2());
}

fn asset_1() -> v2_1_0_state::AssetParams {
    v2_1_0_state::AssetParams {
        denom: "asset_1".to_string(),
        credit_manager: v2_1_0_state::CmSettings {
            whitelisted: false,
            hls: None,
        },
        red_bank: v2_1_0_state::RedBankSettings {
            deposit_enabled: false,
            borrow_enabled: false,
        },
        max_loan_to_value: Decimal::from_str("0.6").unwrap(),
        liquidation_threshold: Decimal::from_str("0.65").unwrap(),
        liquidation_bonus: v2_1_0_state::LiquidationBonus {
            starting_lb: Decimal::from_str("0.1").unwrap(),
            slope: Decimal::from_str("0.2").unwrap(),
            min_lb: Decimal::from_str("0.3").unwrap(),
            max_lb: Decimal::from_str("0.4").unwrap(),
        },
        protocol_liquidation_fee: Decimal::from_str("0.05").unwrap(),
        deposit_cap: Uint128::from(1230000u128),
    }
}

fn expected_asset_1() -> AssetParams {
    AssetParams {
        denom: "asset_1".to_string(),
        credit_manager: CmSettings {
            whitelisted: false,
            hls: None,
            withdraw_enabled: true,
        },
        red_bank: RedBankSettings {
            deposit_enabled: false,
            borrow_enabled: false,
            withdraw_enabled: false,
        },
        max_loan_to_value: Decimal::from_str("0.6").unwrap(),
        liquidation_threshold: Decimal::from_str("0.65").unwrap(),
        liquidation_bonus: LiquidationBonus {
            starting_lb: Decimal::from_str("0.1").unwrap(),
            slope: Decimal::from_str("0.2").unwrap(),
            min_lb: Decimal::from_str("0.3").unwrap(),
            max_lb: Decimal::from_str("0.4").unwrap(),
        },
        protocol_liquidation_fee: Decimal::from_str("0.05").unwrap(),
        deposit_cap: Uint128::from(1230000u128),
        close_factor: Decimal::from_str("0.9").unwrap(),
    }
}

fn asset_2() -> v2_1_0_state::AssetParams {
    v2_1_0_state::AssetParams {
        denom: "asset_2".to_string(),
        credit_manager: v2_1_0_state::CmSettings {
            whitelisted: true,
            hls: Some(v2_1_0_state::HlsParams {
                max_loan_to_value: Decimal::from_str("0.2").unwrap(),
                liquidation_threshold: Decimal::from_str("0.22").unwrap(),
                correlations: vec![
                    v2_1_0_state::HlsAssetType::Coin {
                        denom: "denom_1".to_string(),
                    },
                    v2_1_0_state::HlsAssetType::Vault {
                        addr: Addr::unchecked("vault_addr_123"),
                    },
                    v2_1_0_state::HlsAssetType::Coin {
                        denom: "denom_2".to_string(),
                    },
                ],
            }),
        },
        red_bank: v2_1_0_state::RedBankSettings {
            deposit_enabled: true,
            borrow_enabled: true,
        },
        max_loan_to_value: Decimal::from_str("0.89").unwrap(),
        liquidation_threshold: Decimal::from_str("0.67").unwrap(),
        liquidation_bonus: v2_1_0_state::LiquidationBonus {
            starting_lb: Decimal::from_str("0.2").unwrap(),
            slope: Decimal::from_str("0.1").unwrap(),
            min_lb: Decimal::from_str("0.4").unwrap(),
            max_lb: Decimal::from_str("0.3").unwrap(),
        },
        protocol_liquidation_fee: Decimal::from_str("0.15").unwrap(),
        deposit_cap: Uint128::from(123u128),
    }
}

fn expected_asset_2() -> AssetParams {
    AssetParams {
        denom: "asset_2".to_string(),
        credit_manager: CmSettings {
            whitelisted: true,
            hls: Some(HlsParams {
                max_loan_to_value: Decimal::from_str("0.2").unwrap(),
                liquidation_threshold: Decimal::from_str("0.22").unwrap(),
                correlations: vec![
                    HlsAssetType::Coin {
                        denom: "denom_1".to_string(),
                    },
                    HlsAssetType::Vault {
                        addr: Addr::unchecked("vault_addr_123"),
                    },
                    HlsAssetType::Coin {
                        denom: "denom_2".to_string(),
                    },
                ],
            }),
            withdraw_enabled: true,
        },
        red_bank: RedBankSettings {
            deposit_enabled: true,
            borrow_enabled: true,
            withdraw_enabled: true,
        },
        max_loan_to_value: Decimal::from_str("0.89").unwrap(),
        liquidation_threshold: Decimal::from_str("0.67").unwrap(),
        liquidation_bonus: LiquidationBonus {
            starting_lb: Decimal::from_str("0.2").unwrap(),
            slope: Decimal::from_str("0.1").unwrap(),
            min_lb: Decimal::from_str("0.4").unwrap(),
            max_lb: Decimal::from_str("0.3").unwrap(),
        },
        protocol_liquidation_fee: Decimal::from_str("0.15").unwrap(),
        deposit_cap: Uint128::from(123u128),
        close_factor: Decimal::from_str("0.9").unwrap(),
    }
}
