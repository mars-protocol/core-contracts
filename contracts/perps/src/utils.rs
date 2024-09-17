use cosmwasm_std::{Addr, Uint128};
use mars_types::{
    adapters::{oracle::OracleBase, params::ParamsBase},
    keys::{UserId, UserIdKey},
    params::PerpParams,
};

use crate::error::{ContractError, ContractResult};

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

pub fn get_oracle_adapter(address: &Addr) -> OracleBase<Addr> {
    OracleBase::new(address.clone())
}

pub fn get_params_adapter(address: &Addr) -> ParamsBase<Addr> {
    ParamsBase::new(address.clone())
}
