use cosmwasm_std::{Addr, Coin, Decimal, Int128, Uint128};
use cw_multi_test::AppResponse;
use mars_types::{
    credit_manager::{
        Action::{Deposit, ExecutePerpOrder},
        ExecutePerpOrderType,
    },
    params::PerpParamsUpdate,
};
use test_case::test_case;

use super::helpers::{coin_info, uatom_info, AccountToFund, MockEnv};
use crate::tests::helpers::default_perp_params;

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
    "tier_10",
    Decimal::percent(0),
    200;
    "tier 10: 0 power -> 0% discount, size 200"
)]
#[test_case(
    25_000,
    "tier_5", 
    Decimal::percent(25),
    200;
    "tier 5: >= 25_000 power -> 25% discount, size 200"
)]
#[test_case(
    200_000,
    "tier_2",
    Decimal::percent(60),
    200;
    "tier 2: >= 200_000 power -> 60% discount, size 200"
)]
#[test_case(
    50_000,
    "tier_4",
    Decimal::percent(35),
    100;
    "tier 4: >= 50_000 power -> 35% discount, size 100"
)]
#[test_case(
    150_000,
    "tier_3",
    Decimal::percent(45),
    500;
    "tier 3: >= 150_000 power -> 45% discount, size 500"
)]
#[test_case(
    10_000,
    "tier_6",
    Decimal::percent(15),
    150;
    "tier 6: >= 10_000 power -> 15% discount, size 150"
)]
#[test_case(
    5_000,
    "tier_7",
    Decimal::percent(10),
    300;
    "tier 7: >= 5_000 power -> 10% discount, size 300"
)]
#[test_case(
    1_000,
    "tier_8",
    Decimal::percent(5),
    400;
    "tier 8: >= 1_000 power -> 5% discount, size 400"
)]
#[test_case(
    100,
    "tier_9",
    Decimal::percent(1),
    600;
    "tier 9: >= 100 power -> 1% discount, size 600"
)]
#[test_case(
    350_000,
    "tier_1",
    Decimal::percent(75),
    800;
    "tier 1: >= 350_000 power -> 75% discount, size 800"
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
    "tier_10",
    Decimal::percent(0);
    "close_perp_position tier 10: 0 power -> 0% discount"
)]
#[test_case(
    25_000,
    "tier_5",
    Decimal::percent(25);
    "close_perp_position tier 5: >= 25_000 power -> 25% discount"
)]
#[test_case(
    200_000,
    "tier_2",
    Decimal::percent(60);
    "close_perp_position tier 2: >= 200_000 power -> 60% discount"
)]
#[test_case(
    50_000,
    "tier_4",
    Decimal::percent(35);
    "close_perp_position tier 4: >= 50_000 power -> 35% discount"
)]
#[test_case(
    100_000,
    "tier_3",
    Decimal::percent(45);
    "close_perp_position tier 3: >= 100_000 power -> 45% discount"
)]
#[test_case(
    350_000,
    "tier_1",
    Decimal::percent(75);
    "close_perp_position tier 1: >= 350_000 power -> 75% discount"
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
    "tier_10",
    Decimal::percent(0);
    "multiple positions tier 10: 0 power -> 0% discount"
)]
#[test_case(
    25_000,
    "tier_5",
    Decimal::percent(25);
    "multiple positions tier 5: >= 25_000 power -> 25% discount"
)]
#[test_case(
    200_000,
    "tier_2",
    Decimal::percent(60);
    "multiple positions tier 2: >= 200_000 power -> 60% discount"
)]
#[test_case(
    50_000,
    "tier_4",
    Decimal::percent(35);
    "multiple positions tier 4: >= 50_000 power -> 35% discount"
)]
#[test_case(
    100_000,
    "tier_3",
    Decimal::percent(45);
    "multiple positions tier 3: >= 100_000 power -> 45% discount"
)]
#[test_case(
    350_000,
    "tier_1",
    Decimal::percent(75);
    "multiple positions tier 1: >= 350_000 power -> 75% discount"
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
