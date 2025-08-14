use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{active_delta_neutral::query::MarketConfig, swapper::SwapperRoute};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum ExecuteMsg {
    Buy {
        amount: Uint128,
        market_id: String,
        swapper_route: SwapperRoute,
    },
    Sell {
        amount: Uint128,
        market_id: String,
        swapper_route: SwapperRoute,
    },
    Hedge {
        swap_exact_in_amount: Uint128,
        market_id: String,
        increasing: bool,
    },
    AddMarket {
        config: MarketConfig,
    },
    Deposit {},
    Withdraw {
        amount: Uint128,
        recipient: Option<String>,
    },
}
