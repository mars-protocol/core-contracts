use cosmwasm_std::{Addr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub credit_account_id: String,
    pub credit_manager_addr: Addr,
    pub oracle_addr: Addr,
    pub perps_addr: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum QueryMsg {
    Config {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MarketConfig {
    pub market_id: String,
    pub usdc_denom: String,
    pub spot_denom: String,
    pub perp_denom: String,
    pub k: Uint128,
}
