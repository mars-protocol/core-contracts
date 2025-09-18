use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use cw_multi_test::AppResponse;
use mars_types::{
    credit_manager::Action::{Deposit, SwapExactIn},
    swapper::{OsmoRoute, OsmoSwap, SwapperRoute},
};

use super::helpers::{uatom_info, uosmo_info, AccountToFund, MockEnv};

fn setup_env_with_swap_fee() -> (MockEnv, Addr, String) {
    let atom = uatom_info();
    let osmo = uosmo_info();
    let user = Addr::unchecked("user");

    let mut mock = MockEnv::new()
        .set_params(&[osmo.clone(), atom.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![Coin::new(30_000, atom.denom.clone())],
        })
        .swap_fee(Decimal::percent(1))
        .build()
        .unwrap();

    let account_id = mock.create_credit_account(&user).unwrap();
    (mock, user, account_id)
}

fn do_swap(
    mock: &mut MockEnv,
    account_id: &str,
    user: &Addr,
    amount: u128,
    denom_in: &str,
    denom_out: &str,
) -> AppResponse {
    let route = SwapperRoute::Osmo(OsmoRoute {
        swaps: vec![OsmoSwap {
            pool_id: 101,
            to: denom_out.to_string(),
        }],
    });
    let estimate = mock.query_swap_estimate(&Coin::new(amount, denom_in), denom_out, route.clone());
    let min_receive = estimate.amount - Uint128::one();
    let atom = uatom_info();
    mock.update_credit_account(
        account_id,
        user,
        vec![
            Deposit(Coin::new(amount, denom_in)),
            SwapExactIn {
                coin_in: atom.to_action_coin(amount),
                denom_out: denom_out.to_string(),
                min_receive,
                route: Some(route),
            },
        ],
        &[Coin::new(amount, denom_in)],
    )
    .unwrap()
}

