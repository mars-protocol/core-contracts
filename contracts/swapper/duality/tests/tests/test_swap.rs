use std::str::FromStr;

use cosmwasm_std::{coin, Decimal, Decimal256, Uint128, Uint256};
#[cfg(feature = "duality")]
use mars_testing::duality_swapper::DualitySwapperTester;
use mars_types::swapper::{DualityRoute, SwapperRoute};
#[cfg(feature = "duality")]
use neutron_test_tube::{Account, NeutronTestApp};
use test_case::test_case;

#[test_case("untrn", "uusdc", 1_000_000, Decimal::percent(200); "direct swap with 2:1 price ratio")]
#[test_case("untrn", "uusdc", 1_000_000, Decimal::percent(1000000); "direct swap with very high price ratio (10000:1)")]
#[test_case("untrn", "uusdc", 1_000_000, Decimal::percent(100000000); "direct swap with extreme price ratio (1000000:1)")]
#[test_case("uatom", "ujuno", 1_000_000, Decimal::percent(100); "different pair with 1:1 price ratio")]
#[test_case("untrn", "uusdc", 10000, Decimal::percent(500); "tiny amount swap with 5:1 ratio")]
#[test_case("untrn", "uusdc", 50_000_000, Decimal::percent(100); "medium amount swap with 1:1 ratio")]
#[test_case("untrn", "uusdc", 50_000_000, Decimal::percent(300); "medium amount swap with 3:1 ratio")]
#[test_case("untrn", "uusdc", 500_000_000_000_000, Decimal::percent(1000); "large amount swap with 10:1 ratio")]
#[test_case("untrn", "uusdc", 500_000_000_000_000_000_000_000_000_000, Decimal::percent(1000); "extreme amount swap with 10:1 ratio")]
#[test_case("untrn", "uusdc", 10_000_000, Decimal::percent(10); "swap with 0.1:1 price ratio")]
#[test_case("untrn", "uusdc", 100_000_000, Decimal::percent(1); "swap with 0.01:1 price ratio")]
fn test_custom_price_swaps(denom_in: &str, denom_out: &str, amount_in: u128, price_ratio: Decimal) {
    test_swap_with_custom_pricing(denom_in, denom_out, amount_in, price_ratio);
}
#[test_case("untrn", "ujuno", 1_000_000, vec!["uusdc"], vec![2.0, 0.5], None; "3-hop swap with varied rates")]
#[test_case("untrn", "uusdc", 5_000_000, vec!["uatom"], vec![0.5, 4.0], None; "3-hop swap with offsetting rates")]
#[test_case("uatom", "ujuno", 1_000_000, vec!["uusdc"], vec![1.5, 0.75], Some(1.125); "3-hop swap with calculated aggregate rate")]
fn test_multi_hop_swaps(
    denom_in: &str,
    denom_out: &str,
    amount_in: u128,
    intermediates: Vec<&str>,
    exchange_rates: Vec<f64>,
    expected_output_multiplier: Option<f64>,
) {
    test_multi_hop_with_varied_exchange_rates(
        denom_in,
        denom_out,
        amount_in,
        intermediates,
        exchange_rates,
        expected_output_multiplier,
    );
}

