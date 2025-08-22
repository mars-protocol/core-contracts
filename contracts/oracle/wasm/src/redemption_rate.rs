use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_json_binary, Addr, Decimal, QuerierWrapper, QueryRequest, StdResult, WasmQuery,
};

/// The query message for the neutron LST oracle
/// This is the only type we need so we don't import the package
#[cw_serde]
enum NeutronQueryMsg {
    GetRedemptionRate {},
    GetLstAssetDenom {},
}

/// The redemption rate provided by neutron has a different interface than the standard
/// redemption rate. It uses the query `GetRedemptionRate` instead of `RedemptionRate`, and
/// does not return the update time.
pub fn query_redemption_rate(querier: &QuerierWrapper, contract_addr: Addr) -> StdResult<Decimal> {
    querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: contract_addr.into_string(),
        msg: to_json_binary(&NeutronQueryMsg::GetRedemptionRate {})?,
    }))
}

pub fn query_slinky_lst_denom(querier: &QuerierWrapper, contract_addr: &Addr) -> StdResult<String> {
    let response: String = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: contract_addr.to_string(),
        msg: to_json_binary(&NeutronQueryMsg::GetLstAssetDenom {})?,
    }))?;

    Ok(response)
}
