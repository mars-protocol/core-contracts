use cosmwasm_std::Addr;
use cw_paginate::PaginationResponse;
use mars_testing::multitest::helpers::MockEnv;
use mars_types::active_delta_neutral::query::MarketConfig;

use crate::tests::helpers::delta_neutral_helpers::{
    add_active_delta_neutral_market, deploy_active_delta_neutral_contract,
    query_active_delta_neutral_market, query_all_active_delta_neutral_markets,
};

#[test]
fn test_query_market_config() {
    let owner = Addr::unchecked("owner");
    let mut mock = MockEnv::new().build().unwrap();

    // Add a market
    let market_config = MarketConfig {
        market_id: "market_1".to_string(),
        usdc_denom: "ibc/B559A80D62249C8AA07A380E2A2BEA6E5CA9A6F079C912C3A9E9B494105E4F81"
            .to_string(),
        spot_denom: "ibc/0000000000000000000000000000000000000000000000000000000000000000"
            .to_string(),
        perp_denom: "perps/ubtc".to_string(),
        k: 300u64,
    };
    let active_delta_neutral = deploy_active_delta_neutral_contract(&mut mock);
    let res = add_active_delta_neutral_market(
        &owner,
        market_config.clone(),
        &mut mock,
        &active_delta_neutral,
    );
    assert!(res.is_ok());

    // Query the saved market config
    let loaded: MarketConfig =
        query_active_delta_neutral_market(&mock, &active_delta_neutral, &market_config.market_id);

    assert_eq!(market_config, loaded);
}

#[test]
fn test_query_all_market_configs() {
    let owner = Addr::unchecked("owner");
    let mut mock = MockEnv::new().build().unwrap();
    let active_delta_neutral = deploy_active_delta_neutral_contract(&mut mock);

    // Add a market
    let market_config = valid_config();
    let mut market_config2 = valid_config();
    market_config2.market_id = "market_2".to_string();
    let res = add_active_delta_neutral_market(
        &owner,
        market_config.clone(),
        &mut mock,
        &active_delta_neutral,
    );
    let res2 = add_active_delta_neutral_market(
        &owner,
        market_config2.clone(),
        &mut mock,
        &active_delta_neutral,
    );

    assert!(res.is_ok());
    assert!(res2.is_ok());

    // Query the saved market config
    let loaded: PaginationResponse<MarketConfig> =
        query_all_active_delta_neutral_markets(&mock, &active_delta_neutral, None, None);

    assert_eq!(vec![market_config, market_config2], loaded.data);
}

fn valid_config() -> MarketConfig {
    MarketConfig {
        market_id: "market_1".to_string(),
        usdc_denom: "ibc/B559A80D62249C8AA07A380E2A2BEA6E5CA9A6F079C912C3A9E9B494105E4F81"
            .to_string(),
        spot_denom: "ibc/0000000000000000000000000000000000000000000000000000000000000000"
            .to_string(),
        perp_denom: "perps/ubtc".to_string(),
        k: 1,
    }
}

#[test]
fn valid_config_passes() {
    let config = valid_config();
    assert!(config.validate().is_ok());
}

#[test]
fn invalid_usdc_denom_fails() {
    let mut config = valid_config();
    config.usdc_denom = "".to_string();
    assert!(config.validate().is_err());
}

#[test]
fn invalid_spot_denom_fails() {
    let mut config = valid_config();
    config.spot_denom = "".to_string();
    assert!(config.validate().is_err());
}

#[test]
fn perp_denom_not_perps_prefix_fails() {
    let mut config = valid_config();
    config.perp_denom = "BTCUSD".to_string();
    let err = config.validate().unwrap_err().to_string();
    assert!(err.contains("Perp denom must start with 'perps/'"));
}

#[test]
fn k_zero_fails() {
    let mut config = valid_config();
    config.k = 0;
    assert!(config.validate().is_err());
}
