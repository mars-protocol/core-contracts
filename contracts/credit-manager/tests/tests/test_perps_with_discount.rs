use std::str::FromStr;

use cosmwasm_std::{Addr, Coin, Decimal, Int128, Uint128};
use cw_multi_test::AppResponse;
use mars_types::{
    credit_manager::{
        Action::{ClosePerpPosition, Deposit, ExecutePerpOrder},
        ExecutePerpOrderType,
    },
    params::PerpParamsUpdate,
};
use test_case::test_case;

use super::helpers::{coin_info, uatom_info, AccountToFund, MockEnv};
use crate::tests::helpers::{default_perp_params, get_coin};

fn setup_env() -> (MockEnv, Addr, String) {
    let atom = uatom_info();
    let usdc = coin_info("uusdc");
    let user = Addr::unchecked("user");

    let mut mock = MockEnv::new()
        .set_params(&[atom.clone(), usdc.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![Coin::new(100_000, usdc.denom.clone())],
        })
        .build()
        .unwrap();

    let account_id = mock.create_credit_account(&user).unwrap();

    // Setup perps params for the market and seed vault liquidity via CM
    mock.update_perp_params(PerpParamsUpdate::AddOrUpdate {
        params: default_perp_params(&atom.denom),
    });
    // Fund the CM (rover) with USDC and deposit into perps vault so opening fees can be handled
    let rover_addr = mock.rover.clone();
    let usdc_denom = usdc.denom.clone();
    mock.fund_addr(&rover_addr, vec![Coin::new(100_0000, usdc_denom.clone())]);
    mock.deposit_to_perp_vault(&account_id, &Coin::new(50_0000, usdc_denom), None).unwrap();
    (mock, user, account_id)
}

fn open_perp(
    mock: &mut MockEnv,
    account_id: &str,
    user: &Addr,
    denom: &str,
    size: Int128,
    usdc_to_deposit: u128,
) -> AppResponse {
    // Ensure some USDC deposit to pay opening fee
    mock.update_credit_account(
        account_id,
        user,
        vec![
            Deposit(Coin::new(usdc_to_deposit, "uusdc")),
            ExecutePerpOrder {
                denom: denom.to_string(),
                order_size: size,
                reduce_only: None,
                order_type: Some(ExecutePerpOrderType::Default),
            },
        ],
        &[Coin::new(usdc_to_deposit, "uusdc")],
    )
    .unwrap()
}

// Test fee discount across different voting power tiers and scenarios
#[test_case(
    0,
    "tier_1",
    Decimal::percent(0),
    200;
    "tier 1: 0 power -> 0% discount, size 200"
)]
#[test_case(
    250_000_000_000,
    "tier_5", 
    Decimal::percent(45),
    200;
    "tier 5: >= 250_000 MARS power -> 45% discount, size 200"
)]
#[test_case(
    1_000_000_000_000,
    "tier_7",
    Decimal::percent(70),
    200;
    "tier 7: >= 1_000_000 MARS power -> 70% discount, size 200"
)]
#[test_case(
    100_000_000_000,
    "tier_4",
    Decimal::percent(30),
    100;
    "tier 4: >= 100_000 MARS power -> 30% discount, size 100"
)]
#[test_case(
    500_000_000_000,
    "tier_6",
    Decimal::percent(60),
    500;
    "tier 6: >= 500_000 MARS power -> 60% discount, size 500"
)]
#[test_case(
    10_000_000_000,
    "tier_2",
    Decimal::percent(10),
    150;
    "tier 2: >= 10_000 MARS power -> 10% discount, size 150"
)]
#[test_case(
    50_000_000_000,
    "tier_3",
    Decimal::percent(20),
    300;
    "tier 3: >= 50_000 MARS power -> 20% discount, size 300"
)]
#[test_case(
    1_500_000_000_000,
    "tier_8",
    Decimal::percent(80),
    400;
    "tier 8: >= 1_500_000 MARS power -> 80% discount, size 400"
)]
fn test_perps_with_discount_events(
    voting_power: u128,
    expected_tier: &str,
    expected_discount: Decimal,
    position_size: i128,
) {
    let (mut mock, user, account_id) = setup_env();
    let atom = uatom_info();

    // Set voting power for this test case
    mock.set_voting_power(&user, Uint128::new(voting_power));

    let find_attrs = |res: &AppResponse| {
        let evt = res
            .events
            .iter()
            .find(|e| e.attributes.iter().any(|a| a.key == "discount_pct"))
            .or_else(|| {
                res.events.iter().find(|e| {
                    e.attributes.iter().any(|a| {
                        a.key == "action"
                            && (a.value == "open_perp_position" || a.value == "execute_perp_order")
                    })
                })
            })
            .expect("expected perps event with discount attributes");
        evt.attributes
            .iter()
            .map(|a| (a.key.clone(), a.value.clone()))
            .collect::<std::collections::HashMap<_, _>>()
    };

    // Open perp position and verify discount attributes
    let res =
        open_perp(&mut mock, &account_id, &user, &atom.denom, Int128::new(position_size), 10_000);
    let attrs = find_attrs(&res);

    assert_eq!(attrs.get("voting_power").unwrap(), &voting_power.to_string());
    assert_eq!(attrs.get("tier_id").unwrap(), expected_tier);
    assert_eq!(attrs.get("discount_pct").unwrap(), &expected_discount.to_string());
    assert_eq!(attrs.get("new_size").unwrap(), &position_size.to_string());
}

