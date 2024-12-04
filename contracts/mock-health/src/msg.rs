use cosmwasm_schema::cw_serde;
use mars_types::health::HealthValuesResponse;

#[cw_serde]
pub enum ExecuteMsg {
    SetHealthResponse {
        account_id: String,
        response: HealthValuesResponse,
    },
}
