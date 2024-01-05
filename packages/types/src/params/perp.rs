use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;

#[cw_serde]
pub struct PerpParams {
    pub denom: String,
    pub max_net_oi: Uint128,
    pub max_long_oi: Uint128,
    pub max_short_oi: Uint128,
}
