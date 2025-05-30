use cosmwasm_std::{testing::mock_dependencies, to_json_binary, Addr, Empty, MessageInfo, Response, SubMsg, WasmMsg};

use mars_swapper_base::{ContractError, Route};
use mars_swapper_duality::{contract, DualityConfig, DualityRoute};
use mars_testing::MarsMockQuerier;
use mars_types::swapper::{DualitySwap as SwapperDualityRoute, ExecuteMsg, InstantiateMsg, QueryMsg, RouteResponse, RoutesResponse, SwapperRoute};
use neutron_sdk::bindings::msg::NeutronMsg;
use test_case::test_case;

use crate::tests::helpers::{create_direct_route, create_multi_hop_route};

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

// Helper function to set up the test environment with the duality swapper contract
fn setup_duality_swapper() -> (App, Addr) {
    // Create app
    let app = AppBuilder::new().build(|_, _, _| {});

    // Store and instantiate the duality swapper contract
    let contract = Box::new(ContractWrapper::new(
        contract::execute,
        contract::instantiate,
        contract::query,
    ));
    let code_id = app.store_code(contract);

    // Instantiate the contract
    let owner = Addr::unchecked("owner");
    let contract_addr = app
        .instantiate_contract(
            code_id,
            owner.clone(),
            &InstantiateMsg {
                owner: owner.to_string(),
            },
            &[],
            "Duality Swapper",
            None,
        )
        .unwrap();

    (app, contract_addr)
}

#[test]
fn test_set_direct_route() {
    let (mut app, contract_addr) = setup_duality_swapper();
    let owner = Addr::unchecked("owner");
    
    // Create a direct route
    let denom_in = "untrn";
    let denom_out = "usdc";
    let direct_route = create_direct_route(denom_in, denom_out);
    
    // Set the route
    let set_route_msg = ExecuteMsg::SetRoute {
        denom_in: denom_in.to_string(),
        denom_out: denom_out.to_string(),
        route: direct_route.clone(),
    };
    
    // Execute set route
    let res = app.execute_contract(owner.clone(), contract_addr.clone(), &set_route_msg, &[]).unwrap();
    
    // Verify attributes
    assert_eq!(
        res.events
            .iter()
            .find(|e| e.ty == "wasm")
            .unwrap()
            .attributes
            .iter()
            .find(|attr| attr.key == "action")
            .unwrap()
            .value,
        "rover/base/set_route"
    );
    
    // Query the route to ensure it was saved correctly
    let query_msg = QueryMsg::Route {
        denom_in: denom_in.to_string(),
        denom_out: denom_out.to_string(),
    };
    
    let res: RouteResponse<DualityRoute> = app
        .wrap()
        .query_wasm_smart(contract_addr.clone(), &query_msg)
        .unwrap();
    
    assert_eq!(res.denom_in, denom_in);
    assert_eq!(res.denom_out, denom_out);
    assert_eq!(res.route.from, direct_route.from);
    assert_eq!(res.route.to, direct_route.to);
    assert_eq!(res.route.swap_denoms, direct_route.swap_denoms);
}

#[test]
fn test_set_multi_hop_route() {
    let (mut app, contract_addr) = setup_duality_swapper();
    let owner = Addr::unchecked("owner");
    
    // Create a multi-hop route
    let denom_in = "untrn";
    let intermediate = "uusdc";
    let denom_out = "uatom";
    let multi_hop_route = create_multi_hop_route(denom_in, &[intermediate], denom_out);
    
    // Set the route
    let set_route_msg = ExecuteMsg::SetRoute {
        denom_in: denom_in.to_string(),
        denom_out: denom_out.to_string(),
        route: multi_hop_route.clone(),
    };
    
    // Execute set route
    app.execute_contract(owner.clone(), contract_addr.clone(), &set_route_msg, &[]).unwrap();
    
    // Query the route to ensure it was saved correctly
    let query_msg = QueryMsg::Route {
        denom_in: denom_in.to_string(),
        denom_out: denom_out.to_string(),
    };
    
    let res: RouteResponse<DualityRoute> = app
        .wrap()
        .query_wasm_smart(contract_addr.clone(), &query_msg)
        .unwrap();
    
    assert_eq!(res.denom_in, denom_in);
    assert_eq!(res.denom_out, denom_out);
    assert_eq!(res.route.from, multi_hop_route.from);
    assert_eq!(res.route.to, multi_hop_route.to);
    assert_eq!(res.route.swap_denoms, multi_hop_route.swap_denoms);
}

