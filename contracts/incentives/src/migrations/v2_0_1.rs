use cosmwasm_std::{DepsMut, Empty, Env, MessageInfo, Order, Response, StdResult};
use cw2::{assert_contract_version, set_contract_version};
use cw_storage_plus::Bound;
use mars_types::{
    incentives::{IncentiveKind, MigrateV2ToV2_0_1},
    keys::{IncentiveId, IncentiveIdKey, IncentiveKindKey, UserId, UserIdKey},
};

use crate::{
    contract::{CONTRACT_NAME, CONTRACT_VERSION},
    error::ContractError,
    state::{
        EMISSIONS, INCENTIVE_STATES, MIGRATION_GUARD, OWNER, USER_ASSET_INDICES,
        USER_UNCLAIMED_REWARDS,
    },
};

const FROM_VERSION: &str = "2.0.0";

pub mod v2_state {
    use cosmwasm_std::{Decimal, Uint128};
    use cw_storage_plus::Map;
    use mars_types::{incentives::IncentiveState, keys::UserIdKey};

    pub const INCENTIVE_STATES: Map<(&str, &str), IncentiveState> = Map::new("incentive_states");
    pub const EMISSIONS: Map<(&str, &str, u64), Uint128> = Map::new("emissions");
    pub const USER_ASSET_INDICES: Map<(&UserIdKey, &str, &str), Decimal> = Map::new("indices_v2");
    pub const USER_UNCLAIMED_REWARDS: Map<(&UserIdKey, &str, &str), Uint128> =
        Map::new("unclaimed_rewards_v2");
}

pub fn migrate(mut deps: DepsMut, _env: Env, _msg: Empty) -> Result<Response, ContractError> {
    // Lock incentives to prevent any operations during migration.
    // Unlock is executed after full migration in `migrate_users_indexes_and_rewards`.
    MIGRATION_GUARD.try_lock(deps.storage)?;

    // make sure we're migrating the correct contract and from the correct version
    assert_contract_version(deps.storage, &format!("crates.io:{CONTRACT_NAME}"), FROM_VERSION)?;

    // Migrate the states that are not user bound
    migrate_incentive_states(&mut deps)?;
    migrate_emissions(&mut deps)?;

    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("from_version", FROM_VERSION)
        .add_attribute("to_version", CONTRACT_VERSION))
}

fn migrate_incentive_states(deps: &mut DepsMut) -> Result<(), ContractError> {
    let incentive_states = v2_state::INCENTIVE_STATES
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    let kind_key = IncentiveKindKey::try_from(&IncentiveKind::RedBank)?;

    for ((col_denom, incentive_denom), incentive_state) in incentive_states.into_iter() {
        INCENTIVE_STATES.save(
            deps.storage,
            (&kind_key, &col_denom, &incentive_denom),
            &incentive_state,
        )?;
    }

    Ok(())
}

fn migrate_emissions(deps: &mut DepsMut) -> Result<(), ContractError> {
    let emissions = v2_state::EMISSIONS
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    for ((col_denom, incentive_denom, start), emission) in emissions.into_iter() {
        let incentive_id = IncentiveId::create(IncentiveKind::RedBank, col_denom);
        let incentive_id_key = IncentiveIdKey::try_from(incentive_id)?;
        EMISSIONS.save(deps.storage, (&incentive_id_key, &incentive_denom, start), &emission)?;
    }

    Ok(())
}

pub fn execute_migration(
    deps: DepsMut,
    info: MessageInfo,
    msg: MigrateV2ToV2_0_1,
) -> Result<Response, ContractError> {
    match msg {
        MigrateV2ToV2_0_1::UserUnclaimedRewards {
            limit,
        } => migrate_user_unclaimed_rewards(deps, limit as usize),
        MigrateV2ToV2_0_1::UserAssetIndices {
            limit,
        } => migrate_user_asset_indices(deps, limit as usize),
        MigrateV2ToV2_0_1::ClearV2State {} => {
            OWNER.assert_owner(deps.storage, &info.sender)?;
            clear_v2_state(deps)
        }
    }
}