fn test_swap_basic(
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
        if use_multihop {
            if let Some(intermediate) = intermediate_denom {
                // Multi-hop route through intermediate token
                Some(tester.create_multi_hop_route(denom_in, intermediate, denom_out))
            } else {
                // Direct route
                Some(tester.create_direct_route(denom_in, denom_out))
            }
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
    let base_liquidity = Uint256::from(1_000_000_000u128);
    tester.add_liquidity(
        denom_in,
        denom_out,
        base_liquidity,
        base_liquidity, // 1:1 ratio
    );

    // If using multi-hop, also add liquidity to intermediate pools
    if use_multihop && intermediate_denom.is_some() {
        let intermediate = intermediate_denom.unwrap();
        // Add liquidity for both hops with 1:1 ratios
        tester.add_liquidity(denom_in, intermediate, base_liquidity, base_liquidity);

        tester.add_liquidity(intermediate, denom_out, base_liquidity, base_liquidity);
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

// Test function for custom pricing scenarios - simplified for direct swaps only
fn test_swap_with_custom_pricing(
    denom_in: &str,
    denom_out: &str,
    amount_in: u128,
    price_ratio: Decimal, // price_out / price_in (how many output tokens for 1 input token)
) {
    // Create the test environment
    let app = NeutronTestApp::default();
    let tester = DualitySwapperTester::new(&app);

    let price_ratio: Decimal256 = price_ratio.into();
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
    let base_liquidity_in = amount_in * 2; // Large enough liquidity pool
    let base_liquidity_out = Decimal256::from_atomics(base_liquidity_in, 0)
        .unwrap()
        .checked_mul(price_ratio.into())
        .unwrap();

    // Direct swap with custom price ratio. - we only add liquidity
    let _res = tester.add_liquidity(
        denom_in,
        denom_out,
        Uint256::from(base_liquidity_in),
        base_liquidity_out.to_uint_floor(),
    );

    // Allow for 0.1% slippage, rounding
    let slippage_adjusted_price_ratio: Decimal256 =
        price_ratio.checked_mul(Decimal256::from_ratio(999u128, 1000u128)).unwrap();

    // Calculate expected output
    let expected_amount_out = Decimal256::from_atomics(amount_in, 0)
        .unwrap()
        .checked_mul(slippage_adjusted_price_ratio)
        .unwrap()
        .to_uint_floor();

    let expected_amount_out: Uint128 = expected_amount_out.try_into().unwrap();

    // Execute the swap
    let result =
        tester.execute_swap(coin_in, denom_out, expected_amount_out, swapper_route, &tester.user);

    // Unwrap and verify the result
    let _result = result.unwrap();

    // Verify the user received at least the minimum amount
    let user_balance = tester.get_balance(&tester.user.address(), denom_out);
    let max_rounding_error =
        expected_amount_out.checked_mul_floor(Decimal::from_str("0.02").unwrap()).unwrap();

    assert!(
        user_balance >= user_balance_before + expected_amount_out - max_rounding_error,
        "User should have received at least the minimum amount of tokens"
    );

    // Also verify the user didn't receive more than expected (should be close to expected)
    assert!(
        user_balance <= user_balance_before + expected_amount_out + max_rounding_error,
        "User received significantly more tokens than expected"
    );
}

/// Tests multi-hop swaps with different exchange rates at each hop
///
/// This test specifically verifies that:
/// 1. Multi-hop routes correctly calculate the expected output across multiple hops
/// 2. Different exchange rates at each hop are properly accounted for
/// 3. The final token amount matches the expected calculation
fn test_multi_hop_with_varied_exchange_rates(
    // Input and output token denoms
    denom_in: &str,
    denom_out: &str,
    amount_in: u128,
    // Intermediate tokens for the route
    intermediates: Vec<&str>,
    // Exchange rates for each hop as a ratio of output:input
    // Length must be equal to intermediates.len() + 1
    exchange_rates: Vec<f64>,
    // The expected output multiplier (optional - calculated from exchange rates if None)
    expected_output_multiplier: Option<f64>,
) {
    // Create the test environment
    let app = NeutronTestApp::default();
    let tester = DualitySwapperTester::new(&app);

    // Build the complete swap path including input and output tokens
    let mut swap_denoms = vec![denom_in.to_string()];

    // Add all intermediate tokens to the path
    for intermediate in &intermediates {
        swap_denoms.push(intermediate.to_string());
    }

    // Add the final output token
    swap_denoms.push(denom_out.to_string());

    // Create the route
    let route = DualityRoute {
        from: denom_in.to_string(),
        to: denom_out.to_string(),
        swap_denoms: swap_denoms.clone(),
    };

    // Verify we have the correct number of exchange rates (one for each hop)
    assert_eq!(
        exchange_rates.len(),
        intermediates.len() + 1,
        "Number of exchange rates must match the number of hops (intermediates + 1)"
    );

    // Add liquidity with the specified exchange rates
    // First hop: denom_in to first intermediate (or denom_out if no intermediates)
    let first_target = if !intermediates.is_empty() {
        intermediates[0]
    } else {
        denom_out
    };
    tester.add_liquidity(
        denom_in,
        first_target,
        Uint256::from(10_000_000u128),
        Uint256::from((10_000_000.0 * exchange_rates[0]) as u128),
    );

    // Add liquidity for all intermediate hops
    for i in 0..intermediates.len() {
        // If this is not the last intermediate, connect to next intermediate
        if i < intermediates.len() - 1 {
            tester.add_liquidity(
                intermediates[i],
                intermediates[i + 1],
                Uint256::from(10_000_000u128),
                Uint256::from((10_000_000.0 * exchange_rates[i + 1]) as u128),
            );
        } else {
            // Last intermediate connects to final output token
            tester.add_liquidity(
                intermediates[i],
                denom_out,
                Uint256::from(10_000_000u128),
                Uint256::from((10_000_000.0 * exchange_rates[i + 1]) as u128),
            );
        }
    }

    // Calculate the expected output amount by multiplying through all exchange rates
    let mut calculated_multiplier = 1.0;
    for rate in &exchange_rates {
        calculated_multiplier *= rate;
    }

    let multiplier = expected_output_multiplier.unwrap_or(calculated_multiplier);
    let expected_amount_out = (amount_in as f64 * multiplier) as u128;

    // Set the route in the contract
    let res = tester.set_route(route.clone(), denom_in, denom_out);
    assert!(res.is_ok(), "Route should pass validation");

    // Record initial balance
    let user_balance_before = tester.get_balance(&tester.user.address(), denom_out);

    // Execute the swap
    let coin_in = coin(amount_in, denom_in);

    // Allow for a small margin of error due to rounding (0.1%)
    let max_rounding_error = (expected_amount_out as f64 * 0.001) as u128;

    let result = tester.execute_swap(
        coin_in.clone(),
        denom_out,
        Uint128::new(expected_amount_out - max_rounding_error),
        None, // Using the saved route
        &tester.user,
    );

    // Verify the swap succeeded
    assert!(result.is_ok(), "Multi-hop swap should succeed");

    // Verify the user received the expected amount
    let user_balance_after = tester.get_balance(&tester.user.address(), denom_out);
    let received_amount = user_balance_after - user_balance_before;

    assert!(
        received_amount.u128() >= expected_amount_out - max_rounding_error,
        "User should have received at least the expected amount minus rounding error. Expected: {}, Received: {}",
        expected_amount_out,
        received_amount
    );

    assert!(
        received_amount.u128() <= expected_amount_out + max_rounding_error,
        "User should have received at most the expected amount plus rounding error. Expected: {}, Received: {}",
        expected_amount_out,
        received_amount
    );
}

#[test]
fn test_insufficient_output_direct_swap() {
    // Create the test environment
    let app = NeutronTestApp::default();
    let tester = DualitySwapperTester::new(&app);

    // Test parameters
    let denom_in = "untrn";
    let denom_out = "uusdc";
    let amount_in = 1_000_000u128;

    // Add liquidity with a 3:1 ratio
    tester.add_liquidity(
        denom_in,
        denom_out,
        Uint256::from(10_000_000u128),
        Uint256::from(30_000_000u128),
    );

    // Create and set up a direct route
    let route = DualityRoute {
        from: denom_in.to_string(),
        to: denom_out.to_string(),
        swap_denoms: vec![denom_in.to_string(), denom_out.to_string()],
    };

    let res = tester.set_route(route.clone(), denom_in, denom_out);
    assert!(res.is_ok(), "Route should pass validation");

    // Execute the swap with a minimum receive amount HIGHER than what the pool will provide
    // With a 3:1 pool ratio, we expect 3:1 output, but we're requiring even more
    let coin_in = coin(amount_in, denom_in);
    let expected_out = amount_in * 3;
    let min_receive = expected_out + 1_000u128; // Asking for slightly more than the pool will give

    // This should fail because the minimum receive amount is too high
    let result = tester.execute_swap(
        coin_in.clone(),
        denom_out,
        Uint128::new(min_receive),
        None, // Using the saved route
        &tester.user,
    );

    // Verify the swap failed with the expected error message
    assert!(result.is_err(), "Swap should fail when minimum output cannot be met");

    // Check for error messages related to price or limit order execution
    let error_message = result.unwrap_err().to_string().to_lowercase();
    assert!(
        error_message.contains("fill or kill")
            || error_message.contains("couldn't be executed")
            || error_message.contains("price")
            || error_message.contains("limit"),
        "Error should indicate the price/limit requirements weren't met: {}",
        error_message
    );
}

#[test]
fn test_insufficient_output_multi_hop_swap() {
    // Create the test environment
    let app = NeutronTestApp::default();
    let tester = DualitySwapperTester::new(&app);

    // Test parameters
    let denom_in = "untrn";
    let intermediate = "uatom";
    let denom_out = "uusdc";
    let amount_in = 1_000_000u128;

    // Add liquidity with a 3:1 ratio for the first hop (3 intermediate tokens for 1 input token)
    tester.add_liquidity(
        denom_in,
        intermediate,
        Uint256::from(10_000_000u128), // 10M input tokens
        Uint256::from(30_000_000u128), // 30M intermediate tokens (3:1 ratio)
    );

    // Add liquidity with a 1:1 ratio for the second hop
    tester.add_liquidity(
        intermediate,
        denom_out,
        Uint256::from(30_000_000u128), // 30M intermediate tokens
        Uint256::from(30_000_000u128), // 30M output tokens (1:1 ratio)
    );

    // Create and set up a multi-hop route
    let route = DualityRoute {
        from: denom_in.to_string(),
        to: denom_out.to_string(),
        swap_denoms: vec![denom_in.to_string(), intermediate.to_string(), denom_out.to_string()],
    };

    let res = tester.set_route(route.clone(), denom_in, denom_out);
    assert!(res.is_ok(), "Route should pass validation");

    // Calculate expected output - with 3:1 for first hop and 1:1 for second hop
    // For 1M input tokens, we expect ~3M output tokens
    let expected_amount_out = amount_in * 3;

    // Set minimum receive slightly higher than the expected output to trigger the error
    let min_receive = expected_amount_out + 1000u128; // Asking for more than the pools will provide

    // This should fail because the minimum receive amount is too high
    let result = tester.execute_swap(
        coin(amount_in, denom_in),
        denom_out,
        Uint128::new(min_receive),
        None, // Using the saved route
        &tester.user,
    );

    // Verify the swap failed with the expected error message
    assert!(result.is_err(), "Swap should fail when minimum output cannot be met");

    // Check for error messages related to price or multi-hop execution
    let error_message = result.unwrap_err().to_string().to_lowercase();
    assert!(
        error_message.contains("exit limit price")
            || error_message.contains("price")
            || error_message.contains("limit")
            || error_message.contains("multi"),
        "Error should indicate the price/limit requirements weren't met: {}",
        error_message
    );
}

#[test]
fn test_default_slippage_protection() {
    // Create the test environment
    let app = NeutronTestApp::default();
    let tester = DualitySwapperTester::new(&app);

    // Test parameters
    let denom_in = "untrn";
    let denom_out = "uusdc";
    let amount_in = 5_000_000u128;

    // Add small liquidity so we'll have high slippage
    // Just enough to execute the swap, but with significant price impact
    tester.add_liquidity(
        denom_in,
        denom_out,
        Uint256::from(10_000_000u128), // Base amount
        Uint256::from(10_000_000u128), // 1:1 initial ratio
    );

    // Create and set up a direct route
    let route = DualityRoute {
        from: denom_in.to_string(),
        to: denom_out.to_string(),
        swap_denoms: vec![denom_in.to_string(), denom_out.to_string()],
    };

    let res = tester.set_route(route.clone(), denom_in, denom_out);
    assert!(res.is_ok(), "Route should pass validation");

    // First execute a swap that should succeed with a low minimum receive
    let coin_in = coin(amount_in, denom_in);
    let min_receive_low = amount_in / 2; // Allow for 50% slippage

    let result_success = tester.execute_swap(
        coin_in.clone(),
        denom_out,
        Uint128::new(min_receive_low),
        None,
        &tester.user,
    );

    // This swap should succeed even with potential slippage
    assert!(result_success.is_ok(), "Swap should succeed with generous slippage tolerance");

    // Now try the same swap but with tight slippage tolerance
    // Reset the app and tester to have fresh state
    let app = NeutronTestApp::default();
    let tester = DualitySwapperTester::new(&app);

    // Add the same liquidity
    tester.add_liquidity(
        denom_in,
        denom_out,
        Uint256::from(10_000_000u128),
        Uint256::from(9_500_000u128),
    );

    // Set the route again
    let res: Result<
        neutron_test_tube::ExecuteResponse<
            neutron_test_tube::neutron_std::types::cosmwasm::wasm::v1::MsgExecuteContractResponse,
        >,
        neutron_test_tube::RunnerError,
    > = tester.set_route(route.clone(), denom_in, denom_out);
    assert!(res.is_ok(), "Route should pass validation");

    // Now execute a swap with tight slippage tolerance
    let min_receive_high = amount_in * 99 / 100; // Allow only 1% slippage
    let result_failure = tester.execute_swap(
        coin_in.clone(),
        denom_out,
        Uint128::new(min_receive_high),
        None,
        &tester.user,
    );

    // This swap should fail due to tight slippage tolerance
    assert!(result_failure.is_err(), "Swap should fail with tight slippage tolerance");

    let error_message = result_failure.unwrap_err().to_string().to_lowercase();
    assert!(
        error_message.contains("fill or kill")
            || error_message.contains("couldn't be executed")
            || error_message.contains("price")
            || error_message.contains("limit"),
        "Error should indicate the price/limit requirements weren't met: {}",
        error_message
    );
}

/// Test that direct provided routes also respect minimum output requirements
#[test]
fn test_insufficient_output_with_provided_route() {
    // Create the test environment
    let app = NeutronTestApp::default();
    let tester = DualitySwapperTester::new(&app);

    // Test parameters
    let denom_in = "untrn";
    let denom_out = "uusdc";
    let amount_in = 1_000_000u128;

    // Add liquidity with a 1:1 ratio
    tester.add_liquidity(
        denom_in,
        denom_out,
        Uint128::new(10_000_000).into(),
        Uint128::new(10_000_000).into(),
    );

    // Create a route but DON'T save it in the contract state
    let route = DualityRoute {
        from: denom_in.to_string(),
        to: denom_out.to_string(),
        swap_denoms: vec![denom_in.to_string(), denom_out.to_string()],
    };

    // Execute the swap with a minimum receive amount HIGHER than what the pool will provide
    // With a 1:1 pool ratio, we expect 1:1 output, but we're requiring 2:1
    let coin_in = coin(amount_in, denom_in);
    let min_receive = amount_in * 2; // Asking for twice what the pool will give

    // This should fail because the minimum receive amount is too high
    let result = tester.execute_swap(
        coin_in.clone(),
        denom_out,
        Uint128::new(min_receive),
        Some(SwapperRoute::Duality(route)), // Using a provided route
        &tester.user,
    );

    // Verify the swap failed with the expected error message
    assert!(result.is_err(), "Swap should fail when minimum output cannot be met");

    // Check for error messages related to price or limit order execution
    let error_message = result.unwrap_err().to_string().to_lowercase();
    assert!(
        error_message.contains("fill or kill")
            || error_message.contains("couldn't be executed")
            || error_message.contains("price")
            || error_message.contains("limit"),
        "Error should indicate the price/limit requirements weren't met: {}",
        error_message
    );
}
