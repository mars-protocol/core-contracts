use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;

#[cw_serde]
pub struct PerpParams {
    /// Perp denomination
    pub denom: String,
    /// The maximum net open interest value (in oracle uusd denomination)
    pub max_net_oi_value: Uint128,
    /// The maximum long open interest value (in oracle uusd denomination)
    pub max_long_oi_value: Uint128,
    /// The maximum short open interest value (in oracle uusd denomination)
    pub max_short_oi_value: Uint128,
}
