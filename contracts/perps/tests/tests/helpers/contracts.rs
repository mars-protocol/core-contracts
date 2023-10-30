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

mod mock_credit_manager {
    #[cfg(not(feature = "library"))]
    use cosmwasm_std::entry_point;
    use cosmwasm_std::{Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult};
    use mars_types::credit_manager::{ExecuteMsg, QueryMsg};

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
    pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
        unimplemented!("query not supported")
    }
}
