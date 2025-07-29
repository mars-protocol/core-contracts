use mars_testing::multitest::helpers::MockEnv;
use mars_types::address_provider::MarsAddressType;

#[test]
fn test_position_modification() {
    // Set up the mars mocks
    let mut mock = MockEnv::new().build().unwrap();
    let credit_manager_addr = mock.query_address_provider(MarsAddressType::CreditManager);
    println!("Credit Manager Address: {}", credit_manager_addr);

    // Deploy the contract
    let contract_addr = mock.deploy_contract("dynamic_delta_neutral", "").unwrap();

    //
    // Dep.o
    // Create the mocks
    // open a position
    // Then,
}


