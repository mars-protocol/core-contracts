use std::str::FromStr;

use cosmwasm_std::{coin, testing::MockStorage, Addr, Decimal, Int128, SignedDecimal, Uint128};
use mars_mock_oracle::msg::CoinPrice;
use mars_testing::multitest::helpers::{coin_info, default_perp_params, uatom_info};
use mars_types::{
    oracle::ActionKind,
    params::{PerpParams, PerpParamsUpdate},
};
use mars_vault::{
    helpers::i128_from_u128,
    pnl::{query_current_vault_pnl_index, query_user_pnl},
    state::{LAST_NET_WORTH, USER_ENTRY_PNL_INDEX},
};
use proptest::prelude::*;
use test_case::test_case;

use super::vault_helpers::{execute_deposit, instantiate_vault, VaultSetup};
use crate::tests::vault_helpers::{
    execute_redeem, execute_unlock, open_perp_position, query_user_pnl as query_user_pnl_mock,
    query_vault_info, query_vault_pnl_mock,
};

const MAX_INT128: u128 = Int128::MAX.unsigned_abs().u128();

#[test]
fn two_users_sequential_deposits() {
    let uusdc_info = coin_info("uusdc");
    let uatom_info = uatom_info();
    let base_denom = uusdc_info.denom.clone();
    let btc_perp_denom = "perp/btc";

    let user1 = Addr::unchecked("user");
    let user2 = Addr::unchecked("user2");

    let VaultSetup {
        mut mock,
        fund_manager,
        managed_vault_addr,
        fund_acc_id,
    } = instantiate_vault(&uusdc_info, &uatom_info, &base_denom);

    let vault_info_res = query_vault_info(&mock, &managed_vault_addr);
    let vault_token = vault_info_res.vault_token;

    // add perp params
    mock.update_perp_params(PerpParamsUpdate::AddOrUpdate {
        params: PerpParams {
            max_funding_velocity: Decimal::from_str("0.00").unwrap(),
            closing_fee_rate: Decimal::from_str("0.0").unwrap(),
            opening_fee_rate: Decimal::from_str("0.0").unwrap(),
            ..default_perp_params(btc_perp_denom)
        },
    });

    // set usdc price to 1 USD
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: uusdc_info.denom.clone(),
        price: Decimal::from_str("1").unwrap(),
    });

    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: btc_perp_denom.to_string(),
        price: Decimal::from_str("10").unwrap(),
    });

    // deposit into vault
    let deposited_amt: Uint128 = Uint128::new(100_000_000);

    // user 1 deposits
    execute_deposit(
        &mut mock,
        &user1,
        &managed_vault_addr,
        Uint128::zero(),
        None,
        &[coin(deposited_amt.u128(), base_denom.clone())],
    )
    .unwrap();

    // open perp position
    open_perp_position(
        &mut mock,
        &fund_acc_id,
        &fund_manager,
        btc_perp_denom,
        Int128::from_str("1000000").unwrap(),
    );

    // increase price
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: btc_perp_denom.to_string(),
        price: Decimal::from_str("11").unwrap(),
    });

    // assert user 1 shares
    let user_vault_token_balance = mock.query_balance(&user1, &vault_token).amount;
    assert_eq!(user_vault_token_balance, Uint128::new(100000000000000));

    // assert user 1 pnl
    let user1_pnl = query_user_pnl_mock(&mock, &managed_vault_addr, &user1);
    assert_eq!(user1_pnl.pnl, SignedDecimal::from_str("1000000").unwrap().to_string());

    // user 2 deposits
    execute_deposit(
        &mut mock,
        &user2,
        &managed_vault_addr,
        Uint128::zero(),
        None,
        &[coin(deposited_amt.u128(), base_denom.clone())],
    )
    .unwrap();

    // assert user 2 shares
    let user2_vault_token_balance = mock.query_balance(&user2, &vault_token).amount;
    assert_eq!(user2_vault_token_balance, Uint128::new(99009900990099));

    // assert user 2 pnl
    let user2_pnl = query_user_pnl_mock(&mock, &managed_vault_addr, &user2);
    assert_eq!(user2_pnl.pnl, SignedDecimal::from_str("0").unwrap().to_string());

    // decrease price
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: btc_perp_denom.to_string(),
        price: Decimal::from_str("10.8").unwrap(),
    });

    // assert user 1 pnl
    let user1_pnl = query_user_pnl_mock(&mock, &managed_vault_addr, &user1);
    assert_eq!(user1_pnl.pnl, SignedDecimal::from_str("899502").unwrap().to_string());

    // assert user 2 pnl
    let user2_pnl = query_user_pnl_mock(&mock, &managed_vault_addr, &user2);
    assert_eq!(user2_pnl.pnl, SignedDecimal::from_str("-99503").unwrap().to_string());

    // assert vault pnl
    let vault_pnl = query_vault_pnl_mock(&mock, &managed_vault_addr);
    assert_eq!(vault_pnl.total_pnl, SignedDecimal::from_str("800000").unwrap().to_string());
}

