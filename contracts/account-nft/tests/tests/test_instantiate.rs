use super::helpers::{MockEnv, MAX_VALUE_FOR_BURN};

#[test]
fn instantiated_storage_vars() {
    let mut mock = MockEnv::new().set_minter("spiderman_1337").build().unwrap();

    let config = mock.query_config();
    assert_eq!(config.address_provider_contract_addr, "contract0".to_string());
    assert_eq!(config.max_value_for_burn, MAX_VALUE_FOR_BURN);

    let ownership = mock.query_ownership();
    assert_eq!("spiderman_1337", ownership.owner.unwrap());
    assert_eq!(None, ownership.pending_owner);

    mock.assert_next_id("1");
}
