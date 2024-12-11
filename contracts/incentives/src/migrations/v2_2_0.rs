use cosmwasm_std::{DepsMut, Empty, Env, MessageInfo, Order, Response, StdResult};
use cw2::{assert_contract_version, set_contract_version};
use cw_storage_plus::Bound;
use mars_types::{
    incentives::{IncentiveKind, MigrateV2_1_0ToV2_2_0},
    keys::{IncentiveId, IncentiveIdKey, IncentiveKindKey, UserId, UserIdKey},
};

use crate::{
    contract::{CONTRACT_NAME, CONTRACT_VERSION},
    error::ContractError,
    state::{
        ASTRO_USER_LP_DEPOSITS, EMISSIONS, INCENTIVE_STATES, MIGRATION_GUARD, OWNER,
        USER_ASSET_INDICES, USER_ASTRO_INCENTIVE_STATES, USER_UNCLAIMED_REWARDS,
    },
};

const FROM_VERSION: &str = "2.1.0";

pub mod v1_state {
    use cosmwasm_std::{Addr, Decimal, DepsMut, Uint128};
    use cw_storage_plus::Map;

    /// Don't care about the actual types, just use some dummy types to clear the storage
    pub const ASSET_INCENTIVES: Map<&str, String> = Map::new("incentives");
    pub const USER_ASSET_INDICES: Map<(&Addr, &str), Decimal> = Map::new("indices");
    pub const USER_UNCLAIMED_REWARDS: Map<&Addr, Uint128> = Map::new("unclaimed_rewards");
    pub const USER_UNCLAIMED_REWARDS_BACKUP: Map<&Addr, Uint128> = Map::new("ur_backup");

    /// Clear old state so we can re-use the keys
    pub fn clear_state(deps: &mut DepsMut) {
        ASSET_INCENTIVES.clear(deps.storage);
        USER_ASSET_INDICES.clear(deps.storage);
        USER_UNCLAIMED_REWARDS.clear(deps.storage);
        USER_UNCLAIMED_REWARDS_BACKUP.clear(deps.storage);
    }
}

pub mod v2_state {
    use cosmwasm_std::{Decimal, Uint128};
    use cw_storage_plus::Map;
    use mars_types::{incentives::IncentiveState, keys::UserIdKey};

    pub const INCENTIVE_STATES: Map<(&str, &str), IncentiveState> = Map::new("incentive_states");
    pub const EMISSIONS: Map<(&str, &str, u64), Uint128> = Map::new("emissions");
    pub const USER_ASSET_INDICES: Map<(&UserIdKey, &str, &str), Decimal> = Map::new("indices_v2");
    pub const USER_UNCLAIMED_REWARDS: Map<(&UserIdKey, &str, &str), Uint128> =
        Map::new("unclaimed_rewards_v2");

    // Map of User Lp deposits. Key is (user_id, lp_denom)
    pub const ASTRO_USER_LP_DEPOSITS: Map<(&str, &str), Uint128> = Map::new("lp_deposits");

    /// A map containing the individual incentive index for each unique user
    /// Note - this may contain many denoms for one user
    /// The key is (account_id, lp_token_denom, reward_denom)
    pub const USER_ASTRO_INCENTIVE_STATES: Map<(&str, &str, &str), Decimal> =
        Map::new("user_astroport_incentive_states");
}

