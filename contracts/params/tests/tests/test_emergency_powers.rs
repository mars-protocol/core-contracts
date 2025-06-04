use cosmwasm_std::{Addr, Decimal};
use mars_owner::OwnerError;
use mars_params::error::ContractError::Owner;
use mars_testing::multitest::helpers::default_perp_params;
use mars_types::{
    params::{
        AssetParamsUpdate, CmEmergencyUpdate, EmergencyUpdate, PerpParamsUpdate,
        PerpsEmergencyUpdate, RedBankEmergencyUpdate, VaultConfigUpdate,
    },
    perps::Config,
};

use super::helpers::{assert_err, default_asset_params, default_vault_config, MockEnv};

#[test]
fn only_owner_can_invoke_emergency_powers() {
    let mut mock = MockEnv::new().build().unwrap();
    let bad_guy = Addr::unchecked("doctor_otto_983");
    let res = mock.emergency_update(
        &bad_guy,
        EmergencyUpdate::RedBank(RedBankEmergencyUpdate::DisableBorrowing("xyz".to_string())),
    );
    assert_err(res, Owner(OwnerError::NotEmergencyOwner {}));

    let res = mock.emergency_update(
        &bad_guy,
        EmergencyUpdate::CreditManager(CmEmergencyUpdate::DisallowCoin("xyz".to_string())),
    );
    assert_err(res, Owner(OwnerError::NotEmergencyOwner {}));

    let res = mock.emergency_update(
        &bad_guy,
        EmergencyUpdate::CreditManager(CmEmergencyUpdate::SetZeroDepositCapOnVault(
            "xyz".to_string(),
        )),
    );
    assert_err(res, Owner(OwnerError::NotEmergencyOwner {}));

    let res = mock.emergency_update(
        &bad_guy,
        EmergencyUpdate::CreditManager(CmEmergencyUpdate::SetZeroMaxLtvOnVault("xyz".to_string())),
    );
    assert_err(res, Owner(OwnerError::NotEmergencyOwner {}));
}

#[test]
fn disabling_borrowing() {
    let emergency_owner = Addr::unchecked("miles_morales");
    let mut mock = MockEnv::new().emergency_owner(emergency_owner.as_str()).build().unwrap();
    let denom = "atom".to_string();

    let mut params = default_asset_params(&denom);
    params.red_bank.borrow_enabled = true;

    mock.set_price_source_fixed(&denom, Decimal::one());

    mock.update_asset_params(
        &mock.query_owner(),
        AssetParamsUpdate::AddOrUpdate {
            params,
        },
    )
    .unwrap();

    let params = mock.query_asset_params(&denom);
    assert!(params.red_bank.borrow_enabled);

    mock.emergency_update(
        &emergency_owner,
        EmergencyUpdate::RedBank(RedBankEmergencyUpdate::DisableBorrowing(denom.clone())),
    )
    .unwrap();

    let params = mock.query_asset_params(&denom);
    assert!(!params.red_bank.borrow_enabled);
}

#[test]
fn disallow_coin() {
    let emergency_owner = Addr::unchecked("miles_morales");
    let mut mock = MockEnv::new().emergency_owner(emergency_owner.as_str()).build().unwrap();
    let denom = "atom".to_string();

    let mut params = default_asset_params(&denom);
    params.credit_manager.whitelisted = true;

    mock.set_price_source_fixed(&denom, Decimal::one());

    mock.update_asset_params(
        &mock.query_owner(),
        AssetParamsUpdate::AddOrUpdate {
            params,
        },
    )
    .unwrap();

    let params = mock.query_asset_params(&denom);
    assert!(params.credit_manager.whitelisted);

    mock.emergency_update(
        &emergency_owner,
        EmergencyUpdate::CreditManager(CmEmergencyUpdate::DisallowCoin(denom.clone())),
    )
    .unwrap();

    let params = mock.query_asset_params(&denom);
    assert!(!params.credit_manager.whitelisted);
}

#[test]
fn disabled_withdraw_cm() {
    let emergency_owner = Addr::unchecked("miles_morales");
    let mut mock = MockEnv::new().emergency_owner(emergency_owner.as_str()).build().unwrap();
    let denom = "atom".to_string();

    let params = default_asset_params(&denom);

    mock.set_price_source_fixed(&denom, Decimal::one());

    mock.update_asset_params(
        &mock.query_owner(),
        AssetParamsUpdate::AddOrUpdate {
            params,
        },
    )
    .unwrap();

    let params = mock.query_asset_params(&denom);

    // Withdraw enabled should be true by default
    assert!(params.credit_manager.withdraw_enabled);

    mock.emergency_update(
        &emergency_owner,
        EmergencyUpdate::CreditManager(CmEmergencyUpdate::DisableWithdraw(denom.clone())),
    )
    .unwrap();

    let params = mock.query_asset_params(&denom);
    assert!(!params.credit_manager.withdraw_enabled);
}

