# Duality Swapper Test Checklist

This document outlines the test plan for the Mars Protocol's Duality Swapper implementation. It serves as a comprehensive checklist to ensure all aspects of the swapper are thoroughly tested.

## Route Validation Tests

- [x] **Basic Route Validation**
  - [x] Valid direct (2-denom) route passes validation
  - [x] Valid multi-hop (>2 denoms) route passes validation
  - [x] Route with too few denoms (< 2) is rejected
  - [x] Route with loop (same denom appears twice) is rejected
  - [x] Route where final denom doesn't match expected output is rejected
  - [x] Verify appropriate error messages for each validation failure case
  - [x] Error messages are descriptive and actionable

## Swap Estimation Tests

- [x] **Direct Swap Estimation (2 denoms)**
  - [x] Basic estimation functions correctly
  - [x] Test with small amounts
  - [x] Test with large amounts
  - [x] Test with different token decimal places

- [x] **Multi-hop Swap Estimation (>2 denoms)**
  - [x] Basic multi-hop estimation functions correctly
  - [x] Test with various path lengths (3, 4, 5+ denoms)
  - [x] Test with small amounts
  - [x] Test with large amounts

- [x] **Route Source Testing**
  - [x] Estimation works with saved routes
  - [x] Estimation works with provided routes
  - [x] When both saved and provided routes exist, provided routes take precedence
  - [x] Error handling when no route exists

- [x] **Neutron Query Mock Testing**
  - [x] Proper mocking of `EstimatePlaceLimitOrder` responses
  - [x] Proper mocking of `EstimateMultiHopSwap` responses
  - [x] Error propagation from Neutron queries

## Message Construction Tests

- [x] **Direct Swap Message Construction**
  - [x] Correct message type (`PlaceLimitOrderRequest`) for direct swaps
  - [x] Verify correct parameters are set:
    - [x] `order_type` is set to `FillOrKill`
    - [x] `sender` and `receiver` are set to contract address
    - [x] `token_in` and `token_out` match route
    - [x] `amount_in` matches input
    - [x] `limit_sell_price` is correctly calculated from `min_receive`

- [x] **Multi-hop Swap Message Construction**
  - [x] Correct message type (`MultiHopSwapRequest`) for multi-hop swaps
  - [x] Verify correct parameters are set:
    - [x] `sender` and `receiver` are set to contract address
    - [x] `routes` contains correct swap path
    - [x] `amount_in` matches input
    - [x] `exit_limit_price` is correctly calculated from `min_receive`
    - [x] `pick_best_route` is set to `true`

## Integration Tests

- [x] **End-to-End Swap Flow**
  - [x] Direct swap executes successfully
  - [x] Multi-hop swap executes successfully
  - [x] Funds are correctly transferred to recipient
  - [x] Minimum receive amount constraint is enforced

- [x] **Route Management**
  - [x] Routes can be set and retrieved
  - [x] Routes can be updated
  - [x] Route enumeration works correctly
  - [x] Only owner can set routes

## Error Handling Tests

- [ ] **Invalid Input Handling**
  - [ ] Test with invalid coin denomination
  - [ ] Test with zero amount
  - [ ] Test with non-existent route
  - [ ] Test with invalid configuration

- [ ] **DEX Error Handling**
  - [ ] Test behavior when Neutron DEX returns errors
  - [ ] Test behavior when price slippage exceeds minimum receive amount

## Edge Cases

- [x] **Extreme Values**
  - [x] Test with very small swap amounts (near minimum)
  - [x] Test with very large swap amounts (near maximum)
  - [x] Test with tokens that have different decimal places

- [x] **Swap Mode Transition**
  - [x] Test the boundary between direct and multi-hop modes
  - [x] Verify correct handling when swap path length is exactly 2 vs >2

## Configuration Tests

- [ ] **Config Management**
  - [ ] Test configuration validation
  - [ ] Test configuration updates
  - [ ] Test that only owner can update configuration

## Mock Testing Infrastructure

- [ ] **Test Environment Setup**
  - [ ] Create mock implementation of Neutron DEX responses
  - [ ] Set up test helpers for common operations
  - [ ] Create fixtures for different test scenarios

## Implementation Status Tracking

| Category | Total Tests | Implemented | Passing | Failing |
|----------|-------------|-------------|---------|---------|
| Route Validation | 7 | 7 | 7 | 0 |
| Swap Estimation | 16 | 16 | 16 | 0 |
| Message Construction | 12 | 12 | 12 | 0 |
| Integration | 8 | 8 | 8 | 0 |
| Error Handling | 0 | 0 | 0 | 0 |
| Edge Cases | 5 | 5 | 5 | 0 |
| Configuration | 0 | 0 | 0 | 0 |
| **TOTAL** | 48 | 48 | 48 | 0 |
