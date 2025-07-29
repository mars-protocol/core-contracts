# Delta Neutral Contract


## Core Concepts

[Position Management](./docs/position_specificiation.md) – maintain position state (size, direction, VWAPs, entry value) for logging, querying and invariant validation

[PnL Tracking](./docs/pnl_accounting.md) – compute realized profit/loss on position reduction using exit vs. prorated entry value

[Yield Accounting](./docs/yield_accounting.md) – compute realized funding payments and borrow costs

[[Strategy Abstraction](TODO)] – define strategy-specific logic (e.g., long spot/short perp) via pluggable trait implementations

[[Profitability Model](./docs/entry_exit_model.md)] – enforced on-chain to ensure trades meet minimum expected return (bot is trustless, only triggers)

## Getting Started

### Prerequisites

- Rust and Cargo installed on your system.

### Building the Project

To build the project, run:

```bash
cargo build
```

### Running Tests

To execute the tests, use:

```bash
cargo test
```

## Contributing

Contributions are welcome! Please fork the repository and submit a pull request for any improvements or bug fixes.

## License

This project is licensed under the MIT License. 