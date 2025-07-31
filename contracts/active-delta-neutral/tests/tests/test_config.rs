use mars_testing::multitest::helpers::MockEnv;

#[test]
fn test_config_is_created_on_instantiate() {
    let mock = MockEnv::new().build().unwrap();
    // query Config - it should be created by default
    let config = mock.query_active_delta_neutral_config();

    assert_eq!(config.owner, "owner");
    assert_eq!(config.credit_account_id, "2");
    // TODO should we query for mock? This will fail when new contracts added to mock or order of deployment changed
    assert_eq!(config.credit_manager_addr, "contract11");
    assert_eq!(config.oracle_addr, "contract4");
    assert_eq!(config.perps_addr, "contract13");
    assert_eq!(config.health_addr, "contract7");
    assert_eq!(config.red_bank_addr, "contract0");
}
