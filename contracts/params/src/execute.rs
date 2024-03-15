use cosmwasm_std::{DepsMut, MessageInfo, Response};
use mars_types::params::{AssetParamsUpdate, PerpParamsUpdate, VaultConfigUpdate};
use mars_utils::helpers::option_string_to_addr;

use crate::{
    error::{ContractError, ContractResult},
    state::{ADDRESS_PROVIDER, ASSET_PARAMS, OWNER, PERP_PARAMS, VAULT_CONFIGS},
};

pub const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    address_provider: Option<String>,
) -> Result<Response, ContractError> {
    OWNER.assert_owner(deps.storage, &info.sender)?;

    let current_addr = ADDRESS_PROVIDER.load(deps.storage)?;
    let updated_addr = option_string_to_addr(deps.api, address_provider, current_addr)?;
    ADDRESS_PROVIDER.save(deps.storage, &updated_addr)?;

    Ok(Response::new()
        .add_attribute("action", "update_config")
        .add_attribute("address_provider", updated_addr.to_string()))
}

pub fn update_asset_params(
    deps: DepsMut,
    info: MessageInfo,
    update: AssetParamsUpdate,
) -> ContractResult<Response> {
    OWNER.assert_owner(deps.storage, &info.sender)?;

    let mut response = Response::new().add_attribute("action", "update_asset_param");

    match update {
        AssetParamsUpdate::AddOrUpdate {
            params: unchecked,
        } => {
            let params = unchecked.check(deps.api)?;

            ASSET_PARAMS.save(deps.storage, &params.denom, &params)?;
            response = response
                .add_attribute("action_type", "add_or_update")
                .add_attribute("denom", params.denom);
        }
    }

    Ok(response)
}

pub fn update_vault_config(
    deps: DepsMut,
    info: MessageInfo,
    update: VaultConfigUpdate,
) -> ContractResult<Response> {
    OWNER.assert_owner(deps.storage, &info.sender)?;

    let mut response = Response::new().add_attribute("action", "update_vault_config");

    match update {
        VaultConfigUpdate::AddOrUpdate {
            config,
        } => {
            let checked = config.check(deps.api)?;
            VAULT_CONFIGS.save(deps.storage, &checked.addr, &checked)?;
            response = response
                .add_attribute("action_type", "add_or_update")
                .add_attribute("addr", checked.addr);
        }
    }

    Ok(response)
}

pub fn update_perp_params(
    deps: DepsMut,
    info: MessageInfo,
    update: PerpParamsUpdate,
) -> ContractResult<Response> {
    OWNER.assert_owner(deps.storage, &info.sender)?;

    let mut response = Response::new().add_attribute("action", "update_perp_param");

    match update {
        PerpParamsUpdate::AddOrUpdate {
            params,
        } => {
            let checked = params.check()?;

            PERP_PARAMS.save(deps.storage, &checked.denom, &checked)?;
            response = response
                .add_attribute("action_type", "add_or_update")
                .add_attribute("denom", params.denom);
        }
    }

    Ok(response)
}
