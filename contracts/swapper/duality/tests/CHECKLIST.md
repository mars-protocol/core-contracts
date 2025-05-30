# Duality Swapper Test Checklist

This document outlines the test plan for the Mars Protocol's Duality Swapper implementation. It serves as a comprehensive checklist to ensure all aspects of the swapper are thoroughly tested.

## Route Validation Tests

- [x] **Basic Route Validation**
  - [x] Valid direct (2-denom) route passes validation
  - [x] Valid multi-hop (>2 denoms) route passes validation
  - [x] Route with too few denoms (< 2) is rejected
  - [x] Route with loop (same denom appears twice) is rejected
  - [x] Route where final denom doesn't match expected output is rejected

- [x] **Error Messages**
  - [x] Verify appropriate error messages for each validation failure case
  - [x] Error messages are descriptive and actionable

## Swap Estimation Tests

- [ ] **Direct Swap Estimation (2 denoms)**
  - [ ] Basic estimation functions correctly
  - [ ] Test with small amounts
  - [ ] Test with large amounts
  - [ ] Test with different token decimal places

- [ ] **Multi-hop Swap Estimation (>2 denoms)**
  - [ ] Basic multi-hop estimation functions correctly
  - [ ] Test with various path lengths (3, 4, 5+ denoms)
  - [ ] Test with small amounts
  - [ ] Test with large amounts

- [ ] **Route Source Testing**
  - [ ] Estimation works with saved routes
  - [ ] Estimation works with provided routes
  - [ ] When both saved and provided routes exist, provided routes take precedence
  - [ ] Error handling when no route exists

- [ ] **Neutron Query Mock Testing**
  - [ ] Proper mocking of `EstimatePlaceLimitOrder` responses
  - [ ] Proper mocking of `EstimateMultiHopSwap` responses
  - [ ] Error propagation from Neutron queries

## Message Construction Tests

- [ ] **Direct Swap Message Construction**
  - [ ] Correct message type (`PlaceLimitOrderRequest`) for direct swaps
  - [ ] Verify correct parameters are set:
    - [ ] `order_type` is set to `FillOrKill`
    - [ ] `sender` and `receiver` are set to contract address
    - [ ] `token_in` and `token_out` match route
    - [ ] `amount_in` matches input
    - [ ] `limit_sell_price` is correctly calculated from `min_receive`

- [ ] **Multi-hop Swap Message Construction**
  - [ ] Correct message type (`MultiHopSwapRequest`) for multi-hop swaps
  - [ ] Verify correct parameters are set:
    - [ ] `sender` and `receiver` are set to contract address
    - [ ] `routes` contains correct swap path
    - [ ] `amount_in` matches input
    - [ ] `exit_limit_price` is correctly calculated from `min_receive`
    - [ ] `pick_best_route` is set to `true`

## Integration Tests

- [ ] **End-to-End Swap Flow**
  - [ ] Direct swap executes successfully
  - [ ] Multi-hop swap executes successfully
  - [ ] Funds are correctly transferred to recipient
  - [ ] Minimum receive amount constraint is enforced

- [ ] **Route Management**
  - [ ] Routes can be set and retrieved
  - [ ] Routes can be updated
  - [ ] Route enumeration works correctly
  - [ ] Only owner can set routes

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

- [ ] **Extreme Values**
  - [ ] Test with very small swap amounts (near minimum)
  - [ ] Test with very large swap amounts (near maximum)
  - [ ] Test with tokens that have different decimal places

- [ ] **Swap Mode Transition**
  - [ ] Test the boundary between direct and multi-hop modes
  - [ ] Verify correct handling when swap path length is exactly 2 vs >2

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
| Route Validation | 0 | 0 | 0 | 0 |
| Swap Estimation | 0 | 0 | 0 | 0 |
| Message Construction | 0 | 0 | 0 | 0 |
| Integration | 0 | 0 | 0 | 0 |
| Error Handling | 0 | 0 | 0 | 0 |
| Edge Cases | 0 | 0 | 0 | 0 |
| Configuration | 0 | 0 | 0 | 0 |
| **TOTAL** | 0 | 0 | 0 | 0 |
