use std::collections::HashMap;

use cosmwasm_std::{Decimal, Deps, StdResult};
use mars_rover_health_computer::{HealthComputer, PerpsData, VaultsData};
use mars_types::{
    credit_manager::Positions,
    health::{HealthResult, HealthState, HealthValuesResponse},
    oracle::ActionKind,
    params::AssetParams,
};

use crate::querier::HealthQuerier;

/// Uses `mars-rover-health-computer` which is a data agnostic package given
/// it's compiled to .wasm and shared with the frontend.
/// This function queries all necessary data to pass to `HealthComputer`.
pub fn compute_health(
    deps: Deps,
    q: HealthQuerier,
    positions: Positions,
    action: ActionKind,
) -> HealthResult<HealthValuesResponse> {
    // Get the denoms that need prices + markets
    let deposit_denoms = positions.deposits.iter().map(|d| &d.denom).collect::<Vec<_>>();
    let debt_denoms = positions.debts.iter().map(|d| &d.denom).collect::<Vec<_>>();
    let lend_denoms = positions.lends.iter().map(|d| &d.denom).collect::<Vec<_>>();
    let vault_infos = positions
        .vaults
        .iter()
        .map(|v| {
            let info = v.vault.query_info(&deps.querier)?;
            Ok((v.vault.address.clone(), info))
        })
        .collect::<StdResult<HashMap<_, _>>>()?;
    let vault_base_token_denoms = vault_infos.values().map(|v| &v.base_token).collect::<Vec<_>>();
    let staked_lp_denoms = positions.staked_astro_lps.iter().map(|d| &d.denom).collect::<Vec<_>>();
    let perp_denoms = positions.perps.iter().map(|p| &p.denom).collect::<Vec<_>>();
    let perp_vault_denoms = if let Some(p) = &positions.perp_vault {
        vec![&p.denom]
    } else {
        vec![]
    };

    // Collect prices + asset
    let mut asset_params: HashMap<String, AssetParams> = HashMap::new();
    let mut oracle_prices: HashMap<String, Decimal> = HashMap::new();

    deposit_denoms
        .into_iter()
        .chain(debt_denoms)
        .chain(lend_denoms)
        .chain(vault_base_token_denoms)
        .chain(staked_lp_denoms)
        .chain(perp_vault_denoms)
        .try_for_each(|denom| -> StdResult<()> {
            let params_opt = q.params.query_asset_params(&deps.querier, denom)?;
            // If the asset is not supported, we skip it (both params and price)
            if let Some(params) = params_opt {
                asset_params.insert(denom.to_string(), params);

                let price = q.oracle.query_price(&deps.querier, denom, action.clone())?.price;
                oracle_prices.insert(denom.to_string(), price);
            }
            Ok(())
        })?;

    // Collect all vault data
    let mut vaults_data: VaultsData = Default::default();
    positions.vaults.iter().try_for_each(|v| -> HealthResult<()> {
        let vault_coin_value = v.query_values(&deps.querier, &q.oracle, action.clone())?;
        vaults_data.vault_values.insert(v.vault.address.clone(), vault_coin_value);
        let config = q.query_vault_config(&v.vault)?;
        vaults_data.vault_configs.insert(v.vault.address.clone(), config);
        Ok(())
    })?;

    let mut perps_data: PerpsData = Default::default();
    perp_denoms.into_iter().try_for_each(|denom| -> StdResult<()> {
        // Perp data
        let perp_params = q.params.query_perp_params(&deps.querier, denom)?;
        perps_data.params.insert(denom.clone(), perp_params);
        Ok(())
    })?;

    let computer = HealthComputer {
        kind: positions.account_kind.clone(),
        positions,
        asset_params,
        oracle_prices,
        vaults_data,
        perps_data,
    };

    Ok(computer.compute_health()?.into())
}

pub fn compute_health_state(
    deps: Deps,
    querier: HealthQuerier,
    action: ActionKind,
    positions: Positions,
) -> HealthResult<HealthState> {
    // Helpful to not have to do computations & query the oracle for cases
    // like liquidations where oracle circuit breakers may hinder it.
    if positions.debts.is_empty() && positions.perps.is_empty() {
        return Ok(HealthState::Healthy);
    }

    let health = compute_health(deps, querier, positions, action)?;
    if !health.above_max_ltv {
        Ok(HealthState::Healthy)
    } else {
        Ok(HealthState::Unhealthy {
            max_ltv_health_factor: health.max_ltv_health_factor.unwrap(),
        })
    }
}

pub fn health_values(
    deps: Deps,
    account_id: &str,
    action: ActionKind,
) -> HealthResult<HealthValuesResponse> {
    let querier = HealthQuerier::new(&deps)?;
    let positions = querier.query_positions(account_id, action.clone())?;
    compute_health(deps, querier, positions, action)
}

pub fn health_state(deps: Deps, account_id: &str, action: ActionKind) -> HealthResult<HealthState> {
    let querier = HealthQuerier::new(&deps)?;
    let positions = querier.query_positions(account_id, action.clone())?;
    compute_health_state(deps, querier, action, positions)
}
