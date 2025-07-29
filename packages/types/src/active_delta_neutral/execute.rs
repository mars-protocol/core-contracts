use crate::swapper::SwapperRoute;
use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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