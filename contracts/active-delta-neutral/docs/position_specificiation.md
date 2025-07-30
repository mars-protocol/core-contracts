## 📦 Position Management Deep Dive

### 🎯 Purpose

Track the state of an open delta-neutral position with consistent, auditable updates — enabling incremental entries, partial exits, and yield tracking over time.

---

### 🧱 Core Struct: `Position`

```rust
pub struct Position {
    pub spot_amount: Uint128,
    pub perp_amount: Uint128,
    pub avg_spot_price: Decimal,
    pub avg_perp_price: Decimal,
    pub entry_value: Int128,           // total value of spot - perp over all increases
    pub direction: Side,                // LongSpotShortPerp | ShortSpotLongPerp
    pub net_funding_accrued: Int128,   // funding yield (updated via Mars query)
    pub net_borrow_accrued: Int128,    // borrow cost (updated via Mars query)
    pub last_updated: u64,
}
```

---

### 🔼 `increase()` Logic

#### Inputs:
- `amount`
- `spot_price`
- `perp_price`
- `direction`

#### Behavior:
- Validate direction:
  - If empty → accept any direction
  - If not empty → must match existing
- Update `spot_amount` and `perp_amount`
- Recalculate VWAPs:
```text
new_avg_spot = (old_spot * old_size + spot_price * amount) / (old_size + amount)
new_avg_perp = (old_perp * old_size + perp_price * amount) / (old_size + amount)
```
- Update `entry_value += (spot_price - perp_price) * amount`
- Optionally accrue funding & borrow from Mars before update

---

### 🔽 `decrease()` Logic

#### Inputs:
- `amount`
- `spot_exit_price`
- `perp_exit_price`

#### Behavior:
- Ensure enough size exists
- Prorate entry value:
```text
entry_value_slice = (entry_value / total_size) * amount
```
- Prorate funding/borrow (from `net_funding_accrued`, `net_borrow_accrued`)
- Reduce sizes
- Update `entry_value -= entry_value_slice`
- Reset state if fully closed

#### Output:
Return struct with everything needed for PnL:

```rust
pub struct DecreaseResult {
    pub spot_exit_price: Decimal,
    pub perp_exit_price: Decimal,
    pub size_closed: Uint128,
    pub entry_value_slice: Int128,
    pub realized_funding: Int128,
    pub realized_borrow: Int128,
}
```

---

### 🧪 Invariants

- `spot_amount == perp_amount` always (delta-neutral)
- Direction cannot flip unless position is fully closed
- Accrual fields (`net_*`) must be updated before state mutation
- All state transitions must emit clear events