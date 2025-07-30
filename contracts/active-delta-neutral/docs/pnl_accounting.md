## 📘 PnL Calculation Overview for Delta-Neutral Strategies

This technical readme outlines how **realized PnL** is calculated for delta-neutral trading strategies involving **spot and perpetual (perp) markets**.

---
Details
[Funding + borrow calcs](./Funding%20&%20Borrow%20Integration%20with%20PnL.md)

### 🎯 Purpose

- Track **realized PnL** only when decreasing a position
- Account for both **spot** and **perp** legs in every trade
- Use a consistent, portfolio-based approach to valuation
- Track **funding gains** and **borrow costs** as part of total PnL breakdown
- Avoid complexity by using aggregated entry value and size

---

## 📥 Increase Position Logic

On each `IncreasePosition`, the contract does **not realize PnL**. Instead, it tracks the net value of the new position:

### Inputs:
- `spot_price_entry`
- `perp_price_entry`
- `amount` (base asset units, e.g., ETH)

### Calculation:
```text
entry_value = (spot_price_entry * amount) - (perp_price_entry * amount)
```

This value is added to a running total:
```text
total_position_entry_value += entry_value
total_position_size += amount
```

Optionally, funding and borrow tracking is initialized or updated here.

---

## 📤 Decrease Position Logic

On `DecreasePosition`, we **realize PnL** based on the change in value of the reduced portion of the position.

### Inputs:
- `spot_price_exit`
- `perp_price_exit`
- `amount` (to be reduced)
- `total_position_entry_value`
- `total_position_size`
- `funding_accrued`
- `borrow_cost_accrued`

### Step 1: Calculate Exit Value
```text
exit_value = (spot_price_exit * amount) - (perp_price_exit * amount)
```

### Step 2: Prorate Entry Value
```text
entry_value_for_this_slice = (total_position_entry_value / total_position_size) * amount
```

### Step 3: Base PnL
```text
raw_pnl = exit_value - entry_value_for_this_slice
```

### Step 4: Yield Adjustment
```text
net_yield = prorated_funding - prorated_borrow_cost
realized_pnl = raw_pnl + net_yield
```

### Example:
- Position size = 30 ETH
- Entry value = -30
- Decrease = 10 ETH
- Spot exit = $99
- Perp exit = $96
- Funding gain (prorated) = $1.20
- Borrow cost (prorated) = $0.70

```text
exit_value = 990 - 960 = 30
entry_value_slice = (-30 / 30) * 10 = -10
raw_pnl = 30 - (-10) = 40
net_yield = 1.2 - 0.7 = 0.5
realized_pnl = 40 + 0.5 = 40.5
```

✅ Realized PnL = $40.50

---

### 🧾 Notes
- This method assumes 1:1 spot:perp ratio
- Works regardless of entry fragmentation (VWAP handled implicitly)
- **Funding and borrow are tracked over time** and prorated during unwind
- Fees may be deducted separately or added into cost basis
- Unused size and value remain tracked in `Position`