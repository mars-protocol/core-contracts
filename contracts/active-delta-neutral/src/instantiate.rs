use cosmwasm_std::{DepsMut, Env, MessageInfo, ReplyOn, Response, SubMsg};
use mars_owner::OwnerInit;
use mars_types::{
    active_delta_neutral::{
        instantiate::InstantiateMsg, query::Config, reply::INSTANTIATE_CREDIT_ACCOUNT_REPLY_ID,
    },
    adapters::credit_manager,
    address_provider::{self, MarsAddressType},
    health::AccountKind,
};
use mars_utils::helpers::validate_native_denom;

use crate::{
    error::ContractResult,
    state::{CONFIG, OWNER},
};

pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    // validate inputs
    let address_provider = deps.api.addr_validate(&msg.address_provider)?;
    validate_native_denom(&msg.base_denom)?;
    let base_denom = msg.base_denom;
    let owner = info.sender;

    // load the addresses we need
    let required_addresses: Vec<MarsAddressType> = vec![
        MarsAddressType::CreditManager,
        MarsAddressType::Oracle,
        MarsAddressType::Perps,
        MarsAddressType::RedBank,
        MarsAddressType::Params,
    ];
    let addresses = address_provider::helpers::query_contract_addrs(
        deps.as_ref(),
        &address_provider,
        required_addresses,
    )?;

    let cm_addr = &addresses[&MarsAddressType::CreditManager];
    let oracle_addr = &addresses[&MarsAddressType::Oracle];
    let perps_addr = &addresses[&MarsAddressType::Perps];
    let red_bank_addr = &addresses[&MarsAddressType::RedBank];
    let params_addr = &addresses[&MarsAddressType::Params];

    let credit_manager = credit_manager::CreditManager::new(cm_addr.clone());
    let create_credit_account_msg = credit_manager.create_credit_account(AccountKind::Default)?;
    let config: Config = Config {
        credit_account_id: None,
        credit_manager_addr: cm_addr.clone(),
        oracle_addr: oracle_addr.clone(),
        perps_addr: perps_addr.clone(),
        red_bank_addr: red_bank_addr.clone(),
        params_addr: params_addr.clone(),
        base_denom,
    };

    let create_credit_account_sub_msg = SubMsg {
        msg: create_credit_account_msg,
        gas_limit: None,
        id: INSTANTIATE_CREDIT_ACCOUNT_REPLY_ID,
        reply_on: ReplyOn::Success,
    };

    OWNER.initialize(
        deps.storage,
        deps.api,
        OwnerInit::SetInitialOwner {
            owner: owner.to_string(),
        },
    )?;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new()
        .add_submessage(create_credit_account_sub_msg)
        .add_attribute("method", "instantiate")
        .add_attribute("contract_name", env.contract.address)
        .add_attribute("contract_version", "0.1.0")
        .add_attribute("sender", owner))
}
