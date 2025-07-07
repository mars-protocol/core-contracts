#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response};
use cw2::set_contract_version;
use mars_owner::OwnerInit::SetInitialOwner;
use mars_types::params::{
    CmEmergencyUpdate, EmergencyUpdate, ExecuteMsg, InstantiateMsg, MigrateMsg,
    PerpsEmergencyUpdate, QueryMsg, RedBankEmergencyUpdate,
};

use crate::{
    emergency_powers::{
        disable_borrowing, disable_counterparty_vault_withdraw, disable_deleverage,
        disable_perp_trading, disable_withdraw_cm, disable_withdraw_rb, disallow_coin,
        set_zero_deposit_cap, set_zero_max_ltv,
    },
    error::{ContractError, ContractResult},
    execute::{
        reset_risk_manager, update_asset_params, update_config, update_managed_vault_config,
        update_perp_params, update_vault_config,
    },
    migrations,
    query::{
        query_all_asset_params, query_all_asset_params_v2, query_all_perp_params,
        query_all_perp_params_v2, query_all_total_deposits_v2, query_all_vault_configs,
        query_all_vault_configs_v2, query_config, query_managed_vault_config, query_total_deposit,
        query_vault_config,
    },
    state::{ADDRESS_PROVIDER, ASSET_PARAMS, MAX_PERP_PARAMS, OWNER, PERP_PARAMS, RISK_MANAGER},
};

pub const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _: Env,
    _: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;

    OWNER.initialize(
        deps.storage,
        deps.api,
        SetInitialOwner {
            owner: msg.owner.clone(),
        },
    )?;

    RISK_MANAGER.initialize(
        deps.storage,
        deps.api,
        SetInitialOwner {
            owner: msg.risk_manager.unwrap_or(msg.owner),
        },
    )?;

    let address_provider_addr = deps.api.addr_validate(&msg.address_provider)?;
    ADDRESS_PROVIDER.save(deps.storage, &address_provider_addr)?;

    MAX_PERP_PARAMS.save(deps.storage, &msg.max_perp_params)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::UpdateOwner(update) => Ok(OWNER.update(deps, info, update)?),
        ExecuteMsg::UpdateRiskManager(update) => Ok(RISK_MANAGER.update(deps, info, update)?),
        ExecuteMsg::ResetRiskManager() => reset_risk_manager(deps, info),
        ExecuteMsg::UpdateConfig {
            address_provider,
            max_perp_params,
        } => update_config(deps, info, address_provider, max_perp_params),
        ExecuteMsg::UpdateAssetParams(update) => update_asset_params(deps, info, update),
        ExecuteMsg::UpdateVaultConfig(update) => update_vault_config(deps, info, update),
        ExecuteMsg::UpdatePerpParams(update) => update_perp_params(deps, info, update),
        ExecuteMsg::EmergencyUpdate(update) => match update {
            EmergencyUpdate::RedBank(rb_u) => match rb_u {
                RedBankEmergencyUpdate::DisableBorrowing(denom) => {
                    disable_borrowing(deps, info, &denom)
                }
                RedBankEmergencyUpdate::DisableWithdraw(denom) => {
                    disable_withdraw_rb(deps, info, &denom)
                }
            },
            EmergencyUpdate::CreditManager(rv_u) => match rv_u {
                CmEmergencyUpdate::DisallowCoin(denom) => disallow_coin(deps, info, &denom),
                CmEmergencyUpdate::SetZeroMaxLtvOnVault(v) => set_zero_max_ltv(deps, info, &v),
                CmEmergencyUpdate::SetZeroDepositCapOnVault(v) => {
                    set_zero_deposit_cap(deps, info, &v)
                }
                CmEmergencyUpdate::DisableWithdraw(denom) => {
                    disable_withdraw_cm(deps, info, &denom)
                }
            },
            EmergencyUpdate::Perps(p_u) => match p_u {
                PerpsEmergencyUpdate::DisableTrading(denom) => {
                    disable_perp_trading(deps, info, &denom)
                }
                PerpsEmergencyUpdate::DisableDeleverage() => disable_deleverage(deps, info),
                PerpsEmergencyUpdate::DisableCounterpartyVaultWithdraw() => {
                    disable_counterparty_vault_withdraw(deps, info)
                }
            },
        },
        ExecuteMsg::UpdateManagedVaultConfig(managed_vault_config_update) => {
            update_managed_vault_config(deps, info, managed_vault_config_update)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    let res = match msg {
        QueryMsg::Owner {} => to_json_binary(&OWNER.query(deps.storage)?),
        QueryMsg::RiskManager {} => to_json_binary(&RISK_MANAGER.query(deps.storage)?),
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::ManagedVaultConfig {} => to_json_binary(&query_managed_vault_config(deps)?),
        QueryMsg::AssetParams {
            denom,
        } => to_json_binary(&ASSET_PARAMS.may_load(deps.storage, &denom)?),
        QueryMsg::AllAssetParams {
            start_after,
            limit,
        } => to_json_binary(&query_all_asset_params(deps, start_after, limit)?),
        QueryMsg::AllAssetParamsV2 {
            start_after,
            limit,
        } => to_json_binary(&query_all_asset_params_v2(deps, start_after, limit)?),
        QueryMsg::VaultConfig {
            address,
        } => to_json_binary(&query_vault_config(deps, &address)?),
        QueryMsg::AllVaultConfigs {
            start_after,
            limit,
        } => to_json_binary(&query_all_vault_configs(deps, start_after, limit)?),
        QueryMsg::AllVaultConfigsV2 {
            start_after,
            limit,
        } => to_json_binary(&query_all_vault_configs_v2(deps, start_after, limit)?),
        QueryMsg::PerpParams {
            denom,
        } => to_json_binary(&PERP_PARAMS.load(deps.storage, &denom)?),
        QueryMsg::AllPerpParams {
            start_after,
            limit,
        } => to_json_binary(&query_all_perp_params(deps, start_after, limit)?),
        QueryMsg::AllPerpParamsV2 {
            start_after,
            limit,
        } => to_json_binary(&query_all_perp_params_v2(deps, start_after, limit)?),
        QueryMsg::TotalDeposit {
            denom,
        } => to_json_binary(&query_total_deposit(deps, &env, denom)?),
        QueryMsg::AllTotalDepositsV2 {
            start_after,
            limit,
        } => to_json_binary(&query_all_total_deposits_v2(deps, start_after, limit)?),
    };
    res.map_err(Into::into)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    match msg {
        MigrateMsg::V2_2_3 {} => migrations::v2_2_3::migrate(deps),
        MigrateMsg::V2_3_0 {
            reserve_factor,
            interest_rate_model,
        } => migrations::v2_3_0::migrate(deps, reserve_factor, interest_rate_model),
    }
}
