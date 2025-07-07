# Mars Protocol Swapper Architecture

The Mars Protocol's swapper module provides a flexible and extensible architecture for implementing DEX integrations across different blockchain ecosystems. This document outlines the architecture, implementation patterns, and usage guidelines.

## Overview

The swapper is designed with a modular architecture consisting of:

1. **Base Contract** (`mars-swapper-base`): Chain-agnostic logic and interfaces
2. **Implementation-specific Contracts**: Chain-specific swapper implementations
3. **Route Implementations**: DEX-specific routing logic

## Security Warning

⚠️ **IMPORTANT**: Swapper contracts should NEVER hold any funds. Any funds sent to the contract, except as part of executing the `SwapExactIn` message, can be stolen by an attacker. See [Oak Audit 2023-08-01](https://github.com/oak-security/audit-reports/blob/master/Mars/2023-08-01%20Audit%20Report%20-%20Mars%20Red%20Bank%20Updates%20v1.0.pdf) issue 14.

## Architecture

### Base Swapper

The `SwapBase` struct in `mars-swapper-base` provides the foundation for all swapper implementations with:

- Standard contract entry points (instantiate, execute, query)
- Route storage and management
- Owner management
- Configuration storage

```rust
pub struct SwapBase<'a, Q, M, R, C>
where
    Q: CustomQuery,
    M: CustomMsg,
    C: Config,
    R: Route<M, Q, C>,
{
    pub owner: Owner<'a>,
    pub routes: Map<'a, (String, String), R>,
    pub config: Item<'a, C>,
    pub custom_query: PhantomData<Q>,
    pub custom_message: PhantomData<M>,
}
```

### Core Traits

#### Route Trait

The `Route` trait defines the interface that all swap route implementations must provide:

```rust
pub trait Route<M, Q, C>:
    Serialize + DeserializeOwned + Clone + Debug + Display + PartialEq + JsonSchema
where
    M: CustomMsg,
    Q: CustomQuery,
    C: Config,
{
    fn from(route: SwapperRoute, config: Option<C>) -> ContractResult<Self>;
    
    fn validate(
        &self,
        querier: &QuerierWrapper<Q>,
        denom_in: &str,
        denom_out: &str,
    ) -> ContractResult<()>;
    
    fn build_exact_in_swap_msg(
        &self,
        querier: &QuerierWrapper<Q>,
        env: &Env,
        coin_in: &Coin,
        min_receive: Uint128,
    ) -> ContractResult<CosmosMsg<M>>;
    
    fn estimate_exact_in_swap(
        &self,
        querier: &QuerierWrapper<Q>,
        env: &Env,
        coin_in: &Coin,
    ) -> ContractResult<EstimateExactInSwapResponse>;
}
```

#### Config Trait

The `Config` trait is used for implementation-specific configuration:

```rust
pub trait Config: Serialize + DeserializeOwned + Clone + Debug + PartialEq + JsonSchema {
    fn validate(&self, api: &dyn Api) -> ContractResult<()>;
}
```

## Swapper Implementations

The repository includes several swapper implementations for different DEXes:

### Osmosis Swapper

- Integrates with the Osmosis DEX
- Uses pool IDs for routing
- Supports TWAP (Time-Weighted Average Price) for price estimation
- Handles both standard pools and CosmWasm pools

### Duality Swapper

- Integrates with Neutron's Duality DEX
- Handles both direct swaps (via limit orders) and multi-hop swaps
- Uses Neutron's Stargate messages for DEX operations

### Astroport Swapper

- Integrates with Astroport DEX
- Uses router contract for swap execution
- Handles different pool types supported by Astroport

### Mock Swapper

- A simplified implementation for testing purposes
- Only implements essential functions (`SwapExactIn` and `EstimateExactInSwap`)
- When calling `ExecuteMsg::SwapExactIn`, `denom_out` must be `uosmo` and the resulting amount will always be `1337uosmo`
- The contract MUST be prefunded with this amount

## Core Functions

### Setting Routes

Routes can be set by the contract owner:

```rust
ExecuteMsg::SetRoute {
    denom_in: String,
    denom_out: String,
    route: R,
}
```

### Executing Swaps

Swaps can be executed using:

```rust
ExecuteMsg::SwapExactIn {
    coin_in: Coin,
    denom_out: String,
    min_receive: Uint128,
    route: Option<SwapperRoute>,
}
```

- If `route` is provided, it will be used for the swap
- If `route` is not provided, a previously saved route for the given `denom_in` and `denom_out` will be used
- `min_receive` specifies the minimum amount of tokens to receive, protecting against slippage

### Estimating Swaps

Swap output can be estimated using:

```rust
QueryMsg::EstimateExactInSwap {
    coin_in: Coin,
    denom_out: String,
    route: Option<SwapperRoute>,
}
```

## Creating a New Swapper Implementation

To implement a new swapper for a different DEX:

1. Create a new package in the `contracts/swapper` directory
2. Define a route struct specific to the DEX
3. Implement the `Route` trait for your route struct
4. Define a configuration struct if needed and implement the `Config` trait
5. Use `SwapBase` to implement the contract entry points

## Testing Considerations

When testing swapper implementations:

1. **Route Validation**: Test that routes are properly validated
2. **Swap Estimation**: Test estimation with saved and provided routes
3. **Swap Execution**: Verify correct message construction and execution
4. **Error Handling**: Test behavior with invalid inputs and routes
5. **Integration**: Test end-to-end swap flows with mock DEX responses

## Examples

See the implementations in the repository for working examples:

- `./osmosis/` - Osmosis DEX integration
- `./duality/` - Neutron's Duality DEX integration
- `./astroport/` - Astroport DEX integration
- `./mock/` - Mock implementation for testing
