# Delta-Neutral Implementation Checklist

## ✅ PnL Calculation
- [x] Documented methodology in `docs/pnl.md`
- [x] Core implementation of `compute_realized_pnl`
- [x] Use safe arithmetic (e.g., `checked_mul`, `checked_div`)
- [x] Zero-division protections
- [x] Funding and borrow included in realized PnL
- [x] Realized PnL test suite
  - [x] Simple PnL
  - [x] Multi-increase VWAP
  - [x] Partial close accuracy
  - [x] Fee handling
  - [x] Error cases

## 🔄 Position Implementation
- [x] `increase()` method
  - [x] Input amount validation
    - [ ] Units on increase and decrease - swap exact in will be wrong on one
  - [x] Direction enforcement
  - [x] Apply funding/borrow deltas
  - [x] VWAP recalculation
  - [x] Entry value update
  - [x] Size increment
  - [ ] Invariant checks
  - [ ] Emit event (optional)
- [x] `decrease()` method
  - [x] Input validation
  - [x] Apply funding/borrow deltas
  - [x] Prorate entry value
  - [x] Prorate funding/borrow
  - [x] Size decrement
  - [x] Reset if fully closed
  - [ ] Emit event (optional)
  - [x] Return data for PnL module

## 🔗 Mars Protocol Integration
- [x] Install Mars into project
- [x] Query funding/borrow rates
  - [x] Store principal borrowed
  - [x] Calculate interest accrued as (debt - principal) (make a helper method)
  - [x] Only update principal when we borrow more or repay debt (reduce principal, but remove the prorated borrow cost from this )
- [x] Compute funding deltas
- [x] Helper to pass values to position

## 📐 Core Structures & Logic
- [x] Define `Side` enum
- [x] Implement `Position` struct
- [x] VWAP + weighted average helpers
- [ ] Validate position size math
- [ ] Store `StrategyType` in config

## 💬 Message Handling
- [x] 🧾 Message Structs
  - [x] Define `IncreasePosition` message struct
  - [x] Define `DecreasePosition` message struct
  - [x] Define `CompleteHedge` message struct
  - [x] Add messages to `ExecuteMsg` enum
  - [x] Derive `JsonSchema`, `Serialize`, `Deserialize` as needed
- [ ] 🧠 Validation Logic
  - [x] Validate `direction` and `amount` are present and valid
  - [x] Validate `amount > 0`
  - [x] Validate token pair route is supported
    - [x] Use Astroport route validation
    - [ ] Use Duality route validation
    - [ ] Use Mars-native route validator
  - [ ] Validate slippage tolerances
  - [ ] Validate caller permissions if required (e.g. internal-only hedge)
- [ ] 🔐 Access Control
  - [ ] Enforce `CompleteHedge` is only callable by contract address
  - [ ] Ensure any owner-only methods are gated
  - [ ] (Optional) Add config for allowed callers, multisig, etc.
- [x] 📨 Message Routing
  - [x] Implement `execute_increase_position()`
    - [x] Dispatch spot leg
    - [x] Dispatch self-call to `CompleteHedge`
  - [x] Implement `execute_complete_hedge()` (placeholder implementation)
    - [x] Query balance delta
    - [x] Query funding and borrow rates
    - [ ] Run profitability check
    - [ ] Dispatch perp leg if profitable
    - [ ] Update position state
  - [x] Implement `execute_decrease_position()` (placeholder implementation)
    - [x] Validate size
    - [x] Close spot leg
    - [x] Close perp leg
    - [ ] Update PnL and state
- [ ] 📊 Event & Logging
  - [ ] Emit `PositionIncreased` event with spot details
  - [ ] Emit `PositionHedged` event with profit check outcome
  - [ ] Emit `PositionDecreased` event with PnL data
- [ ] 🦧 Message Handler Tests
  - [x] Test `IncreasePosition` dispatches spot + hedge messages (placeholder tests)
  - [ ] Test `CompleteHedge` executes only if profitable
  - [ ] Test rejection paths (unprofitable, slippage too high, invalid route)
  - [ ] Test `DecreasePosition` with proper state and PnL updates
  - [ ] Fuzz or prop test `slippage`, `balance deltas`, and route handling

## ⚙️ Trade Execution
- [x] Spot execution: Mars
- [x] Perp execution: Mars
- [x] Price calc helpers
- [ ] Slippage protection

## 💸 Profitability Checks
- [ ] Profitability formula implementation README.md
- [ ] Implementation of check in order_validation.rs
- [ ] Considers market rate for borrow and funding, config params etc
- [ ] Considers fees

## 🧠 State Management
- [x] Atomic position updates
- [x] Position init/removal
- [ ] User-based storage map

## 📣 Events & Logging
- [ ] Define event schemas
- [ ] Emit events for position changes
- [ ] Optional metrics

