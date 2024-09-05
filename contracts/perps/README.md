# Mars Perps

A smart contract responsible for managing perpetual (perps) trading functionality.

## Overview

The Perps contract is a standalone module that allows users to open, modify, or close perpetual positions, as well as add or remove liquidity from the perp vault. The Credit Manager contract initiates these actions.

### Example Workflow

1. **Opening a Position**: When a user opens a perp position through the Credit Manager, it calculates the required fee and sends USDC to the Perps contract with an open position message. From there, the Perps contract handles the rest of the process.

2. **Modifying or Closing a Position**: To modify or close a position, the Credit Manager queries the Perps contract to check the position’s Profit and Loss (PnL). If the position is at a loss (negative PnL), the Credit Manager prepares the required USDC, either from the user’s deposit or by borrowing from another contract (e.g., Red Bank). It then sends the USDC to the Perps contract with an `execute_order` message, where the USDC is added to the vault. For profitable positions, the account state is updated, and the Perps contract transfers USDC from the vault back to the Credit Manager.

### Vault Mechanics

The vault manages counterparty risk. Stakers deposit USDC into the vault and earn interest based on trade performance. When traders lose money, profits flow into the vault and the protocol. If many traders are profitable, the vault can become undercollateralized, risking liquidation. This is mitigated by the "deleverage" mechanism, which prioritizes closing the most profitable positions or those causing an Open Interest (OI) breach.

### Deleverage Process

The deleverage process is triggered when the Collateralization Ratio (CR) falls below the target or when OI exceeds allowed limits. The process targets positions that either have the highest profit or contribute to an OI breach. After closing these positions, unrealized PnL is applied, the CR is re-evaluated, and realized PnL is transferred to the account.

## Tests

We use automated tests to verify final numbers (see `tests/tests/test_risk_verification.rs`). These tests rely on two key files:
- `config.json` contains the market configurations.
- `input_actions.json` contains transactions for the smart contract.

These files are shared with the Risk team. The output from the Risk team (`risk_snapshot_state.json`) is compared with the smart contract’s output (`sc_snapshot_state.json`). Any discrepancies cause the test to fail.

When risk formulas are updated, the Risk team re-runs their scripts with the updated `config.json` and `input_actions.json`. The resulting `risk_snapshot_state.json` is committed to the repo, and the tests in `test_risk_verification.rs` are re-run to check for any differences.

## License

The contents of this crate are open source under the [GNU General Public License v3](../../LICENSE) or later.