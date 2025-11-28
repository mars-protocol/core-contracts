Title: Enable spot swaps in trigger orders (minimal TWAP v1)

TL;DR
- Allow Action::SwapExactIn in trigger orders so users (incl. HLS/Amber) can place “limit-like” spot swaps with a min execution price.
- Frontend creates many small independent orders to approximate TWAP over time; each order carries a price condition and min_receive guard.
- No protocol-side scheduling in this task; a later follow-up will add native TWAP chunking if desired.

Problem
- Trigger orders today do not support spot swaps, blocking users from automating entries/exits at target prices unless using perps.
- HLS/Amber strategies need the ability to scale in/out with price guards; FE can orchestrate cadence, but the contract must permit spot triggers.

Goals (this task)
- Enable spot swap execution via triggers with price protection:
  - Permit Action::SwapExactIn inside CreateTriggerOrder actions.
  - Rely on existing min_receive and optional Oracle/Relative price conditions to enforce min execution price.
- Maintain all existing safety checks (HLS rules, Max-LTV, deposit caps) post-execution.

Non‑Goals (follow-ups)
- Native TWAP order type with on-chain chunking and cooldowns.
- Time-based or block-based conditions.
- Keeper fee schedule changes or bulk pre-funding.

Design Overview
- Minimal contract change: adjust allowed actions validation during trigger creation to include spot swaps.
  - File: contracts/credit-manager/src/trigger.rs:41
  - Current: only ExecutePerpOrder, Lend, ClosePerpPosition are allowed in triggers.
  - Change: add Action::SwapExactIn to the allowed set.
- Execution path remains unchanged:
  - ExecuteTriggerOrder → dispatch_actions → CallbackMsg::SwapExactIn → swap_exact_in.
  - swap_exact_in applies fee discount, enforces min_receive, and updates balances.
- Price enforcement
  - On-chain: conditions can assert OraclePrice or RelativePrice; if false, execution is rejected.
  - DEX-level: SwapExactIn’s min_receive guarantees execution at or better than the caller’s bound.

Frontend Plan (TWAP via many small orders)
- Compose K independent CreateTriggerOrder items, each with:
  - Action::SwapExactIn { coin_in, denom_out, min_receive, route? }.
  - Condition::OraclePrice or Condition::RelativePrice for the target level.
  - keeper_fee (per order).
- Scheduler executes eligible orders over time (e.g., every N minutes), achieving a TWAP-like fill.
- min_receive computation:
  - Fetch swap fee via QueryMsg::SwapFeeRate (contract responds with Decimal).
  - amount_in_effective = amount_in × (1 – fee).
  - min_receive = floor(amount_in_effective × min_price_out_per_in × 10^decimals_out / 10^decimals_in).

Contract Changes (precise)
- File to modify: contracts/credit-manager/src/trigger.rs:41
- In create_trigger_order, broaden the allowed actions predicate:
  - Before: matches!(action, ExecutePerpOrder | Lend | ClosePerpPosition)
  - After:  matches!(action, ExecutePerpOrder | Lend | ClosePerpPosition | SwapExactIn)
- No storage, query, or message format changes; no migrations required.

Safety/Constraints preserved
- HLS assertions: CallbackMsg::AssertHlsRules still runs for HLS accounts.
- Max LTV assertion: CallbackMsg::AssertMaxLTV enforces health invariants after actions.
- Deposit caps checked via AssertDepositCaps.

Testing Plan
- Unit tests in contracts/credit-manager/tests:
  1) Creation
     - CreateTriggerOrder with a single SwapExactIn action is accepted.
     - Illegal actions remain rejected.
  2) Execution success
     - Given price condition true and DEX route returning >= min_receive, balances update and keeper fee transfers.
  3) Execution guard
     - If DEX returns < min_receive, execution fails; trigger order remains stored.
  4) HLS behavior
     - For AccountKind::HighLeveredStrategy, ensure AssertHlsRules passes/fails as expected after swap.
  5) Order relations
     - Parent/child logic remains unaffected; independent swap triggers execute individually.

Acceptance Criteria
- create_trigger_order accepts SwapExactIn in actions.
- execute_trigger_order can execute a spot-swap trigger when conditions are met.
- Min price protection via min_receive works (revert if not met).
- All existing post-action assertions still run and protect state.
- No migration required; existing orders unaffected.

Rollout
- Ship as a minor version bump of credit-manager.
- Indexers and FE unaffected by schema changes.
- FE to expose “Price-protected spot trigger” and enable batch creation for TWAP.

Risks/Mitigations
- Keeper fee overhead with many small orders → FE can batch reasonable chunk sizes; later native TWAP can reduce fees.
- Miscomputed min_receive client-side → add helper in FE/SDK to compute from on-chain fee; include decimals handling.

Follow‑ups (separate tickets)
- MP-XXXX: Native TWAP trigger type with on-chain slice_count, cooldown_secs, and auto re-store until complete.
- MP-XXXX: TimePassed/BlockPassed condition type.
- MP-XXXX: Bulk trigger creation helper and fee optimization.

Estimated Effort
- Contract change + tests: S (≤1 day dev + review).
- FE wiring (batch creation + helper): S.

Code Touchpoints (references)
- contracts/credit-manager/src/trigger.rs:41
- contracts/credit-manager/src/execute.rs:340–392 (dispatch SwapExactIn) and 700–740 (CallbackMsg::SwapExactIn handling)
- contracts/credit-manager/src/swap.rs:1 (swap_exact_in logic)
- contracts/credit-manager/src/contract.rs:183 (QueryMsg::SwapFeeRate)

