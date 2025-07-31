use cosmwasm_std::{DepsMut, Env, MessageInfo, ReplyOn, Response, SubMsg};
use mars_types::{
    active_delta_neutral::{
        instantiate::InstantiateMsg, query::Config, reply::INSTANTIATE_CREDIT_ACCOUNT_REPLY_ID,
    },
    adapters::credit_manager,
    address_provider::{self, MarsAddressType},
    health::AccountKind,
};

use crate::{error::ContractResult, state::CONFIG};

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

    let addresses = address_provider::helpers::query_contract_addrs(
        deps.as_ref(),
        &address_provider,
        required_addresses,
    )?;

    let cm_addr = &addresses[&MarsAddressType::CreditManager];
    let oracle_addr = &addresses[&MarsAddressType::Oracle];
    let perps_addr = &addresses[&MarsAddressType::Perps];
    let health_addr = &addresses[&MarsAddressType::Health];
    let red_bank_addr = &addresses[&MarsAddressType::RedBank];

    let owner = info.sender.clone();
    let credit_manager = credit_manager::CreditManager::new(cm_addr.clone());
    let create_credit_account_msg = credit_manager.create_credit_account(AccountKind::Default)?;
    let config: Config = Config {
        owner: owner.clone(),
        // Initially set to 0, as we haven't created the credit account yet
        // TODO make this nicer - maybe set as an option?
        credit_account_id: "0".to_string(),
        credit_manager_addr: cm_addr.clone(),
        oracle_addr: oracle_addr.clone(),
        perps_addr: perps_addr.clone(),
        health_addr: health_addr.clone(),
        red_bank_addr: red_bank_addr.clone(),
    };

    let submsg = SubMsg {
        msg: create_credit_account_msg,
        gas_limit: None,
        id: INSTANTIATE_CREDIT_ACCOUNT_REPLY_ID,
        reply_on: ReplyOn::Success,
    };

    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new()
        .add_submessage(submsg)
        .add_attribute("method", "instantiate")
        .add_attribute("contract_name", env.contract.address)
        .add_attribute("contract_version", "0.1.0")
        .add_attribute("sender", owner))
}
