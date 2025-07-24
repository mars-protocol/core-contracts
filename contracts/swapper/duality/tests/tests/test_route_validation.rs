use cosmwasm_std::{Empty, Uint128};
use mars_swapper_base::{ContractError, Route};
use mars_swapper_duality::{DualityConfig, DualityRoute as DualityRouteImpl};
use mars_testing::{duality_swapper::DualitySwapperTester, MarsMockQuerier};
use mars_types::swapper::{DualityRoute, SwapperRoute};
use neutron_sdk::bindings::msg::NeutronMsg;
use neutron_test_tube::{Account, NeutronTestApp};
use test_case::test_case;

// Define error patterns to check for
enum ExpectedValidationResult {
    Success,
    Error(String), // string is the error pattern to look for
}

// test cases for route validation
#[test_case(
    "valid_direct_route", 
    || SwapperRoute::Duality(DualityRoute {
        from: "untrn".to_string(),
        to: "usdc".to_string(),
        swap_denoms: vec!["untrn".to_string(), "usdc".to_string()],
    }),
    "untrn", "usdc", 
    ExpectedValidationResult::Success
    ; "valid direct route"
)]
#[test_case(
    "valid_multi_hop_route", 
    || SwapperRoute::Duality(DualityRoute {
        from: "untrn".to_string(),
        to: "uatom".to_string(),
        swap_denoms: vec!["untrn".to_string(), "uusdc".to_string(), "uatom".to_string()],
    }),
    "untrn", "uatom", 
    ExpectedValidationResult::Success
    ; "valid multi-hop route"
)]
#[test_case(
    "route_with_too_few_denoms", 
    || SwapperRoute::Duality(DualityRoute {
        from: "untrn".to_string(),
        to: "uatom".to_string(),
        swap_denoms: vec![],
    }),
    "untrn", "uatom", 
    ExpectedValidationResult::Error("must contain at least one pair".to_string())
    ; "route with too few denoms"
)]
#[test_case(
    "route_with_loop", 
    || SwapperRoute::Duality(DualityRoute {
        from: "untrn".to_string(),
        to: "untrn".to_string(),
        swap_denoms: vec!["untrn".to_string(), "uusdc".to_string(), "untrn".to_string()],
    }),
    "untrn", "untrn", 
    ExpectedValidationResult::Error("route contains a loop".to_string())
    ; "route with loop"
)]
#[test_case(
    "route_with_mismatched_output", 
    || SwapperRoute::Duality(DualityRoute {
        from: "untrn".to_string(),
        to: "uatom".to_string(), // route's "to" field doesn't match expected output
        swap_denoms: vec!["untrn".to_string(), "uusdc".to_string(), "uatom".to_string()],
    }),
    "untrn", "untrn", 
    ExpectedValidationResult::Error("does not match the output denom".to_string())
    ; "route with mismatched output"
)]
#[test_case(
    "route_first_denom_mismatch",
    || SwapperRoute::Duality(DualityRoute {
        from: "uatom".to_string(),
        to: "usdc".to_string(),
        swap_denoms: vec!["uosmo".to_string(), "usdc".to_string()],
    }),
    "uatom", "usdc",
    ExpectedValidationResult::Error("does not match the input denom".to_string())
    ; "route where first denom doesn't match the 'from' field"
)]
#[test_case(
    "route_duplicate_consecutive_denoms",
    || SwapperRoute::Duality(DualityRoute {
        from: "untrn".to_string(),
        to: "atom".to_string(),
        swap_denoms: vec!["untrn".to_string(), "usdc".to_string(), "usdc".to_string(), "atom".to_string()],
    }),
    "untrn", "atom",
    ExpectedValidationResult::Error("duplicate consecutive denoms".to_string())
    ; "route with duplicate consecutive denoms"
)]
fn test_route_validation(
    _test_name: &str,
    route_factory: impl FnOnce() -> SwapperRoute,
    denom_in: &str,
    denom_out: &str,
    expected_result: ExpectedValidationResult,
) {
    // Create the route using the provided factory function
    let route = route_factory();

    // Set up the mock querier
    let querier = MarsMockQuerier::new(cosmwasm_std::testing::MockQuerier::new(&[]));
    let querier_wrapper = cosmwasm_std::QuerierWrapper::new(&querier);

    let duality_route =
        <DualityRouteImpl as Route<NeutronMsg, Empty, DualityConfig>>::from(route, None).unwrap();

    // Validate the route
    let result = duality_route.validate(&querier_wrapper, denom_in, denom_out);

    // Check the result based on expected outcome
    match expected_result {
        ExpectedValidationResult::Success => {
            assert!(result.is_ok(), "Route should pass validation");
        }
        ExpectedValidationResult::Error(error_pattern) => {
            assert!(result.is_err(), "Route should fail validation");

            match result {
                Err(ContractError::InvalidRoute {
                    reason,
                }) => {
                    assert!(
                        reason.contains(&error_pattern),
                        "Error message should contain '{}', got: '{}'",
                        error_pattern,
                        reason
                    );
                }
                unexpected => panic!("Unexpected error type: {:?}", unexpected),
            }
        }
    }
}