#[test]
fn single_user_flow_then_second_user() {
    let uusdc_info = coin_info("uusdc");
    let uatom_info = uatom_info();
    let base_denom = uusdc_info.denom.clone();
    let btc_perp_denom = "perp/btc";
    let user = Addr::unchecked("user");
    let VaultSetup {
        mut mock,
        fund_manager,
        managed_vault_addr,
        fund_acc_id,
    } = instantiate_vault(&uusdc_info, &uatom_info, &base_denom);

    let vault_info_res = query_vault_info(&mock, &managed_vault_addr);
    let vault_token = vault_info_res.vault_token;

    // add perp params
    mock.update_perp_params(PerpParamsUpdate::AddOrUpdate {
        params: PerpParams {
            max_funding_velocity: Decimal::from_str("0.00").unwrap(),
            closing_fee_rate: Decimal::from_str("0.0").unwrap(),
            opening_fee_rate: Decimal::from_str("0.0").unwrap(),
            ..default_perp_params(btc_perp_denom)
        },
    });

    // set usdc price to 1 USD
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: uusdc_info.denom.clone(),
        price: Decimal::from_str("1").unwrap(),
    });

    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: btc_perp_denom.to_string(),
        price: Decimal::from_str("10").unwrap(),
    });

    // deposit into vault
    let deposited_amt: Uint128 = Uint128::new(100_000_000);
    execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(deposited_amt.u128(), base_denom.clone())],
    )
    .unwrap();

    let user_vault_token_balance = mock.query_balance(&user, &vault_token).amount;

    // open perp position
    open_perp_position(
        &mut mock,
        &fund_acc_id,
        &fund_manager,
        btc_perp_denom,
        Int128::from_str("1000000").unwrap(),
    );

    // increase price
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: btc_perp_denom.to_string(),
        price: Decimal::from_str("11").unwrap(),
    });

    // check pnl
    // opening fee = 10_000_000 * 0.0 = 0
    // closing fee = 11_000_000 * 0.0 = 0
    // pnl = 11_000_000 - 10_000_000 = 1_000_000
    let pnl = query_user_pnl_mock(&mock, &managed_vault_addr, &user);
    assert_eq!(pnl.pnl, SignedDecimal::from_str("1000000").unwrap().to_string());

    let ten_percent = user_vault_token_balance.checked_div(10u128.into()).unwrap();
    // unlock
    execute_unlock(&mut mock, &user, &managed_vault_addr, ten_percent, &[]).unwrap();

    // verify pnl is the same
    let pnl = query_user_pnl_mock(&mock, &managed_vault_addr, &user);
    assert_eq!(pnl.pnl, SignedDecimal::from_str("1000000").unwrap().to_string()); // Not quite 790_000 due to rounding

    // move time forward to pass cooldown period
    mock.increment_by_time(vault_info_res.cooldown_period + 1);
    // redeem
    execute_redeem(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(),
        None,
        &[coin(ten_percent.u128(), vault_token.clone())],
    )
    .unwrap();

    let pnl = query_user_pnl_mock(&mock, &managed_vault_addr, &user);
    assert_eq!(pnl.pnl, SignedDecimal::from_str("1000000").unwrap().to_string());

    // unlock
    execute_unlock(&mut mock, &user, &managed_vault_addr, ten_percent, &[]).unwrap();

    // move time forward to pass cooldown period
    mock.increment_by_time(vault_info_res.cooldown_period + 1);

    // redeem
    execute_redeem(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(),
        None,
        &[coin(ten_percent.u128(), vault_token.clone())],
    )
    .unwrap();

    // Query pnl
    let pnl = query_user_pnl_mock(&mock, &managed_vault_addr, &user);

    assert_eq!(pnl.pnl, SignedDecimal::from_str("1000000").unwrap().to_string());

    // query vault pnl
    let pnl = query_vault_pnl_mock(&mock, &managed_vault_addr);
    assert_eq!(pnl.total_pnl, SignedDecimal::from_str("1000000").unwrap().to_string());

    // make another user deposit
    let user2 = Addr::unchecked("user2");
    execute_deposit(
        &mut mock,
        &user2,
        &managed_vault_addr,
        Uint128::zero(),
        None,
        &[coin(deposited_amt.u128(), base_denom.clone())],
    )
    .unwrap();

    // query pnl
    let pnl = query_user_pnl_mock(&mock, &managed_vault_addr, &user2);
    assert_eq!(pnl.pnl, SignedDecimal::from_str("0").unwrap().to_string());

    // move price down
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: btc_perp_denom.to_string(),
        price: Decimal::from_str("10.5").unwrap(),
    });

    // query vault pnl
    let pnl = query_vault_pnl_mock(&mock, &managed_vault_addr);
    assert_eq!(pnl.total_pnl, SignedDecimal::from_str("500000").unwrap().to_string());

    // query user 2 pnl
    let pnl = query_user_pnl_mock(&mock, &managed_vault_addr, &user2);
    assert_eq!(pnl.pnl, SignedDecimal::from_str("-276549").unwrap().to_string());

    // query user 1 pnl
    let pnl = query_user_pnl_mock(&mock, &managed_vault_addr, &user);
    assert_eq!(pnl.pnl, SignedDecimal::from_str("776548").unwrap().to_string());

    // query vault pnl
    let pnl = query_vault_pnl_mock(&mock, &managed_vault_addr);
    assert_eq!(pnl.total_pnl, SignedDecimal::from_str("500000").unwrap().to_string());

    // unlock user 2
    execute_unlock(&mut mock, &user2, &managed_vault_addr, ten_percent, &[]).unwrap();

    // query vault pnl
    let pnl = query_vault_pnl_mock(&mock, &managed_vault_addr);
    assert_eq!(pnl.total_pnl, SignedDecimal::from_str("500000").unwrap().to_string());

    // query user 1 pnl
    let pnl = query_user_pnl_mock(&mock, &managed_vault_addr, &user);
    assert_eq!(pnl.pnl, SignedDecimal::from_str("776548").unwrap().to_string());

    // query user 2 pnl
    let pnl = query_user_pnl_mock(&mock, &managed_vault_addr, &user2);
    assert_eq!(pnl.pnl, SignedDecimal::from_str("-276549").unwrap().to_string());

    // move forward to pass cooldown period
    mock.increment_by_time(vault_info_res.cooldown_period + 1);

    // redeem user 2
    execute_redeem(
        &mut mock,
        &user2,
        &managed_vault_addr,
        Uint128::zero(),
        None,
        &[coin(ten_percent.u128(), vault_token.clone())],
    )
    .unwrap();

    // query user 1 pnl
    let pnl = query_user_pnl_mock(&mock, &managed_vault_addr, &user);
    assert_eq!(pnl.pnl, SignedDecimal::from_str("776548").unwrap().to_string());

    // query user 2 pnl
    let pnl = query_user_pnl_mock(&mock, &managed_vault_addr, &user2);
    assert_eq!(pnl.pnl, SignedDecimal::from_str("-276549").unwrap().to_string());

    // query vault pnl
    let pnl = query_vault_pnl_mock(&mock, &managed_vault_addr);
    assert_eq!(pnl.total_pnl, SignedDecimal::from_str("500000").unwrap().to_string());
}