// Test close_perp_position with discount functionality
#[test_case(
    0,
    "tier_1",
    Decimal::percent(0);
    "close_perp_position tier 1: 0 power -> 0% discount"
)]
#[test_case(
    250_000_000_000,
    "tier_5",
    Decimal::percent(45);
    "close_perp_position tier 5: >= 250_000 MARS power -> 45% discount"
)]
#[test_case(
    1_000_000_000_000,
    "tier_7",
    Decimal::percent(70);
    "close_perp_position tier 7: >= 1_000_000 MARS power -> 70% discount"
)]
#[test_case(
    100_000_000_000,
    "tier_4",
    Decimal::percent(30);
    "close_perp_position tier 4: >= 100_000 MARS power -> 30% discount"
)]
#[test_case(
    500_000_000_000,
    "tier_6",
    Decimal::percent(60);
    "close_perp_position tier 6: >= 500_000 MARS power -> 60% discount"
)]
#[test_case(
    1_500_000_000_000,
    "tier_8",
    Decimal::percent(80);
    "close_perp_position tier 8: >= 1_500_000 MARS power -> 80% discount"
)]
fn test_close_perp_position_with_discount(
    voting_power: u128,
    _expected_tier: &str,
    expected_discount: Decimal,
) {
    let (mut mock, user, account_id) = setup_env();
    let atom = uatom_info();

    // Set voting power for this test case
    mock.set_voting_power(&user, Uint128::new(voting_power));

    // Open a position first
    let res = open_perp(&mut mock, &account_id, &user, &atom.denom, Int128::new(200), 10_000);
    let attrs = find_attrs(&res);
    assert_eq!(attrs.get("action").unwrap(), "open_perp_position");
    assert_eq!(attrs.get("discount_pct").unwrap(), &expected_discount.to_string());

    // Now close the position and verify discount is applied
    let close_res = mock
        .update_credit_account(
            &account_id,
            &user,
            vec![mars_types::credit_manager::Action::ClosePerpPosition {
                denom: atom.denom.clone(),
            }],
            &[],
        )
        .unwrap();

    // Find the execute_perp_order event (which is what close_perp_position actually emits)
    let close_attrs = close_res
        .events
        .iter()
        .find(|e| e.attributes.iter().any(|a| a.key == "action" && a.value == "execute_perp_order"))
        .expect("expected execute_perp_order event from close_perp_position")
        .attributes
        .iter()
        .map(|a| (a.key.clone(), a.value.clone()))
        .collect::<std::collections::HashMap<_, _>>();

    // Verify discount attributes are present in the execute_perp_order event
    assert_eq!(close_attrs.get("discount_pct").unwrap(), &expected_discount.to_string());
    assert_eq!(close_attrs.get("reduce_only").unwrap(), "true"); // close_perp_position sets reduce_only=true
    assert_eq!(close_attrs.get("order_size").unwrap(), "-200"); // negative size to close position
    assert_eq!(close_attrs.get("new_size").unwrap(), "0"); // position should be closed (size 0)
}

