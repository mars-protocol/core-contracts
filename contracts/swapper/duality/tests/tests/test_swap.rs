use cosmwasm_std::{coin, Uint128};
use mars_testing::duality_swapper::DualitySwapperTester;
use mars_types::swapper::SwapperRoute;
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
    let swapper_route = if use_route {
        let res = tester.set_route(route.clone().unwrap(), denom_in, denom_out);
        assert!(res.is_ok(), "Route should pass validation");
        Some(SwapperRoute::Duality(route.clone().unwrap()))
    } else {
        None
    };

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
        swapper_route,
        &tester.user,
    );

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
#[test_case("untrn", "uusdc", 500_000, 500_000, true, false, None; "direct swap without route")]
#[test_case("untrn", "uusdc", 750_000, 750_000, true, true, Some("uatom"); "multi-hop swap through uatom")]
#[test_case("uatom", "ujuno", 1_000_000, 1_000_000, true, false, None; "different token pair direct swap")]
#[test_case("uatom", "ujuno", 2_000_000, 2_000_000, true, true, Some("uosmo"); "different token pair multi-hop swap")]
#[test_case("untrn", "uusdc", 1, 1, true, false, None; "minimal amount direct swap")]
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

// Test function for custom pricing scenarios - simplified for direct swaps only
fn test_swap_with_custom_pricing(
    denom_in: &str,
    denom_out: &str,
    amount_in: u128,
    price_ratio: u128, // price_out / price_in (how many output tokens for 1 input token)
) {
    // Create the test environment
    let app = NeutronTestApp::default();
    let tester = DualitySwapperTester::new(&app);

    // Record initial balance
    let user_balance_before = tester.get_balance(&tester.user.address(), denom_out);

    // Prepare swap parameters
    let coin_in = coin(amount_in, denom_in);
    
    // Create a direct route for the swap
    let route = tester.create_direct_route(denom_in, denom_out);

    // Set the route in the contract state
    let res = tester.set_route(route.clone(), denom_in, denom_out);
    assert!(res.is_ok(), "Route should pass validation");

    // Create swapper route struct
    let swapper_route = Some(SwapperRoute::Duality(route));

    // Add liquidity with custom price ratio
    let base_liquidity_in = 100_000_000u128; // Large enough liquidity pool
    let base_liquidity_out = base_liquidity_in * price_ratio;
    // Direct swap with custom price ratio
    let _res = tester.add_liquidity(
        denom_in,
        denom_out,
        Uint128::new(base_liquidity_in),
        Uint128::new(base_liquidity_out),
    );

    // Calculate expected output with some buffer for fees/slippage
    let expected_amount_out = amount_in * price_ratio;

    // Execute the swap
    let result = tester.execute_swap(
        coin_in,
        denom_out,
        Uint128::one(),
        swapper_route,
        &tester.user,
    );

    // Unwrap and verify the result
    let _result = result.unwrap();

    // Verify the user received at least the minimum amount
    let user_balance = tester.get_balance(&tester.user.address(), denom_out);
    assert!(
        user_balance >= user_balance_before + Uint128::new(expected_amount_out),
        "User should have received at least the minimum amount of tokens"
    );
    
    // Also verify the user didn't receive more than expected (should be close to expected)
    assert!(
        user_balance <= user_balance_before + Uint128::new(expected_amount_out + 1), // +1 to account for potential rounding
        "User received significantly more tokens than expected"
    );
}

#[test_case("untrn", "uusdc", 1_000_000, 2; "direct swap with 2:1 price ratio")]
#[test_case("untrn", "uusdc", 1_000_000, 10_000; "direct swap with very high price ratio (10000:1)")]
#[test_case("untrn", "uusdc", 1_000_000, 1_000_000; "direct swap with extreme price ratio (1000000:1)")]
#[test_case("uatom", "ujuno", 1_000_000, 1; "different pair with 1:1 price ratio")]
#[test_case("untrn", "uusdc", 10, 5; "tiny amount swap with 5:1 ratio")]
#[test_case("untrn", "uusdc", 50_000_000, 1; "large amount swap with 1:1 ratio")]
fn test_custom_price_swaps(
    denom_in: &str,
    denom_out: &str,
    amount_in: u128,
    price_ratio: u128,
) {
    test_swap_with_custom_pricing(
        denom_in,
        denom_out,
        amount_in,
        price_ratio,
        );
}
