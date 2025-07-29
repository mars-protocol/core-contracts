
This document outlines how **funding payments and spot borrow costs** are integrated into the realized PnL calculation in a delta-neutral trading strategy.

---

### 🎯 Purpose

To accurately reflect the yield impact of:
- **Funding rate payments** on the perp leg
- **Borrow costs** on the spot leg

These values are:
- **Queried directly from the Mars protocol** at the time of position modification
- **Realized proportionally** based on the amount of position reduced

---

## 🧠 Key Concepts

- Funding and borrow values are **not estimated**, but **fetched** from Mars at execution time
- The total realized funding and borrow costs are recorded **at the point of action**
- On `DecreasePosition`, a **portion** of those values is attributed to the closed amount

---

## 📈 Accrual Model

### Tracked Fields in Position:
```rust
pub struct Position {
    pub net_funding_accrued: Decimal,  // updated via Mars query
    pub net_borrow_accrued: Decimal,   // updated via Mars query
    pub last_updated: u64,
    pub perp_amount: Uint128,
    pub spot_amount: Uint128,
    ...
}
```

### Funding/Borrow Update (before modify):
```text
// Mars protocol returns net funding and borrow since last update
let (funding_delta, borrow_delta) = query_mars_funding_and_borrow(position);

net_funding_accrued += funding_delta
net_borrow_accrued  += borrow_delta

last_updated = now
```

---

## 📤 Realization on Decrease

When a position is partially closed, funding and borrow accruals are **realized proportionally**:

```text
realized_funding = net_funding_accrued * (decrease_amount / total_position_size)
realized_borrow  = net_borrow_accrued  * (decrease_amount / total_position_size)

net_yield = realized_funding - realized_borrow
```

This `net_yield` is added to:
```text
realized_pnl = exit_value - entry_value + net_yield
```

---

## ✅ Benefits of This Model

- Leverages authoritative funding/borrow values from Mars protocol
- Ensures fair and clean attribution of yield across multiple entry/exit cycles
- Avoids any need for on-chain rate estimation or compounding

---

This structure allows for accurate tracking of yield-driven performance in delta-neutral strategies, and can be extended later to include fee rebates, rewards, or leverage mechanics.