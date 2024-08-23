use cosmwasm_std::{
    attr, Addr, BankMsg, Coin, Coins, Decimal, DepsMut, Env, Event, MessageInfo, Order, Response,
    StdError, StdResult, Uint128,
};
use mars_types::{
    address_provider,
    address_provider::MarsAddressType,
    error::MarsError,
    incentives::{IncentiveKind, IncentiveState},
    keys::{IncentiveId, IncentiveIdKey, IncentiveKindKey, UserId, UserIdKey},
};
use mars_utils::helpers::validate_native_denom;

use crate::{
    helpers,
    helpers::{
        compute_user_accrued_rewards, compute_user_unclaimed_rewards, update_incentive_index,
    },
    query::query_address,
    state,
    state::{
        CONFIG, EMISSIONS, EPOCH_DURATION, INCENTIVE_STATES, USER_ASSET_INDICES,
        USER_UNCLAIMED_REWARDS, WHITELIST,
    },
    ContractError,
};

pub fn execute_claim_rewards(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    account_id: Option<String>,
    start_after_kind: Option<IncentiveKind>,
    start_after_denom: Option<String>,
    start_after_incentive_denom: Option<String>,
    limit: Option<u32>,
) -> Result<Response, ContractError> {
    let user_addr = info.sender;
    let acc_id = account_id.clone().unwrap_or("".to_string());
    let user_id = UserId::credit_manager(user_addr.clone(), acc_id.clone());
    let user_id_key: UserIdKey = user_id.try_into()?;

    let mut response = Response::new();
    let base_event = Event::new("mars/incentives/claim_rewards")
        .add_attribute("action", "claim_rewards")
        .add_attribute("user", user_addr.to_string());
    let base_event = if account_id.is_some() {
        base_event.add_attribute("account_id", &acc_id)
    } else {
        base_event
    };
    response = response.add_event(base_event);

    let asset_incentives = state::paginate_incentive_states(
        deps.storage,
        start_after_kind,
        start_after_denom,
        start_after_incentive_denom,
        limit,
    )?;

    let config = CONFIG.load(deps.storage)?;
    let mut total_unclaimed_rewards = Coins::default();

    let addresses = address_provider::helpers::query_contract_addrs(
        deps.as_ref(),
        &config.address_provider,
        vec![MarsAddressType::RedBank, MarsAddressType::Perps],
    )?;
    let red_bank_addr = &addresses[&MarsAddressType::RedBank];
    let perps_addr = &addresses[&MarsAddressType::Perps];

    for ((kind_key, denom, incentive_denom), _) in asset_incentives {
        let querier = deps.querier;
        let unclaimed_rewards = compute_user_unclaimed_rewards(
            &mut deps.branch().storage.into(),
            &querier,
            &env.block,
            red_bank_addr,
            perps_addr,
            &user_addr,
            &account_id,
            &kind_key.clone().try_into()?,
            &denom,
            &incentive_denom,
        )?;

        let incentive_id = IncentiveId::create(kind_key.try_into()?, denom);
        let incentive_id_key = IncentiveIdKey::try_from(incentive_id)?;

        // clear unclaimed rewards
        USER_UNCLAIMED_REWARDS.save(
            deps.storage,
            (&user_id_key, &incentive_id_key, &incentive_denom),
            &Uint128::zero(),
        )?;

        total_unclaimed_rewards.add(Coin {
            denom: incentive_denom,
            amount: unclaimed_rewards,
        })?;
    }

    if !total_unclaimed_rewards.is_empty() {
        response = response
            .add_event(
                Event::new("mars/incentives/claim_rewards/claimed_rewards")
                    .add_attribute("coins", total_unclaimed_rewards.to_string()),
            )
            .add_message(BankMsg::Send {
                to_address: user_addr.into(),
                amount: total_unclaimed_rewards.into(),
            });
    }

    Ok(response)
}

