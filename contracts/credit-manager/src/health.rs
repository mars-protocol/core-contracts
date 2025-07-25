use cosmwasm_std::{Deps, Env, Response};
use mars_rover_health::{
    compute::{compute_health, compute_health_state},
    querier::HealthQuerier,
};
use mars_types::{
    health::{HealthState, HealthValuesResponse},
    oracle::ActionKind,
};

use crate::{
    error::{ContractError, ContractResult},
    query::{query_config, query_positions},
};

pub fn query_health_state(
    deps: Deps,
    env: Env,
    account_id: &str,
    action: ActionKind,
) -> ContractResult<HealthState> {
    let config = query_config(deps)?;
    let querier = HealthQuerier::new_with_config(&deps, env.contract.address.clone(), config)?;
    let positions = query_positions(deps, account_id, action.clone())?;
    let health = compute_health_state(deps, querier, action, positions)?;
    Ok(health)
}

pub fn query_health_values(
    deps: Deps,
    env: Env,
    account_id: &str,
    action: ActionKind,
) -> ContractResult<HealthValuesResponse> {
    let config = query_config(deps)?;
    let health_querier =
        HealthQuerier::new_with_config(&deps, env.contract.address.clone(), config)?;
    let positions = query_positions(deps, account_id, action.clone())?;
    let health = compute_health(deps, health_querier, positions, action)?;
    Ok(health)
}

pub fn assert_max_ltv(
    deps: Deps,
    env: Env,
    account_id: &str,
    prev_health: HealthState,
) -> ContractResult<Response> {
    let new_health = query_health_state(deps, env, account_id, ActionKind::Default)?;

    match (&prev_health, &new_health) {
        // If account ends in a healthy state, all good! ✅
        (_, HealthState::Healthy) => {}
        // If previous health was in an unhealthy state, assert it did not further weaken ⚠️
        (
            HealthState::Unhealthy {
                max_ltv_health_factor: prev_max_ltv_hf,
                liquidation_health_factor: prev_liq_hf,
            },
            HealthState::Unhealthy {
                max_ltv_health_factor: new_max_ltv_hf,
                liquidation_health_factor: new_liq_hf,
            },
        ) => {
            if prev_max_ltv_hf > new_max_ltv_hf {
                return Err(ContractError::HealthNotImproved {
                    prev_hf: prev_max_ltv_hf.to_string(),
                    new_hf: new_max_ltv_hf.to_string(),
                });
            }

            // Max LTV health factor is the same, but liquidation health factor has decreased, raise! ⚠️
            if prev_max_ltv_hf == new_max_ltv_hf && prev_liq_hf > new_liq_hf {
                return Err(ContractError::UnhealthyLiquidationHfDecrease {
                    prev_hf: prev_liq_hf.to_string(),
                    new_hf: new_liq_hf.to_string(),
                });
            }
        }
        // Else, it went from healthy to unhealthy, raise! ❌
        (
            HealthState::Healthy,
            HealthState::Unhealthy {
                max_ltv_health_factor,
                ..
            },
        ) => {
            return Err(ContractError::AboveMaxLTV {
                account_id: account_id.to_string(),
                max_ltv_health_factor: max_ltv_health_factor.to_string(),
            });
        }
    }

    Ok(Response::new()
        .add_attribute("action", "callback/assert_health")
        .add_attribute("account_id", account_id)
        .add_attribute("prev_health_state", prev_health.to_string())
        .add_attribute("new_health_state", new_health.to_string()))
}