// Test multiple perp positions with discount functionality
#[test_case(
    0,
    "tier_1",
    Decimal::percent(0);
    "multiple positions tier 1: 0 power -> 0% discount"
)]
#[test_case(
    250_000_000_000,
    "tier_5",
    Decimal::percent(45);
    "multiple positions tier 5: >= 250_000 MARS power -> 45% discount"
)]
#[test_case(
    1_000_000_000_000,
    "tier_7",
    Decimal::percent(70);
    "multiple positions tier 7: >= 1_000_000 MARS power -> 70% discount"
)]
#[test_case(
    100_000_000_000,
    "tier_4",
    Decimal::percent(30);
    "multiple positions tier 4: >= 100_000 MARS power -> 30% discount"
)]
#[test_case(
    500_000_000_000,
    "tier_6",
    Decimal::percent(60);
    "multiple positions tier 6: >= 500_000 MARS power -> 60% discount"
)]
#[test_case(
    1_500_000_000_000,
    "tier_8",
    Decimal::percent(80);
    "multiple positions tier 8: >= 1_500_000 MARS power -> 80% discount"
)]
fn test_multiple_perp_positions_with_discount(
    voting_power: u128,
    expected_tier: &str,
    expected_discount: Decimal,
) {
    let (mut mock, user, account_id) = setup_env();
    let atom = uatom_info();

    // Set voting power for this test case
    mock.set_voting_power(&user, Uint128::new(voting_power));

    // Create additional credit accounts for multiple positions
    let account_id_2 = mock.create_credit_account(&user).unwrap();
    let account_id_3 = mock.create_credit_account(&user).unwrap();

    // Fund additional accounts with USDC for perps vault
    let rover_addr = mock.rover.clone();
    let usdc_denom = "uusdc".to_string();
    mock.fund_addr(&rover_addr, vec![Coin::new(100_0000, usdc_denom.clone())]);
    mock.deposit_to_perp_vault(&account_id_2, &Coin::new(50_0000, usdc_denom.clone()), None)
        .unwrap();
    mock.deposit_to_perp_vault(&account_id_3, &Coin::new(50_0000, usdc_denom), None).unwrap();

    // Open first position on account_id
    let res1 = open_perp(&mut mock, &account_id, &user, &atom.denom, Int128::new(200), 10_000);
    let attrs1 = find_attrs(&res1);
    assert_eq!(attrs1.get("action").unwrap(), "open_perp_position");
    assert_eq!(attrs1.get("discount_pct").unwrap(), &expected_discount.to_string());
    assert_eq!(attrs1.get("tier_id").unwrap(), expected_tier);
    assert_eq!(attrs1.get("voting_power").unwrap(), &voting_power.to_string());

    // Open a second position on account_id_2
    let res2 = open_perp(&mut mock, &account_id_2, &user, &atom.denom, Int128::new(100), 10_000);
    let attrs2 = find_attrs(&res2);
    assert_eq!(attrs2.get("action").unwrap(), "open_perp_position");
    assert_eq!(attrs2.get("discount_pct").unwrap(), &expected_discount.to_string());
    assert_eq!(attrs2.get("tier_id").unwrap(), expected_tier);
    assert_eq!(attrs2.get("voting_power").unwrap(), &voting_power.to_string());

    // Close first position and verify discount is applied
    let close_res1 = mock
        .update_credit_account(
            &account_id,
            &user,
            vec![mars_types::credit_manager::Action::ClosePerpPosition {
                denom: atom.denom.clone(),
            }],
            &[],
        )
        .unwrap();

    // Find the execute_perp_order event from closing the first position
    let close_attrs1 = close_res1
        .events
        .iter()
        .find(|e| e.attributes.iter().any(|a| a.key == "action" && a.value == "execute_perp_order"))
        .expect("expected execute_perp_order event from close_perp_position")
        .attributes
        .iter()
        .map(|a| (a.key.clone(), a.value.clone()))
        .collect::<std::collections::HashMap<_, _>>();

    // Verify discount attributes are present in the close event
    assert_eq!(close_attrs1.get("discount_pct").unwrap(), &expected_discount.to_string());
    assert_eq!(close_attrs1.get("reduce_only").unwrap(), "true");
    assert_eq!(close_attrs1.get("order_size").unwrap(), "-200"); // negative to close 200 size

    // Close second position and verify discount is applied
    let close_res2 = mock
        .update_credit_account(
            &account_id_2,
            &user,
            vec![mars_types::credit_manager::Action::ClosePerpPosition {
                denom: atom.denom.clone(),
            }],
            &[],
        )
        .unwrap();

    // Find the execute_perp_order event from closing the second position
    let close_attrs2 = close_res2
        .events
        .iter()
        .find(|e| e.attributes.iter().any(|a| a.key == "action" && a.value == "execute_perp_order"))
        .expect("expected execute_perp_order event from close_perp_position")
        .attributes
        .iter()
        .map(|a| (a.key.clone(), a.value.clone()))
        .collect::<std::collections::HashMap<_, _>>();

    // Verify discount attributes are present in the second close event
    assert_eq!(close_attrs2.get("discount_pct").unwrap(), &expected_discount.to_string());
    assert_eq!(close_attrs2.get("reduce_only").unwrap(), "true");
    assert_eq!(close_attrs2.get("order_size").unwrap(), "-100"); // negative to close 100 size
}

