use cosmwasm_std::Empty;
use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_nft_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mars_account_nft::contract::execute,
        mars_account_nft::contract::instantiate,
        mars_account_nft::contract::query,
    );
    Box::new(contract)
}

pub fn mock_health_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mars_mock_rover_health::contract::execute,
        mars_mock_rover_health::contract::instantiate,
        mars_mock_rover_health::contract::query,
    );
    Box::new(contract)
}

pub fn mock_credit_manager_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mars_mock_credit_manager::contract::execute,
        mars_mock_credit_manager::contract::instantiate,
        mars_mock_credit_manager::contract::query,
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

pub fn mock_oracle_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mars_mock_oracle::contract::execute,
        mars_mock_oracle::contract::instantiate,
        mars_mock_oracle::contract::query,
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

pub fn mock_incentives_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mars_mock_incentives::contract::execute,
        mars_mock_incentives::contract::instantiate,
        mars_mock_incentives::contract::query,
    );
    Box::new(contract)
}