## 🧪 Testing
- [x] Unit Tests
  - [x] PnL core + funding/borrow calculations
  - [x] VWAP + weighted average calculations
  - [x] Position increase operations
  - [x] Position decrease operations
  - [x] Helper functions and math utilities
  - [ ] Mars adapter functions
  - [ ] Funding rate computations
  - [ ] Borrow rate computations
  - [ ] Slippage calculations
  - [ ] Profitability check formulas

- [ ] Integration Tests
  - [ ] Full position lifecycle (increase → hedge → decrease)
  - [ ] Mars protocol integration
  - [ ] Astroport swap operations
  - [ ] Multi-operation scenarios
  - [ ] External query responses

- [ ] Advanced Test Types
  - [ ] Property-based tests for invariants
  - [ ] Fuzz testing for edge cases
  - [ ] Simulation tests for market conditions
  - [ ] Stress tests for high volume/extreme scenarios

- [ ] Validation Tests
  - [ ] Access control enforcement
  - [ ] Route validation
  - [ ] Slippage protection
  - [ ] Profitability thresholds
  - [ ] Error handling for all failure paths

## 🔐 Validation & Safety
- [ ] Validate decrease ≤ size
- [ ] Validate oracles + price freshness
- [ ] Slippage checks
- [ ] Error enum cleanup
- [ ] Optional pause guard

## 📊 Advanced Features (Later)
- [ ] Non-1:1 hedge ratios
- [ ] Dynamic hedge ratio
- [ ] Portfolio-wide metrics
- [ ] Margin / leverage accounting
- [ ] Unrealized PnL tracking

## 🌐 Integrations
- [ ] Mars
- [ ] Astroport
- [ ] Duality
- [ ] Oracle feeds
- [ ] Automated mgmt triggers

## 📈 Analytics & Reporting
- [ ] Historical PnL tracking
- [ ] Strategy performance charts
- [ ] Risk metrics

## 💹 Share Management System
- [x] 📝 Documentation
  - [x] Detailed minting/redemption methodology in `docs/mint_and_redeem_accounting.md`
  - [x] Defined total-value based share accounting model
  - [x] Example scenarios for different market conditions
  - [x] Value leakage prevention strategies documented
  - [ ] API documentation
  - [ ] Error handling guidelines

- [ ] 🏦 Share Token Implementation
  - [ ] Define `ShareInfo` struct
  - [ ] Implement storage for total shares
  - [ ] Token Factory integration
    - [ ] Create share token denom with Token Factory
    - [ ] Implement mint functionality with Token Factory
    - [ ] Implement burn functionality with Token Factory
    - [ ] Handle token transfer restrictions if needed
  - [ ] Implement user share balances map
  - [ ] Share price calculation functions

- [ ] 💰 Value Calculation
  - [x] Total strategy value calculation formula defined
    - [x] Spot position valuation approach
    - [x] Perpetual position valuation approach (including unrealized PnL)
    - [x] USDC balance accounting
    - [x] Funding payment accumulation
    - [x] Borrowing cost tracking
  - [x] Share price derivation formula defined
  - [x] Precision handling strategy defined
  - [ ] Implementation of value calculation functions

- [ ] 🔄 Share Operations
  - [ ] Implement `execute_deposit()` function
    - [ ] Validate deposit amount
    - [ ] Record value before deposit
    - [ ] Execute spot purchase
    - [ ] Execute perp short
    - [ ] Calculate value added
    - [ ] Mint shares proportionally
  - [ ] Implement `execute_redeem()` function
    - [ ] Validate share amount
    - [ ] Calculate redemption proportion
    - [ ] Record USDC balance before
    - [ ] Sell proportional spot position
    - [ ] Close proportional perp position
    - [ ] Calculate USDC gain
    - [ ] Burn shares
    - [ ] Transfer proceeds to user

- [ ] 🛡️ Protection Mechanisms
  - [ ] Implement TWAPs for price oracles
  - [ ] Multiple oracle validation
  - [ ] Rebalancing before mint/redeem
  - [ ] Deposit/withdrawal fee structure
  - [ ] Rate limiting for large transactions
  - [ ] Circuit breakers for extreme conditions

- [ ] 🧪 Share System Testing
  - [ ] Unit tests for share calculations
  - [ ] Minting test cases (first mint, subsequent mints)
  - [ ] Redemption test cases
  - [ ] Edge case testing (price volatility)
  - [ ] Security tests (value leakage prevention)
  - [ ] Fuzz testing with random market conditions

- [ ] 🌉 Share Token Extensions
  - [ ] CW20 compliance for interoperability
  - [ ] Staking functionality
  - [ ] Governance features
  - [ ] Performance fee structure

## 🛂 Access Control
- [ ] Role-based access
- [ ] Admin configs
- [ ] Emergency pause