// Helper function to find attributes (extracted to avoid duplication)
fn find_attrs(res: &AppResponse) -> std::collections::HashMap<String, String> {
    // First try to find any perps event with discount_pct (including "0")
    let evt = res
        .events
        .iter()
        .find(|e| e.attributes.iter().any(|a| a.key == "discount_pct"))
        .or_else(|| {
            // Fallback: look for perps events by action type
            res.events.iter().find(|e| {
                e.attributes.iter().any(|a| {
                    a.key == "action"
                        && (a.value == "open_perp_position" || a.value == "execute_perp_order")
                })
            })
        })
        .expect("expected perps event with discount attributes");

    evt.attributes
        .iter()
        .map(|a| (a.key.clone(), a.value.clone()))
        .collect::<std::collections::HashMap<_, _>>()
}

// Helper function to execute perp order with tier validation
fn execute_perp_order_with_tier_validation(
    mock: &mut MockEnv,
    account_id: &str,
    user: &Addr,
    denom: &str,
    size: Int128,
    expected_tier: &str,
    expected_discount: Decimal,
    expected_action: &str,
) -> AppResponse {
    let res = mock
        .update_credit_account(
            account_id,
            user,
            vec![ExecutePerpOrder {
                denom: denom.to_string(),
                order_size: size,
                reduce_only: None,
                order_type: Some(ExecutePerpOrderType::Default),
            }],
            &[],
        )
        .unwrap();

    let attrs = find_attrs(&res);
    assert_eq!(attrs.get("action").unwrap(), expected_action);
    assert_eq!(attrs.get("discount_pct").unwrap(), &expected_discount.to_string());
    assert_eq!(attrs.get("tier_id").unwrap(), expected_tier);

    res
}

// Helper function to assert vault balance increase
fn assert_vault_balance_increase(
    mock: &MockEnv,
    previous_balance: Uint128,
    expected_fee: Uint128,
) -> Uint128 {
    let current_balance = mock.query_balance(mock.perps.address(), "uusdc");
    let expected_balance = previous_balance + expected_fee;
    assert_eq!(current_balance.amount, expected_balance);
    current_balance.amount
}

// Helper function to assert position size
fn assert_position_size(mock: &MockEnv, account_id: &str, denom: &str, expected_size: Int128) {
    let position = mock.query_perp_position(account_id, denom);
    assert!(position.position.is_some());
    assert_eq!(position.position.unwrap().size, expected_size);
}

// Helper function to query and validate opening fee
fn query_and_validate_opening_fee(
    mock: &MockEnv,
    denom: &str,
    size: Int128,
    discount_pct: Decimal,
    expected_rate: Decimal,
    expected_fee: Uint128,
) -> Uint128 {
    let opening_fee = mock.query_perp_opening_fee(denom, size, Some(discount_pct));
    assert_eq!(opening_fee.rate, expected_rate);
    assert_eq!(opening_fee.fee.amount, expected_fee);
    opening_fee.fee.amount
}

