use cosmwasm_std::Uint128;
use mars_types::{math::SignedDecimal, params::PerpParams};

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
    // could be set to None if not needed
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
