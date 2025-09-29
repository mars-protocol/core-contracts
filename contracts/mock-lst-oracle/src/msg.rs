use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub redemption_rate: Decimal,
    pub lst_asset_denom: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    SetRedemptionRate {
        redemption_rate: Decimal,
    },
    SetLstAssetDenom {
        denom: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Decimal)]
    RedemptionRate {},
    #[returns(String)]
    GetLstAssetDenom {},
}
