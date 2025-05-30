// use cosmwasm_std::{coin, Coin, Empty, QuerierWrapper, Uint128};
// use mars_swapper_base::Route;
// use mars_swapper_duality::{DualityConfig, DualityRoute};
// use mars_testing::MarsMockQuerier;
// use mars_types::swapper::EstimateExactInSwapResponse;
// use neutron_sdk::bindings::msg::NeutronMsg;
// use test_case::test_case;

// use crate::tests::helpers::{create_direct_route, create_multi_hop_route, mock_env};

// // Test cases for direct swap estimation
// #[test_case(
//     "basic_direct_swap",
//     || create_direct_route("untrn", "uusdc"),
//     coin("1000000", "untrn"),
//     Uint128::new(900000)
//     ; "basic direct swap estimation"
// )]
// #[test_case(
//     "small_amount_swap",
//     || create_direct_route("untrn", "uusdc"),
//     coin("100", "untrn"),
//     Uint128::new(90)
//     ; "swap with small amount"
// )]
// #[test_case(
//     "large_amount_swap",
//     || create_direct_route("untrn", "uusdc"),
//     coin("1000000000000", "untrn"),
//     Uint128::new(900000000000)
//     ; "swap with large amount"
// )]
// #[test_case(
//     "different_decimal_places",
//     || create_direct_route("uatom", "usdc"),
//     coin("1000000", "uatom"), // 6 decimal places
//     Uint128::new(2000000)     // 6 decimal places (with 2:1 exchange rate)
//     ; "tokens with different decimal places"
// )]
// fn test_direct_swap_estimation(
//     _test_name: &str,
//     route_factory: impl FnOnce() -> DualityRoute,
//     coin_in: Coin,
//     expected_amount_out: Uint128,
// ) {
//     // create the route using the provided factory function
//     let route = route_factory();

//     // set up mock environment
//     let env = mock_env();

//     // TODO: Set up the mock querier with appropriate responses for EstimatePlaceLimitOrder
//     let querier = MarsMockQuerier::new(cosmwasm_std::testing::MockQuerier::new(&[]));
//     let querier_wrapper = cosmwasm_std::QuerierWrapper::new(&querier);

//     // perform the estimation
//     let result = route.estimate_exact_in_swap(&querier_wrapper, &env, &coin_in);

//     // verify the result
//     assert!(result.is_ok(), "Estimation should succeed");
//     let response = result.unwrap();
//     assert_eq!(
//         response.amount,
//         expected_amount_out,
//         "Expected output amount {} but got {}",
//         expected_amount_out,
//         response.amount
//     );
// }

// // Test cases for multi-hop swap estimation
// #[test_case(
//     "basic_multi_hop_swap",
//     || create_multi_hop_route("untrn", &["uusdc"], "uatom"),
//     coin("1000000", "untrn"),
//     Uint128::new(800000)
//     ; "basic multi-hop swap estimation"
// )]
// #[test_case(
//     "long_path_multi_hop",
//     || create_multi_hop_route("untrn", &["uusdc", "uluna", "uosmo"], "uatom"),
//     coin("1000000", "untrn"),
//     Uint128::new(700000)
//     ; "multi-hop with long path (more hops = more slippage)"
// )]
// fn test_multi_hop_swap_estimation(
//     _test_name: &str,
//     route_factory: impl FnOnce() -> DualityRoute,
//     coin_in: Coin,
//     expected_amount_out: Uint128,
// ) {
//     // create the route using the provided factory function
//     let route = route_factory();

//     // set up mock environment
//     let env = mock_env();

//     // TODO: Set up the mock querier with appropriate responses for EstimateMultiHopSwap
//     let querier = MarsMockQuerier::new(cosmwasm_std::testing::MockQuerier::new(&[]));
//     let querier_wrapper = cosmwasm_std::QuerierWrapper::new(&querier);

//     // perform the estimation
//     let result = route.estimate_exact_in_swap(&querier_wrapper, &env, &coin_in);

//     // verify the result
//     assert!(result.is_ok(), "Estimation should succeed");
//     let response = result.unwrap();
//     assert_eq!(
//         response.amount,
//         expected_amount_out,
//         "Expected output amount {} but got {}",
//         expected_amount_out,
//         response.amount
//     );
// }

// #[test]
// fn test_estimation_with_invalid_route() {
//     // test with route having too few denoms
//     let invalid_route = DualityRoute {
//         from: "untrn".to_string(),
//         to: "uusdc".to_string(),
//         swap_denoms: vec![],
//     };

//     let env = mock_env();
//     let querier = MarsMockQuerier::new(cosmwasm_std::testing::MockQuerier::new(&[]));
//     let querier_wrapper = cosmwasm_std::QuerierWrapper::new(&querier);

//     let result = invalid_route.estimate_exact_in_swap(
//         &querier_wrapper,
//         &env,
//         &coin("1000000", "untrn")
//     );

//     assert!(result.is_err(), "Estimation with invalid route should fail");
//     // TODO: Check specific error message
// }

// #[test]
// fn test_neutron_query_errors() {
//     // create a valid route
//     let route = create_direct_route("untrn", "uusdc");

//     let env = mock_env();

//     // TODO: Set up mock querier to return errors for neutron queries
//     let querier = MarsMockQuerier::new(cosmwasm_std::testing::MockQuerier::new(&[]));
//     let querier_wrapper = cosmwasm_std::QuerierWrapper::new(&querier);

//     let result = route.estimate_exact_in_swap(
//         &querier_wrapper,
//         &env,
//         &coin("1000000", "untrn")
//     );

//     // TODO: Verify error is properly propagated to the caller once mocks are set up
//     // For now, it will likely succeed with a zero amount or fail in an unexpected way
// }
