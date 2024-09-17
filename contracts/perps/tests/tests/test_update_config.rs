use cosmwasm_std::{Addr, Decimal};
use mars_perps::error::ContractError;
use mars_types::{
    error::MarsError,
    perps::{Config, ConfigUpdates},
};

use super::helpers::{assert_err, MockEnv};

#[test]
fn only_owner_can_update_config() {
    let mut mock = MockEnv::new().build().unwrap();
    let new_owner = Addr::unchecked("bad_guy");

    let res = mock.update_config(
        &new_owner,
        ConfigUpdates {
            ..Default::default()
        },
    );

    assert_err(res, ContractError::Mars(MarsError::Unauthorized {}));
}

#[test]
fn update_partial_config() {
    let mut mock = MockEnv::new().build().unwrap();
    let owner = mock.query_owner();
    let config = mock.query_config();
    let original_addr_provider = config.address_provider.as_str();

    let res = mock.update_config(
        &owner,
        ConfigUpdates {
            address_provider: Some("addr_provider_new".to_string()),
            ..Default::default()
        },
    );

    let new_config = mock.query_config();

    assert_ne!(new_config.address_provider.as_str(), original_addr_provider);
    assert_eq!(
        new_config,
        Config {
            address_provider: Addr::unchecked("addr_provider_new"),
            ..config
        }
    );
    assert!(res.is_ok());
}

#[test]
fn update_total_config() {
    let mut mock = MockEnv::new().build().unwrap();
    let owner = mock.query_owner();

    let new_config = Config {
        address_provider: Addr::unchecked("addr_provider_new"),
        // UUSDC is never updated
        base_denom: "uusdc".to_string(),
        cooldown_period: 20,
        deleverage_enabled: false,
        max_positions: 100,
        protocol_fee_rate: Decimal::from_ratio(2u128, 100u128),
        target_vault_collateralization_ratio: Decimal::from_ratio(150u128, 100u128),
        vault_withdraw_enabled: false,
    };

    let res = mock.update_config(
        &owner,
        ConfigUpdates {
            address_provider: Some(new_config.address_provider.to_string()),
            cooldown_period: Some(new_config.cooldown_period),
            deleverage_enabled: Some(new_config.deleverage_enabled),
            max_positions: Some(new_config.max_positions),
            protocol_fee_rate: Some(new_config.protocol_fee_rate),
            target_vault_collateralization_ratio: Some(
                new_config.target_vault_collateralization_ratio,
            ),
            vault_withdraw_enabled: Some(new_config.vault_withdraw_enabled),
        },
    );

    let new_config_loaded = mock.query_config();

    assert_eq!(new_config, new_config_loaded);
    assert!(res.is_ok());
}
