use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal};

use crate::fee_tiers::FeeTierConfig;

#[cw_serde]
pub enum MigrateMsg {
    V2_2_0ToV2_2_3 {},
    V2_2_3ToV2_3_0 {
        max_trigger_orders: u8,
    },
    V2_3_0ToV2_3_1 {
        swap_fee: Decimal,
    },
    V2_3_0ToV2_4_0 {
        fee_tier_config: FeeTierConfig,
        governance_address: Addr,
    },
}
