use cosmwasm_std::{
    ensure, ensure_eq, to_json_binary, Addr, CosmosMsg, Deps, DepsMut, MessageInfo, Order,
    Response, WasmMsg,
};
use cw_storage_plus::Item;
use mars_owner::OwnerInit::SetInitialOwner;
use mars_types::{
    adapters::oracle::OracleBase,
    address_provider::{self, helpers::query_contract_addr, MarsAddressType},
    params::{
        AssetParams, AssetParamsUpdate, ManagedVaultConfigUpdate, PerpParams, PerpParamsUpdate,
        VaultConfigUpdate,
    },
    perps::ExecuteMsg,
    red_bank::{ExecuteMsg as RedBankExecuteMsg, MarketParams, MarketParamsUpdate},
};
use mars_utils::helpers::option_string_to_addr;

use crate::{
    error::{ContractError, ContractResult},
    state::{
        ADDRESS_PROVIDER, ASSET_PARAMS, BLACKLISTED_VAULTS, MANAGED_VAULT_CODE_IDS,
        MANAGED_VAULT_MIN_CREATION_FEE_IN_UUSD, MAX_PERP_PARAMS, OWNER, PERP_PARAMS, RISK_MANAGER,
        RISK_MANAGER_KEY, VAULT_CONFIGS,
    },
};

pub const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// Force resets the risk manager to the contract owner.
pub fn reset_risk_manager(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    OWNER.assert_owner(deps.storage, &info.sender)?;

    // Use same storage key as current RISK_MANAGER to remove existing state
    let storage_key = Item::<()>::new(RISK_MANAGER_KEY);
    storage_key.remove(deps.storage);

    RISK_MANAGER.initialize(
        deps.storage,
        deps.api,
        SetInitialOwner {
            owner: info.sender.to_string(),
        },
    )?;

    Ok(Response::new()
        .add_attribute("action", "reset_risk_manager")
        .add_attribute("new_risk_manager", info.sender.to_string()))
}

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    address_provider: Option<String>,
    max_perp_params: Option<u8>,
) -> Result<Response, ContractError> {
    OWNER.assert_owner(deps.storage, &info.sender)?;

    let current_addr = ADDRESS_PROVIDER.load(deps.storage)?;
    let updated_addr = option_string_to_addr(deps.api, address_provider, current_addr)?;
    ADDRESS_PROVIDER.save(deps.storage, &updated_addr)?;

    let mut res = Response::new()
        .add_attribute("action", "update_config")
        .add_attribute("address_provider", updated_addr.to_string());

    if let Some(max) = max_perp_params {
        MAX_PERP_PARAMS.save(deps.storage, &max)?;
        res = res.add_attribute("max_perp_params", max.to_string());
    }

    Ok(res)
}

/// Asserts that the price source is set for the given denom.
/// Returns an error if the price source is not set.
/// Helps to prevent updating params without setting the price source first.
///
/// # Arguments
///
/// * `deps` - The dependencies of the contract.
/// * `denom` - The denom to check the price source for.
///
/// # Returns
///
/// * `()` - If the price source is set.
/// * `ContractError::PriceSourceNotFound` - If the price source is not set.
fn assert_oracle_price_source(deps: Deps, denom: &str) -> ContractResult<()> {
    let address_provider = ADDRESS_PROVIDER.load(deps.storage)?;
    let oracle_addr = query_contract_addr(deps, &address_provider, MarsAddressType::Oracle)?;
    let oracle_addr_adapter = OracleBase::new(oracle_addr);

    let has_price_source =
        oracle_addr_adapter.has_price_source(&deps.querier, denom)?.has_price_source;
    if !has_price_source {
        return Err(ContractError::PriceSourceNotFound {
            denom: denom.to_string(),
        });
    }
    Ok(())
}