fn migrate_user_unclaimed_rewards(deps: DepsMut, limit: usize) -> Result<Response, ContractError> {
    // Only allow to migrate users unclaimed rewards if guard is locked via `migrate` entrypoint
    MIGRATION_GUARD.assert_locked(deps.storage)?;
    // convert last key from v2_0_1 to v2
    let last_key = USER_UNCLAIMED_REWARDS.last(deps.storage)?.map(|kv| kv.0);
    let last_key = if let Some((user_id_key, incentive_id_key, incentive_denom)) = last_key {
        let incentive_id: IncentiveId = incentive_id_key.try_into()?;
        Some((user_id_key, incentive_id.collateral_denom, incentive_denom))
    } else {
        None
    };

    // last key from new user asset indices is first key (excluded) for v2 during pagination
    let start_after = last_key.as_ref().map(|(key, col_denom, incentive_denom)| {
        Bound::exclusive((key, col_denom.as_str(), incentive_denom.as_str()))
    });
    let mut unclaimed_rewards_v2 = v2_state::USER_UNCLAIMED_REWARDS
        .range(deps.storage, start_after, None, Order::Ascending)
        .take(limit + 1)
        .collect::<StdResult<Vec<_>>>()?;
    let has_more = unclaimed_rewards_v2.len() > limit;
    if has_more {
        unclaimed_rewards_v2.pop(); // Remove the extra item used for checking if there are more items
    }

    // save user unclaimed rewards
    for ((user_id_key, col_denom, incentive_denom), amount) in unclaimed_rewards_v2.into_iter() {
        let incentive_id = IncentiveId::create(IncentiveKind::RedBank, col_denom);
        let incentive_id_key = IncentiveIdKey::try_from(incentive_id)?;
        USER_UNCLAIMED_REWARDS.save(
            deps.storage,
            (&user_id_key, &incentive_id_key, &incentive_denom),
            &amount,
        )?;
    }

    if !has_more {
        // incentives locked via `migrate` entrypoint. Unlock incentives after full migration
        MIGRATION_GUARD.try_unlock(deps.storage)?;
    }

    Ok(Response::new()
        .add_attribute("action", "migrate_user_unclaimed_rewards")
        .add_attribute(
            "result",
            if has_more {
                "in_progress"
            } else {
                "done"
            },
        )
        .add_attribute("start_after", key_to_str(last_key))
        .add_attribute("limit", limit.to_string())
        .add_attribute("has_more", has_more.to_string()))
}

fn migrate_user_asset_indices(deps: DepsMut, limit: usize) -> Result<Response, ContractError> {
    // Only allow to migrate users asset indices if guard is locked via `migrate` entrypoint
    MIGRATION_GUARD.assert_locked(deps.storage)?;

    // convert last key from v2_0_1 to v2
    let last_key = USER_ASSET_INDICES.last(deps.storage)?.map(|kv| kv.0);
    let last_key = if let Some((user_id_key, kind_col, incentive_denom)) = last_key {
        let incentive_id: IncentiveId = kind_col.try_into()?;
        Some((user_id_key, incentive_id.collateral_denom, incentive_denom))
    } else {
        None
    };

    // last key from new user asset indices is first key (excluded) for v2 during pagination
    let start_after = last_key.as_ref().map(|(user_id_key, col_denom, incentive_denom)| {
        Bound::exclusive((user_id_key, col_denom.as_str(), incentive_denom.as_str()))
    });
    let mut unclaimed_rewards_v2 = v2_state::USER_ASSET_INDICES
        .range(deps.storage, start_after, None, Order::Ascending)
        .take(limit + 1)
        .collect::<StdResult<Vec<_>>>()?;

    let has_more = unclaimed_rewards_v2.len() > limit;
    if has_more {
        unclaimed_rewards_v2.pop(); // Remove the extra item used for checking if there are more items
    }

    // save user asset indexes
    for ((user_id_key, col_denom, incentive_denom), user_asset_index) in
        unclaimed_rewards_v2.into_iter()
    {
        let incentive_id = IncentiveId::create(IncentiveKind::RedBank, col_denom);
        let incentive_id_key = IncentiveIdKey::try_from(incentive_id)?;
        USER_ASSET_INDICES.save(
            deps.storage,
            (&user_id_key, &incentive_id_key, &incentive_denom),
            &user_asset_index,
        )?;
    }

    if !has_more {
        // incentives locked via `migrate` entrypoint. Unlock incentives after full migration
        MIGRATION_GUARD.try_unlock(deps.storage)?;
    }

    Ok(Response::new()
        .add_attribute("action", "migrate_user_asset_indices")
        .add_attribute(
            "result",
            if has_more {
                "in_progress"
            } else {
                "done"
            },
        )
        .add_attribute("start_after", key_to_str(last_key))
        .add_attribute("limit", limit.to_string())
        .add_attribute("has_more", has_more.to_string()))
}