pub fn execute_set_asset_incentive(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    kind: &IncentiveKind,
    denom: String,
    incentive_denom: String,
    emission_per_second: Uint128,
    start_time: u64,
    duration: u64,
) -> Result<Response, ContractError> {
    validate_native_denom(&denom)?;
    validate_native_denom(&incentive_denom)?;

    // Check that the incentive denom is whitelisted
    if !WHITELIST.key(&incentive_denom).has(deps.storage) {
        return Err(ContractError::NotWhitelisted {
            denom: incentive_denom,
        });
    }

    let config = CONFIG.load(deps.storage)?;
    let epoch_duration = EPOCH_DURATION.load(deps.storage)?;
    let current_time = env.block.time.seconds();

    // Validate incentive schedule
    helpers::validate_incentive_schedule(
        deps.storage,
        &info,
        epoch_duration,
        current_time,
        kind,
        &denom,
        &incentive_denom,
        emission_per_second,
        start_time,
        duration,
    )?;

    let addresses = address_provider::helpers::query_contract_addrs(
        deps.as_ref(),
        &config.address_provider,
        vec![MarsAddressType::RedBank, MarsAddressType::Perps],
    )?;
    let red_bank_addr = &addresses[&MarsAddressType::RedBank];
    let perps_addr = &addresses[&MarsAddressType::Perps];

    let total_amount =
        helpers::query_total_amount(&deps.querier, red_bank_addr, perps_addr, kind, &denom)?;

    update_incentive_index(
        &mut deps.branch().storage.into(),
        kind,
        &denom,
        &incentive_denom,
        total_amount,
        current_time,
    )?;

    let kind_key = IncentiveKindKey::try_from(kind)?;

    let incentive_id = IncentiveId::create(kind.clone(), denom.clone());
    let incentive_id_key = IncentiveIdKey::try_from(incentive_id)?;

    // To simplify the logic and prevent too much gas usage, we split the new schedule into separate
    // schedules that are exactly one epoch long. This way we can easily merge them with existing
    // schedules.
    // Loop over each epoch duration of the new schedule and merge into any existing schedules
    let mut epoch_start_time = start_time;
    while epoch_start_time < start_time + duration {
        // Check if an schedule exists for the current epoch. If it does, merge the new schedule
        // with the existing schedule. Else add a new schedule.
        let key = (&incentive_id_key, incentive_denom.as_str(), epoch_start_time);
        let existing_schedule = EMISSIONS.may_load(deps.storage, key)?;
        if let Some(existing_schedule) = existing_schedule {
            EMISSIONS.save(deps.storage, key, &(existing_schedule + emission_per_second))?;
        } else {
            EMISSIONS.save(deps.storage, key, &emission_per_second)?;
        }

        epoch_start_time += epoch_duration;
    }

    // Set up the incentive state if it doesn't exist
    INCENTIVE_STATES.update(deps.storage, (&kind_key, &denom, &incentive_denom), |old| {
        Ok::<_, StdError>(old.unwrap_or_else(|| IncentiveState {
            index: Decimal::zero(),
            last_updated: current_time,
        }))
    })?;

    let response = Response::new().add_attributes(vec![
        attr("action", "set_asset_incentive"),
        attr("denom", denom),
        attr("incentive_denom", incentive_denom),
        attr("emission_per_second", emission_per_second),
        attr("start_time", start_time.to_string()),
        attr("duration", duration.to_string()),
    ]);
    Ok(response)
}

pub fn execute_balance_change(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    user_addr: Addr,
    account_id: Option<String>,
    kind: IncentiveKind,
    denom: String,
    user_amount: Uint128,
    total_amount: Uint128,
) -> Result<Response, ContractError> {
    // this method can only be invoked by the correct contract
    let address_type = kind.get_address_type();
    let addr = query_address(deps.as_ref(), address_type)?;
    if info.sender != addr {
        return Err(MarsError::Unauthorized {}.into());
    }

    let acc_id = account_id.clone().unwrap_or("".to_string());

    let user_id = UserId::credit_manager(user_addr.clone(), acc_id.clone());
    let user_id_key: UserIdKey = user_id.try_into()?;

    let base_event = Event::new("mars/incentives/balance_change")
        .add_attribute("action", "balance_change")
        .add_attribute("kind", kind.to_string())
        .add_attribute("denom", denom.clone())
        .add_attribute("user", user_addr.to_string());
    let base_event = if account_id.is_some() {
        base_event.add_attribute("account_id", &acc_id)
    } else {
        base_event
    };

    let mut events = vec![base_event];

    let kind_key = IncentiveKindKey::try_from(&kind)?;

    let incentive_id = IncentiveId::create(kind.clone(), denom.clone());
    let incentive_id_key = IncentiveIdKey::try_from(incentive_id)?;

    let incentive_states = INCENTIVE_STATES
        .prefix((&kind_key, &denom))
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    for (incentive_denom, _) in incentive_states {
        let incentive_state = update_incentive_index(
            &mut deps.branch().storage.into(),
            &kind,
            &denom,
            &incentive_denom,
            total_amount,
            env.block.time.seconds(),
        )?;

        // Check if user has accumulated uncomputed rewards (which means index is not up to date)
        let user_asset_index_key =
            USER_ASSET_INDICES.key((&user_id_key.clone(), &incentive_id_key, &incentive_denom));

        let user_asset_index =
            user_asset_index_key.may_load(deps.storage)?.unwrap_or_else(Decimal::zero);

        let mut accrued_rewards = Uint128::zero();

        if user_asset_index != incentive_state.index {
            // Compute user accrued rewards and update state
            accrued_rewards =
                compute_user_accrued_rewards(user_amount, user_asset_index, incentive_state.index)?;

            // Store user accrued rewards as unclaimed
            if !accrued_rewards.is_zero() {
                state::increase_unclaimed_rewards(
                    deps.storage,
                    &user_addr,
                    &acc_id,
                    &kind,
                    &denom,
                    &incentive_denom,
                    accrued_rewards,
                )?;
            }

            user_asset_index_key.save(deps.storage, &incentive_state.index)?;
        }

        events.push(
            Event::new("mars/incentives/balance_change/reward_accrued")
                .add_attribute("incentive_denom", incentive_denom)
                .add_attribute("rewards_accrued", accrued_rewards)
                .add_attribute("asset_index", incentive_state.index.to_string()),
        );
    }

    Ok(Response::new().add_events(events))
}
