use std::collections::HashMap;

use cosmwasm_std::{Addr, Decimal, Deps, Order, StdResult, Uint128};
use mars_types::{
    adapters::{
        oracle::{Oracle, OracleBase},
        params::ParamsBase,
    },
    keys::{UserId, UserIdKey},
    oracle::ActionKind,
    params::PerpParams,
};

use crate::{
    error::{ContractError, ContractResult},
    state::MARKET_STATES,
};

pub fn ensure_min_position(
    position_value: Uint128,
    perp_params: &PerpParams,
) -> ContractResult<()> {
    if position_value < perp_params.min_position_value {
        return Err(ContractError::PositionTooSmall {
            min: perp_params.min_position_value,
            found: position_value,
        });
    }
    Ok(())
}

pub fn ensure_max_position(
    position_value: Uint128,
    perp_params: &PerpParams,
) -> ContractResult<()> {
    // Could be set to None if not needed
    if let Some(max_pos_value) = perp_params.max_position_value {
        if position_value > max_pos_value {
            return Err(ContractError::PositionTooBig {
                max: max_pos_value,
                found: position_value,
            });
        }
    }
    Ok(())
}

pub fn create_user_id_key(
    user_addr: &Addr,
    account_id: Option<String>,
) -> ContractResult<UserIdKey> {
    let acc_id = account_id.unwrap_or("".to_string());
    let user_id = UserId::credit_manager(user_addr.clone(), acc_id);
    let user_id_key: UserIdKey = user_id.try_into()?;
    Ok(user_id_key)
}

pub fn get_markets_and_base_denom_prices(
    deps: &Deps,
    oracle: &Oracle,
    base_denom: &str,
    action: ActionKind,
) -> StdResult<HashMap<String, Decimal>> {
    let mut denoms = MARKET_STATES
        .keys(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    if !denoms.contains(&base_denom.to_string()) {
        denoms.push(base_denom.to_string())
    }

    oracle.query_prices_by_denoms(&deps.querier, denoms, action)
}

pub fn get_oracle_adapter(address: &Addr) -> OracleBase<Addr> {
    OracleBase::new(address.clone())
}

pub fn get_params_adapter(address: &Addr) -> ParamsBase<Addr> {
    ParamsBase::new(address.clone())
}