#[test]
fn test_perp_position_modification_with_tier_changes() {
    // Test scenario: Open position with no discount, then modify with different discount tiers
    // This validates that discount tiers are correctly applied during position lifecycle

    let (mut mock, user, account_id) = setup_env();
    let atom = uatom_info();

    // Setup: Deposit USDC for position health
    mock.update_credit_account(
        &account_id,
        &user,
        vec![Deposit(Coin::new(50_000, "uusdc"))],
        &[Coin::new(50_000, "uusdc")],
    )
    .unwrap();

    let initial_vault_balance = mock.query_balance(mock.perps.address(), "uusdc").amount;
    let query_market_acct = |mock: &MockEnv| -> mars_types::perps::AccountingResponse {
        mock.app
            .wrap()
            .query_wasm_smart(
                mock.perps.address(),
                &mars_types::perps::QueryMsg::MarketAccounting {
                    denom: atom.denom.clone(),
                },
            )
            .unwrap()
    };

    // Step 1: Open initial position with Tier 1 (0% discount)
    mock.set_voting_power(&user, Uint128::new(0));

    // Check user balance before opening position
    let position_before_1 = mock.query_positions(&account_id);
    let user_usdc_balance_before_1 = get_coin("uusdc", &position_before_1.deposits).amount;

    let opening_fee_1 = query_and_validate_opening_fee(
        &mock,
        &atom.denom,
        Int128::new(200),
        Decimal::percent(0),
        Decimal::percent(1),
        Uint128::new(9),
    );

    execute_perp_order_with_tier_validation(
        &mut mock,
        &account_id,
        &user,
        &atom.denom,
        Int128::new(200),
        "tier_1",
        Decimal::percent(0),
        "open_perp_position",
    );

    // Verify user paid the correct fee (balance should decrease by opening fee)
    let position_after_1 = mock.query_positions(&account_id);
    let user_usdc_balance_after_1 = get_coin("uusdc", &position_after_1.deposits).amount;
    assert_eq!(user_usdc_balance_after_1, user_usdc_balance_before_1 - opening_fee_1);

    assert_position_size(&mock, &account_id, &atom.denom, Int128::new(200));
    let vault_balance_1 =
        assert_vault_balance_increase(&mock, initial_vault_balance, opening_fee_1);

    // Market accounting after step 1
    let acct_after_1 = query_market_acct(&mock);
    assert_eq!(
        acct_after_1.accounting.cash_flow.opening_fee,
        Int128::try_from(opening_fee_1).unwrap()
    );
    assert_eq!(acct_after_1.accounting.cash_flow.price_pnl, Int128::zero());
    assert_eq!(acct_after_1.accounting.cash_flow.accrued_funding, Int128::zero());

    // Step 2: Increase position with Tier 2 (10% discount)
    mock.set_voting_power(&user, Uint128::new(10_000_000_000)); // 10,000 MARS

    // Check user balance before increasing position
    let position_before_2 = mock.query_positions(&account_id);
    let user_usdc_balance_before_2 = get_coin("uusdc", &position_before_2.deposits).amount;

    let opening_fee_2 = query_and_validate_opening_fee(
        &mock,
        &atom.denom,
        Int128::new(200),
        Decimal::percent(10),
        Decimal::from_str("0.009").unwrap(),
        Uint128::new(8),
    );

    execute_perp_order_with_tier_validation(
        &mut mock,
        &account_id,
        &user,
        &atom.denom,
        Int128::new(200),
        "tier_2",
        Decimal::percent(10),
        "execute_perp_order",
    );

    // Verify user paid the correct discounted fee (balance should decrease by discounted opening fee)
    let position_after_2 = mock.query_positions(&account_id);
    let user_usdc_balance_after_2 = get_coin("uusdc", &position_after_2.deposits).amount;
    assert_eq!(user_usdc_balance_after_2, user_usdc_balance_before_2 - opening_fee_2);

    let expected_total_size = Int128::new(200 + 200);
    assert_position_size(&mock, &account_id, &atom.denom, expected_total_size);
    let vault_balance_2 = assert_vault_balance_increase(&mock, vault_balance_1, opening_fee_2);

    // Market accounting after step 2 (opening fees accumulate)
    let acct_after_2 = query_market_acct(&mock);
    let expected_opening_fees_1_2 = opening_fee_1 + opening_fee_2;
    assert_eq!(
        acct_after_2.accounting.cash_flow.opening_fee,
        Int128::try_from(expected_opening_fees_1_2).unwrap()
    );
    assert_eq!(acct_after_2.accounting.cash_flow.price_pnl, Int128::zero());
    assert_eq!(acct_after_2.accounting.cash_flow.accrued_funding, Int128::zero());

    // Step 3: Increase position with Tier 5 (45% discount)
    mock.set_voting_power(&user, Uint128::new(250_000_000_000)); // 250,000 MARS

    // Check user balance before increasing position
    let position_before_3 = mock.query_positions(&account_id);
    let user_usdc_balance_before_3 = get_coin("uusdc", &position_before_3.deposits).amount;

    let opening_fee_3 = query_and_validate_opening_fee(
        &mock,
        &atom.denom,
        Int128::new(200),
        Decimal::percent(45),
        Decimal::from_str("0.0055").unwrap(),
        Uint128::new(5),
    );

    execute_perp_order_with_tier_validation(
        &mut mock,
        &account_id,
        &user,
        &atom.denom,
        Int128::new(200),
        "tier_5",
        Decimal::percent(45),
        "execute_perp_order",
    );

    // Verify user paid the correct discounted fee (balance should decrease by discounted opening fee)
    let position_after_3 = mock.query_positions(&account_id);
    let user_usdc_balance_after_3 = get_coin("uusdc", &position_after_3.deposits).amount;
    assert_eq!(user_usdc_balance_after_3, user_usdc_balance_before_3 - opening_fee_3);

    let final_expected_size = Int128::new(200 + 200 + 200);
    assert_position_size(&mock, &account_id, &atom.denom, final_expected_size);
    let vault_balance_3 = assert_vault_balance_increase(&mock, vault_balance_2, opening_fee_3);

    // Market accounting after step 3 (opening fees accumulate)
    let acct_after_3 = query_market_acct(&mock);
    let expected_opening_fees_total = opening_fee_1 + opening_fee_2 + opening_fee_3;
    assert_eq!(
        acct_after_3.accounting.cash_flow.opening_fee,
        Int128::try_from(expected_opening_fees_total).unwrap()
    );
    assert_eq!(acct_after_3.accounting.cash_flow.price_pnl, Int128::zero());
    assert_eq!(acct_after_3.accounting.cash_flow.accrued_funding, Int128::zero());

    // Step 4: Validate total fees collected and user balance changes
    let total_fees_collected = vault_balance_3 - initial_vault_balance;
    let expected_total_fees = opening_fee_1 + opening_fee_2 + opening_fee_3;
    assert_eq!(total_fees_collected, expected_total_fees);

    // Verify total user balance decrease equals total fees paid
    let total_user_balance_decrease = user_usdc_balance_before_1 - user_usdc_balance_after_3;
    assert_eq!(total_user_balance_decrease, expected_total_fees);

    // Step 5: Close position with current tier (Tier 5)
    // Pre-calc expected closing fee from query
    let closing_fee_estimate: mars_types::perps::PositionFeesResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            mock.perps.address(),
            &mars_types::perps::QueryMsg::PositionFees {
                account_id: account_id.clone(),
                denom: atom.denom.clone(),
                new_size: Int128::zero(),
            },
        )
        .unwrap();
    let close_res = mock
        .update_credit_account(
            &account_id,
            &user,
            vec![ClosePerpPosition {
                denom: atom.denom.clone(),
            }],
            &[],
        )
        .unwrap();

    let close_attrs = find_attrs(&close_res);
    assert_eq!(close_attrs.get("action").unwrap(), "execute_perp_order");
    assert_eq!(close_attrs.get("discount_pct").unwrap(), &Decimal::percent(45).to_string());
    assert_eq!(close_attrs.get("tier_id").unwrap(), "tier_5");

    // Verify position is closed
    let position_after_close = mock.query_perp_position(&account_id, &atom.denom);
    assert!(position_after_close.position.is_none());

    // Market accounting after close: opening fees + closing fee must be realized, no unrealized pnl
    let acct_after_close = query_market_acct(&mock);
    let expected_opening_total_i = Int128::try_from(expected_total_fees).unwrap();
    assert_eq!(acct_after_close.accounting.cash_flow.opening_fee, expected_opening_total_i);
    assert_eq!(acct_after_close.accounting.cash_flow.price_pnl, Int128::zero());
    assert_eq!(acct_after_close.accounting.cash_flow.accrued_funding, Int128::zero());
    // closing fee equals estimate
    assert_eq!(
        acct_after_close.accounting.cash_flow.closing_fee,
        Int128::try_from(closing_fee_estimate.closing_fee).unwrap()
    );
}

