use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Decimal;

#[cw_serde]
pub struct FeeTier {
    pub id: String,
    pub min_voting_power: String, // Uint128 as string
    pub discount_pct: Decimal,    // Percentage as Decimal (e.g., 0.25 for 25%)
}

#[cw_serde]
pub struct FeeTierConfig {
    pub tiers: Vec<FeeTier>,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum FeeTierQueryMsg {
    #[returns(TradingFeeResponse)]
    TradingFee {
        address: String,
        market_type: MarketType,
    },
}

#[cw_serde]
pub struct TradingFeeResponse {
    pub base_fee_bps: u16,
    pub discount_pct: Decimal,
    pub effective_fee_bps: u16,
    pub bucket_id: String,
}

#[cw_serde]
pub enum MarketType {
    Spot,
    Perp,
}