#[test]
fn test_vault_pnl_index_calculation() {
    let mut storage = MockStorage::new();

    // test initial state
    let net_worth = Uint128::new(1_000_000);
    let shares = Uint128::new(1_000_000);

    let (pnl_index, pnl_delta) =
        query_current_vault_pnl_index(&storage, net_worth, shares).unwrap();
    assert_eq!(pnl_index, SignedDecimal::zero());
    assert_eq!(pnl_delta, Int128::zero());

    // set initial state
    LAST_NET_WORTH.save(&mut storage, &net_worth).unwrap();

    // test profit scenario
    let new_net_worth = Uint128::new(1_100_000);
    let (pnl_index, pnl_delta) =
        query_current_vault_pnl_index(&storage, new_net_worth, shares).unwrap();
    assert_eq!(pnl_delta, Int128::new(100_000));
    assert_eq!(pnl_index, SignedDecimal::from_str("100000").unwrap());

    // test loss scenario
    let new_net_worth = Uint128::new(900_000);
    let (pnl_index, pnl_delta) =
        query_current_vault_pnl_index(&storage, new_net_worth, shares).unwrap();
    assert_eq!(pnl_delta, Int128::new(-100_000));
    assert_eq!(pnl_index, SignedDecimal::from_str("-100000").unwrap());
}

