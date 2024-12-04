use cosmwasm_std::Addr;
use mars_owner::{OwnerError, OwnerUpdate};
use mars_params::error::ContractError::Owner;

use super::helpers::{assert_err, MockEnv};

#[test]
fn risk_manager_on_init_default_to_owner() {
    let mock = MockEnv::new().build().unwrap();
    let risk_manager = mock.query_risk_manager();
    assert_eq!("owner", &risk_manager.to_string())
}

#[test]
fn risk_manager_set_on_init() {
    let mock =
        MockEnv::new().build_with_risk_manager(Some("risk_manager_123".to_string())).unwrap();
    let risk_manager = mock.query_risk_manager();
    assert_eq!("risk_manager_123".to_string(), risk_manager.to_string())
}

#[test]
fn only_risk_manager_can_update_risk_mananger() {
    // Baddie tries to update
    let mut mock =
        MockEnv::new().build_with_risk_manager(Some("risk_manager_123".to_string())).unwrap();
    let bad_guy = Addr::unchecked("doctor_otto_983");
    let res = mock.update_risk_manager(
        &bad_guy,
        OwnerUpdate::ProposeNewOwner {
            proposed: bad_guy.to_string(),
        },
    );
    assert_err(res, Owner(OwnerError::NotOwner {}));

    // Owner tries to update
    let owner = Addr::unchecked("owner");
    let res = mock.update_risk_manager(
        &owner,
        OwnerUpdate::ProposeNewOwner {
            proposed: owner.to_string(),
        },
    );
    assert_err(res, Owner(OwnerError::NotOwner {}));
}

#[test]
fn reset_risk_manager() {
    let mut mock =
        MockEnv::new().build_with_risk_manager(Some("risk_manager_123".to_string())).unwrap();

    let owner = Addr::unchecked("owner");
    mock.reset_risk_manager(&owner).unwrap();
    let risk_manager = mock.query_risk_manager();
    assert_eq!("owner", &risk_manager.to_string())
}

#[test]
fn only_owner_can_reset_risk_mananger() {
    let risk_manager = "risk_manager_123".to_string();
    let mut mock = MockEnv::new().build_with_risk_manager(Some(risk_manager.clone())).unwrap();

    // Baddie tries to update
    let bad_guy = Addr::unchecked("doctor_otto_983");
    let res = mock.reset_risk_manager(&bad_guy);
    assert_err(res, Owner(OwnerError::NotOwner {}));

    // Risk manager tries to update
    let res = mock.reset_risk_manager(&Addr::unchecked(risk_manager));
    assert_err(res, Owner(OwnerError::NotOwner {}));
}

#[test]
fn once_risk_manager_reset_can_update_risk_manager_again() {
    let mut mock =
        MockEnv::new().build_with_risk_manager(Some("risk_manager_123".to_string())).unwrap();

    let owner = Addr::unchecked("owner");
    mock.reset_risk_manager(&owner).unwrap();
    let risk_manager = mock.query_risk_manager();
    assert_eq!("owner", &risk_manager.to_string());

    let new_risk_manager = Addr::unchecked("risk_manager_456");
    mock.update_risk_manager(
        &owner,
        OwnerUpdate::ProposeNewOwner {
            proposed: new_risk_manager.to_string(),
        },
    )
    .unwrap();
    mock.update_risk_manager(&new_risk_manager, OwnerUpdate::AcceptProposed {}).unwrap();

    let risk_manager = mock.query_risk_manager();
    assert_eq!("risk_manager_456", &risk_manager.to_string())
}