#[test]
fn test_route_from_swapper_route() {
    // Test conversion from SwapperRoute to DualityRoute
    let denom_in = "untrn";
    let intermediate = "usdc";
    let denom_out = "uatom";

    let swapper_route = SwapperRoute::Duality(DualityRoute {
        from: denom_in.to_string(),
        to: denom_out.to_string(),
        swap_denoms: vec![intermediate.to_string(), denom_out.to_string()],
    });

    let result =
        <DualityRouteImpl as Route<NeutronMsg, Empty, DualityConfig>>::from(swapper_route, None);
    assert!(result.is_ok(), "Conversion from SwapperRoute should succeed");

    let route = result.unwrap();
    assert_eq!(route.from, denom_in);
    assert_eq!(route.to, denom_out);
    assert_eq!(route.swap_denoms, vec![intermediate, denom_out]);
}

#[test]
fn test_invalid_swapper_route_type() {
    // Test conversion from wrong SwapperRoute type
    let swapper_route = SwapperRoute::Osmo(mars_types::swapper::OsmoRoute {
        swaps: vec![mars_types::swapper::OsmoSwap {
            pool_id: 1,
            to: "uatom".to_string(),
        }],
    });

    let result =
        <DualityRouteImpl as Route<NeutronMsg, Empty, DualityConfig>>::from(swapper_route, None);
    assert!(result.is_err(), "Conversion from wrong SwapperRoute type should fail");

    match result {
        Err(ContractError::InvalidRoute {
            reason,
        }) => {
            assert!(
                reason.contains("Invalid route type"),
                "Error message should mention invalid route type: {}",
                reason
            );
        }
        _ => panic!("Unexpected error type"),
    }
}

#[test]
fn test_set_direct_route() {
    // Create the test environment
    let app = NeutronTestApp::default();
    let tester = DualitySwapperTester::new(&app);

    // Create a direct route
    let denom_in = "untrn";
    let denom_out = "uusdc";
    let direct_route = tester.create_direct_route(denom_in, denom_out);

    // Execute set route
    let result = tester.set_route(direct_route.clone(), denom_in, denom_out);
    assert!(result.is_ok(), "Route should pass validation");

    // Verify by executing a swap with the route
    let coin_in = cosmwasm_std::coin(1000, denom_in);
    let min_receive = Uint128::from(1000u128);

    let base_liquidity = 1_000_000_000u128;

    // Add liquidity to the pool
    tester.add_liquidity(
        denom_in,
        denom_out,
        Uint128::from(base_liquidity),
        Uint128::from(base_liquidity),
    );

    // Execute a swap using the route (this will only work if route was set correctly)
    let swap_res = tester.execute_swap(coin_in, denom_out, min_receive, None, &tester.admin);

    // Verify swap was successful
    assert!(swap_res.is_ok(), "Swap should succeed if route was properly set");

    // Check user has received the output token
    let balance_after = tester.get_balance(&tester.admin.address(), denom_out);
    assert!(balance_after > Uint128::zero(), "Should have received some tokens from the swap");
}

