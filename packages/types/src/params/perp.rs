use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, Uint128};

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
    /// The fee paid by the user to close a position (as a percent)
    pub closing_fee_rate: Decimal,
    /// The fee paid by the user to open a position (as a percent)
    pub opening_fee_rate: Decimal,
    /// The minimum value of a position (in oracle uusd denomination)
    pub min_position_value: Uint128,
    /// The maximum value of a position (in oracle uusd denomination)
    pub max_position_value: Option<Uint128>,
    /// Max loan to position value for the position.
    pub max_loan_to_value: Decimal,
    /// LTV at which a position becomes liquidatable
    pub liquidation_threshold: Decimal,
}
