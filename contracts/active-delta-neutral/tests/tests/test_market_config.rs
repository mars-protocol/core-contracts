use cosmwasm_std::{Addr, Uint128};
use mars_testing::multitest::helpers::MockEnv;
use mars_types::active_delta_neutral::query::MarketConfig;

#[test]
fn test_market_config() {
    let owner = Addr::unchecked("owner");

    // Set up the mars mocks
    let mut mock = MockEnv::new().build().unwrap();
    // let credit_manager_addr = mock.query_address_provider(MarsAddressType::CreditManager);
    // let active_delta_neutral_addr =
    //     mock.query_address_provider(MarsAddressType::ActiveDeltaNeutral);

    // Add a market
    let market_config = MarketConfig {
        market_id: "market_1".to_string(),
        usdc_denom: "uusdc".to_string(),
        spot_denom: "spot".to_string(),
        perp_denom: "perp".to_string(),
        k: Uint128::new(300u128),
    };
    let res = mock.add_active_delta_neutral_market(&owner, market_config);
    assert!(res.is_ok());
}