#[test]
fn disabled_withdraw_rb() {
    let emergency_owner = Addr::unchecked("miles_morales");
    let mut mock = MockEnv::new().emergency_owner(emergency_owner.as_str()).build().unwrap();
    let denom = "atom".to_string();

    let params = default_asset_params(&denom);

    mock.set_price_source_fixed(&denom, Decimal::one());

    mock.update_asset_params(
        &mock.query_owner(),
        AssetParamsUpdate::AddOrUpdate {
            params,
        },
    )
    .unwrap();

    let params = mock.query_asset_params(&denom);

    // Withdraw enabled should be true by default
    assert!(params.red_bank.withdraw_enabled);

    mock.emergency_update(
        &emergency_owner,
        EmergencyUpdate::RedBank(RedBankEmergencyUpdate::DisableWithdraw(denom.clone())),
    )
    .unwrap();

    let params = mock.query_asset_params(&denom);
    assert!(!params.red_bank.withdraw_enabled);
}

#[test]
fn set_zero_max_ltv() {
    let emergency_owner = Addr::unchecked("miles_morales");
    let mut mock = MockEnv::new().emergency_owner(emergency_owner.as_str()).build().unwrap();
    let vault = "vault_addr_123".to_string();

    mock.update_vault_config(
        &mock.query_owner(),
        VaultConfigUpdate::AddOrUpdate {
            config: default_vault_config(&vault),
        },
    )
    .unwrap();

    let params = mock.query_vault_config(&vault);
    assert!(!params.max_loan_to_value.is_zero());

    mock.emergency_update(
        &emergency_owner,
        EmergencyUpdate::CreditManager(CmEmergencyUpdate::SetZeroMaxLtvOnVault(vault.clone())),
    )
    .unwrap();

    let params = mock.query_vault_config(&vault);
    assert!(params.max_loan_to_value.is_zero());
}

#[test]
fn set_zero_deposit_cap() {
    let emergency_owner = Addr::unchecked("miles_morales");
    let mut mock = MockEnv::new().emergency_owner(emergency_owner.as_str()).build().unwrap();
    let vault = "vault_addr_123".to_string();

    mock.update_vault_config(
        &mock.query_owner(),
        VaultConfigUpdate::AddOrUpdate {
            config: default_vault_config(&vault),
        },
    )
    .unwrap();

    let params = mock.query_vault_config(&vault);
    assert!(!params.deposit_cap.amount.is_zero());

    mock.emergency_update(
        &emergency_owner,
        EmergencyUpdate::CreditManager(CmEmergencyUpdate::SetZeroDepositCapOnVault(vault.clone())),
    )
    .unwrap();

    let params = mock.query_vault_config(&vault);
    assert!(params.deposit_cap.amount.is_zero());
}

#[test]
fn disabled_perp_trading() {
    let emergency_owner = Addr::unchecked("miles_morales");
    let mut mock = MockEnv::new().emergency_owner(emergency_owner.as_str()).build().unwrap();
    let denom = "atom".to_string();

    let params = default_perp_params(&denom);

    mock.set_price_source_fixed(&denom, Decimal::one());

    mock.update_perp_params(
        &mock.query_owner(),
        PerpParamsUpdate::AddOrUpdate {
            params,
        },
    )
    .unwrap();

    let params = mock.query_perp_params(&denom);

    assert!(params.enabled);

    mock.emergency_update(
        &emergency_owner,
        EmergencyUpdate::Perps(PerpsEmergencyUpdate::DisableTrading(denom.clone())),
    )
    .unwrap();

    let params = mock.query_perp_params(&denom);
    assert!(!params.enabled);
}

#[test]
fn disable_perp_cpv_deleverage() {
    // Set up and ensure deleverage is enabled
    let emergency_owner = Addr::unchecked("miles_morales");
    let mut mock = MockEnv::new().emergency_owner(emergency_owner.as_str()).build().unwrap();
    let initial_config = mock.query_perp_config();
    assert!(initial_config.deleverage_enabled);

    // Disable deleverage
    mock.emergency_update(
        &emergency_owner,
        EmergencyUpdate::Perps(PerpsEmergencyUpdate::DisableDeleverage()),
    )
    .unwrap();

    // Verify deleverage disabled
    let updated_config = mock.query_perp_config();
    assert_eq!(
        updated_config,
        Config {
            deleverage_enabled: false,
            ..initial_config
        }
    );
    assert!(!updated_config.deleverage_enabled);
}

#[test]
fn disable_perp_vault_withdraw() {
    // Set up and ensure withdraw is enabled
    let emergency_owner = Addr::unchecked("miles_morales");
    let mut mock = MockEnv::new().emergency_owner(emergency_owner.as_str()).build().unwrap();
    let initial_config = mock.query_perp_config();
    assert!(initial_config.vault_withdraw_enabled);

    // Disable withdraw
    mock.emergency_update(
        &emergency_owner,
        EmergencyUpdate::Perps(PerpsEmergencyUpdate::DisableCounterpartyVaultWithdraw()),
    )
    .unwrap();

    // Verify withdraw disabled
    let updated_config = mock.query_perp_config();
    assert_eq!(
        updated_config,
        Config {
            vault_withdraw_enabled: false,
            ..initial_config
        }
    );
    assert!(!updated_config.vault_withdraw_enabled);
}
