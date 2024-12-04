use std::collections::HashMap;

use cosmwasm_std::{to_json_binary, Binary, ContractResult, QuerierResult};
use mars_types::perps::{QueryMsg, VaultPositionResponse, VaultResponse};

#[derive(Default)]
pub struct PerpsQuerier {
    pub vault: VaultResponse,
    pub vault_positions: HashMap<String, VaultPositionResponse>,
}

impl PerpsQuerier {
    pub fn handle_query(&self, query: QueryMsg) -> QuerierResult {
        let res: ContractResult<Binary> = match query {
            QueryMsg::VaultPosition {
                user_address,
                account_id: _,
            } => match self.vault_positions.get(&user_address.clone()) {
                Some(position) => to_json_binary(&position).into(),
                None => {
                    Err(format!("[mock]: could not find the position for {user_address}")).into()
                }
            },
            QueryMsg::Vault {
                action: _,
            } => to_json_binary(&self.vault).into(),
            _ => Err("[mock]: Unsupported perps query".to_string()).into(),
        };

        Ok(res).into()
    }
}
