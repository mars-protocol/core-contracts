use cosmwasm_std::{
    to_json_binary, Addr, DepsMut, Empty, Env, MessageInfo, QueryRequest, Response, WasmQuery,
};
use cw721::Cw721Execute;
use cw721_base::{
    ContractError::Ownership,
    OwnershipError::{NoOwner, NotOwner},
};
use mars_types::{
    account_nft::NftConfigUpdates,
    adapters::perps::PerpsBase,
    address_provider::{self, MarsAddressType},
    health::{HealthValuesResponse, QueryMsg::HealthValues},
    oracle::ActionKind,
};

use crate::{
    contract::Parent,
    error::ContractError::{self, BaseError, BurnNotAllowed},
    state::{CONFIG, NEXT_ID},
};

pub fn mint(deps: DepsMut, info: MessageInfo, user: &str) -> Result<Response, ContractError> {
    let next_id = NEXT_ID.load(deps.storage)?;
    NEXT_ID.save(deps.storage, &(next_id + 1))?;
    Parent::default()
        .mint(deps, info, next_id.to_string(), user.to_string(), None, Empty {})
        .map_err(Into::into)
}

/// A few checks to ensure accounts are not accidentally deleted:
/// - Cannot burn if debt balance
/// - Cannot burn if collateral exceeding config set amount
pub fn burn(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let addresses = address_provider::helpers::query_contract_addrs(
        deps.as_ref(),
        &config.address_provider_contract_addr,
        vec![MarsAddressType::Health, MarsAddressType::CreditManager, MarsAddressType::Perps],
    )?;
    let health_addr = &addresses[&MarsAddressType::Health];
    let cm_addr = &addresses[&MarsAddressType::CreditManager];
    let perps_addr = &addresses[&MarsAddressType::Perps];

    let response: HealthValuesResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: health_addr.into(),
            msg: to_json_binary(&HealthValues {
                account_id: token_id.clone(),
                action: ActionKind::Default,
            })?,
        }))?;

    if !response.total_debt_value.is_zero() {
        return Err(BurnNotAllowed {
            reason: format!("Account has a debt balance. Value: {}.", response.total_debt_value),
        });
    }

    if response.total_collateral_value > config.max_value_for_burn {
        return Err(BurnNotAllowed {
            reason: format!(
                "Account collateral value exceeds config set max ({}). Total collateral value: {}.",
                config.max_value_for_burn, response.total_collateral_value
            ),
        });
    }

    if response.has_perps {
        return Err(BurnNotAllowed {
            reason: "Account has active perp positions".to_string(),
        });
    }

    let perps: PerpsBase<Addr> = PerpsBase::new(perps_addr.clone());
    let vault_pos = perps.query_vault_position(&deps.querier, cm_addr, token_id.clone())?;
    if let Some(pos) = vault_pos {
        if !pos.deposit.amount.is_zero() || !pos.unlocks.is_empty() {
            return Err(BurnNotAllowed {
                reason: "Account has active perp vault deposits / unlocks".to_string(),
            });
        }
    }

    Parent::default().burn(deps, env, info, token_id).map_err(Into::into)
}

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    updates: NftConfigUpdates,
) -> Result<Response, ContractError> {
    let current_minter =
        Parent::default().minter(deps.as_ref())?.minter.ok_or(BaseError(Ownership(NoOwner)))?;

    if info.sender != current_minter {
        return Err(BaseError(Ownership(NotOwner)));
    }

    let mut response = Response::new().add_attribute("action", "update_config");
    let mut config = CONFIG.load(deps.storage)?;

    if let Some(unchecked) = updates.address_provider_contract_addr {
        let addr = deps.api.addr_validate(&unchecked)?;
        config.address_provider_contract_addr = addr.clone();
        response = response
            .add_attribute("key", "address_provider_contract_addr")
            .add_attribute("value", addr.to_string());
    }

    if let Some(max) = updates.max_value_for_burn {
        config.max_value_for_burn = max;
        response = response
            .add_attribute("key", "max_value_for_burn")
            .add_attribute("value", max.to_string());
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(response)
}