fn key_to_str(key: Option<(UserIdKey, String, String)>) -> String {
    key.map(|(user_id_key, col_denom, incentive_denom)| {
        let user_id: UserId = user_id_key.try_into().unwrap();
        format!("{}-{}-{}-{}", user_id.addr, user_id.acc_id, col_denom, incentive_denom)
    })
    .unwrap_or("none".to_string())
}

fn clear_v2_state(deps: DepsMut) -> Result<Response, ContractError> {
    // It is safe to clear v2 state only after full migration (guard is unlocked)
    MIGRATION_GUARD.assert_unlocked(deps.storage)?;
    v2_state::INCENTIVE_STATES.clear(deps.storage);
    v2_state::EMISSIONS.clear(deps.storage);
    v2_state::USER_ASSET_INDICES.clear(deps.storage);
    v2_state::USER_UNCLAIMED_REWARDS.clear(deps.storage);
    Ok(Response::new().add_attribute("action", "clear_v2_state"))
}

#[cfg(test)]
pub mod tests {
    use cosmwasm_std::{attr, testing::mock_dependencies, Addr, Decimal, Uint128};
    use mars_types::incentives::IncentiveState;
    use mars_utils::error::GuardError;

    use super::*;
    #[test]
    fn cannot_migrate_without_lock() {
        let mut deps = mock_dependencies();

        let res_error = migrate_user_unclaimed_rewards(deps.as_mut(), 10).unwrap_err();
        assert_eq!(res_error, ContractError::Guard(GuardError::Inactive {}));

        let res_error = migrate_user_asset_indices(deps.as_mut(), 10).unwrap_err();
        assert_eq!(res_error, ContractError::Guard(GuardError::Inactive {}));
    }

