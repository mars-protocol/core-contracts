use cosmwasm_std::{Addr, Uint128};
use mars_types::{math::SignedDecimal, perps::Config};

use crate::error::{ContractError, ContractResult};

pub fn ensure_min_position(
    position_in_base_denom: Uint128,
    cfg: &Config<Addr>,
) -> ContractResult<()> {
    if position_in_base_denom < cfg.min_position_in_base_denom {
        return Err(ContractError::PositionTooSmall {
            min: cfg.min_position_in_base_denom,
            found: position_in_base_denom,
            base_denom: cfg.base_denom.clone(),
        });
    }
    Ok(())
}

pub fn ensure_max_position(
    position_in_base_denom: Uint128,
    cfg: &Config<Addr>,
) -> ContractResult<()> {
    // could be set to None if not needed
    if let Some(max_pos_in_base_denom) = cfg.max_position_in_base_denom {
        if position_in_base_denom > max_pos_in_base_denom {
            return Err(ContractError::PositionTooBig {
                max: max_pos_in_base_denom,
                found: position_in_base_denom,
                base_denom: cfg.base_denom.clone(),
            });
        }
    }
    Ok(())
}

/// Ensure that the new position size does not flip the position from long to short or vice versa
pub fn ensure_position_not_flipped(
    old_size: SignedDecimal,
    new_size: SignedDecimal,
) -> ContractResult<()> {
    if !new_size.is_zero() && new_size.is_positive() != old_size.is_positive() {
        return Err(ContractError::IllegalPositionModification {
            reason: "Cannot flip Position. Submit independent close and open messages".to_string(),
        });
    }
    Ok(())
}
