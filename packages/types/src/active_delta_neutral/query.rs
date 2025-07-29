use cosmwasm_std::{Addr, Decimal};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub credit_account_id: String,
    pub credit_manager_addr: Addr,
    pub oracle_addr: Addr,
    pub perps_addr: Addr,
    pub usdc_denom: String,
    pub spot_denom: String,
    pub perp_denom: String,
    pub acceptable_entry_delta: Decimal,
}
