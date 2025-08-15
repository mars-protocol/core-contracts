use cosmwasm_std::Addr;
use mars_testing::multitest::helpers::MockEnv;
use mars_types::{
    active_delta_neutral::query::MarketConfig,
    swapper::{DualityRoute, SwapperRoute},
};

use crate::tests::helpers::delta_neutral_helpers::{
    add_active_delta_neutral_market, buy_delta_neutral_market,
    deploy_active_delta_neutral_contract, deposit, query_contract_credit_manager_positions,
};

#[test]
fn test_position_modification() {
    // Set up the mars mocks
    let user = Addr::unchecked("user");
    let owner = Addr::unchecked("owner");
    let bot = Addr::unchecked("bot");

    let usdc_denom = "ibc/B559A80D62249C8AA07A380E2A2BEA6E5CA9A6F079C912C3A9E9B494105E4F81";
    let spot_denom = "uosmo";
    let perp_denom = "perps/ubtc";

    // Fund the user and bot accounts
    use cosmwasm_std::{Coin, Uint128};
    let addrs = vec![user.clone(), bot.clone(), owner.clone()];
    let coins = vec![
        Coin {
            denom: usdc_denom.to_string(),
            amount: Uint128::new(1_000_000_000),
        },
        Coin {
            denom: spot_denom.to_string(),
            amount: Uint128::new(1_000_000_000),
        },
    ];
    let mut mock = MockEnv::new().fund_accounts(addrs, coins).build().unwrap();

    let active_delta_neutral = deploy_active_delta_neutral_contract(&mut mock);
    add_active_delta_neutral_market(
        &owner,
        MarketConfig {
            market_id: "btc".to_string(),
            usdc_denom: usdc_denom.to_string(),
            spot_denom: spot_denom.to_string(),
            perp_denom: perp_denom.to_string(),
            k: 1000,
        },
        &mut mock,
        &active_delta_neutral,
    )
    .unwrap();

    let deposit_coins = vec![Coin {
        denom: usdc_denom.to_string(),
        amount: Uint128::new(1_000_000),
    }];

    let deposit_res = deposit(&owner, deposit_coins, &mut mock, &active_delta_neutral);

    assert!(deposit_res.is_ok());

    let positions = query_contract_credit_manager_positions(&mock, &active_delta_neutral);
    println!("postions: {:#?}", positions);

    let res = buy_delta_neutral_market(
        &owner,
        "btc",
        Uint128::new(1000000),
        SwapperRoute::Duality(DualityRoute {
            from: usdc_denom.to_string(),
            to: spot_denom.to_string(),
            swap_denoms: vec![
                usdc_denom.to_string(),
                spot_denom.to_string(),
            ],
        }),
        &mut mock,
        &active_delta_neutral,
    );

    println!("res: {:#?}", res);
    assert!(res.is_ok());
}