#[test]
fn test_swap_with_discount() {
    let (mut mock, user, account_id) = setup_env_with_swap_fee();

    // Helper to extract attributes for the swapper event
    let extract = |res: &AppResponse| {
        res.events
            .iter()
            .find(|e| e.attributes.iter().any(|a| a.key == "action" && a.value == "swapper"))
            .unwrap()
            .attributes
            .iter()
            .map(|a| (a.key.clone(), a.value.clone()))
            .collect::<std::collections::HashMap<_, _>>()
    };

    // Helper to extract and parse fee values
    let extract_fees = |res: &AppResponse| {
        let attrs = extract(res);
        let base_fee = attrs.get("base_swap_fee").unwrap().parse::<Decimal>().unwrap();
        let effective_fee = attrs.get("effective_swap_fee").unwrap().parse::<Decimal>().unwrap();
        (base_fee, effective_fee)
    };

    // Tier 1 (min power 0) → 0% discount
    mock.set_voting_power(&user, Uint128::new(0));
    let res = do_swap(&mut mock, &account_id, &user, 10_000, "uatom", "uosmo");
    let attrs = extract(&res);
    assert_eq!(attrs.get("voting_power").unwrap(), "0");
    assert_eq!(attrs.get("tier_id").unwrap(), "tier_1");
    assert_eq!(attrs.get("discount_pct").unwrap(), &Decimal::percent(0).to_string());

    // Verify fees: no discount means base_fee == effective_fee
    let (base_fee, effective_fee) = extract_fees(&res);
    assert_eq!(base_fee, Decimal::percent(1)); // 1% base fee
    assert_eq!(effective_fee, Decimal::percent(1)); // No discount applied

    // Verify account balances for tier 1 (no discount)
    let positions = mock.query_positions(&account_id);
    assert_eq!(positions.deposits.len(), 1);
    let osmo_deposit = positions.deposits.iter().find(|d| d.denom == "uosmo").unwrap();
    assert!(osmo_deposit.amount > Uint128::zero());

    let atom_deposit = positions.deposits.iter().find(|d| d.denom == "uatom");
    assert!(atom_deposit.is_none());

    // Verify Credit Manager contract balances
    let rover_atom_balance = mock.query_balance(&mock.rover, "uatom");
    // Rover retains the effective swap fee in input denom (1% of 10,000 = 100)
    assert_eq!(rover_atom_balance.amount, Uint128::new(100));

    let rover_osmo_balance = mock.query_balance(&mock.rover, "uosmo");
    assert_eq!(rover_osmo_balance.amount, osmo_deposit.amount);

    // Verify user wallet balances - check that swap amount is correctly debited
    let user_atom_balance = mock.query_balance(&user, "uatom");
    assert_eq!(user_atom_balance.amount, Uint128::new(30_000 - 10_000)); // 20,000 ATOM remaining (10,000 debited for swap)

    let user_osmo_balance = mock.query_balance(&user, "uosmo");
    assert_eq!(user_osmo_balance.amount, Uint128::zero());

    // Tier 2 (>= 10_000 MARS) → 10% discount
    mock.set_voting_power(&user, Uint128::new(10_000_000_000));
    let res = do_swap(&mut mock, &account_id, &user, 10_000, "uatom", "uosmo");
    let attrs = extract(&res);
    assert_eq!(attrs.get("voting_power").unwrap(), "10000000000");
    assert_eq!(attrs.get("tier_id").unwrap(), "tier_2");
    assert_eq!(attrs.get("discount_pct").unwrap(), &Decimal::percent(10).to_string());

    // Verify fees: 10% discount means effective_fee = base_fee * (1 - 0.1) = base_fee * 0.9
    let (base_fee, effective_fee) = extract_fees(&res);
    assert_eq!(base_fee, Decimal::percent(1)); // 1% base fee
    assert_eq!(effective_fee, Decimal::percent(1) * (Decimal::one() - Decimal::percent(10))); // 0.9% effective fee

    // Verify account balances for tier 2 (10% discount)
    let positions = mock.query_positions(&account_id);
    assert_eq!(positions.deposits.len(), 1);
    let osmo_deposit = positions.deposits.iter().find(|d| d.denom == "uosmo").unwrap();
    assert!(osmo_deposit.amount > Uint128::zero());

    let atom_deposit = positions.deposits.iter().find(|d| d.denom == "uatom");
    assert!(atom_deposit.is_none());

    // Verify Credit Manager contract balances
    let rover_atom_balance = mock.query_balance(&mock.rover, "uatom");
    // Cumulative retained fee: 100 (first) + 90 (second with 0.9%) = 190
    assert_eq!(rover_atom_balance.amount, Uint128::new(190));

    let rover_osmo_balance = mock.query_balance(&mock.rover, "uosmo");
    assert_eq!(rover_osmo_balance.amount, osmo_deposit.amount);

    // Verify user wallet balances - check that swap amount is correctly debited
    let user_atom_balance = mock.query_balance(&user, "uatom");
    assert_eq!(user_atom_balance.amount, Uint128::new(30_000 - 20_000)); // 10,000 ATOM remaining (20,000 debited for 2 swaps)

    let user_osmo_balance = mock.query_balance(&user, "uosmo");
    assert_eq!(user_osmo_balance.amount, Uint128::zero());

    // Tier 5 (>= 250_000 MARS) → 45% discount
    mock.set_voting_power(&user, Uint128::new(250_000_000_000));
    let res = do_swap(&mut mock, &account_id, &user, 10_000, "uatom", "uosmo");
    let attrs = extract(&res);
    assert_eq!(attrs.get("voting_power").unwrap(), "250000000000");
    assert_eq!(attrs.get("tier_id").unwrap(), "tier_5");
    assert_eq!(attrs.get("discount_pct").unwrap(), &Decimal::percent(45).to_string());

    // Verify fees: 45% discount means effective_fee = base_fee * (1 - 0.45) = base_fee * 0.55
    let (base_fee, effective_fee) = extract_fees(&res);
    assert_eq!(base_fee, Decimal::percent(1)); // 1% base fee
    assert_eq!(effective_fee, Decimal::percent(1) * (Decimal::one() - Decimal::percent(45))); // 0.55% effective fee

    // Verify account balances to ensure internal accounting is correct
    let positions = mock.query_positions(&account_id);
    assert_eq!(positions.deposits.len(), 1);
    let osmo_deposit = positions.deposits.iter().find(|d| d.denom == "uosmo").unwrap();
    // Should have received OSMO from the swap (minus fees)
    assert!(osmo_deposit.amount > Uint128::zero());

    // Verify no ATOM remains in account (it was swapped)
    let atom_deposit = positions.deposits.iter().find(|d| d.denom == "uatom");
    assert!(atom_deposit.is_none());

    // Verify Credit Manager contract balances
    let rover_atom_balance = mock.query_balance(&mock.rover, "uatom");
    // Cumulative retained fee: 190 + 55 (third with 0.55%) = 245
    assert_eq!(rover_atom_balance.amount, Uint128::new(245));

    let rover_osmo_balance = mock.query_balance(&mock.rover, "uosmo");
    assert_eq!(rover_osmo_balance.amount, osmo_deposit.amount); // OSMO should match account deposit

    // Verify user wallet balances - check that swap amount is correctly debited
    let user_atom_balance = mock.query_balance(&user, "uatom");
    assert_eq!(user_atom_balance.amount, Uint128::new(30_000 - 30_000)); // 0 ATOM remaining (30,000 debited for 3 swaps)

    let user_osmo_balance = mock.query_balance(&user, "uosmo");
    assert_eq!(user_osmo_balance.amount, Uint128::zero()); // No OSMO in user wallet

    assert!(res
        .events
        .iter()
        .any(|e| e.attributes.iter().any(|a| a.key == "action" && a.value == "swapper")));
}
