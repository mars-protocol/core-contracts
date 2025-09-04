# Mock LST Oracle Contract

This contract, `mars-mock-lst-oracle`, is a mock implementation of a Liquid Staking Token (LST) oracle from slinky, designed specifically for use within the testing environment.

See the [slinky vault](https://github.com/neutron-org/slinky-vault/tree/main/contracts/lst-oracle) repository for the production version of the LST oracle.

## Purpose

In a production environment, the Mars oracle relies on external LST oracles to fetch redemption rates for various liquid staked assets. To ensure robust and predictable testing without depending on external oracle deployments, this mock contract was created.

It simulates the core behavior of an LST oracle by allowing test setups to instantiate it with a specific `redemption_rate` and `lst_asset_denom`. These values can also be updated during test execution, enabling a wide range of scenarios to be tested reliably.

## State & Configuration

-   **`redemption_rate`**: A `Decimal` value representing the LST's redemption rate.
-   **`lst_asset_denom`**: A `String` for the LST asset's denomination (e.g., `st_atom`).

This mock contract is a crucial component of the `packages/testing` crate, where it is uploaded and instantiated as part of the standard test setup.
