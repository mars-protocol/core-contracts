use cosmwasm_std::{
    attr, coin, coins,
    testing::{mock_env, mock_info},
    Addr, BankMsg, CosmosMsg, Decimal, Int128, SubMsg, Timestamp, Uint128,
};
use mars_incentives::{
    contract::execute,
    helpers::{compute_incentive_index, compute_user_accrued_rewards},
    query,
    state::{EMISSIONS, INCENTIVE_STATES, USER_ASSET_INDICES, USER_UNCLAIMED_REWARDS},
};
use mars_testing::MockEnvParams;
use mars_types::{
    incentives::{ExecuteMsg, IncentiveKind, IncentiveState},
    keys::{IncentiveId, IncentiveIdKey, IncentiveKindKey, UserId, UserIdKey},
    perps::{VaultDeposit, VaultPositionResponse, VaultResponse},
    red_bank::{Market, UserCollateralResponse},
};

use super::helpers::{th_setup, ths_setup_with_epoch_duration};

#[test]
fn execute_claim_rewards_red_bank() {
    // SETUP
    let env = mock_env();
    let mut deps: cosmwasm_std::OwnedDeps<
        cosmwasm_std::MemoryStorage,
        cosmwasm_std::testing::MockApi,
        mars_testing::MarsMockQuerier,
    > = ths_setup_with_epoch_duration(env, 604800);
    let user_addr = Addr::unchecked("user");

    let previous_unclaimed_rewards = Uint128::new(50_000);
    let asset_total_supply = Uint128::new(100_000);
    let asset_user_balance = Uint128::new(10_000);
    let zero_total_supply = Uint128::new(200_000);
    let zero_user_balance = Uint128::new(10_000);
    let no_user_total_supply = Uint128::new(100_000);
    let no_user_user_balance = Uint128::zero();
    let time_start = 500_000_u64;
    let time_contract_call = 600_000_u64;

    // denom of an asset with ongoing rewards
    let asset_denom = "asset";
    // denom of an asset with no pending rewards but with user index (so it had active incentives
    // at some point)
    let zero_denom = "zero";
    // denom of an asset where the user never had a balance during an active
    // incentive -> hence no associated index
    let no_user_denom = "no_user";

    deps.querier.set_redbank_market(Market {
        denom: asset_denom.to_string(),
        collateral_total_scaled: asset_total_supply,
        ..Default::default()
    });
    deps.querier.set_redbank_market(Market {
        denom: zero_denom.to_string(),
        collateral_total_scaled: zero_total_supply,
        ..Default::default()
    });
    deps.querier.set_redbank_market(Market {
        denom: no_user_denom.to_string(),
        collateral_total_scaled: no_user_total_supply,
        ..Default::default()
    });
    deps.querier.set_red_bank_user_collateral(
        &user_addr,
        UserCollateralResponse {
            denom: asset_denom.to_string(),
            amount_scaled: asset_user_balance,
            amount: Uint128::zero(), // doesn't matter for this test
            enabled: true,
        },
    );
    deps.querier.set_red_bank_user_collateral(
        &user_addr,
        UserCollateralResponse {
            denom: zero_denom.to_string(),
            amount_scaled: zero_user_balance,
            amount: Uint128::zero(), // doesn't matter for this test
            enabled: true,
        },
    );
    deps.querier.set_red_bank_user_collateral(
        &user_addr,
        UserCollateralResponse {
            denom: no_user_denom.to_string(),
            amount_scaled: no_user_user_balance,
            amount: Uint128::zero(), // doesn't matter for this test
            enabled: true,
        },
    );

    let kind_key = IncentiveKindKey::try_from(&IncentiveKind::RedBank).unwrap();
    let incentive_id_asset = IncentiveId::create(IncentiveKind::RedBank, asset_denom.to_string());
    let incentive_id_key_asset = IncentiveIdKey::try_from(incentive_id_asset).unwrap();
    let incentive_id_zero = IncentiveId::create(IncentiveKind::RedBank, zero_denom.to_string());
    let incentive_id_key_zero = IncentiveIdKey::try_from(incentive_id_zero).unwrap();
    let incentive_id_no_user =
        IncentiveId::create(IncentiveKind::RedBank, no_user_denom.to_string());
    let incentive_id_key_no_user = IncentiveIdKey::try_from(incentive_id_no_user).unwrap();

    // incentives
    INCENTIVE_STATES
        .save(
            deps.as_mut().storage,
            (&kind_key, asset_denom, "umars"),
            &IncentiveState {
                index: Decimal::one(),
                last_updated: time_start,
            },
        )
        .unwrap();
    for i in 0..7 {
        EMISSIONS
            .save(
                deps.as_mut().storage,
                (&incentive_id_key_asset, "umars", time_start + 604800 * i),
                &Uint128::new(100),
            )
            .unwrap();
    }
    INCENTIVE_STATES
        .save(
            deps.as_mut().storage,
            (&kind_key, zero_denom, "umars"),
            &IncentiveState {
                index: Decimal::one(),
                last_updated: time_start,
            },
        )
        .unwrap();
    INCENTIVE_STATES
        .save(
            deps.as_mut().storage,
            (&kind_key, no_user_denom, "umars"),
            &IncentiveState {
                index: Decimal::one(),
                last_updated: time_start,
            },
        )
        .unwrap();
    EMISSIONS
        .save(
            deps.as_mut().storage,
            (&incentive_id_key_no_user, "umars", time_start),
            &Uint128::new(200),
        )
        .unwrap();

    let user_id = UserId::credit_manager(user_addr.clone(), "".to_string());
    let user_id_key: UserIdKey = user_id.try_into().unwrap();

    // user indices
    USER_ASSET_INDICES
        .save(
            deps.as_mut().storage,
            (&user_id_key, &incentive_id_key_asset, "umars"),
            &Decimal::one(),
        )
        .unwrap();

    USER_ASSET_INDICES
        .save(
            deps.as_mut().storage,
            (&user_id_key, &incentive_id_key_zero, "umars"),
            &Decimal::from_ratio(1_u128, 2_u128),
        )
        .unwrap();

    // unclaimed_rewards
    USER_UNCLAIMED_REWARDS
        .save(
            deps.as_mut().storage,
            (&user_id_key, &incentive_id_key_asset, "umars"),
            &previous_unclaimed_rewards,
        )
        .unwrap();

    let expected_asset_incentive_index = compute_incentive_index(
        Decimal::one(),
        Uint128::new(100),
        asset_total_supply,
        time_start,
        time_contract_call,
    )
    .unwrap();

    let expected_asset_accrued_rewards = compute_user_accrued_rewards(
        asset_user_balance,
        Decimal::one(),
        expected_asset_incentive_index,
    )
    .unwrap();

    let expected_zero_accrued_rewards = compute_user_accrued_rewards(
        zero_user_balance,
        Decimal::from_ratio(1_u128, 2_u128),
        Decimal::one(),
    )
    .unwrap();

    let expected_accrued_rewards =
        previous_unclaimed_rewards + expected_asset_accrued_rewards + expected_zero_accrued_rewards;

    // MSG
    let info = mock_info("user", &[]);
    let env = mars_testing::mock_env(MockEnvParams {
        block_time: Timestamp::from_seconds(time_contract_call),
        ..Default::default()
    });
    let msg = ExecuteMsg::ClaimRewards {
        account_id: None,
        start_after_kind: None,
        start_after_denom: None,
        start_after_incentive_denom: None,
        limit: None,
    };

    // query a bit before gives less rewards
    let env_before = mars_testing::mock_env(MockEnvParams {
        block_time: Timestamp::from_seconds(time_contract_call - 10_000),
        ..Default::default()
    });
    let rewards_query_before = query::query_user_unclaimed_rewards(
        deps.as_ref(),
        env_before,
        String::from("user"),
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert!(rewards_query_before.len() == 1);
    assert!(rewards_query_before[0].amount < expected_accrued_rewards);

    // query before execution gives expected rewards
    let rewards_query = query::query_user_unclaimed_rewards(
        deps.as_ref(),
        env.clone(),
        String::from("user"),
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(rewards_query[0].amount, expected_accrued_rewards);

    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    // query after execution gives 0 rewards
    //
    // NOTE: the query should return an empty array, instead of a non-empty array
    // with a zero-amount coin! the latter is considered an invalid coins array
    // and will result in error.
    let rewards_query_after = query::query_user_unclaimed_rewards(
        deps.as_ref(),
        env,
        String::from("user"),
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert!(rewards_query_after.is_empty());

    // ASSERT

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: user_addr.to_string(),
            amount: coins(expected_accrued_rewards.u128(), "umars".to_string())
        }))]
    );

    assert_eq!(
        res.events[0].attributes,
        vec![attr("action", "claim_rewards"), attr("user", "user")]
    );
    assert_eq!(
        res.events[1].attributes,
        vec![attr("coins", format!("{expected_accrued_rewards}umars"))]
    );
    // asset and zero incentives get updated, no_user does not
    let asset_incentive =
        INCENTIVE_STATES.load(deps.as_ref().storage, (&kind_key, asset_denom, "umars")).unwrap();
    assert_eq!(asset_incentive.index, expected_asset_incentive_index);
    assert_eq!(asset_incentive.last_updated, time_contract_call);

    let zero_incentive =
        INCENTIVE_STATES.load(deps.as_ref().storage, (&kind_key, zero_denom, "umars")).unwrap();
    assert_eq!(zero_incentive.index, Decimal::one());
    assert_eq!(zero_incentive.last_updated, time_contract_call);

    let no_user_incentive =
        INCENTIVE_STATES.load(deps.as_ref().storage, (&kind_key, no_user_denom, "umars")).unwrap();
    assert_eq!(no_user_incentive.index, Decimal::one());
    assert_eq!(no_user_incentive.last_updated, time_start);

    let user_id = UserId::credit_manager(user_addr, "".to_string());
    let user_id_key: UserIdKey = user_id.try_into().unwrap();

    // user's asset and zero indices are updated
    let user_asset_index = USER_ASSET_INDICES
        .load(deps.as_ref().storage, (&user_id_key, &incentive_id_key_asset, "umars"))
        .unwrap();
    assert_eq!(user_asset_index, expected_asset_incentive_index);

    let user_zero_index = USER_ASSET_INDICES
        .load(deps.as_ref().storage, (&user_id_key, &incentive_id_key_zero, "umars"))
        .unwrap();
    assert_eq!(user_zero_index, Decimal::one());

    // user's no_user does not get updated
    let user_no_user_index = USER_ASSET_INDICES
        .may_load(deps.as_ref().storage, (&user_id_key, &incentive_id_key_no_user, "umars"))
        .unwrap();
    assert_eq!(user_no_user_index, None);

    // user rewards are cleared
    let user_unclaimed_rewards = USER_UNCLAIMED_REWARDS
        .load(deps.as_ref().storage, (&user_id_key, &incentive_id_key_asset, "umars"))
        .unwrap();
    assert_eq!(user_unclaimed_rewards, Uint128::zero())
}

