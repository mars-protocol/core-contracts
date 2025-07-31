use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use mars_types::{
    active_delta_neutral::{instantiate::InstantiateMsg, query::Config},
    address_provider::{AddressResponseItem, MarsAddressType, QueryMsg},
};

use crate::{error::ContractResult, helpers::get_address_by_type, state::CONFIG};

pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    // load the addresses we need
    let address_provider = deps.api.addr_validate(&msg.address_provider)?;
    let required_addresses = vec![
        MarsAddressType::CreditManager,
        MarsAddressType::Oracle,
        MarsAddressType::Perps,
        MarsAddressType::Health,
        MarsAddressType::RedBank,
    ];
    let addresses: Vec<AddressResponseItem> = deps
        .querier
        .query_wasm_smart(address_provider, &QueryMsg::Addresses(required_addresses))?;

    let owner = info.sender.clone();

    let credit_manager_addr = get_address_by_type(&addresses, MarsAddressType::CreditManager)?;
    let oracle_addr = get_address_by_type(&addresses, MarsAddressType::Oracle)?;
    let perps_addr = get_address_by_type(&addresses, MarsAddressType::Perps)?;
    let health_addr = get_address_by_type(&addresses, MarsAddressType::Health)?;
    let red_bank_addr = get_address_by_type(&addresses, MarsAddressType::RedBank)?;

    // TODO isntantiate credit account
    let credit_account_id = "1";

    let config: Config = Config {
        owner: owner.clone(),
        credit_account_id: credit_account_id.to_string(),
        credit_manager_addr: deps.api.addr_validate(&credit_manager_addr.address)?,
        oracle_addr: deps.api.addr_validate(&oracle_addr.address)?,
        perps_addr: deps.api.addr_validate(&perps_addr.address)?,
        health_addr: deps.api.addr_validate(&health_addr.address)?,
        red_bank_addr: deps.api.addr_validate(&red_bank_addr.address)?,
    };

    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("contract_name", env.contract.address)
        .add_attribute("contract_version", "0.1.0")
        .add_attribute("sender", owner))
}
