use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_json_binary, Addr, Api, Coin, CosmosMsg, QuerierWrapper, QueryRequest, StdResult, WasmMsg, WasmQuery
};

use crate::{
    credit_manager::{self},
    health::AccountKind,
};

#[cw_serde]
pub struct CreditManagerUnchecked(String);

impl CreditManagerUnchecked {
    pub fn new(address: String) -> Self {
        Self(address)
    }

    pub fn address(&self) -> &str {
        &self.0
    }

    pub fn check(&self, api: &dyn Api) -> StdResult<CreditManager> {
        let addr = api.addr_validate(self.address())?;
        Ok(CreditManager::new(addr))
    }
}

impl From<CreditManager> for CreditManagerUnchecked {
    fn from(credit_manager: CreditManager) -> Self {
        Self(credit_manager.addr.to_string())
    }
}

#[cw_serde]
pub struct CreditManager {
    pub addr: Addr,
}

impl CreditManager {
    pub fn new(addr: Addr) -> Self {
        Self {
            addr,
        }
    }
}

impl CreditManager {
    pub fn create_credit_account(&self, account_kind: AccountKind) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.addr.to_string(),
            msg: to_json_binary(&credit_manager::ExecuteMsg::CreateCreditAccount(account_kind))?,
            funds: vec![],
        }))
    }

    pub fn execute_actions_msg(
        &self,
        account_id: &str,
        actions: Vec<credit_manager::Action>,
        funds: &Vec<Coin>,
    ) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.addr.to_string(),
            msg: to_json_binary(&credit_manager::ExecuteMsg::UpdateCreditAccount {
                account_id: Some(account_id.to_string()),
                account_kind: Some(AccountKind::Default),
                actions,
            })?,
            funds: funds.to_vec(),
        }))
    }

    pub fn query_positions(
        &self,
        querier: &QuerierWrapper,
        account_id: &str,
    ) -> StdResult<credit_manager::Positions> {
        let response: credit_manager::Positions =
            querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: self.addr.to_string(),
                msg: to_json_binary(&credit_manager::QueryMsg::Positions {
                    account_id: account_id.to_string(),
                    action: None,
                })?,
            }))?;

        Ok(response)
    }
}
