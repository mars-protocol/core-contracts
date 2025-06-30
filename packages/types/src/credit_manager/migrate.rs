use cosmwasm_schema::cw_serde;

use crate::oracle::V2Updates;

#[cw_serde]
pub enum MigrateMsg {
    V1_0_0ToV2_0_0(V2Updates),
    V2_0_2ToV2_0_3 {},
    V2_2_0ToV2_2_3 {},
}
