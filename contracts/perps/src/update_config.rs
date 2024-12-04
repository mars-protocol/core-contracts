use cosmwasm_std::{Addr, DepsMut, Response};
use mars_types::{
    address_provider::{self, MarsAddressType},
    error::MarsError,
    perps::ConfigUpdates,
};

use crate::{
    error::{ContractError, ContractResult},
    state::{CONFIG, OWNER},
};

pub fn update_config(
    deps: DepsMut,
    sender: Addr,
    updates: ConfigUpdates,
) -> Result<Response, ContractError> {
    let mut existing_cfg = CONFIG.load(deps.storage)?;

    assert_is_authorized(&deps, &sender, &existing_cfg.address_provider)?;

    let mut response = Response::new().add_attribute("action", "update_config");

    if let Some(ap_unchecked) = updates.address_provider {
        response = response.add_attribute("address_provider", ap_unchecked.to_string());
        existing_cfg.address_provider = deps.api.addr_validate(&ap_unchecked)?;
    }

    if let Some(cooldown_period) = updates.cooldown_period {
        response = response.add_attribute("cooldown_period", cooldown_period.to_string());
        existing_cfg.cooldown_period = cooldown_period;
    }

    if let Some(max_positions) = updates.max_positions {
        response = response.add_attribute("max_positions", max_positions.to_string());
        existing_cfg.max_positions = max_positions;
    }

    if let Some(protocol_fee_rate) = updates.protocol_fee_rate {
        response = response.add_attribute("protocol_fee_rate", protocol_fee_rate.to_string());
        existing_cfg.protocol_fee_rate = protocol_fee_rate;
    }

    if let Some(tcr) = updates.target_vault_collateralization_ratio {
        response = response.add_attribute("target_vault_collateralization_ratio", tcr.to_string());
        existing_cfg.target_vault_collateralization_ratio = tcr
    }

    if let Some(deleverage_enabled) = updates.deleverage_enabled {
        response = response.add_attribute("deleverage_enabled", deleverage_enabled.to_string());
        existing_cfg.deleverage_enabled = deleverage_enabled
    }

    if let Some(vwe) = updates.vault_withdraw_enabled {
        response = response.add_attribute("vault_withdraw_enabled", vwe.to_string());
        existing_cfg.vault_withdraw_enabled = vwe
    }

    if let Some(max_unlocks) = updates.max_unlocks {
        response = response.add_attribute("max_unlocks", max_unlocks.to_string());
        existing_cfg.max_unlocks = max_unlocks;
    }

    CONFIG.save(deps.storage, &existing_cfg)?;

    Ok(response)
}

/// Asserts that the sender is authorized to update the parameters
fn assert_is_authorized(deps: &DepsMut, sender: &Addr, ap_addr: &Addr) -> ContractResult<()> {
    let params_addr = address_provider::helpers::query_contract_addr(
        deps.as_ref(),
        ap_addr,
        MarsAddressType::Params,
    )?;

    // Only the owner or the params contract can update the configuration
    if !(sender == params_addr || OWNER.is_owner(deps.storage, sender)?) {
        return Err(ContractError::Mars(MarsError::Unauthorized {}));
    }

    Ok(())
}