#[test_case(50000000000, "1.5", 75000, false; "basic profit calculation")]
#[test_case(500000000000, "-0.5", -250000, false; "basic loss calculation")]
#[test_case(500000000000, "0", 0, false; "no change calculation")]
#[test_case(5000000000000000, "0.01", 50000000, false; "large shares tiny pnl")]
#[test_case(5000000000000000, "-0.01", -50000000, false; "large shares tiny loss")]
#[test_case(50000000000000, "1000", 50000000000, false; "very large profit")]
#[test_case(8507059173023461586584365, "0.000000000000000058", 493, false; "large shares tiny delta")]
#[test_case(MAX_INT128 / 2, "1.0", 0, true; "overflow test - extremely large index")]
fn test_user_pnl_scenarios(
    user_shares: u128,
    vault_pnl_index: &str,
    expected_pnl: i128,
    expect_overflow: bool,
) {
    let mut storage = MockStorage::new();
    let user = Addr::unchecked("user");

    // setup initial state
    USER_ENTRY_PNL_INDEX.save(&mut storage, &user, &SignedDecimal::zero()).unwrap();

    let vault_pnl_index = SignedDecimal::from_str(vault_pnl_index).unwrap();

    let result = query_user_pnl(&storage, &user, Uint128::new(user_shares), vault_pnl_index);

    if expect_overflow {
        assert!(result.is_err(), "Expected an overflow error but got a successful result");
    } else {
        let user_pnl = result.unwrap();
        assert_eq!(user_pnl, Int128::new(expected_pnl));
    }
}

#[test]
fn test_edge_cases() {
    let storage = MockStorage::new();

    // test zero shares
    let (pnl_index, pnl_delta) =
        query_current_vault_pnl_index(&storage, Uint128::new(1_000_000), Uint128::zero()).unwrap();
    assert_eq!(pnl_index, SignedDecimal::zero());
    assert_eq!(pnl_delta, Int128::zero());

    // test max values
    let max_uint = Uint128::MAX;
    let result = query_current_vault_pnl_index(&storage, max_uint, Uint128::new(1_000_000));
    assert!(result.is_ok());

    // test overflow scenarios
    let user = Addr::unchecked("user");
    let result =
        query_user_pnl(&storage, &user, max_uint, SignedDecimal::from_str("1000.0").unwrap());
    assert!(result.is_err());
}

#[test_case(1000000, 100000000, 1100000, 100000, "1000", false; "basic profit scenario - $1 to $1.1")]
#[test_case(1000000, 100000000, 900000, -100000, "-1000", false; "basic loss scenario - $1 to $0.9")]
#[test_case(1000000, 100000000, 1000000, 0, "0", false; "no change scenario - $1")]
#[test_case(0, 100000000, 1000000, 1000000, "10000", false; "starting from zero to $1")]
#[test_case(1000000000, 10000000000000000, 1100000000, 100000000, "0.01", false; "large profit scenario - $1000 to $1100")]
#[test_case(1000000000, 10000000000000000, 900000000, -100000000, "-0.01", false; "large loss scenario - $1000 to $900")]
#[test_case(1000000000000, 100000000000000, 1100000000000, 100000000000, "1000", false; "very large profit scenario - $1M to $1.1M")]
#[test_case(1701411834604, 1701411834604692317316873037, 1701411934604, 100000, "0.000000000000000058", false; "large shares tiny pnl delta")] // at this size we start to incurr rounding errors
#[test_case(MAX_INT128, 1, u128::MAX, 0, "", true; "overflow test - extremely large numbers")]
fn test_vault_pnl_index_scenarios(
    initial_net_worth: u128,
    total_shares: u128,
    new_net_worth: u128,
    expected_pnl_delta: i128,
    expected_pnl_index: &str,
    expect_overflow: bool,
) {
    let mut storage = MockStorage::new();

    LAST_NET_WORTH.save(&mut storage, &Uint128::new(initial_net_worth)).unwrap();

    let result = query_current_vault_pnl_index(
        &storage,
        Uint128::new(new_net_worth),
        Uint128::new(total_shares),
    );

    if expect_overflow {
        assert!(result.is_err(), "Expected an overflow error but got a successful result");
    } else {
        let (pnl_index, pnl_delta) = result.unwrap();
        assert_eq!(pnl_delta, Int128::new(expected_pnl_delta));
        assert_eq!(pnl_index, SignedDecimal::from_str(expected_pnl_index).unwrap());
    }
}

