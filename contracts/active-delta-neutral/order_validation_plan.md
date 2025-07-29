# Order Validation Plan

This document outlines the parameters required and the proposed methods for validating order entries in the dynamic contract module. The goal is to ensure that only profitable and risk-appropriate trades are executed, in line with the delta-neutral entry/exit model.

---

## 1. Required Parameters

### Market & Position Parameters
- **spot_price**: Current spot market price.
- **perp_price**: Current perpetual contract price.
- **funding_rate**: Perpetual funding rate (annualized or per period).
- **spot_supply_rate**: Supply interest rate for the spot asset (if applicable).
- **spot_borrow_rate**: Borrow interest rate for the spot asset.
- **position_side**: Long (long spot, short perp) or Short (short spot, long perp).
- **leverage**: Current account leverage. Under current design entire system shares one account (1 credit account, n markets)
- **trade_size**: Size of the proposed trade.
- **entry_ratio**: Model parameter controlling entry threshold sensitivity.
- **acceptable_entry_delta**: Maximum acceptable price impact for entry.
- **slippage_tolerance**: Maximum allowed slippage for execution.

### Protocol & Config Parameters
- **min_profit_threshold**: Minimum profit required to allow entry.
- **max_risk_exposure**: Maximum allowed exposure per asset or overall.
- **max_leverage**: Maximum leverage allowed per position (key risk control).
- **oracle_price_source**: Reference for price feeds (for validation).

---

## 2. Proposed Validation Methods

### Model-Based Validation
- Use the entry/exit model:
  - For **longs**: Only enter if net yield > 0 and price impact < threshold.
  - For **shorts**: Only enter if net yield < 0 and price impact < threshold.
  - Calculate: `Acceptable Price Impact = Net Yield / K`
- Allow for dynamic adjustment of `K` based on volatility or market conditions.

### Risk-Based Validation
- Ensure proposed leverage does not exceed `max_leverage` (critical risk check). Each market has an individual max leverage.
- Validate against protocol-level risk checks? (e.g., liquidity, open interest).
- ensure that market sizes are capped for safety with `max_market_size`.

---

## 3. Next Steps
- Review and finalize the parameter list.
- Decide on which validation method(s) to implement first.
- Implement parameter checks and core validation logic in `order_validation.rs`.
- Add tests for each validation scenario.

---

_Add further notes, edge cases, or design decisions as development progresses._
