# pnl.rs – Delta-Neutral Position Realized PnL Calculation

This module provides utilities for calculating the realized profit and loss (PnL) when unwinding a delta-neutral position in the Mars Protocol.

## Overview

The core function, `compute_realized_pnl`, computes the realized PnL for a partial or full unwind of a delta-neutral position, accounting for:

- Spot and perp exit prices
- Proportional entry value
- Trading fees
- Funding and borrow accruals

It assumes a 1:1 hedge ratio between spot and perp legs, and expects all fees to be passed in explicitly.

## Formula

```
RealizedPnL = (SpotExitPrice - PerpExitPrice) * DecreaseAmount
            - (TotalEntryValue / TotalPositionSize) * DecreaseAmount
            - FeeAmount
            + ProportionalFunding
            - ProportionalBorrow
```

## Function

### `compute_realized_pnl`

**Parameters:**
- `spot_exit_price`: Spot price at unwind (Decimal)
- `perp_exit_price`: Perp price at unwind (Decimal)
- `decrease_amount`: Position size being closed (Uint128)
- `total_entry_value`: Total entry value (Int128)
- `total_position_size`: Total position size (Uint128)
- `perp_trading_fee_amount`: Fees paid for this unwind (Int128)
- `net_funding_accrued`: Total funding accrued (Int128)
- `net_borrow_accrued`: Total borrow accrued (Int128)

**Returns:**  
`ContractResult<Int128>` — The realized PnL as a signed integer.

**Errors:**  
Returns an error if `decrease_amount` or `total_position_size` is zero to avoid division by zero.

## Usage Example

```rust
let pnl = compute_realized_pnl(
    spot_exit_price,
    perp_exit_price,
    decrease_amount,
    total_entry_value,
    total_position_size,
    perp_trading_fee_amount,
    net_funding_accrued,
    net_borrow_accrued,
)?;
```

## Notes

- Designed for use within Mars Protocol’s delta-neutral strategies.
- All values should be provided in the quote asset denomination.
