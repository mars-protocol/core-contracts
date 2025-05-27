use cosmwasm_std::Empty;
use mars_swapper_base::{ContractError, Route};
use mars_swapper_duality::{DualityConfig, DualityRoute};
use mars_testing::MarsMockQuerier;
use mars_types::swapper::{DualityRoute as SwapperDualityRoute, SwapperRoute};
use neutron_sdk::bindings::msg::NeutronMsg;
use test_case::test_case;

// helper function to create a simple direct route
fn create_direct_route(from: &str, to: &str) -> DualityRoute {
    DualityRoute {
        from: from.to_string(),
        to: to.to_string(),
        swap_denoms: vec![from.to_string(), to.to_string()],
    }
}

// helper function to create a multi-hop route
fn create_multi_hop_route(from: &str, via: &[&str], to: &str) -> DualityRoute {
    let mut swap_denoms = vec![];
    swap_denoms.push(from.to_string());
    for denom in via {
        swap_denoms.push(denom.to_string());
    }
    swap_denoms.push(to.to_string());

    DualityRoute {
        from: from.to_string(),
        to: to.to_string(),
        swap_denoms,
    }
}

// define error patterns to check for
enum ExpectedValidationResult {
    Success,
    Error(String), // string is the error pattern to look for
}

// test cases for route validation
#[test_case(
    "valid_direct_route", 
    || create_direct_route("untrn", "usdc"), 
    "untrn", "usdc", 
    ExpectedValidationResult::Success
    ; "valid direct route"
)]
#[test_case(
    "valid_multi_hop_route", 
    || create_multi_hop_route("untrn", &["uusdc"], "uatom"),
    "untrn", "uatom", 
    ExpectedValidationResult::Success
    ; "valid multi-hop route"
)]
#[test_case(
    "route_with_too_few_denoms", 
    || DualityRoute {
        from: "untrn".to_string(),
        to: "uatom".to_string(),
        swap_denoms: vec![],
    },
    "untrn", "uatom", 
    ExpectedValidationResult::Error("must contain at least one pair".to_string())
    ; "route with too few denoms"
)]
#[test_case(
    "route_with_loop", 
    || DualityRoute {
        from: "untrn".to_string(),
        to: "untrn".to_string(),
        swap_denoms: vec!["untrn".to_string(), "uusdc".to_string(), "untrn".to_string()],
    },
    "untrn", "untrn", 
    ExpectedValidationResult::Error("route contains a loop".to_string())
    ; "route with loop"
)]
#[test_case(
    "route_with_mismatched_output", 
    || DualityRoute {
        from: "untrn".to_string(),
        to: "uatom".to_string(), // route's "to" field doesn't match expected output
        swap_denoms: vec!["untrn".to_string(), "uusdc".to_string(), "uatom".to_string()],
    },
    "untrn", "untrn", 
    ExpectedValidationResult::Error("does not match the output denom".to_string())
    ; "route with mismatched output"
)]
fn test_route_validation(
    _test_name: &str,
    route_factory: impl FnOnce() -> DualityRoute,
    denom_in: &str,
    denom_out: &str,
    expected_result: ExpectedValidationResult,
) {
    // create the route using the provided factory function
    let route = route_factory();

    // set up the mock querier
    let querier = MarsMockQuerier::new(cosmwasm_std::testing::MockQuerier::new(&[]));
    let querier_wrapper = cosmwasm_std::QuerierWrapper::new(&querier);

    // validate the route
    let result = route.validate(&querier_wrapper, denom_in, denom_out);

    // check the result based on expected outcome
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
    // test conversion from SwapperRoute to DualityRoute
    let denom_in = "untrn";
    let intermediate = "usdc";
    let denom_out = "uatom";

    let swapper_route = SwapperRoute::Duality(SwapperDualityRoute {
        from: denom_in.to_string(),
        to: denom_out.to_string(),
        swap_denoms: vec![intermediate.to_string(), denom_out.to_string()],
    });

    let result =
        <DualityRoute as Route<NeutronMsg, Empty, DualityConfig>>::from(swapper_route, None);
    assert!(result.is_ok(), "Conversion from SwapperRoute should succeed");

    let route = result.unwrap();
    assert_eq!(route.from, denom_in);
    assert_eq!(route.to, denom_out);
    assert_eq!(route.swap_denoms, vec![intermediate, denom_out]);
}

#[test]
fn test_invalid_swapper_route_type() {
    // test conversion from wrong SwapperRoute type
    let swapper_route = SwapperRoute::Osmo(mars_types::swapper::OsmoRoute {
        swaps: vec![mars_types::swapper::OsmoSwap {
            pool_id: 1,
            to: "uatom".to_string(),
        }],
    });

    let result =
        <DualityRoute as Route<NeutronMsg, Empty, DualityConfig>>::from(swapper_route, None);
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