    #[test]
    fn empty_v2_unclaimed_rewards() {
        let mut deps = mock_dependencies();

        MIGRATION_GUARD.try_lock(deps.as_mut().storage).unwrap();

        let res = migrate_user_unclaimed_rewards(deps.as_mut(), 10).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "migrate_user_unclaimed_rewards"),
                attr("result", "done"),
                attr("start_after", "none"),
                attr("limit", "10"),
                attr("has_more", "false"),
            ]
        );
    }

    #[test]
    fn empty_v2_user_asset_indices() {
        let mut deps = mock_dependencies();

        MIGRATION_GUARD.try_lock(deps.as_mut().storage).unwrap();

        let res = migrate_user_asset_indices(deps.as_mut(), 10).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "migrate_user_asset_indices"),
                attr("result", "done"),
                attr("start_after", "none"),
                attr("limit", "10"),
                attr("has_more", "false"),
            ]
        );
    }

    #[test]
    fn migrate_v2_user_unclaimed_rewards() {
        let mut deps = mock_dependencies();

        MIGRATION_GUARD.try_lock(deps.as_mut().storage).unwrap();

        // Prepare the unclaimed rewards V2 state
        let acc_1: UserIdKey =
            UserId::credit_manager(Addr::unchecked("user1"), "1".to_string()).try_into().unwrap();
        let acc_2: UserIdKey =
            UserId::credit_manager(Addr::unchecked("user1"), "2".to_string()).try_into().unwrap();
        let acc_3: UserIdKey =
            UserId::credit_manager(Addr::unchecked("user2"), "1".to_string()).try_into().unwrap();
        let acc_4: UserIdKey =
            UserId::credit_manager(Addr::unchecked("user3"), "1".to_string()).try_into().unwrap();

        v2_state::USER_UNCLAIMED_REWARDS
            .save(deps.as_mut().storage, (&acc_1, "umars", "untrn"), &Uint128::from(133u128))
            .unwrap();
        v2_state::USER_UNCLAIMED_REWARDS
            .save(deps.as_mut().storage, (&acc_1, "umars", "uusdc"), &Uint128::from(133u128))
            .unwrap();
        v2_state::USER_UNCLAIMED_REWARDS
            .save(deps.as_mut().storage, (&acc_2, "umars", "uusdc"), &Uint128::from(133u128))
            .unwrap();
        v2_state::USER_UNCLAIMED_REWARDS
            .save(deps.as_mut().storage, (&acc_3, "umars", "uusdc"), &Uint128::from(133u128))
            .unwrap();
        v2_state::USER_UNCLAIMED_REWARDS
            .save(deps.as_mut().storage, (&acc_3, "untrn", "umars"), &Uint128::from(133u128))
            .unwrap();
        v2_state::USER_UNCLAIMED_REWARDS
            .save(deps.as_mut().storage, (&acc_4, "untrn", "uusdc"), &Uint128::from(133u128))
            .unwrap();

        // Migrate first two
        let res = migrate_user_unclaimed_rewards(deps.as_mut(), 2).unwrap();

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "migrate_user_unclaimed_rewards"),
                attr("result", "in_progress"),
                attr("start_after", "none"),
                attr("limit", "2"),
                attr("has_more", "true"),
            ]
        );

        // in new debts we should have 2 debts
        let unclaimed_rewards = USER_UNCLAIMED_REWARDS
            .range(deps.as_ref().storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();
        assert_eq!(unclaimed_rewards.len(), 2);

        // Migrate the next two
        let res = migrate_user_unclaimed_rewards(deps.as_mut(), 2).unwrap();

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "migrate_user_unclaimed_rewards"),
                attr("result", "in_progress"),
                attr("start_after", "user1-1-umars-uusdc"),
                attr("limit", "2"),
                attr("has_more", "true"),
            ]
        );

        // in new debts we should have 2 debts
        let unclaimed_rewards = USER_UNCLAIMED_REWARDS
            .range(deps.as_ref().storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();
        assert_eq!(unclaimed_rewards.len(), 4);

        // Migrate the rest
        let res = migrate_user_unclaimed_rewards(deps.as_mut(), 10).unwrap();

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "migrate_user_unclaimed_rewards"),
                attr("result", "done"),
                attr("start_after", "user2-1-umars-uusdc"),
                attr("limit", "10"),
                attr("has_more", "false"),
            ]
        );

        // in new debts we should have 2 debts
        let unclaimed_rewards = USER_UNCLAIMED_REWARDS
            .range(deps.as_ref().storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();
        assert_eq!(unclaimed_rewards.len(), 6);
    }

    #[test]
    fn migrate_v2_user_asset_indices() {
        let mut deps = mock_dependencies();

        MIGRATION_GUARD.try_lock(deps.as_mut().storage).unwrap();

        // Prepare the unclaimed rewards V2 state
        let acc_1: UserIdKey =
            UserId::credit_manager(Addr::unchecked("user1"), "1".to_string()).try_into().unwrap();
        let acc_2: UserIdKey =
            UserId::credit_manager(Addr::unchecked("user1"), "2".to_string()).try_into().unwrap();
        let acc_3: UserIdKey =
            UserId::credit_manager(Addr::unchecked("user2"), "1".to_string()).try_into().unwrap();
        let acc_4: UserIdKey =
            UserId::credit_manager(Addr::unchecked("user3"), "1".to_string()).try_into().unwrap();

        v2_state::USER_ASSET_INDICES
            .save(
                deps.as_mut().storage,
                (&acc_1, "umars", "untrn"),
                &Decimal::from_ratio(1u128, 2u128),
            )
            .unwrap();
        v2_state::USER_ASSET_INDICES
            .save(
                deps.as_mut().storage,
                (&acc_1, "umars", "untrn"),
                &Decimal::from_ratio(1u128, 2u128),
            )
            .unwrap();
        v2_state::USER_ASSET_INDICES
            .save(
                deps.as_mut().storage,
                (&acc_1, "umars", "uusdc"),
                &Decimal::from_ratio(1u128, 2u128),
            )
            .unwrap();
        v2_state::USER_ASSET_INDICES
            .save(
                deps.as_mut().storage,
                (&acc_2, "umars", "uusdc"),
                &Decimal::from_ratio(1u128, 2u128),
            )
            .unwrap();
        v2_state::USER_ASSET_INDICES
            .save(
                deps.as_mut().storage,
                (&acc_3, "umars", "uusdc"),
                &Decimal::from_ratio(1u128, 2u128),
            )
            .unwrap();
        v2_state::USER_ASSET_INDICES
            .save(
                deps.as_mut().storage,
                (&acc_3, "untrn", "umars"),
                &Decimal::from_ratio(1u128, 2u128),
            )
            .unwrap();
        v2_state::USER_ASSET_INDICES
            .save(
                deps.as_mut().storage,
                (&acc_4, "untrn", "uusdc"),
                &Decimal::from_ratio(1u128, 2u128),
            )
            .unwrap();

        // Migrate the first 2
        let res = migrate_user_asset_indices(deps.as_mut(), 2).unwrap();

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "migrate_user_asset_indices"),
                attr("result", "in_progress"),
                attr("start_after", "none"),
                attr("limit", "2"),
                attr("has_more", "true"),
            ]
        );

        // in new indices there should be 2 items
        let user_asset_indices = USER_ASSET_INDICES
            .range(&deps.storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();
        assert_eq!(user_asset_indices.len(), 2);

        // Migrate the next 2
        let res = migrate_user_asset_indices(deps.as_mut(), 2).unwrap();

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "migrate_user_asset_indices"),
                attr("result", "in_progress"),
                attr("start_after", "user1-1-umars-uusdc"),
                attr("limit", "2"),
                attr("has_more", "true"),
            ]
        );

        // in new indices there should be 2 items
        let user_asset_indices = USER_ASSET_INDICES
            .range(&deps.storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();
        assert_eq!(user_asset_indices.len(), 4);

        // Migrate the rest
        let res = migrate_user_asset_indices(deps.as_mut(), 10).unwrap();

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "migrate_user_asset_indices"),
                attr("result", "done"),
                attr("start_after", "user2-1-umars-uusdc"),
                attr("limit", "10"),
                attr("has_more", "false"),
            ]
        );

        // in new indices there should be 2 items
        let user_asset_indices = USER_ASSET_INDICES
            .range(&deps.storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();
        assert_eq!(user_asset_indices.len(), 6);
    }

    #[test]
    fn migrate_v2_emissions() {
        let mut deps = mock_dependencies();

        MIGRATION_GUARD.try_lock(deps.as_mut().storage).unwrap();

        v2_state::EMISSIONS
            .save(deps.as_mut().storage, ("umars", "utia", 1u64), &Uint128::from(33u128))
            .unwrap();
        v2_state::EMISSIONS
            .save(deps.as_mut().storage, ("umars", "untrn", 1u64), &Uint128::from(91u128))
            .unwrap();
        v2_state::EMISSIONS
            .save(deps.as_mut().storage, ("untrn", "uusdc", 1u64), &Uint128::from(12u128))
            .unwrap();
        v2_state::EMISSIONS
            .save(deps.as_mut().storage, ("untrn", "umars", 1u64), &Uint128::from(22u128))
            .unwrap();
        v2_state::EMISSIONS
            .save(deps.as_mut().storage, ("uusdc", "umars", 1u64), &Uint128::from(90u128))
            .unwrap();
        v2_state::EMISSIONS
            .save(deps.as_mut().storage, ("untrn", "utia", 1u64), &Uint128::from(67u128))
            .unwrap();

        migrate_emissions(&mut deps.as_mut()).unwrap();

        let user_asset_indices = EMISSIONS
            .range(&deps.storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();
        assert_eq!(user_asset_indices.len(), 6);

        let incentive_id_umars = IncentiveId::create(IncentiveKind::RedBank, "umars".to_string());
        let incentive_id_key_umars = IncentiveIdKey::try_from(incentive_id_umars).unwrap();
        let incentive_id_uusdc = IncentiveId::create(IncentiveKind::RedBank, "uusdc".to_string());
        let incentive_id_key_uusdc = IncentiveIdKey::try_from(incentive_id_uusdc).unwrap();
        let incentive_id_untrn = IncentiveId::create(IncentiveKind::RedBank, "untrn".to_string());
        let incentive_id_key_untrn = IncentiveIdKey::try_from(incentive_id_untrn).unwrap();

        assert_eq!(
            user_asset_indices[0],
            ((incentive_id_key_umars.clone(), "utia".to_string(), 1u64), Uint128::from(33u128))
        );
        assert_eq!(
            user_asset_indices[1],
            ((incentive_id_key_umars.clone(), "untrn".to_string(), 1u64), Uint128::from(91u128))
        );
        assert_eq!(
            user_asset_indices[2],
            ((incentive_id_key_untrn.clone(), "utia".to_string(), 1u64), Uint128::from(67u128))
        );
        assert_eq!(
            user_asset_indices[3],
            ((incentive_id_key_untrn.clone(), "umars".to_string(), 1u64), Uint128::from(22u128))
        );
        assert_eq!(
            user_asset_indices[4],
            ((incentive_id_key_untrn.clone(), "uusdc".to_string(), 1u64), Uint128::from(12u128))
        );
        assert_eq!(
            user_asset_indices[5],
            ((incentive_id_key_uusdc.clone(), "umars".to_string(), 1u64), Uint128::from(90u128))
        );
    }

    #[test]
    fn migrate_v2_incentive_states() {
        let mut deps = mock_dependencies();

        MIGRATION_GUARD.try_lock(deps.as_mut().storage).unwrap();

        let asset_incentive = IncentiveState {
            index: Decimal::zero(),
            last_updated: 0,
        };
        let incentives = vec![
            (("collat1".to_string(), "incen1".to_string()), asset_incentive.clone()),
            (("collat1".to_string(), "incen2".to_string()), asset_incentive.clone()),
            (("collat2".to_string(), "incen1".to_string()), asset_incentive.clone()),
            (("collat2".to_string(), "incen2".to_string()), asset_incentive.clone()),
        ];

        for ((collat, incen), incentive) in incentives.iter() {
            v2_state::INCENTIVE_STATES
                .save(deps.as_mut().storage, (collat.as_str(), incen.as_str()), incentive)
                .unwrap();
        }

        let rb_key = IncentiveKindKey::try_from(&IncentiveKind::RedBank).unwrap();

        migrate_incentive_states(&mut deps.as_mut()).unwrap();

        let incentive_states = INCENTIVE_STATES
            .range(&deps.storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();
        assert_eq!(incentive_states.len(), 4);

        assert_eq!(
            incentive_states[0],
            (
                (rb_key.clone(), "collat1".to_string(), "incen1".to_string()),
                asset_incentive.clone()
            )
        );
        assert_eq!(
            incentive_states[1],
            (
                (rb_key.clone(), "collat1".to_string(), "incen2".to_string()),
                asset_incentive.clone()
            )
        );
        assert_eq!(
            incentive_states[2],
            (
                (rb_key.clone(), "collat2".to_string(), "incen1".to_string()),
                asset_incentive.clone()
            )
        );
        assert_eq!(
            incentive_states[3],
            (
                (rb_key.clone(), "collat2".to_string(), "incen2".to_string()),
                asset_incentive.clone()
            )
        );
    }
}
