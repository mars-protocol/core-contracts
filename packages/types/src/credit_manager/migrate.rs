use cosmwasm_schema::cw_serde;

#[cw_serde]
pub enum MigrateMsg {
    V2_2_0ToV2_2_3 {},
    V2_2_3ToV2_3_0 {
        max_trigger_orders: u8,
    },
}
