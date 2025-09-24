use cosmwasm_std::Empty;
use cw_multi_test::{Contract, ContractWrapper};

/// Use Osmosis oracle instance for testing. We just need to be able to set/change prices.
pub fn mock_oracle_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mars_oracle_osmosis::contract::entry::execute,
        mars_oracle_osmosis::contract::entry::instantiate,
        mars_oracle_osmosis::contract::entry::query,
    );
    Box::new(contract)
}

pub fn mock_params_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mars_params::contract::execute,
        mars_params::contract::instantiate,
        mars_params::contract::query,
    );
    Box::new(contract)
}

pub fn mock_address_provider_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mars_address_provider::contract::execute,
        mars_address_provider::contract::instantiate,
        mars_address_provider::contract::query,
    );
    Box::new(contract)
}

pub fn mock_perps_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mars_perps::contract::execute,
        mars_perps::contract::instantiate,
        mars_perps::contract::query,
    );
    Box::new(contract)
}

pub fn mock_credit_manager_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mock_credit_manager::execute,
        mock_credit_manager::instantiate,
        mock_credit_manager::query,
    );
    Box::new(contract)
}

pub fn mock_incentives_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mars_incentives::contract::execute,
        mars_incentives::contract::instantiate,
        mars_incentives::contract::query,
    );
    Box::new(contract)
}

mod mock_credit_manager {
    #[cfg(not(feature = "library"))]
    use cosmwasm_std::entry_point;
    use cosmwasm_std::{Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult};
    use mars_types::{
        credit_manager::{Account, ExecuteMsg, Positions, QueryMsg},
        health::AccountKind,
    };

    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn instantiate(
        _deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        _msg: Empty,
    ) -> StdResult<Response> {
        Ok(Response::default())
    }

    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn execute(
        _deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        _msg: ExecuteMsg,
    ) -> StdResult<Response> {
        Ok(Response::default())
    }

    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
        match msg {
            QueryMsg::AccountKind {
                account_id: _,
            } => {
                // Return a mock account kind
                let account_kind = AccountKind::Default;
                cosmwasm_std::to_json_binary(&account_kind)
            }
            QueryMsg::Positions {
                account_id: _,
                action: _,
            } => {
                // Return empty positions
                let positions = Positions {
                    account_id: "1".to_string(),
                    account_kind: AccountKind::Default,
                    deposits: vec![],
                    debts: vec![],
                    lends: vec![],
                    vaults: vec![],
                    staked_astro_lps: vec![],
                    perps: vec![],
                };
                cosmwasm_std::to_json_binary(&positions)
            }
            QueryMsg::Accounts {
                owner: _,
                start_after: _,
                limit: _,
            } => {
                // Return a mock account
                let account = Account {
                    id: "1".to_string(),
                    kind: AccountKind::Default,
                };
                cosmwasm_std::to_json_binary(&vec![account])
            }
            QueryMsg::GetAccountTierAndDiscount {
                account_id: _,
            } => {
                // Return a mock tier and discount response
                let response = mars_types::credit_manager::AccountTierAndDiscountResponse {
                    tier_id: "default".to_string(),
                    discount_pct: cosmwasm_std::Decimal::zero(),
                    voting_power: cosmwasm_std::Uint128::zero(),
                };
                cosmwasm_std::to_json_binary(&response)
            }
            _ => unimplemented!("query not supported: {:?}", msg),
        }
    }
}