#[test]
fn execute_claim_rewards_perp_vault() {
    // SETUP
    let env = mock_env();
    let mut deps: cosmwasm_std::OwnedDeps<
        cosmwasm_std::MemoryStorage,
        cosmwasm_std::testing::MockApi,
        mars_testing::MarsMockQuerier,
    > = ths_setup_with_epoch_duration(env, 604800);
    let user_addr = Addr::unchecked("user");

    let previous_unclaimed_rewards = Uint128::new(50_000);
    let total_shares = Uint128::new(100_000); // total shares in the perp vault
    let user_shares = Uint128::new(10_000);
    let time_start = 500_000_u64;
    let time_contract_call = 600_000_u64;

    // denom of the vault with ongoing rewards
    let perp_vault_denom = "asset";

    let incentive_denom_1 = "umars";
    let incentive_denom_2 = "untrn";

    deps.querier.set_perp_vault_state(VaultResponse {
        total_shares,
        total_unlocking_or_unlocked_shares: Uint128::zero(),
        total_unlocking_or_unlocked_amount: Uint128::zero(),
        total_balance: Int128::zero(),
        total_liquidity: Uint128::zero(),
        collateralization_ratio: None,
        share_price: None,
        total_debt: Uint128::zero(),
        total_withdrawal_balance: Uint128::zero(),
    });

    deps.querier.set_perp_vault_position(
        &user_addr,
        VaultPositionResponse {
            denom: perp_vault_denom.to_string(),
            deposit: VaultDeposit {
                amount: Uint128::zero(),
                shares: user_shares,
            },
            unlocks: vec![],
        },
    );

    let kind_key = IncentiveKindKey::try_from(&IncentiveKind::PerpVault).unwrap();
    let incentive_id = IncentiveId::create(IncentiveKind::PerpVault, perp_vault_denom.to_string());
    let incentive_id_key = IncentiveIdKey::try_from(incentive_id).unwrap();

    // incentives
    INCENTIVE_STATES
        .save(
            deps.as_mut().storage,
            (&kind_key, perp_vault_denom, incentive_denom_1),
            &IncentiveState {
                index: Decimal::one(),
                last_updated: time_start,
            },
        )
        .unwrap();

    INCENTIVE_STATES
        .save(
            deps.as_mut().storage,
            (&kind_key, perp_vault_denom, incentive_denom_2),
            &IncentiveState {
                index: Decimal::one(),
                last_updated: time_start,
            },
        )
        .unwrap();

    for i in 0..7 {
        EMISSIONS
            .save(
                deps.as_mut().storage,
                (&incentive_id_key, incentive_denom_1, time_start + 604800 * i),
                &Uint128::new(100),
            )
            .unwrap();
    }
    EMISSIONS
        .save(
            deps.as_mut().storage,
            (&incentive_id_key, incentive_denom_2, time_start),
            &Uint128::new(200),
        )
        .unwrap();

    let user_id = UserId::credit_manager(user_addr.clone(), "".to_string());
    let user_id_key: UserIdKey = user_id.try_into().unwrap();

    // user indices
    USER_ASSET_INDICES
        .save(
            deps.as_mut().storage,
            (&user_id_key, &incentive_id_key, incentive_denom_1),
            &Decimal::one(),
        )
        .unwrap();
    USER_ASSET_INDICES
        .save(
            deps.as_mut().storage,
            (&user_id_key, &incentive_id_key, incentive_denom_2),
            &Decimal::from_ratio(1u128, 2u128),
        )
        .unwrap();

    // unclaimed_rewards
    USER_UNCLAIMED_REWARDS
        .save(
            deps.as_mut().storage,
            (&user_id_key, &incentive_id_key, incentive_denom_1),
            &previous_unclaimed_rewards,
        )
        .unwrap();

    let expected_asset_incentive_index_1 = compute_incentive_index(
        Decimal::one(),
        Uint128::new(100),
        total_shares,
        time_start,
        time_contract_call,
    )
    .unwrap();

    let expected_asset_accrued_rewards_1 =
        compute_user_accrued_rewards(user_shares, Decimal::one(), expected_asset_incentive_index_1)
            .unwrap();

    let expected_accrued_rewards_1 = previous_unclaimed_rewards + expected_asset_accrued_rewards_1;

    let expected_asset_incentive_index_2 = compute_incentive_index(
        Decimal::from_ratio(1u128, 1u128),
        Uint128::new(200),
        total_shares,
        time_start,
        time_contract_call,
    )
    .unwrap();

    let expected_accrued_rewards_2 = compute_user_accrued_rewards(
        user_shares,
        Decimal::from_ratio(1u128, 2u128),
        expected_asset_incentive_index_2,
    )
    .unwrap();

    // MSG
    let info = mock_info("user", &[]);
    let env = mars_testing::mock_env(MockEnvParams {
        block_time: Timestamp::from_seconds(time_contract_call),
        ..Default::default()
    });
    let msg = ExecuteMsg::ClaimRewards {
        account_id: None,
        start_after_kind: None,
        start_after_denom: None,
        start_after_incentive_denom: None,
        limit: None,
    };

    // query a bit before gives less rewards
    let env_before = mars_testing::mock_env(MockEnvParams {
        block_time: Timestamp::from_seconds(time_contract_call - 10_000),
        ..Default::default()
    });
    let rewards_query_before = query::query_user_unclaimed_rewards(
        deps.as_ref(),
        env_before,
        String::from("user"),
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(rewards_query_before.len(), 2);
    assert!(rewards_query_before[0].amount < expected_accrued_rewards_1);
    assert!(rewards_query_before[1].amount < expected_accrued_rewards_2);

    println!("rewards_query_before: {:?}", rewards_query_before);

    // query before execution gives expected rewards
    let rewards_query = query::query_user_unclaimed_rewards(
        deps.as_ref(),
        env.clone(),
        String::from("user"),
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(rewards_query[0].amount, expected_accrued_rewards_1);
    assert_eq!(rewards_query[1].amount, expected_accrued_rewards_2);

    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    // query after execution gives 0 rewards
    //
    // NOTE: the query should return an empty array, instead of a non-empty array
    // with a zero-amount coin! the latter is considered an invalid coins array
    // and will result in error.
    let rewards_query_after = query::query_user_unclaimed_rewards(
        deps.as_ref(),
        env,
        String::from("user"),
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert!(rewards_query_after.is_empty());

    // ASSERT
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: user_addr.to_string(),
            amount: vec![
                coin(expected_accrued_rewards_1.u128(), incentive_denom_1.to_string()),
                coin(expected_accrued_rewards_2.u128(), incentive_denom_2.to_string())
            ]
        }))]
    );

    assert_eq!(
        res.events[0].attributes,
        vec![attr("action", "claim_rewards"), attr("user", "user")]
    );
    assert_eq!(
        res.events[1].attributes,
        vec![attr(
            "coins",
            format!("{expected_accrued_rewards_1}umars,{expected_accrued_rewards_2}untrn")
        )]
    );
    // asset incentives get updated
    let asset_incentive = INCENTIVE_STATES
        .load(deps.as_ref().storage, (&kind_key, perp_vault_denom, incentive_denom_1))
        .unwrap();
    assert_eq!(asset_incentive.index, expected_asset_incentive_index_1);
    assert_eq!(asset_incentive.last_updated, time_contract_call);

    let asset_incentive = INCENTIVE_STATES
        .load(deps.as_ref().storage, (&kind_key, perp_vault_denom, incentive_denom_2))
        .unwrap();
    assert_eq!(asset_incentive.index, expected_asset_incentive_index_2);
    assert_eq!(asset_incentive.last_updated, time_contract_call);

    let user_id = UserId::credit_manager(user_addr, "".to_string());
    let user_id_key: UserIdKey = user_id.try_into().unwrap();

    // user's asset indices are updated
    let user_asset_index = USER_ASSET_INDICES
        .load(deps.as_ref().storage, (&user_id_key, &incentive_id_key, incentive_denom_1))
        .unwrap();
    assert_eq!(user_asset_index, expected_asset_incentive_index_1);

    // user rewards are cleared
    let user_unclaimed_rewards = USER_UNCLAIMED_REWARDS
        .load(deps.as_ref().storage, (&user_id_key, &incentive_id_key, incentive_denom_1))
        .unwrap();
    assert_eq!(user_unclaimed_rewards, Uint128::zero());

    // user's asset indices are updated
    let user_asset_index = USER_ASSET_INDICES
        .load(deps.as_ref().storage, (&user_id_key, &incentive_id_key, incentive_denom_2))
        .unwrap();
    assert_eq!(user_asset_index, expected_asset_incentive_index_2);

    // user rewards are cleared
    let user_unclaimed_rewards = USER_UNCLAIMED_REWARDS
        .load(deps.as_ref().storage, (&user_id_key, &incentive_id_key, incentive_denom_2))
        .unwrap();
    assert_eq!(user_unclaimed_rewards, Uint128::zero())
}

#[test]
fn claim_zero_rewards() {
    // SETUP
    let mut deps = th_setup();

    let info = mock_info("user", &[]);
    let msg = ExecuteMsg::ClaimRewards {
        account_id: None,
        start_after_kind: None,
        start_after_denom: None,
        start_after_incentive_denom: None,
        limit: None,
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(res.messages.len(), 0);
    assert_eq!(
        res.events[0].attributes,
        vec![attr("action", "claim_rewards"), attr("user", "user")]
    );
}
