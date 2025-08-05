use mars_testing::multitest::helpers::MockEnv;

use crate::tests::helpers::delta_neutral_helpers::{
    deploy_active_delta_neutral_contract, query_active_delta_neutral_config,
};

#[test]
fn test_config_is_created_on_instantiate() {
    let mut mock = MockEnv::new().build().unwrap();
    let active_delta_neutral = deploy_active_delta_neutral_contract(&mut mock);
    // query Config - it should be created by default
    let config = query_active_delta_neutral_config(&mock, &active_delta_neutral);

    assert_eq!(config.owner, "owner");
    assert_eq!(config.credit_account_id, Some("2".to_string()));
    // TODO should we query from mock? This will fail when new contracts added to mock or order of deployment changed
    assert_eq!(config.credit_manager_addr, "contract11");
    assert_eq!(config.oracle_addr, "contract4");
    assert_eq!(config.perps_addr, "contract13");
    assert_eq!(config.health_addr, "contract7");
    assert_eq!(config.red_bank_addr, "contract0");
}
