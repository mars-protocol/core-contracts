use mars_testing::multitest::helpers::MockEnv;

#[test]
fn test_config() {
    let mut mock = MockEnv::new().build().unwrap();
    // query Config - it should be default
    let config = mock.query_active_delta_neutral_config();
    println!("config: {:#?}", config);
}
