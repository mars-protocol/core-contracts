use cosmwasm_std::Addr;
use mars_owner::OwnerError;
use mars_params::error::ContractError::Owner;

use super::helpers::{assert_err, MockEnv};

#[test]
fn address_provider_set_on_init() {
    let mock = MockEnv::new().build().unwrap();
    let config = mock.query_config();
    assert_eq!(config.address_provider, mock.address_provider_contract.to_string());
}

#[test]
fn only_owner_can_update_address_provider() {
    let mut mock = MockEnv::new().build().unwrap();
    let bad_guy = Addr::unchecked("doctor_otto_983");
    let res = mock.update_config(&bad_guy, Some("new_address".to_string()), None);
    assert_err(res, Owner(OwnerError::NotOwner {}));
}

#[test]
fn update_address_provider() {
    let mut mock = MockEnv::new().build().unwrap();
    let init_config = mock.query_config();

    // passing None does not change the address provider
    mock.update_config(&mock.query_owner(), None, None).unwrap();
    let current_config = mock.query_config();
    assert_eq!(current_config.address_provider, init_config.address_provider);

    let new_ap = "address_provider_123".to_string();
    mock.update_config(&mock.query_owner(), Some(new_ap.clone()), None).unwrap();
    let current_config = mock.query_config();
    assert_eq!(current_config.address_provider, new_ap);
}

#[test]
fn update_max_perp_params() {
    let mut mock = MockEnv::new().max_perp_params(34).build().unwrap();
    let init_config = mock.query_config();
    assert_eq!(init_config.max_perp_params, 34);

    // passing None does not change the max_perp_params
    mock.update_config(&mock.query_owner(), None, None).unwrap();
    let current_config = mock.query_config();
    assert_eq!(current_config.max_perp_params, init_config.max_perp_params);

    mock.update_config(&mock.query_owner(), None, Some(92)).unwrap();
    let current_config = mock.query_config();
    assert_eq!(current_config.max_perp_params, 92);
}