proptest! {

    #[test]
    fn prop_vault_pnl_index_calculation(
        // test with a wide range of initial net worth values. We can be confident that we can handle up to Int128::MAX for net worth values
        initial_net_worth in 1u128..=std::cmp::min(u128::MAX / 2, Int128::MAX.i128() as u128),
        // test with a wide range of share values. We can be confident that we can handle up to Int128::MAX for share values
        total_shares in 100000u128..=std::cmp::min(u128::MAX / 2, Int128::MAX.i128() as u128),
        // test with various net worth changes, both positive and negative
        net_worth_change in -1_000_000_000_000i128..1_000_000_000_000i128
    ) {
        // set up the test
        let mut storage = MockStorage::new();

        // save the initial net worth
        LAST_NET_WORTH.save(&mut storage, &Uint128::new(initial_net_worth)).unwrap();

        // calculate the new net worth, ensuring we don't underflow or overflow
        let new_net_worth = if net_worth_change < 0 && initial_net_worth < net_worth_change.unsigned_abs() {
            // if net_worth_change is negative and its absolute value is greater than initial_net_worth,
            // set new_net_worth to 0 to avoid underflow
            0u128
        } else if net_worth_change > 0 && initial_net_worth > u128::MAX - net_worth_change as u128 {
            // if adding net_worth_change would overflow, cap at u128::MAX
            u128::MAX
        } else {
            // otherwise, safely calculate the new net worth
            (initial_net_worth as i128 + net_worth_change).max(0) as u128
        };

        // call the function we're testing
        let result = query_current_vault_pnl_index(
            &storage,
            Uint128::new(new_net_worth),
            Uint128::new(total_shares)
        );

        // if we get a successful result, verify the properties
        if let Ok((pnl_index, pnl_delta)) = result {
            // property 1: pnl delta should match the difference between new and initial net worth
            let expected_delta = if new_net_worth >= initial_net_worth {
                Int128::new((new_net_worth - initial_net_worth).try_into().unwrap_or(i128::MAX))
            } else {
                -Int128::new((initial_net_worth - new_net_worth).try_into().unwrap_or(i128::MAX))
            };
            prop_assert_eq!(pnl_delta, expected_delta);

            // property 2: if total_shares is 0, pnl index should be 0
            if total_shares == 0 {
                prop_assert_eq!(pnl_index, SignedDecimal::zero());
            } else {

                // property 3: pnl index calculation should be accurate
                // pnl index = pnl delta / total_shares
                let expected_pnl_index = SignedDecimal::from_ratio(pnl_delta.checked_mul(Int128::from(1_000_000i128)).unwrap(), i128_from_u128(Uint128::new(total_shares)).unwrap());

                // we may have minor rounding differences, so check that they're close
                let diff = if expected_pnl_index > pnl_index {
                    expected_pnl_index - pnl_index
                } else {
                    pnl_index - expected_pnl_index
                };

                // allow for a small epsilon due to decimal precision
                let epsilon = SignedDecimal::from_str("0.00000001").unwrap();
                prop_assert!(diff < epsilon, "pnl index calculation mismatch: expected {}, got {}", expected_pnl_index, pnl_index);
            }
        } else {
            // if the function returns an error, it should be due to overflow/underflow conditions
            // this is expected in extreme cases, so we just verify it's happening in reasonable conditions

            // add additional logging for when errors occur
            if new_net_worth > u128::MAX / 2 || initial_net_worth > u128::MAX / 2 || total_shares < 10000 {
                // these are conditions where errors are reasonable
            } else {
                // otherwise, we should investigate why an error occurred
                prop_assert!(
                    false,
                    "Unexpected error for reasonable values: initial_net_worth={}, new_net_worth={}, total_shares={}, error: {:?}",
                    initial_net_worth, new_net_worth, total_shares, result.unwrap_err()
                );
            }
        }
    }
}
