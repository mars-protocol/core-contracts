use cosmwasm_std::Uint128;
use mars_types::swapper::SwapperRoute;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub address_provider: String,
    pub astroport_router: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum ExecuteMsg {
    Increase {
        amount: Uint128,
        denom: String,
        swapper_route: SwapperRoute,
    },
    Decrease {
        amount: Uint128,
        denom: String,
        swapper_route: SwapperRoute,
    },
    CompleteHedge {
        swap_exact_in_amount: Uint128,
        denom: String,
        increasing: bool,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct QueryMsg {}
