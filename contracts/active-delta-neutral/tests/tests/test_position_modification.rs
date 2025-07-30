use mars_testing::multitest::helpers::MockEnv;
use mars_types::address_provider::MarsAddressType;

#[test]
fn test_position_modification() {
    // Set up the mars mocks
    let mock = MockEnv::new().build().unwrap();
    let credit_manager_addr = mock.query_address_provider(MarsAddressType::CreditManager);
    let active_delta_neutral_addr =
        mock.query_address_provider(MarsAddressType::ActiveDeltaNeutral);
    println!("Credit Manager Address: {}", credit_manager_addr);
    println!("Active Delta Neutral Address: {}", active_delta_neutral_addr);
}
