use cosmwasm_std::{
    coin, testing::mock_env, to_json_binary, CosmosMsg, Decimal, Empty, SubMsg, Uint128, WasmMsg,
};
use mars_rewards_collector_neutron::entry::execute;
use mars_testing::mock_info;
use mars_types::{
    rewards_collector::{ConfigResponse, ExecuteMsg, QueryMsg},
    swapper::{DualityRoute, SwapperRoute},
};

use super::helpers::*;
use crate::tests::helpers;

#[test]
fn test_swap_asset_with_neutron_swapper() {
    let mut deps = setup_test_env();

    let cfg: ConfigResponse = helpers::query(deps.as_ref(), QueryMsg::Config {});

    let usdc_denom = "uusdc".to_string();
    let mars_denom = "umars".to_string();
    let atom_denom = "uatom".to_string();

    let uusdc_usd_price = Decimal::one();
    let umars_uusdc_price = Decimal::from_ratio(5u128, 10u128); // 0.5 uusdc = 1 umars
    let uatom_uusdc_price = Decimal::from_ratio(125u128, 10u128); // 12.5 uusd = 1 uatom

    deps.querier.set_oracle_price(&usdc_denom, uusdc_usd_price);
    deps.querier.set_oracle_price(&mars_denom, umars_uusdc_price);
    deps.querier.set_oracle_price(&atom_denom, uatom_uusdc_price);

    deps.querier.set_swapper_estimate_price(&mars_denom, umars_uusdc_price);
    deps.querier.set_swapper_estimate_price(&atom_denom, uatom_uusdc_price);
    deps.querier.set_swapper_estimate_price(&usdc_denom, uusdc_usd_price);

    let safety_fund_input = Uint128::new(14724);
    let fee_collector_input = Uint128::new(27345);

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("jake"),
        ExecuteMsg::SwapAsset {
            denom: "uatom".to_string(),
            amount: Some(Uint128::new(42069)),
            safety_fund_route: Some(SwapperRoute::Duality(DualityRoute {
                from: "uatom".to_string(),
                to: cfg.safety_fund_config.target_denom.to_string(),
                swap_denoms: vec![
                    "uatom".to_string(),
                    cfg.safety_fund_config.target_denom.to_string(),
                ],
            })),
            fee_collector_route: Some(SwapperRoute::Duality(DualityRoute {
                from: "uatom".to_string(),
                to: cfg.fee_collector_config.target_denom.to_string(),
                swap_denoms: vec![
                    "uatom".to_string(),
                    cfg.fee_collector_config.target_denom.to_string(),
                ],
            })),
            safety_fund_min_receive: Some(Uint128::new(178528)),
            fee_collector_min_receive: Some(Uint128::new(663140)),
        },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 2);

    let swap_msg: CosmosMsg = WasmMsg::Execute {
        contract_addr: "duality_swapper".to_string(),
        msg: to_json_binary(&mars_types::swapper::ExecuteMsg::<Empty, Empty>::SwapExactIn {
            coin_in: coin(safety_fund_input.u128(), "uatom"),
            denom_out: cfg.safety_fund_config.target_denom.to_string(),
            min_receive: Uint128::new(178528),
            route: Some(SwapperRoute::Duality(DualityRoute {
                from: "uatom".to_string(),
                to: cfg.safety_fund_config.target_denom.to_string(),
                swap_denoms: vec![
                    "uatom".to_string(),
                    cfg.safety_fund_config.target_denom.to_string(),
                ],
            })),
        })
        .unwrap(),
        funds: vec![coin(safety_fund_input.u128(), "uatom")],
    }
    .into();
    assert_eq!(res.messages[0], SubMsg::new(swap_msg));

    let swap_msg: CosmosMsg = WasmMsg::Execute {
        contract_addr: "duality_swapper".to_string(),
        msg: to_json_binary(&mars_types::swapper::ExecuteMsg::<Empty, Empty>::SwapExactIn {
            coin_in: coin(fee_collector_input.u128(), "uatom"),
            denom_out: cfg.fee_collector_config.target_denom.to_string(),
            min_receive: Uint128::new(663140),
            route: Some(SwapperRoute::Duality(DualityRoute {
                from: "uatom".to_string(),
                to: cfg.fee_collector_config.target_denom.to_string(),
                swap_denoms: vec![
                    "uatom".to_string(),
                    cfg.fee_collector_config.target_denom.to_string(),
                ],
            })),
        })
        .unwrap(),
        funds: vec![coin(fee_collector_input.u128(), "uatom")],
    }
    .into();
    assert_eq!(res.messages[1], SubMsg::new(swap_msg));
}
