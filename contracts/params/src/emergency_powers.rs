use cosmwasm_std::{
    to_json_binary, CosmosMsg, Decimal, DepsMut, MessageInfo, Response, Uint128, WasmMsg,
};
use mars_types::{
    address_provider::{self, MarsAddressType},
    perps::{ConfigUpdates, ExecuteMsg},
};

use crate::{
    error::ContractError,
    state::{ADDRESS_PROVIDER, ASSET_PARAMS, OWNER, PERP_PARAMS, VAULT_CONFIGS},
};

pub fn disable_borrowing(
    deps: DepsMut,
    info: MessageInfo,
    denom: &str,
) -> Result<Response, ContractError> {
    OWNER.assert_emergency_owner(deps.storage, &info.sender)?;

    let mut params = ASSET_PARAMS.load(deps.storage, denom)?;
    params.red_bank.borrow_enabled = false;
    ASSET_PARAMS.save(deps.storage, denom, &params)?;

    let response = Response::new()
        .add_attribute("action", "emergency_disable_borrowing")
        .add_attribute("denom", denom.to_string());

    Ok(response)
}

pub fn disallow_coin(
    deps: DepsMut,
    info: MessageInfo,
    denom: &str,
) -> Result<Response, ContractError> {
    OWNER.assert_emergency_owner(deps.storage, &info.sender)?;

    let mut params = ASSET_PARAMS.load(deps.storage, denom)?;
    params.credit_manager.whitelisted = false;
    ASSET_PARAMS.save(deps.storage, denom, &params)?;

    let response = Response::new()
        .add_attribute("action", "emergency_disallow_coin")
        .add_attribute("denom", denom.to_string());

    Ok(response)
}

pub fn set_zero_max_ltv(
    deps: DepsMut,
    info: MessageInfo,
    vault: &str,
) -> Result<Response, ContractError> {
    OWNER.assert_emergency_owner(deps.storage, &info.sender)?;

    let vault_addr = deps.api.addr_validate(vault)?;

    let mut config = VAULT_CONFIGS.load(deps.storage, &vault_addr)?;
    config.max_loan_to_value = Decimal::zero();
    VAULT_CONFIGS.save(deps.storage, &vault_addr, &config)?;

    let response = Response::new()
        .add_attribute("action", "emergency_set_zero_max_ltv")
        .add_attribute("vault", vault.to_string());

    Ok(response)
}

pub fn set_zero_deposit_cap(
    deps: DepsMut,
    info: MessageInfo,
    vault: &str,
) -> Result<Response, ContractError> {
    OWNER.assert_emergency_owner(deps.storage, &info.sender)?;

    let vault_addr = deps.api.addr_validate(vault)?;

    let mut config = VAULT_CONFIGS.load(deps.storage, &vault_addr)?;
    config.deposit_cap.amount = Uint128::zero();
    VAULT_CONFIGS.save(deps.storage, &vault_addr, &config)?;

    let response = Response::new()
        .add_attribute("action", "emergency_set_zero_deposit_cap")
        .add_attribute("vault", vault.to_string());

    Ok(response)
}

pub fn disable_withdraw_rb(
    deps: DepsMut,
    info: MessageInfo,
    denom: &str,
) -> Result<Response, ContractError> {
    OWNER.assert_emergency_owner(deps.storage, &info.sender)?;

    let mut params = ASSET_PARAMS.load(deps.storage, denom)?;
    params.red_bank.withdraw_enabled = false;
    ASSET_PARAMS.save(deps.storage, denom, &params)?;

    let response = Response::new()
        .add_attribute("action", "emergency_disable_withdraw_rb")
        .add_attribute("denom", denom.to_string());

    Ok(response)
}

pub fn disable_withdraw_cm(
    deps: DepsMut,
    info: MessageInfo,
    denom: &str,
) -> Result<Response, ContractError> {
    OWNER.assert_emergency_owner(deps.storage, &info.sender)?;

    let mut params = ASSET_PARAMS.load(deps.storage, denom)?;
    params.credit_manager.withdraw_enabled = false;
    ASSET_PARAMS.save(deps.storage, denom, &params)?;

    let response = Response::new()
        .add_attribute("action", "emergency_disable_withdraw_cm")
        .add_attribute("denom", denom.to_string());

    Ok(response)
}

pub fn disable_perp_trading(
    deps: DepsMut,
    info: MessageInfo,
    denom: &str,
) -> Result<Response, ContractError> {
    OWNER.assert_emergency_owner(deps.storage, &info.sender)?;

    let mut params = PERP_PARAMS.load(deps.storage, denom)?;
    params.enabled = false;
    PERP_PARAMS.save(deps.storage, denom, &params)?;

    let current_addr = ADDRESS_PROVIDER.load(deps.storage)?;
    let perps_addr = address_provider::helpers::query_contract_addr(
        deps.as_ref(),
        &current_addr,
        MarsAddressType::Perps,
    )?;

    let msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: perps_addr.to_string(),
        msg: to_json_binary(&ExecuteMsg::UpdateMarket {
            params,
        })?,
        funds: vec![],
    });

    let response = Response::new()
        .add_message(msg)
        .add_attribute("action", "emergency_disable_perp_trading")
        .add_attribute("denom", denom.to_string());

    Ok(response)
}

pub fn disable_deleverage(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    OWNER.assert_emergency_owner(deps.storage, &info.sender)?;
    let updates: ConfigUpdates = ConfigUpdates {
        deleverage_enabled: Some(false),
        ..Default::default()
    };

    let update_config_msg = create_update_perp_config_msg(deps, updates)?;

    Ok(Response::new()
        .add_message(update_config_msg)
        .add_attribute("action", "emergency_disable_perp_deleverage"))
}

pub fn disable_counterparty_vault_withdraw(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    OWNER.assert_emergency_owner(deps.storage, &info.sender)?;
    let updates: ConfigUpdates = ConfigUpdates {
        vault_withdraw_enabled: Some(false),
        ..Default::default()
    };

    let param_update_msg = create_update_perp_config_msg(deps, updates)?;

    Ok(Response::new()
        .add_message(param_update_msg)
        .add_attribute("action", "emergency_disable_vault_withdraw"))
}

fn create_update_perp_config_msg(
    deps: DepsMut,
    updates: ConfigUpdates,
) -> Result<CosmosMsg, ContractError> {
    let current_addr = ADDRESS_PROVIDER.load(deps.storage)?;
    let perps_addr = address_provider::helpers::query_contract_addr(
        deps.as_ref(),
        &current_addr,
        MarsAddressType::Perps,
    )?;

    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: perps_addr.to_string(),
        msg: to_json_binary(&ExecuteMsg::UpdateConfig {
            updates,
        })?,
        funds: vec![],
    }))
}
