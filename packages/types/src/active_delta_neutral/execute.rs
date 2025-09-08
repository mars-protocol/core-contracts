use cosmwasm_std::Uint128;
use cosmwasm_schema::cw_serde;use crate::{active_delta_neutral::query::MarketConfig, swapper::SwapperRoute};

#[cw_serde]
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
