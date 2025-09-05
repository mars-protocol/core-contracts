# Delta Neutral Contract

This contract implements an actively managed delta-neutral strategy utilizing the credit manager. Its primary goal is to maintain a market-neutral position by dynamically balancing long and short exposures via spot assets and perpetual futures, minimizing risk from price movements while capturing yield.


## Core Concepts

[Position Management](./docs/position_specificiation.md) – maintain position state (size, direction, VWAPs, entry value) for logging, querying and invariant validation

[PnL Tracking](./docs/pnl_accounting.md) – compute realized profit/loss on position reduction using exit vs. prorated entry value

[Yield Accounting](./docs/yield_accounting.md) – compute realized funding payments and borrow costs

[[Strategy Abstraction](TODO)] – define strategy-specific logic (e.g., long spot/short perp) via pluggable trait implementations

[[Profitability Model](./docs/entry_exit_model.md)] – enforced on-chain to ensure trades meet minimum expected return (bot is trustless, only triggers)