use cosmwasm_std::{ensure_eq, Addr, Decimal, DepsMut, Env, MessageInfo, Response, Uint128};
use mars_types::{
    address_provider::{self, MarsAddressType},
    error::MarsError,
    red_bank::{Market, MarketParams, MarketParamsUpdate},
};
use mars_utils::helpers::validate_native_denom;

use crate::{
    error::{ContractError, ContractResult},
    interest_rates::{apply_accumulated_interests, update_interest_rates},
    state::{CONFIG, MARKETS},
};

pub fn update_market_params(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    update: MarketParamsUpdate,
) -> ContractResult<Response> {
    // Load the contract's configuration
    let cfg = CONFIG.load(deps.storage)?;

    // Ensure that the sender is authorized to update the parameters
    assert_is_authorized(&deps, &info.sender, &cfg.address_provider)?;

    match update {
        MarketParamsUpdate::AddOrUpdate {
            params,
        } => {
            let market = MARKETS.may_load(deps.storage, &params.denom)?;

            match market {
                Some(market) => update_asset(deps, env, params, market),
                None => {
                    validate_native_denom(&params.denom)?;

                    let new_market = create_market(env.block.time.seconds(), params.clone())?;
                    MARKETS.save(deps.storage, &params.denom, &new_market)?;

                    Ok(Response::new()
                        .add_attribute("action", "init_asset")
                        .add_attribute("denom", params.denom))
                }
            }
        }
    }
}

/// Initialize new market
pub fn create_market(block_time: u64, params: MarketParams) -> Result<Market, ContractError> {
    // Destructuring a struct’s fields into separate variables in order to force
    // compile error if we add more params
    let MarketParams {
        denom,
        reserve_factor,
        interest_rate_model,
    } = params;

    // All fields should be available
    let available = reserve_factor.is_some() && interest_rate_model.is_some();

    if !available {
        return Err(MarsError::InstantiateParamsUnavailable {}.into());
    }

    let new_market = Market {
        denom: denom.to_string(),
        borrow_index: Decimal::one(),
        liquidity_index: Decimal::one(),
        borrow_rate: Decimal::zero(),
        liquidity_rate: Decimal::zero(),
        reserve_factor: reserve_factor.unwrap(),
        indexes_last_updated: block_time,
        collateral_total_scaled: Uint128::zero(),
        debt_total_scaled: Uint128::zero(),
        interest_rate_model: interest_rate_model.unwrap(),
    };

    new_market.validate()?;

    Ok(new_market)
}

/// Update asset with new params.
pub fn update_asset(
    deps: DepsMut,
    env: Env,
    params: MarketParams,
    mut market: Market,
) -> Result<Response, ContractError> {
    // Destructuring a struct’s fields into separate variables in order to force
    // compile error if we add more params
    let MarketParams {
        denom,
        reserve_factor,
        interest_rate_model,
    } = params;

    // If reserve factor or interest rates are updated we update indexes with
    // current values before applying the change to prevent applying these
    // new params to a period where they were not valid yet. Interests rates are
    // recalculated after changes are applied.
    let should_update_interest_rates = (reserve_factor.is_some()
        && reserve_factor.unwrap() != market.reserve_factor)
        || interest_rate_model.is_some();

    let mut response = Response::new();

    if should_update_interest_rates {
        let config = CONFIG.load(deps.storage)?;
        let addresses = address_provider::helpers::query_contract_addrs(
            deps.as_ref(),
            &config.address_provider,
            vec![MarsAddressType::Incentives, MarsAddressType::RewardsCollector],
        )?;
        let rewards_collector_addr = &addresses[&MarsAddressType::RewardsCollector];
        let incentives_addr = &addresses[&MarsAddressType::Incentives];

        response = apply_accumulated_interests(
            deps.storage,
            &env,
            &mut market,
            rewards_collector_addr,
            incentives_addr,
            response,
        )?;
    }

    let mut updated_market = Market {
        reserve_factor: reserve_factor.unwrap_or(market.reserve_factor),
        interest_rate_model: interest_rate_model.unwrap_or(market.interest_rate_model),
        ..market
    };

    updated_market.validate()?;

    if should_update_interest_rates {
        response = update_interest_rates(&env, &mut updated_market, response)?;
    }
    MARKETS.save(deps.storage, &denom, &updated_market)?;

    Ok(response.add_attribute("action", "update_asset").add_attribute("denom", denom))
}

/// Asserts that the sender is authorized to update the parameters
pub fn assert_is_authorized(deps: &DepsMut, sender: &Addr, ap_addr: &Addr) -> ContractResult<()> {
    // Get the address of the contract responsible for managing parameters
    let params_addr = address_provider::helpers::query_contract_addr(
        deps.as_ref(),
        ap_addr,
        MarsAddressType::Params,
    )?;

    // Ensure that only the authorized params contract can update the parameters
    ensure_eq!(sender, &params_addr, ContractError::Mars(MarsError::Unauthorized {}));

    Ok(())
}
