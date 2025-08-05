use cosmwasm_std::Addr;
use mars_testing::multitest::helpers::MockEnv;
use mars_types::active_delta_neutral::query::MarketConfig;

use crate::tests::helpers::delta_neutral_helpers::{
    add_active_delta_neutral_market, deploy_active_delta_neutral_contract,
};

#[test]
fn test_position_modification() {
    // Set up the mars mocks
    let mut mock = MockEnv::new().build().unwrap();

    let active_delta_neutral = deploy_active_delta_neutral_contract(&mut mock);
    add_active_delta_neutral_market(
        &Addr::unchecked("owner"),
        MarketConfig {
            market_id: "btc".to_string(),
            usdc_denom: "ibc/B559A80D62249C8AA07A380E2A2BEA6E5CA9A6F079C912C3A9E9B494105E4F81"
                .to_string(),
            spot_denom: "factory/neutronasdfkldshfkldsjfklfdsaaaaassss111/btc".to_string(),
            perp_denom: "perps/ubtc".to_string(),
            k: 1000,
        },
        &mut mock,
        &active_delta_neutral,
    )
    .unwrap();

    // Increase the position
    // mock.increase_position(
    //     "ubtc".to_string(),
    //     Uint128::new(1000000),
    // );

    // let res = buy_delta_neutral_market(
    //     &Addr::unchecked("owner"),
    //     "btc",
    //     Uint128::new(1000000),
    //     SwapperRoute::Duality(DualityRoute {
    //         from: "ibc/B559A80D62249C8AA07A380E2A2BEA6E5CA9A6F079C912C3A9E9B494105E4F81"
    //             .to_string(),
    //         to: "perps/ubtc".to_string(),
    //         swap_denoms: vec![
    //             "ibc/B559A80D62249C8AA07A380E2A2BEA6E5CA9A6F079C912C3A9E9B494105E4F81".to_string(),
    //             "perps/ubtc".to_string(),
    //         ],
    //     }),
    //     &mut mock,
    //     &active_delta_neutral,
    // );
    // println!("res: {:#?}", res);
    // assert!(res.is_ok());
}