pub fn migrate(mut deps: DepsMut, _env: Env, _msg: Empty) -> Result<Response, ContractError> {
    // Lock incentives to prevent any operations during migration.
    // Unlock is executed after full migration in `migrate_users_indexes_and_rewards`.
    MIGRATION_GUARD.try_lock(deps.storage)?;

    // Clear old state
    v1_state::clear_state(&mut deps);

    // make sure we're migrating the correct contract and from the correct version
    assert_contract_version(deps.storage, &format!("crates.io:{CONTRACT_NAME}"), FROM_VERSION)?;

    // Migrate the states that are not user bound
    migrate_incentive_states(&mut deps)?;
    migrate_emissions(&mut deps)?;

    // Clear all zero balances in astro incentives
    clear_zero_amounts_in_staked_astro_lp(&mut deps)?;
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
    msg: MigrateV2_1_0ToV2_2_0,
) -> Result<Response, ContractError> {
    match msg {
        MigrateV2_1_0ToV2_2_0::UserUnclaimedRewards {
            limit,
        } => migrate_user_unclaimed_rewards(deps, limit as usize),
        MigrateV2_1_0ToV2_2_0::UserAssetIndices {
            limit,
        } => migrate_user_asset_indices(deps, limit as usize),
        MigrateV2_1_0ToV2_2_0::ClearV2State {} => {
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

/// Clear all zero amounts in staked astro LP positions
/// This will delete all incentive states for (account_id, lp_denom) keys where the associated
/// staked amount is zero.
fn clear_zero_amounts_in_staked_astro_lp(deps: &mut DepsMut) -> Result<(), ContractError> {
    // Collect all LP positions that are zero
    let zero_balance_items = ASTRO_USER_LP_DEPOSITS
        .range(deps.storage, None, None, Order::Ascending)
        .filter_map(|item| match item {
            Ok(((user_id, account), value)) if value.is_zero() => {
                Some(Ok(((user_id.to_string(), account.to_string()), value)))
            }
            Ok(_) => None,
            Err(e) => Some(Err(e)),
        })
        .collect::<StdResult<Vec<_>>>()?;

    // Iterate all LP positions that are zero, and delete the associated incentive indexes
    for ((account_id, denom), _) in zero_balance_items.iter() {
        ASTRO_USER_LP_DEPOSITS.remove(deps.storage, (account_id, denom));

        // Get all incentives for (user, lp_token_denom) key
        let prefix = USER_ASTRO_INCENTIVE_STATES.prefix((account_id, denom));

        // Iterate over all reward_denom keys
        let keys_to_remove = prefix
            .keys(deps.storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<String>>>()?;

        // Delete each matching (account_id, lp_token_denom, reward_denom) incentive index.
        for incentive_denom in keys_to_remove {
            USER_ASTRO_INCENTIVE_STATES
                .remove(deps.storage, (account_id, denom.as_str(), &incentive_denom));
        }
    }

    Ok(())
}

fn migrate_user_asset_indices(deps: DepsMut, limit: usize) -> Result<Response, ContractError> {
    // Only allow to migrate users asset indices if guard is locked via `migrate` entrypoint
    MIGRATION_GUARD.assert_locked(deps.storage)?;

    // Don't allow to migrate `user_asset_indices` before the `user_unclaimed_rewards` have been migrated
    let count = USER_UNCLAIMED_REWARDS.range(deps.storage, None, None, Order::Ascending).count();
    let v2_count =
        v2_state::USER_UNCLAIMED_REWARDS.range(deps.storage, None, None, Order::Ascending).count();

    if count != v2_count {
        return Err(ContractError::InvalidMigrationCall {});
    };

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
    use std::str::FromStr;

    use cosmwasm_std::{attr, testing::mock_dependencies, Addr, Decimal, Uint128};
    use mars_types::incentives::IncentiveState;
    use mars_utils::error::GuardError;

    use super::*;
    use crate::error::ContractError;

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
    fn clear_zero_amounts_in_staked_astro_lps() {
        let mut deps = mock_dependencies();

        MIGRATION_GUARD.try_lock(deps.as_mut().storage).unwrap();

        let lp_denom_1 = "factory/neutronasdfkldshfkldsjfklfdsaaaaassss111/astroport/share";
        let lp_denom_2 = "factory/neutronasdfkldshfkldsjfklfdsfdsfdsfd2222/astroport/share";
        let reward_denom_1 = "untrn";
        let reward_denom_2 = "ibc/D189335C6E4A68B513C10AB227BF1C1D38C746766278BA3EEB4FB14124F1D858";

        //
        // Instantiate Deposits
        //

        // User 1
        // Lp_denom 1 has balance > 0
        // LP_denom 2 balance of 0
        v2_state::ASTRO_USER_LP_DEPOSITS
            .save(deps.as_mut().storage, ("1", lp_denom_1), &Uint128::new(100000000))
            .unwrap();
        v2_state::ASTRO_USER_LP_DEPOSITS
            .save(deps.as_mut().storage, ("1", lp_denom_2), &Uint128::new(0))
            .unwrap();

        // User 2
        // Lp_denom 1 has balance of 0
        // LP_denom 2 balance > 0
        v2_state::ASTRO_USER_LP_DEPOSITS
            .save(deps.as_mut().storage, ("2", lp_denom_1), &Uint128::new(0))
            .unwrap();

        v2_state::ASTRO_USER_LP_DEPOSITS
            .save(deps.as_mut().storage, ("2", lp_denom_2), &Uint128::new(100000000))
            .unwrap();

        // User 3
        // Lp_denom 1 has balance > 0
        v2_state::ASTRO_USER_LP_DEPOSITS
            .save(deps.as_mut().storage, ("3", lp_denom_1), &Uint128::new(100000000))
            .unwrap();

        // User 4
        // Lp_denom 1 has balance of 0
        v2_state::ASTRO_USER_LP_DEPOSITS
            .save(deps.as_mut().storage, ("4", lp_denom_1), &Uint128::new(0))
            .unwrap();

        // User 5
        // Lp_denom 1 has balance > 0
        // Lp_denom 2 has balance > 0
        v2_state::ASTRO_USER_LP_DEPOSITS
            .save(deps.as_mut().storage, ("5", lp_denom_1), &Uint128::new(100000000))
            .unwrap();
        v2_state::ASTRO_USER_LP_DEPOSITS
            .save(deps.as_mut().storage, ("5", lp_denom_2), &Uint128::new(100000000))
            .unwrap();

        //
        // Instantiate user reward states
        //

        // User 1
        v2_state::USER_ASTRO_INCENTIVE_STATES
            .save(
                deps.as_mut().storage,
                ("1", lp_denom_1, reward_denom_2),
                &Decimal::from_str("1.0001456").unwrap(),
            )
            .unwrap();

        v2_state::USER_ASTRO_INCENTIVE_STATES
            .save(
                deps.as_mut().storage,
                ("1", lp_denom_1, reward_denom_1),
                &Decimal::from_str("1.0001456").unwrap(),
            )
            .unwrap();

        v2_state::USER_ASTRO_INCENTIVE_STATES
            .save(
                deps.as_mut().storage,
                ("1", lp_denom_2, reward_denom_1),
                &Decimal::from_str("1.21456").unwrap(),
            )
            .unwrap();
        v2_state::USER_ASTRO_INCENTIVE_STATES
            .save(
                deps.as_mut().storage,
                ("1", lp_denom_2, reward_denom_2),
                &Decimal::from_str("1.21456").unwrap(),
            )
            .unwrap();

        // User 2
        v2_state::USER_ASTRO_INCENTIVE_STATES
            .save(
                deps.as_mut().storage,
                ("2", lp_denom_1, reward_denom_2),
                &Decimal::from_str("1.0001456").unwrap(),
            )
            .unwrap();

        v2_state::USER_ASTRO_INCENTIVE_STATES
            .save(
                deps.as_mut().storage,
                ("2", lp_denom_2, reward_denom_1),
                &Decimal::from_str("1.0001456").unwrap(),
            )
            .unwrap();

        // User 3
        v2_state::USER_ASTRO_INCENTIVE_STATES
            .save(
                deps.as_mut().storage,
                ("3", lp_denom_1, reward_denom_2),
                &Decimal::from_str("1.0001456").unwrap(),
            )
            .unwrap();
        v2_state::USER_ASTRO_INCENTIVE_STATES
            .save(
                deps.as_mut().storage,
                ("3", lp_denom_1, reward_denom_1),
                &Decimal::from_str("1.0001456").unwrap(),
            )
            .unwrap();

        // User 4 no incentive states

        // User 5 - only 1 reward asset
        v2_state::USER_ASTRO_INCENTIVE_STATES
            .save(
                deps.as_mut().storage,
                ("5", lp_denom_1, reward_denom_2),
                &Decimal::from_str("1.0001456").unwrap(),
            )
            .unwrap();
        v2_state::USER_ASTRO_INCENTIVE_STATES
            .save(
                deps.as_mut().storage,
                ("5", lp_denom_2, reward_denom_2),
                &Decimal::from_str("1.0001456").unwrap(),
            )
            .unwrap();

        // Assert user positions before
        let user_deposits_before = v2_state::ASTRO_USER_LP_DEPOSITS
            .range(deps.as_ref().storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();

        assert_eq!(user_deposits_before.len(), 8);
        assert_eq!(user_deposits_before[0].0, ("1".to_string(), lp_denom_1.to_string()));
        assert_eq!(user_deposits_before[1].0, ("1".to_string(), lp_denom_2.to_string()));
        assert_eq!(user_deposits_before[2].0, ("2".to_string(), lp_denom_1.to_string()));
        assert_eq!(user_deposits_before[3].0, ("2".to_string(), lp_denom_2.to_string()));
        assert_eq!(user_deposits_before[4].0, ("3".to_string(), lp_denom_1.to_string()));
        assert_eq!(user_deposits_before[5].0, ("4".to_string(), lp_denom_1.to_string()));
        assert_eq!(user_deposits_before[6].0, ("5".to_string(), lp_denom_1.to_string()));
        assert_eq!(user_deposits_before[7].0, ("5".to_string(), lp_denom_2.to_string()));

        let incentive_states_before = v2_state::USER_ASTRO_INCENTIVE_STATES
            .range(deps.as_ref().storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();

        // Assert all incentives are there
        assert_eq!(incentive_states_before.len(), 10);

        // Clear balances
        clear_zero_amounts_in_staked_astro_lp(&mut deps.as_mut()).unwrap();

        let user_deposits_after = v2_state::ASTRO_USER_LP_DEPOSITS
            .range(deps.as_ref().storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();

        assert_eq!(user_deposits_after[0].0, ("1".to_string(), lp_denom_1.to_string()));
        assert_eq!(user_deposits_after[1].0, ("2".to_string(), lp_denom_2.to_string()));
        assert_eq!(user_deposits_after[2].0, ("3".to_string(), lp_denom_1.to_string()));
        assert_eq!(user_deposits_after[3].0, ("5".to_string(), lp_denom_1.to_string()));
        assert_eq!(user_deposits_after[4].0, ("5".to_string(), lp_denom_2.to_string()));

        // Incentive records that should be cleared
        // (user_1,lp_denom_2)
        // - both incentives (2 records deleted)

        // (User 2, lp_denom_1)
        // - one incentive
        let incentive_states_after = v2_state::USER_ASTRO_INCENTIVE_STATES
            .range(deps.as_ref().storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();

        assert_eq!(incentive_states_after.len(), 7); // because we deleted 3 records
        assert_eq!(
            incentive_states_after[0].0,
            ("1".to_string(), lp_denom_1.to_string(), reward_denom_2.to_string())
        );
        assert_eq!(
            incentive_states_after[1].0,
            ("1".to_string(), lp_denom_1.to_string(), reward_denom_1.to_string())
        );
        assert_eq!(
            incentive_states_after[2].0,
            ("2".to_string(), lp_denom_2.to_string(), reward_denom_1.to_string())
        );
        assert_eq!(
            incentive_states_after[3].0,
            ("3".to_string(), lp_denom_1.to_string(), reward_denom_2.to_string())
        );
        assert_eq!(
            incentive_states_after[4].0,
            ("3".to_string(), lp_denom_1.to_string(), reward_denom_1.to_string())
        );
        assert_eq!(
            incentive_states_after[5].0,
            ("5".to_string(), lp_denom_1.to_string(), reward_denom_2.to_string())
        );
        assert_eq!(
            incentive_states_after[6].0,
            ("5".to_string(), lp_denom_2.to_string(), reward_denom_2.to_string())
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
    fn cannot_migrate_asset_indices_before_unclaimed_rewards() {
        let mut deps = mock_dependencies();

        MIGRATION_GUARD.try_lock(deps.as_mut().storage).unwrap();

        // Prepare the unclaimed rewards V2 state
        let acc_1: UserIdKey =
            UserId::credit_manager(Addr::unchecked("user1"), "1".to_string()).try_into().unwrap();
        let acc_2: UserIdKey =
            UserId::credit_manager(Addr::unchecked("user1"), "2".to_string()).try_into().unwrap();

        v2_state::USER_UNCLAIMED_REWARDS
            .save(deps.as_mut().storage, (&acc_1, "umars", "untrn"), &Uint128::from(133u128))
            .unwrap();
        v2_state::USER_UNCLAIMED_REWARDS
            .save(deps.as_mut().storage, (&acc_1, "umars", "uusdc"), &Uint128::from(133u128))
            .unwrap();
        v2_state::USER_UNCLAIMED_REWARDS
            .save(deps.as_mut().storage, (&acc_2, "umars", "uusdc"), &Uint128::from(133u128))
            .unwrap();

        // Migrate first two
        let err = migrate_user_asset_indices(deps.as_mut(), 2).unwrap_err();

        assert_eq!(err, ContractError::InvalidMigrationCall {});
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