#[test]
fn test_perp_fee_discount_comparison_two_users() {
    // Test scenario: Compare fee discounts between two users with different voting power tiers
    // User 1: 0 voting power (tier 1, 0% discount)
    // User 2: 250,000 MARS voting power (tier 5, 45% discount)
    // Both users will: create position, increase position, reduce position, close position
    // After each operation, we'll query their balances to verify fee changes

    let atom = uatom_info();
    let usdc = coin_info("uusdc");

    // Setup two users
    let user1 = Addr::unchecked("user1");
    let user2 = Addr::unchecked("user2");

    let mut mock = MockEnv::new()
        .set_params(&[atom.clone(), usdc.clone()])
        .fund_account(AccountToFund {
            addr: user1.clone(),
            funds: vec![Coin::new(10_000_000, usdc.denom.clone())],
        })
        .fund_account(AccountToFund {
            addr: user2.clone(),
            funds: vec![Coin::new(10_000_000, usdc.denom.clone())],
        })
        .build()
        .unwrap();

    // Create credit accounts for both users
    let account_id_1 = mock.create_credit_account(&user1).unwrap();
    let account_id_2 = mock.create_credit_account(&user2).unwrap();

    // Setup perps params and fund vault
    mock.update_perp_params(PerpParamsUpdate::AddOrUpdate {
        params: default_perp_params(&atom.denom),
    });
    let rover_addr = mock.rover.clone();
    let usdc_denom = usdc.denom.clone();
    mock.fund_addr(&rover_addr, vec![Coin::new(200_000_000, usdc_denom.clone())]);
    mock.deposit_to_perp_vault(&account_id_1, &Coin::new(50_000_000, usdc_denom.clone()), None)
        .unwrap();
    mock.deposit_to_perp_vault(&account_id_2, &Coin::new(50_000_000, usdc_denom), None).unwrap();

    // Set voting power for both users
    mock.set_voting_power(&user1, Uint128::new(0)); // Tier 1: 0% discount
    mock.set_voting_power(&user2, Uint128::new(250_000_000_000)); // Tier 5: 45% discount

    // Deposit USDC into both users' credit accounts for position health
    mock.update_credit_account(
        &account_id_1,
        &user1,
        vec![Deposit(Coin::new(5_000_000, usdc.denom.clone()))],
        &[Coin::new(5_000_000, usdc.denom.clone())],
    )
    .unwrap();

    mock.update_credit_account(
        &account_id_2,
        &user2,
        vec![Deposit(Coin::new(5_000_000, usdc.denom.clone()))],
        &[Coin::new(5_000_000, usdc.denom.clone())],
    )
    .unwrap();

    let initial_vault_balance = mock.query_balance(mock.perps.address(), "uusdc").amount;

    // Helper function to get user USDC balance
    let get_user_usdc_balance = |mock: &MockEnv, account_id: &str| -> Uint128 {
        let position = mock.query_positions(account_id);
        get_coin("uusdc", &position.deposits).amount
    };

    // Helper function to execute perp order and return fee paid
    let execute_perp_and_track_fee = |mock: &mut MockEnv,
                                      account_id: &str,
                                      user: &Addr,
                                      size: i128|
     -> (AppResponse, Uint128, Uint128) {
        let balance_before = get_user_usdc_balance(mock, account_id);
        let res = mock
            .update_credit_account(
                account_id,
                user,
                vec![ExecutePerpOrder {
                    denom: atom.denom.clone(),
                    order_size: Int128::new(size),
                    reduce_only: None,
                    order_type: Some(ExecutePerpOrderType::Default),
                }],
                &[],
            )
            .unwrap();
        let balance_after = get_user_usdc_balance(mock, account_id);
        let fee_paid = balance_before - balance_after;
        (res, balance_before, fee_paid)
    };

    // Helper function to close position and return fee paid
    let close_position_and_track_fee =
        |mock: &mut MockEnv, account_id: &str, user: &Addr| -> (AppResponse, Uint128, Uint128) {
            let balance_before = get_user_usdc_balance(mock, account_id);
            let res = mock
                .update_credit_account(
                    account_id,
                    user,
                    vec![ClosePerpPosition {
                        denom: atom.denom.clone(),
                    }],
                    &[],
                )
                .unwrap();
            let balance_after = get_user_usdc_balance(mock, account_id);
            let fee_paid = balance_before - balance_after;
            (res, balance_before, fee_paid)
        };

    // Helper function to extract discount from response
    let extract_discount = |res: &AppResponse| -> Decimal {
        let attrs = find_attrs(res);
        Decimal::from_str(attrs.get("discount_pct").unwrap()).unwrap()
    };

    // OPERATION 1: Create initial positions (size: 200,000)
    // User 1 creates position with 0% discount (Tier 1)
    let (res1_create, _, fee_1_create) =
        execute_perp_and_track_fee(&mut mock, &account_id_1, &user1, 200000);
    let discount_1_create = extract_discount(&res1_create);
    assert_eq!(discount_1_create, Decimal::percent(0)); // Verify 0% discount for Tier 1

    // User 2 creates position with 45% discount (Tier 5)
    let (res2_create, _, fee_2_create) =
        execute_perp_and_track_fee(&mut mock, &account_id_2, &user2, 200000);
    let discount_2_create = extract_discount(&res2_create);
    assert_eq!(discount_2_create, Decimal::percent(45)); // Verify 45% discount for Tier 5

    // Verify User 2 paid significantly less fee due to discount
    assert!(fee_2_create < fee_1_create, "User 2 should pay less fee due to 45% discount");

    // Verify positions were created with correct sizes
    assert_position_size(&mock, &account_id_1, &atom.denom, Int128::new(200000));
    assert_position_size(&mock, &account_id_2, &atom.denom, Int128::new(200000));

    // OPERATION 2: Increase positions (size: +100,000)
    // User 1 increases position with 0% discount (Tier 1)
    let (res1_increase, _, fee_1_increase) =
        execute_perp_and_track_fee(&mut mock, &account_id_1, &user1, 100000);
    let discount_1_increase = extract_discount(&res1_increase);
    assert_eq!(discount_1_increase, Decimal::percent(0)); // Verify 0% discount maintained

    // User 2 increases position with 45% discount (Tier 5)
    let (res2_increase, _, fee_2_increase) =
        execute_perp_and_track_fee(&mut mock, &account_id_2, &user2, 100000);
    let discount_2_increase = extract_discount(&res2_increase);
    assert_eq!(discount_2_increase, Decimal::percent(45)); // Verify 45% discount maintained

    // Verify User 2 continues to pay less fee due to discount
    assert!(fee_2_increase < fee_1_increase, "User 2 should pay less fee due to 45% discount");

    // Verify positions were increased to correct total sizes (200k + 100k = 300k)
    assert_position_size(&mock, &account_id_1, &atom.denom, Int128::new(300000));
    assert_position_size(&mock, &account_id_2, &atom.denom, Int128::new(300000));

    // OPERATION 3: Reduce positions (size: -50,000)
    // User 1 reduces position with 0% discount (Tier 1)
    let (res1_reduce, _, fee_1_reduce) =
        execute_perp_and_track_fee(&mut mock, &account_id_1, &user1, -50000);
    let discount_1_reduce = extract_discount(&res1_reduce);
    assert_eq!(discount_1_reduce, Decimal::percent(0)); // Verify 0% discount maintained

    // User 2 reduces position with 45% discount (Tier 5)
    let (res2_reduce, _, fee_2_reduce) =
        execute_perp_and_track_fee(&mut mock, &account_id_2, &user2, -50000);
    let discount_2_reduce = extract_discount(&res2_reduce);
    assert_eq!(discount_2_reduce, Decimal::percent(45)); // Verify 45% discount maintained

    // Verify User 2 continues to pay less fee due to discount
    assert!(fee_2_reduce < fee_1_reduce, "User 2 should pay less fee due to 45% discount");

    // Verify positions were reduced to correct total sizes (300k - 50k = 250k)
    assert_position_size(&mock, &account_id_1, &atom.denom, Int128::new(250000));
    assert_position_size(&mock, &account_id_2, &atom.denom, Int128::new(250000));

    // OPERATION 4: Close positions (remaining 250,000)
    // User 1 closes position with 0% discount (Tier 1)
    let (res1_close, _, fee_1_close) =
        close_position_and_track_fee(&mut mock, &account_id_1, &user1);
    let discount_1_close = extract_discount(&res1_close);
    assert_eq!(discount_1_close, Decimal::percent(0)); // Verify 0% discount maintained

    // User 2 closes position with 45% discount (Tier 5)
    let (res2_close, _, fee_2_close) =
        close_position_and_track_fee(&mut mock, &account_id_2, &user2);
    let discount_2_close = extract_discount(&res2_close);
    assert_eq!(discount_2_close, Decimal::percent(45)); // Verify 45% discount maintained

    // Verify User 2 continues to pay less fee due to discount
    assert!(fee_2_close < fee_1_close, "User 2 should pay less fee due to 45% discount");

    // Verify positions were completely closed (size 0)
    let position_1_after_close = mock.query_perp_position(&account_id_1, &atom.denom);
    let position_2_after_close = mock.query_perp_position(&account_id_2, &atom.denom);
    assert!(position_1_after_close.position.is_none()); // User 1 position closed
    assert!(position_2_after_close.position.is_none()); // User 2 position closed

    // SUMMARY: Calculate and verify total fee differences
    // Calculate total fees paid by each user across all operations
    let total_fees_user1 = fee_1_create + fee_1_increase + fee_1_reduce + fee_1_close;
    let total_fees_user2 = fee_2_create + fee_2_increase + fee_2_reduce + fee_2_close;

    // Verify total fees collected by vault matches sum of both users' fees
    let final_vault_balance = mock.query_balance(mock.perps.address(), "uusdc").amount;
    let total_fees_collected = final_vault_balance - initial_vault_balance;
    let expected_total_fees = total_fees_user1 + total_fees_user2;
    assert_eq!(
        total_fees_collected, expected_total_fees,
        "Vault should have collected the sum of both users' fees"
    );

    // Verify that User 2's total fees are approximately 45% less than User 1's
    // (allowing for rounding differences due to fee calculations)
    let expected_user2_fees = total_fees_user1 * Decimal::percent(55); // 100% - 45% = 55%
    let fee_difference = if total_fees_user2 > expected_user2_fees {
        total_fees_user2 - expected_user2_fees
    } else {
        expected_user2_fees - total_fees_user2
    };

    // Allow for small rounding differences (within 2 units due to larger discount)
    assert!(fee_difference <= Uint128::new(2),
            "User 2's total fees should be approximately 45% less than User 1's. Expected: {}, Actual: {}, Difference: {}", 
            expected_user2_fees, total_fees_user2, fee_difference);

    // Verify the discount is working correctly by checking the percentage saved
    let actual_discount_percentage =
        ((total_fees_user1 - total_fees_user2) * Uint128::new(100)) / total_fees_user1;
    assert!(
        actual_discount_percentage >= Uint128::new(40)
            && actual_discount_percentage <= Uint128::new(50),
        "Actual discount should be close to 45%. Got: {}%",
        actual_discount_percentage
    );
}
