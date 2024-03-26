use cosmwasm_std::{Addr, Decimal, DepsMut, Env, Response, Storage, Uint128};
use mars_types::{
    math::SignedDecimal,
    oracle::ActionKind,
    perps::{DenomState, Funding},
    signed_uint::SignedUint,
};

use crate::{
    denom::DenomStateExt,
    error::{ContractError, ContractResult},
    state::{CONFIG, DENOM_STATES, OWNER},
};

pub fn init_denom(
    store: &mut dyn Storage,
    env: Env,
    sender: &Addr,
    denom: &str,
    max_funding_velocity: Decimal,
    skew_scale: Uint128,
) -> ContractResult<Response> {
    OWNER.assert_owner(store, sender)?;

    if DENOM_STATES.has(store, denom) {
        return Err(ContractError::DenomAlreadyExists {
            denom: denom.into(),
        });
    }

    if skew_scale.is_zero() {
        return Err(ContractError::InvalidParam {
            reason: "skew_scale cannot be zero".to_string(),
        });
    }

    let denom_state = DenomState {
        enabled: true,
        funding: Funding {
            max_funding_velocity,
            skew_scale,
            last_funding_rate: SignedDecimal::zero(),
            last_funding_accrued_per_unit_in_base_denom: SignedUint::zero(),
        },
        last_updated: env.block.time.seconds(),
        ..Default::default()
    };
    DENOM_STATES.save(store, denom, &denom_state)?;

    Ok(Response::new()
        .add_attribute("method", "init_denom")
        .add_attribute("denom", denom)
        .add_attribute("max_funding_velocity", max_funding_velocity.to_string())
        .add_attribute("skew_scale", skew_scale.to_string()))
}

pub fn enable_denom(
    store: &mut dyn Storage,
    env: Env,
    sender: &Addr,
    denom: &str,
) -> ContractResult<Response> {
    OWNER.assert_owner(store, sender)?;

    DENOM_STATES.update(store, denom, |maybe_ds| {
        // if the denom does not already exist then we cannot enable it
        let Some(mut ds) = maybe_ds else {
            return Err(ContractError::DenomNotFound {
                denom: denom.into(),
            });
        };

        // if the denom already exists, if must have not already been enabled
        if ds.enabled {
            return Err(ContractError::DenomEnabled {
                denom: denom.into(),
            });
        }

        // now we know the denom exists and is not enabled
        // flip the enabled parameter to true and return
        ds.enabled = true;

        // When denom is disabled there is no trading activity so funding shouldn't be changed.
        // We just shift the last_updated time.
        ds.last_updated = env.block.time.seconds();

        Ok(ds)
    })?;

    Ok(Response::new().add_attribute("method", "enable_denom").add_attribute("denom", denom))
}

pub fn disable_denom(
    deps: DepsMut,
    env: Env,
    sender: &Addr,
    denom: &str,
) -> ContractResult<Response> {
    OWNER.assert_owner(deps.storage, sender)?;

    let cfg = CONFIG.load(deps.storage)?;

    DENOM_STATES.update(deps.storage, denom, |maybe_ds| {
        let Some(mut ds) = maybe_ds else {
            return Err(ContractError::DenomNotFound {
                denom: denom.into(),
            });
        };

        let current_time = env.block.time.seconds();

        let denom_price = cfg.oracle.query_price(&deps.querier, denom, ActionKind::Default)?.price;
        let base_denom_price =
            cfg.oracle.query_price(&deps.querier, &cfg.base_denom, ActionKind::Default)?.price;

        // refresh funding rate and index before disabling trading
        let current_funding = ds.current_funding(current_time, denom_price, base_denom_price)?;
        ds.funding = current_funding;

        ds.enabled = false;
        ds.last_updated = current_time;

        Ok(ds)
    })?;

    Ok(Response::new().add_attribute("method", "disable_denom").add_attribute("denom", denom))
}
