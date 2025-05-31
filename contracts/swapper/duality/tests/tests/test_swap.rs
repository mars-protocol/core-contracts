use cosmwasm_std::{coin, Uint128};
use mars_testing::duality_swapper::DualitySwapperTester;
use neutron_test_tube::{Account, NeutronTestApp};
use test_case::test_case;

// Base test function that will be parameterized with test_case
fn test_swap_integration(
    denom_in: &str,
    denom_out: &str,
    amount_in: u128,
    expected_amount_out: u128, // Expected amount to receive based on price
    use_route: bool,
    use_multihop: bool,
    intermediate_denom: Option<&str>,
) {
    // Create the DualitySwapperTester that sets up the environment and deploys the contract
    let app = NeutronTestApp::default();
    let tester = DualitySwapperTester::new(&app);

    let user_balance_before = tester.get_balance(&tester.user.address(), denom_out);

    // Prepare swap parameters
    let coin_in = coin(amount_in, denom_in);

    // Create appropriate route based on test parameters
    let route = if use_route {
        if use_multihop && intermediate_denom.is_some() {
            // Multi-hop route through intermediate token
            Some(tester.create_multi_hop_route(denom_in, intermediate_denom.unwrap(), denom_out))
        } else {
            // Direct route
            Some(tester.create_direct_route(denom_in, denom_out))
        }
    } else {
        None // No route specified, let the contract figure it out
    };

    // Set route in state
    if use_route {
        let res = tester.set_route(route.clone().unwrap(), denom_in, denom_out);
        assert!(res.is_ok(), "Route should pass validation");
    }

    // Add liquidity to pools with 1:1 ratio for simplicity
    // For the direct route
    let base_liquidity = 1_000_000_000u128;
    tester.add_liquidity(
        denom_in,
        denom_out,
        Uint128::new(base_liquidity),
        Uint128::new(base_liquidity), // 1:1 ratio
    );

    // If using multi-hop, also add liquidity to intermediate pools
    if use_multihop && intermediate_denom.is_some() {
        let intermediate = intermediate_denom.unwrap();
        // Add liquidity for both hops with 1:1 ratios
        tester.add_liquidity(
            denom_in,
            intermediate,
            Uint128::new(base_liquidity),
            Uint128::new(base_liquidity),
        );

        tester.add_liquidity(
            intermediate,
            denom_out,
            Uint128::new(base_liquidity),
            Uint128::new(base_liquidity),
        );
    }

    // Execute the swap
    let result = tester.execute_swap(
        coin_in.clone(),
        denom_out,
        Uint128::new(expected_amount_out), // Minimum amount to receive based on expected output
        route,
        &tester.user,
    );

    println!("result: {:#?}", result);

    let _result = result.unwrap();

    // Verify user balance changed correctly
    let user_balance = tester.get_balance(&tester.user.address(), denom_out);
    assert_eq!(
        user_balance,
        user_balance_before + Uint128::new(expected_amount_out),
        "User should have received the expected amount of tokens"
    );
}

#[test_case("untrn", "uusdc", 1_000_000, 1_000_000, true, false, None; "direct swap with explicit route")]
#[test_case("untrn", "uusdc", 500_000, 500_000, false, false, None; "direct swap without route")]
#[test_case("untrn", "uusdc", 750_000, 750_000, true, true, Some("uatom"); "multi-hop swap through uatom")]
fn test_basic_swaps(
    denom_in: &str,
    denom_out: &str,
    amount_in: u128,
    expected_amount_out: u128,
    use_route: bool,
    use_multihop: bool,
    intermediate_denom: Option<&str>,
) {
    test_swap_integration(
        denom_in,
        denom_out,
        amount_in,
        expected_amount_out,
        use_route,
        use_multihop,
        intermediate_denom,
    );
}