pub fn update_asset_params(
    deps: DepsMut,
    info: MessageInfo,
    update: AssetParamsUpdate,
) -> ContractResult<Response> {
    let permission = Permission::new(deps.as_ref(), &info.sender)?;

    let mut response = Response::new().add_attribute("action", "update_asset_param");

    match update {
        AssetParamsUpdate::AddOrUpdate {
            params: unchecked,
        } => {
            let params = unchecked.check(deps.api)?;

            assert_oracle_price_source(deps.as_ref(), &params.denom)?;

            // Risk manager cannot change the liquidation threshold
            permission.validate_asset_liquidation_threshold_unchanged(&params)?;

            // Risk manager cannot change the reserve factor
            let market_params = (&params).into();
            permission.validate_market_reserve_factor_unchanged(&market_params)?;

            ASSET_PARAMS.save(deps.storage, &params.denom, &params)?;

            let ap_addr = ADDRESS_PROVIDER.load(deps.storage)?;
            let rb_addr = address_provider::helpers::query_contract_addr(
                deps.as_ref(),
                &ap_addr,
                MarsAddressType::RedBank,
            )?;

            let msg = CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: rb_addr.to_string(),
                msg: to_json_binary(&RedBankExecuteMsg::UpdateMarketParams(
                    MarketParamsUpdate::AddOrUpdate {
                        params: market_params,
                    },
                ))?,
                funds: vec![],
            });

            response = response
                .add_message(msg)
                .add_attribute("action_type", "add_or_update")
                .add_attribute("sender", info.sender)
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
    let permission = Permission::new(deps.as_ref(), &info.sender)?;

    let mut response = Response::new().add_attribute("action", "update_perp_param");

    match update {
        PerpParamsUpdate::AddOrUpdate {
            params,
        } => {
            let checked = params.check()?;

            assert_oracle_price_source(deps.as_ref(), &checked.denom)?;

            // Risk manager cannot change the liquidation threshold
            permission.validate_perps_liquidation_threshold_unchanged(&checked)?;

            PERP_PARAMS.save(deps.storage, &checked.denom, &checked)?;

            let current_addr = ADDRESS_PROVIDER.load(deps.storage)?;
            let perps_addr = address_provider::helpers::query_contract_addr(
                deps.as_ref(),
                &current_addr,
                MarsAddressType::Perps,
            )?;

            let msg = CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: perps_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::UpdateMarket {
                    params: checked,
                })?,
                funds: vec![],
            });

            response = response
                .add_message(msg)
                .add_attribute("action_type", "add_or_update")
                .add_attribute("sender", info.sender)
                .add_attribute("denom", params.denom);
        }
    }

    // Check if the number of perp params is within the limit
    let max_perp_params = MAX_PERP_PARAMS.load(deps.storage)?;
    let num = PERP_PARAMS.keys(deps.storage, None, None, Order::Ascending).count();
    ensure!(
        num <= max_perp_params as usize,
        ContractError::MaxPerpParamsReached {
            max: max_perp_params
        }
    );

    Ok(response)
}

struct Permission<'a> {
    deps: Deps<'a>,
    owner: bool,
    risk_manager: bool,
}

impl<'a> Permission<'a> {
    pub fn new(deps: Deps<'a>, sender: &Addr) -> ContractResult<Self> {
        let owner = OWNER.is_owner(deps.storage, sender)?;
        let risk_manager = RISK_MANAGER.is_owner(deps.storage, sender)?;
        ensure!(owner || risk_manager, ContractError::NotOwnerOrRiskManager {});
        Ok(Self {
            deps,
            owner,
            risk_manager,
        })
    }

    pub fn validate_asset_liquidation_threshold_unchanged(
        &self,
        new_params: &AssetParams,
    ) -> ContractResult<()> {
        // If the risk_manager is not set to the default (owner) apply restrictions
        if self.risk_manager && !self.owner {
            let current_asset_params =
                ASSET_PARAMS.may_load(self.deps.storage, &new_params.denom)?;
            if let Some(current_asset_params) = current_asset_params {
                ensure_eq!(
                    current_asset_params.liquidation_threshold,
                    new_params.liquidation_threshold,
                    ContractError::RiskManagerUnauthorized {
                        reason: "asset param liquidation threshold".to_string()
                    }
                )
            } else {
                return Err(ContractError::RiskManagerUnauthorized {
                    reason: "new asset".to_string(),
                });
            }
        }
        Ok(())
    }

    pub fn validate_perps_liquidation_threshold_unchanged(
        &self,
        new_params: &PerpParams,
    ) -> ContractResult<()> {
        // If the risk_manager is not set to the default (owner) apply restrictions
        if self.risk_manager && !self.owner {
            let current_perps_params =
                PERP_PARAMS.may_load(self.deps.storage, &new_params.denom)?;
            if let Some(current_perps_params) = current_perps_params {
                ensure_eq!(
                    current_perps_params.liquidation_threshold,
                    new_params.liquidation_threshold,
                    ContractError::RiskManagerUnauthorized {
                        reason: "perp param liquidation threshold".to_string()
                    }
                );
            } else {
                return Err(ContractError::RiskManagerUnauthorized {
                    reason: "new perp".to_string(),
                });
            }
        }
        Ok(())
    }

