use cosmwasm_std::{Addr, Uint128};
use mars_types::account_nft::NftConfigUpdates;

use super::helpers::MockEnv;

#[test]
fn only_minter_can_update_config() {
    let mut mock = MockEnv::new().build().unwrap();

    let bad_guy = Addr::unchecked("bad_guy");
    let res = mock.update_config(
        &bad_guy,
        &NftConfigUpdates {
            max_value_for_burn: None,
            address_provider_contract_addr: None,
        },
    );

    if res.is_ok() {
        panic!("Non-minter should not be able to propose new minter");
    }
}

#[test]
fn minter_can_update_config() {
    let mut mock = MockEnv::new().build().unwrap();

    let new_max_burn_val = Uint128::new(4918453);
    let new_ap_contract = "new_ap_contract_123".to_string();

    let updates = NftConfigUpdates {
        max_value_for_burn: Some(new_max_burn_val),
        address_provider_contract_addr: Some(new_ap_contract.clone()),
    };

    mock.update_config(&mock.minter.clone(), &updates).unwrap();

    let config = mock.query_config();
    assert_eq!(config.max_value_for_burn, new_max_burn_val);
    assert_eq!(config.address_provider_contract_addr, new_ap_contract);
}
