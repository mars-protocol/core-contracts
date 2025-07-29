pub mod query;

use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::swapper::SwapperRoute;
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub address_provider: String,
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
pub enum QueryMsg {
    Config {},
}
