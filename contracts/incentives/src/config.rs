use cosmwasm_std::{DepsMut, Env, Event, MessageInfo, Order, Response, StdResult};
use mars_owner::OwnerUpdate;
use mars_types::{
    address_provider,
    address_provider::MarsAddressType,
    incentives::WhitelistEntry,
    keys::{IncentiveId, IncentiveIdKey},
};
use mars_utils::helpers::{option_string_to_addr, validate_native_denom};

use crate::{
    helpers,
    helpers::update_incentive_index,
    state::{CONFIG, EMISSIONS, INCENTIVE_STATES, OWNER, WHITELIST, WHITELIST_COUNT},
    ContractError,
};

pub fn execute_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address_provider: Option<String>,
    max_whitelisted_denoms: Option<u8>,
) -> Result<Response, ContractError> {
    OWNER.assert_owner(deps.storage, &info.sender)?;

    let mut config = CONFIG.load(deps.storage)?;

    config.address_provider =
        option_string_to_addr(deps.api, address_provider, config.address_provider)?;

    if let Some(max_whitelisted_denoms) = max_whitelisted_denoms {
        config.max_whitelisted_denoms = max_whitelisted_denoms;
    }

    CONFIG.save(deps.storage, &config)?;

    let response = Response::new().add_attribute("action", "update_config");

    Ok(response)
}

pub fn update_owner(
    deps: DepsMut,
    info: MessageInfo,
    update: OwnerUpdate,
) -> Result<Response, ContractError> {
    Ok(OWNER.update(deps, info, update)?)
}

pub fn execute_update_whitelist(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    add_denoms: Vec<WhitelistEntry>,
    remove_denoms: Vec<String>,
) -> Result<Response, ContractError> {
    OWNER.assert_owner(deps.storage, &info.sender)?;

    let config = CONFIG.load(deps.storage)?;

    let addresses = address_provider::helpers::query_contract_addrs(
        deps.as_ref(),
        &config.address_provider,
        vec![MarsAddressType::RedBank, MarsAddressType::Perps],
    )?;
    let red_bank_addr = &addresses[&MarsAddressType::RedBank];
    let perps_addr = &addresses[&MarsAddressType::Perps];

    // Add add_denoms and remove_denoms to a set to check for duplicates
    let denoms = add_denoms.iter().map(|entry| &entry.denom).chain(remove_denoms.iter());
    let mut denoms_set = std::collections::HashSet::new();
    for denom in denoms {
        if !denoms_set.insert(denom) {
            return Err(ContractError::DuplicateDenom {
                denom: denom.clone(),
            });
        }
    }

    let prev_whitelist_count = WHITELIST_COUNT.may_load(deps.storage)?.unwrap_or_default();
    let mut whitelist_count = prev_whitelist_count;

    for denom in remove_denoms.iter() {
        // If denom is not on the whitelist, we can't remove it
        if !WHITELIST.has(deps.storage, denom) {
            return Err(ContractError::NotWhitelisted {
                denom: denom.clone(),
            });
        }

        whitelist_count -= 1;

        // Before removing from whitelist we must handle ongoing incentives,
        // i.e. update the incentive index, and remove any emissions.
        // So we first get all keys by in the INCENTIVE_STATES Map and then filter out the ones
        // that match the incentive denom we are removing.
        // This could be done more efficiently if we could prefix by incentive_denom, but
        // the map key is (collateral_denom, incentive_denom) so we can't, without introducing
        // another map, or using IndexedMap.
        let keys = INCENTIVE_STATES
            .keys(deps.storage, None, None, Order::Ascending)
            .filter(|res| {
                res.as_ref()
                    .map_or_else(|_| false, |(_, _, incentive_denom)| incentive_denom == denom)
            })
            .collect::<StdResult<Vec<_>>>()?;
        for (kind_key, collateral_denom, incentive_denom) in keys {
            let kind = kind_key.clone().try_into()?;
            let total_collateral = helpers::query_total_amount(
                &deps.querier,
                red_bank_addr,
                perps_addr,
                &kind,
                &collateral_denom,
            )?;

            update_incentive_index(
                &mut deps.branch().storage.into(),
                &kind,
                &collateral_denom,
                &incentive_denom,
                total_collateral,
                env.block.time.seconds(),
            )?;

            let incentive_id = IncentiveId::create(kind, collateral_denom);
            let incentive_id_key = IncentiveIdKey::try_from(incentive_id)?;

            // Remove any incentive emissions
            let emissions = EMISSIONS
                .prefix((&incentive_id_key, &incentive_denom))
                .range(deps.storage, None, None, Order::Ascending)
                .collect::<StdResult<Vec<_>>>()?;
            for (start_time, _) in emissions {
                EMISSIONS.remove(deps.storage, (&incentive_id_key, &incentive_denom, start_time));
            }
        }

        // Finally remove the incentive denom from the whitelist
        WHITELIST.remove(deps.storage, denom);
    }

    for entry in add_denoms.iter() {
        let WhitelistEntry {
            denom,
            min_emission_rate,
        } = entry;
        // If the denom is not already whitelisted, increase the counter and check that we are not
        // exceeding the max whitelist limit. If the denom is already whitelisted, we don't need
        // to change the counter and instead just update the min_emission.
        if !WHITELIST.has(deps.storage, denom) {
            whitelist_count += 1;
            if whitelist_count > config.max_whitelisted_denoms {
                return Err(ContractError::MaxWhitelistLimitReached {
                    max_whitelist_limit: config.max_whitelisted_denoms,
                });
            }
        }

        validate_native_denom(denom)?;
        WHITELIST.save(deps.storage, denom, min_emission_rate)?;
    }

    // Set the new whitelist count, if it has changed
    if whitelist_count != prev_whitelist_count {
        WHITELIST_COUNT.save(deps.storage, &whitelist_count)?;
    }

    let mut event = Event::new("mars/incentives/update_whitelist");
    event = event.add_attribute("add_denoms", format!("{:?}", add_denoms));
    event = event.add_attribute("remove_denoms", format!("{:?}", remove_denoms));

    Ok(Response::default().add_event(event))
}