#[test]
fn test_set_invalid_route() {
    let (mut app, contract_addr) = setup_duality_swapper();
    let owner = Addr::unchecked("owner");
    
    // Create an invalid route with a loop
    let denom_in = "untrn";
    let denom_out = "untrn";
    let invalid_route = DualityRoute {
        from: denom_in.to_string(),
        to: denom_out.to_string(),
        swap_denoms: vec![denom_in.to_string(), "uusdc".to_string(), denom_in.to_string()],
    };
    
    // Set the route
    let set_route_msg = ExecuteMsg::SetRoute {
        denom_in: denom_in.to_string(),
        denom_out: denom_out.to_string(),
        route: invalid_route,
    };
    
    // Execute set route - should fail
    let err = app.execute_contract(owner.clone(), contract_addr.clone(), &set_route_msg, &[]).unwrap_err();
    
    // Verify the error contains the expected message
    assert!(err.to_string().contains("route contains a loop"));
}

#[test]
fn test_routes_query() {
    let (mut app, contract_addr) = setup_duality_swapper();
    let owner = Addr::unchecked("owner");
    
    // Create and set multiple routes
    let routes = [
        ("untrn", "uusdc", create_direct_route("untrn", "uusdc")),
        ("untrn", "uatom", create_multi_hop_route("untrn", &["uusdc"], "uatom")),
        ("uusdc", "uatom", create_direct_route("uusdc", "uatom")),
    ];
    
    // Set all routes
    for (in_denom, out_denom, route) in routes.iter() {
        let set_route_msg = ExecuteMsg::SetRoute {
            denom_in: in_denom.to_string(),
            denom_out: out_denom.to_string(),
            route: route.clone(),
        };
        
        app.execute_contract(owner.clone(), contract_addr.clone(), &set_route_msg, &[]).unwrap();
    }
    
    // Query all routes
    let query_msg = QueryMsg::Routes {
        start_after: None,
        limit: None,
    };
    
    let res: RoutesResponse<DualityRoute> = app
        .wrap()
        .query_wasm_smart(contract_addr.clone(), &query_msg)
        .unwrap();
    
    // Verify all routes are returned correctly
    assert_eq!(res.routes.len(), routes.len());
    
    // Check if each route is in the response
    for (in_denom, out_denom, route) in routes.iter() {
        let found_route = res.routes.iter().find(|r| 
            r.denom_in == *in_denom && 
            r.denom_out == *out_denom
        );
        
        assert!(found_route.is_some(), "Route from {} to {} should exist", in_denom, out_denom);
        let found_route = found_route.unwrap();
        
        assert_eq!(found_route.route.from, route.from);
        assert_eq!(found_route.route.to, route.to);
        assert_eq!(found_route.route.swap_denoms, route.swap_denoms);
    }
}

#[test]
fn test_unauthorized_set_route() {
    let (mut app, contract_addr) = setup_duality_swapper();
    let non_owner = Addr::unchecked("non_owner");
    
    // Create a direct route
    let denom_in = "untrn";
    let denom_out = "usdc";
    let direct_route = create_direct_route(denom_in, denom_out);
    
    // Set the route as non-owner
    let set_route_msg = ExecuteMsg::SetRoute {
        denom_in: denom_in.to_string(),
        denom_out: denom_out.to_string(),
        route: direct_route,
    };
    
    // Execute set route as non-owner - should fail
    let err = app.execute_contract(non_owner, contract_addr, &set_route_msg, &[]).unwrap_err();
    
    // Verify the error contains the expected message
    assert!(err.to_string().contains("Unauthorized"));
}