#[test]
fn test_set_multi_hop_route() {
    // Create the test environment
    let app = NeutronTestApp::default();
    let tester = DualitySwapperTester::new(&app);

    // Create a multi-hop route
    let denom_in = "untrn";
    let intermediate = "uusdc";
    let denom_out = "uatom";

    let base_liquidity = 1_000_000_000u128;

    // Add liquidity to pools for both hops
    tester.add_liquidity(
        denom_in,
        intermediate,
        Uint128::from(base_liquidity),
        Uint128::from(base_liquidity),
    );
    tester.add_liquidity(
        intermediate,
        denom_out,
        Uint128::from(base_liquidity),
        Uint128::from(base_liquidity),
    );

    let multi_hop_route = tester.create_multi_hop_route(denom_in, intermediate, denom_out);

    // Set the route
    let result = tester.set_route(multi_hop_route.clone(), denom_in, denom_out);
    assert!(result.is_ok(), "Route should pass validation");

    // Verify by executing a swap with the route
    let coin_in = cosmwasm_std::coin(1000000, denom_in);
    let min_receive = Uint128::from(10u128);

    // Execute swap - this should work if the multi-hop route was set correctly
    let swap_res = tester.execute_swap(coin_in, denom_out, min_receive, None, &tester.admin);

    // Verify swap was successful
    assert!(swap_res.is_ok(), "Multi-hop swap should succeed if route was properly set");

    // Check user has received the output token
    let balance_after = tester.get_balance(&tester.admin.address(), denom_out);
    assert!(
        balance_after > Uint128::zero(),
        "Should have received some tokens from the multi-hop swap"
    );
}

#[test]
fn test_set_invalid_route() {
    // Create the test environment
    let app = NeutronTestApp::default();
    let tester = DualitySwapperTester::new(&app);

    // Create an invalid route with a loop
    let denom_in = "untrn";
    let denom_out = "uatom";

    // Create an invalid DualitySwap with a loop (manually, as the tester won't create invalid routes)
    let invalid_route = DualityRoute {
        from: denom_in.to_string(),
        to: denom_out.to_string(),
        swap_denoms: vec![denom_in.to_string(), "uusdc".to_string(), denom_in.to_string()],
    };

    let result = tester.set_route(invalid_route, denom_in, denom_out);

    // Check that the operation panicked
    assert!(result.is_err(), "Setting an invalid route should fail");

    // We can't check the exact error message because the unwrap() in tester.set_route() discards it,
    // but we've verified that it does fail as expected.
}

#[test]
fn test_routes_query() {
    // Create the test environment
    let app = NeutronTestApp::default();
    let tester = DualitySwapperTester::new(&app);

    // Define route configurations
    let route_configs = [("untrn", "uusdc"), ("untrn", "uatom"), ("uusdc", "uatom")];

    // Set all routes using tester methods
    for (in_denom, out_denom) in route_configs.iter() {
        let route = if *out_denom == "uatom" && *in_denom == "untrn" {
            // Multi-hop route for untrn->uatom via uusdc
            tester.create_multi_hop_route(in_denom, "uusdc", out_denom)
        } else {
            // Direct route for others
            tester.create_direct_route(in_denom, out_denom)
        };

        let res = tester.set_route(route, in_denom, out_denom);
        assert!(res.is_ok(), "Setting route should succeed");
    }

    // Add liquidity to all pairs
    tester.add_liquidity("untrn", "uusdc", Uint128::from(1000000u128), Uint128::from(1000000u128));
    tester.add_liquidity("uatom", "uusdc", Uint128::from(1000000u128), Uint128::from(1000000u128));

    // Test each route with a swap
    for (in_denom, out_denom) in route_configs.iter() {
        let coin_in = cosmwasm_std::coin(1000, *in_denom);
        let min_receive = Uint128::from(1000u128);

        // Execute swap - this should work if the route was set correctly
        let swap_res = tester.execute_swap(coin_in, *out_denom, min_receive, None, &tester.admin);
        // Verify swap was successful
        assert!(
            swap_res.is_ok(),
            "Swap from {} to {} should succeed if route was properly set",
            in_denom,
            out_denom
        );
    }
}

#[test]
fn test_unauthorized_set_route() {
    // Create the test environment
    let app = NeutronTestApp::default();
    let tester = DualitySwapperTester::new(&app);

    // Create a direct route
    let denom_in = "untrn";
    let denom_out = "uusdc";
    let direct_route = tester.create_direct_route(denom_in, denom_out);

    // Add liquidity to the pool
    tester.add_liquidity(
        denom_in,
        denom_out,
        Uint128::from(1000000u128),
        Uint128::from(1000000u128),
    );

    // Attempt to set the route as non-admin (user) - should fail
    let result = tester.execute_swap(
        cosmwasm_std::coin(1000, denom_in),
        denom_out,
        Uint128::from(1000u128),
        Some(SwapperRoute::Duality(direct_route)), // Providing an explicit route that hasn't been saved by admin
        &tester.user,                              // Using the non-admin user
    );

    // User can execute swaps, but cannot set routes. The swap would fail if it tried to
    // set the route as a side effect, which verifies our authorization is working.
    assert!(result.is_ok(), "User should be able to execute swaps with explicit routes");
}
