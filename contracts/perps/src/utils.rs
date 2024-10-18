use std::collections::HashMap;

use cosmwasm_std::{
    Addr, Attribute, Decimal, Deps, Int128, Order, SignedDecimal, StdResult, Uint128,
};
use mars_types::{
    adapters::{
        oracle::{Oracle, OracleBase},
        params::ParamsBase,
    },
    keys::{UserId, UserIdKey},
    oracle::ActionKind,
    params::PerpParams,
    perps::{PnlAmounts, Position},
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

// Updates the attributes vector with details of a modified position.
///
/// This function is responsible for pushing key attributes related to the original
/// and modified position into the provided `attrs` vector.
///
/// # Parameters:
/// - `attrs`: A mutable vector of `Attribute` to store the event data.
/// - `denom`: The market denomination for the position.
/// - `pos`: The original position being modified.
/// - `new_size`: The new size of the modified position.
/// - `current_denom_price`: The current market price of the denomination.
/// - `new_skew`: The skew of the market prior to the position modification.
/// - `new_accrued_funding_per_unit`: The updated funding accrued per unit after the modification.
/// - `pnl_amt`: The unrealized PnL of the original position, which will be realized as a result of the modification.
pub fn update_position_attributes(
    attrs: &mut Vec<Attribute>,
    denom: &str,
    position: &Position,
    new_size: Int128,
    current_denom_price: Decimal,
    new_skew: Int128,
    new_accrued_funding_per_unit: SignedDecimal,
    pnl_amt: &PnlAmounts,
) {
    attrs.push(Attribute::new("denom", denom));
    attrs.push(Attribute::new("entry_size", position.size.to_string()));
    attrs.push(Attribute::new("entry_price", position.entry_price.to_string()));
    attrs.push(Attribute::new("entry_skew", position.initial_skew.to_string()));
    attrs.push(Attribute::new(
        "entry_accrued_funding_per_unit",
        position.entry_accrued_funding_per_unit_in_base_denom.to_string(),
    ));
    attrs.push(Attribute::new("new_size", new_size.to_string()));
    attrs.push(Attribute::new("current_price", current_denom_price.to_string()));
    attrs.push(Attribute::new("new_skew", new_skew.to_string()));
    attrs.push(Attribute::new(
        "new_accrued_funding_per_unit",
        new_accrued_funding_per_unit.to_string(),
    ));
    attrs.push(Attribute::new("realized_pnl_before", position.realized_pnl.pnl.to_string()));
    attrs.push(Attribute::new("realized_pnl_change", pnl_amt.pnl.to_string()));
}
