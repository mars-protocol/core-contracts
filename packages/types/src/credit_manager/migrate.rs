use cosmwasm_schema::cw_serde;

#[cw_serde]
pub enum MigrateMsg {
    V2_1_0ToV2_2_0 {},
    V2_2_0ToV2_2_1 {
        max_trigger_orders: u8,
    },
}
