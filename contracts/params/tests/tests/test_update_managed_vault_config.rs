use cosmwasm_std::Addr;
use mars_params::error::ContractError;
use mars_types::params::ManagedVaultUpdate;
use test_case::test_case;

use super::helpers::{assert_err, MockEnv};

enum OwnerOrRiskManager {
    Owner,
    RiskManager,
}

#[test]
fn non_owner_or_risk_manager_cannot_update_managed_vault_config() {
    let mut mock =
        MockEnv::new().build_with_risk_manager(Some("risk_manager_123".to_string())).unwrap();

    // Ensure owner and risk manager are different
    let owner = mock.query_owner();
    let risk_manager = mock.query_risk_manager();
    assert_ne!(owner, risk_manager);

    let bad_guy = Addr::unchecked("doctor_otto_983");

    let res = mock.update_managed_vault_config(&bad_guy, ManagedVaultUpdate::AddCodeId(1));
    assert_err(res, ContractError::NotOwnerOrRiskManager {});

    let res = mock.update_managed_vault_config(&bad_guy, ManagedVaultUpdate::RemoveCodeId(1));
    assert_err(res, ContractError::NotOwnerOrRiskManager {});

    let res = mock
        .update_managed_vault_config(&bad_guy, ManagedVaultUpdate::SetMinCreationFeeInUusd(50_000));
    assert_err(res, ContractError::NotOwnerOrRiskManager {});
}

#[test]
fn managed_vault_config_initialized_with_default_values() {
    let mock = MockEnv::new().build().unwrap();
    let init_config = mock.query_managed_vault_config();
    assert_eq!(init_config.code_ids.len(), 0);
    assert_eq!(init_config.min_creation_fee_in_uusd, 0);
}

#[test_case(
    OwnerOrRiskManager::Owner;
    "owner can modify managed vault code ids"
)]
#[test_case(
    OwnerOrRiskManager::RiskManager;
    "risk manager can modify managed vault code ids"
)]
fn owner_or_risk_manager_can_modify_managed_vault_code_ids(
    owner_or_risk_manager: OwnerOrRiskManager,
) {
    let mut mock =
        MockEnv::new().build_with_risk_manager(Some("risk_manager_123".to_string())).unwrap();

    // Ensure owner and risk manager are different
    let owner = mock.query_owner();
    let risk_manager = mock.query_risk_manager();
    assert_ne!(owner, risk_manager);

    let sender = match owner_or_risk_manager {
        OwnerOrRiskManager::Owner => owner,
        OwnerOrRiskManager::RiskManager => risk_manager,
    };

    // Add code id
    mock.update_managed_vault_config(&sender, ManagedVaultUpdate::AddCodeId(102)).unwrap();
    let config = mock.query_managed_vault_config();
    assert_eq!(config.code_ids.len(), 1);
    assert_eq!(config.code_ids[0], 102);
    assert_eq!(config.min_creation_fee_in_uusd, 0);

    // Add code id
    mock.update_managed_vault_config(&sender, ManagedVaultUpdate::AddCodeId(99)).unwrap();
    let config = mock.query_managed_vault_config();
    assert_eq!(config.code_ids.len(), 2);
    assert_eq!(config.code_ids[0], 102);
    assert_eq!(config.code_ids[1], 99);
    assert_eq!(config.min_creation_fee_in_uusd, 0);
    // Add code id
    mock.update_managed_vault_config(&sender, ManagedVaultUpdate::AddCodeId(1005)).unwrap();
    let config = mock.query_managed_vault_config();
    assert_eq!(config.code_ids.len(), 3);
    assert_eq!(config.code_ids[0], 102);
    assert_eq!(config.code_ids[1], 99);
    assert_eq!(config.code_ids[2], 1005);
    assert_eq!(config.min_creation_fee_in_uusd, 0);
    // Remove non-existent code id, should be a no-op
    mock.update_managed_vault_config(&sender, ManagedVaultUpdate::RemoveCodeId(2000)).unwrap();
    let config = mock.query_managed_vault_config();
    assert_eq!(config.code_ids.len(), 3);
    assert_eq!(config.code_ids[0], 102);
    assert_eq!(config.code_ids[1], 99);
    assert_eq!(config.code_ids[2], 1005);
    assert_eq!(config.min_creation_fee_in_uusd, 0);

    // Remove code id
    mock.update_managed_vault_config(&sender, ManagedVaultUpdate::RemoveCodeId(99)).unwrap();
    let config = mock.query_managed_vault_config();
    assert_eq!(config.code_ids.len(), 2);
    assert_eq!(config.code_ids[0], 102);
    assert_eq!(config.code_ids[1], 1005);
    assert_eq!(config.min_creation_fee_in_uusd, 0);
}

#[test_case(
    OwnerOrRiskManager::Owner;
    "owner can update min creation fee"
)]
#[test_case(
    OwnerOrRiskManager::RiskManager;
    "risk manager can update min creation fee"
)]
fn owner_or_risk_manager_can_update_min_creation_fee(owner_or_risk_manager: OwnerOrRiskManager) {
    let mut mock =
        MockEnv::new().build_with_risk_manager(Some("risk_manager_123".to_string())).unwrap();

    let sender = match owner_or_risk_manager {
        OwnerOrRiskManager::Owner => mock.query_owner(),
        OwnerOrRiskManager::RiskManager => mock.query_risk_manager(),
    };

    mock.update_managed_vault_config(&sender, ManagedVaultUpdate::SetMinCreationFeeInUusd(50_123))
        .unwrap();
    let config = mock.query_managed_vault_config();
    assert_eq!(config.min_creation_fee_in_uusd, 50_123);
    assert_eq!(config.code_ids.len(), 0);
}
