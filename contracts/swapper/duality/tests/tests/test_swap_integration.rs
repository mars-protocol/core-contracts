use cosmwasm_std::{coin, Uint128};

use crate::tests::duality_swapper::DualitySwapperTester;


#[test]
fn test_basic_direct_swap() {
    // Create the DualitySwapperTester that sets up the environment and deploys the contract
    let tester = DualitySwapperTester::new();
    
    // Define the denoms for the test
    let denom_in = "untrn";
    let denom_out = "uusdc";
    
    // Create a direct route for the swap
    let route = tester.create_direct_route(denom_in, denom_out);
    
    // Prepare swap parameters
    let coin_in = coin(1_000_000, denom_in);

    // Add liquidity to the pool (1:1 ratio with no fees)
    tester.add_liquidity(
        denom_in, 
        denom_out, 
        Uint128::new(1_000_000_000), 
        Uint128::new(1_000_000_000)
    );
    
    // Check liquidity was added correctly (optional)
    tester.query_deposits(&tester.admin);
    
    // Execute the swap
    let result = tester.execute_swap(
        coin_in.clone(),
        denom_out,
        coin_in.amount, // Minimum amount to receive (1:1 expected)
        Some(route),
        &tester.user,
    );

    // Verify swap succeeded
    assert!(result.is_ok(), "Swap failed - but it should succeed");
    
    // Verify user balance changed correctly
    let user_balance = tester.get_balance(&tester.user.address(), denom_out);
    assert!(
        user_balance > Uint128::zero(),
        "User should have received tokens"
    );
    
    println!("User received {} {}", user_balance, denom_out);
}


// #[test]
// fn test_multi_hop_swap() {
//     // Set up the test environment
//     let (app, admin, user) = neutron_dex_helpers::setup_test_environment();
//     let dex = neutron_dex_helpers::init_dex(&app);
//     // Create the test pools
//     let denom_in = "untrn";
//     let denom_intermediate = "uusdc";
//     let denom_out = "uatom";
    
//     // Pool 1: untrn <-> uusdc
//     let pool1_id = neutron_dex_helpers::create_dex_pool(
//         &dex,
//         &admin,
//         denom_in,
//         denom_intermediate,
//         Uint128::new(6_000_000),  // 6 million untrn
//         Uint128::new(1_500_000),  // 1.5 million uusdc
//     );
    
//     // Pool 2: uusdc <-> uatom
//     let pool2_id = neutron_dex_helpers::create_dex_pool(
//         &dex,
//         &admin,
//         denom_intermediate,
//         denom_out,
//         Uint128::new(1_500_000),  // 1.5 million uusdc
//         Uint128::new(750_000),    // 750k uatom
//     );
    
//     // Create and set the multi-hop route
//     let route = DualityRoute {
//         from: denom_in.to_string(),
//         to: denom_out.to_string(),
//         swap_denoms: vec![
//             denom_in.to_string(), 
//             denom_intermediate.to_string(), 
//             denom_out.to_string()
//         ],
//     };
    
//     // Prepare swap parameters
//     let coin_in = coin(1_000_000, denom_in);
    
//     // Get estimation
//     let estimation = robot.query_estimate_exact_in_swap(
//         &coin_in,
//         denom_out,
//         None,  // Use saved route
//     );
    
//     assert!(
//         estimation.amount > Uint128::zero(),
//         "Estimation should return a positive amount"
//     );
    
//     // Calculate minimum receive with 5% slippage tolerance
//     let min_receive = estimation.amount * (Decimal::one() - Decimal::percent(5));
    
//     // Perform the swap
//     let result = robot.swap_res(
//         coin_in.clone(),
//         denom_out,
//         min_receive,
//         &user,
//         None,  // Use saved route
//     );
    
//     assert!(result.is_ok(), "Swap should succeed");
    
//     // Verify user balance changed correctly
//     let user_balance = app.get_balance(user.address().as_str(), denom_out).unwrap();
//     assert!(
//         user_balance >= min_receive,
//         "User should have received at least the minimum amount"
//     );
// }

// #[test]
// fn test_swap_with_explicit_route() {
//     // Set up the test environment
//     let (app, admin, user) = neutron_dex_helpers::setup_test_environment();
    
//     // Create a new robot with the test app
//     let robot = DualitySwapperRobot::new_with_local(&app, &admin);
    
//     // Create the test pool
//     let denom_in = "untrn";
//     let denom_out = "uusdc";
//     let pool_id = neutron_dex_helpers::create_dex_pool(
//         &app,
//         &admin,
//         denom_in,
//         denom_out,
//         Uint128::new(6_000_000),  // 6 million untrn
//         Uint128::new(1_500_000),  // 1.5 million uusdc
//     );
    
//     // Create some TWAP records
//     neutron_dex_helpers::create_twap_records(
//         &app,
//         &admin,
//         pool_id,
//         coin(10u128, denom_in),
//         denom_out,
//     );
    
//     // Create the route (but don't save it to contract)
//     let route = DualityRoute {
//         from: denom_in.to_string(),
//         to: denom_out.to_string(),
//         swap_denoms: vec![denom_in.to_string(), denom_out.to_string()],
//     };
    
//     // Prepare swap parameters
//     let coin_in = coin(1_000_000, denom_in);
    
//     // Get estimation with explicit route
//     let route_for_swap = SwapperRoute::Duality(route.clone().into());
//     let estimation = robot.query_estimate_exact_in_swap(
//         &coin_in,
//         denom_out,
//         Some(route_for_swap.clone()),
//     );
    
//     assert!(
//         estimation.amount > Uint128::zero(),
//         "Estimation should return a positive amount"
//     );
    
//     // Calculate minimum receive with 5% slippage tolerance
//     let min_receive = estimation.amount * (Decimal::one() - Decimal::percent(5));
    
//     // Perform the swap with explicit route
//     let result = robot.swap_res(
//         coin_in.clone(),
//         denom_out,
//         min_receive,
//         &user,
//         Some(route_for_swap),
//     );
    
//     assert!(result.is_ok(), "Swap should succeed");
    
//     // Verify user balance changed correctly
//     let user_balance = app.get_balance(user.address().as_str(), denom_out).unwrap();
//     assert!(
//         user_balance >= min_receive,
//         "User should have received at least the minimum amount"
//     );
// }


// #[test]
// fn test_swap_slippage_too_high() {
//     // Set up the test environment
//     let (app, admin, user) = neutron_dex_helpers::setup_test_environment();
    
//     // Create a new robot with the test app
//     let robot = DualitySwapperRobot::new_with_local(&app, &admin);
    
//     // Create the test pool
//     let denom_in = "untrn";
//     let denom_out = "uusdc";
//     let pool_id = neutron_dex_helpers::create_dex_pool(
//         &app,
//         &admin,
//         denom_in,
//         denom_out,
//         Uint128::new(6_000_000),  // 6 million untrn
//         Uint128::new(1_500_000),  // 1.5 million uusdc
//     );
    
//     // Prepare swap parameters
//     let coin_in = coin(1_000_000, denom_in);
    
//     // Get estimation
//     let estimation = robot.query_estimate_exact_in_swap(
//         &coin_in,
//         denom_out,
//         None,
//     );
    
//     // Set an unrealistically high min_receive (more than expected output)
//     let min_receive = estimation.amount + Uint128::new(1_000_000);
    
//     // Perform the swap - should fail due to slippage
//     let result = robot.swap_res(
//         coin_in.clone(),
//         denom_out,
//         min_receive,
//         &user,
//         None,
//     );
    
//     assert!(result.is_err(), "Swap should fail due to slippage protection");
// }