    pub fn validate_market_reserve_factor_unchanged(
        &self,
        new_params: &MarketParams,
    ) -> ContractResult<()> {
        // If the risk_manager is not set to the default (owner) apply restrictions
        if self.risk_manager && !self.owner {
            let current_asset_params =
                ASSET_PARAMS.may_load(self.deps.storage, &new_params.denom)?;

            if let Some(current_asset_params) = current_asset_params {
                ensure_eq!(
                    Some(current_asset_params.reserve_factor),
                    new_params.reserve_factor,
                    ContractError::RiskManagerUnauthorized {
                        reason: "market param reserve factor".to_string()
                    }
                )
            } else {
                return Err(ContractError::RiskManagerUnauthorized {
                    reason: "new asset".to_string(),
                });
            }
        }
        Ok(())
    }
}

pub fn update_managed_vault_config(
    deps: DepsMut,
    info: MessageInfo,
    update: ManagedVaultConfigUpdate,
) -> ContractResult<Response> {
    let _permission = Permission::new(deps.as_ref(), &info.sender)?;

    let mut response = Response::new().add_attribute("action", "update_managed_vault_config");

    match update {
        ManagedVaultConfigUpdate::AddCodeId(code_id) => {
            let mut code_ids = MANAGED_VAULT_CODE_IDS.may_load(deps.storage)?.unwrap_or_default();

            if !code_ids.code_ids.contains(&code_id) {
                code_ids.code_ids.push(code_id);
                MANAGED_VAULT_CODE_IDS.save(deps.storage, &code_ids)?;

                response = response
                    .add_attribute("action_type", "add_code_id")
                    .add_attribute("code_id", code_id.to_string());
            }
        }
        ManagedVaultConfigUpdate::RemoveCodeId(code_id) => {
            let mut code_ids = MANAGED_VAULT_CODE_IDS.may_load(deps.storage)?.unwrap_or_default();

            if let Some(index) = code_ids.code_ids.iter().position(|id| *id == code_id) {
                code_ids.code_ids.remove(index);
                MANAGED_VAULT_CODE_IDS.save(deps.storage, &code_ids)?;

                response = response
                    .add_attribute("action_type", "remove_code_id")
                    .add_attribute("code_id", code_id.to_string());
            }
        }
        ManagedVaultConfigUpdate::SetMinCreationFeeInUusd(min_creation_fee_in_uusd) => {
            MANAGED_VAULT_MIN_CREATION_FEE_IN_UUSD.save(deps.storage, &min_creation_fee_in_uusd)?;
            response = response
                .add_attribute("action_type", "set_min_creation_fee_in_uusd")
                .add_attribute("min_creation_fee_in_uusd", min_creation_fee_in_uusd.to_string());
        }
        ManagedVaultConfigUpdate::AddVaultToBlacklist(vault_addr) => {
            let mut blacklisted_vaults =
                BLACKLISTED_VAULTS.may_load(deps.storage)?.unwrap_or_default();
            let addr = deps.api.addr_validate(&vault_addr)?;

            if !blacklisted_vaults.vaults.contains(&addr) {
                blacklisted_vaults.vaults.push(addr);
                BLACKLISTED_VAULTS.save(deps.storage, &blacklisted_vaults)?;

                response = response
                    .add_attribute("action_type", "add_blacklisted_vault")
                    .add_attribute("vault_addr", vault_addr);
            }
        }
        ManagedVaultConfigUpdate::RemoveVaultFromBlacklist(vault_addr) => {
            let mut blacklisted_vaults =
                BLACKLISTED_VAULTS.may_load(deps.storage)?.unwrap_or_default();
            let addr = deps.api.addr_validate(&vault_addr)?;

            if let Some(index) = blacklisted_vaults.vaults.iter().position(|v| v == addr) {
                blacklisted_vaults.vaults.remove(index);
                BLACKLISTED_VAULTS.save(deps.storage, &blacklisted_vaults)?;

                response = response
                    .add_attribute("action_type", "remove_blacklisted_vault")
                    .add_attribute("vault_addr", vault_addr);
            }
        }
    }

    Ok(response)
}
