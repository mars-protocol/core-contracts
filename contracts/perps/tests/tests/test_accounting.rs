use std::{cmp::max, str::FromStr};

use cosmwasm_std::{coin, Coin, Decimal, Uint128};
use mars_perps::error::ContractError;
use mars_types::{
    params::{PerpParams, PerpParamsUpdate},
    perps::{Accounting, Balance, CashFlow, PerpPosition, PnL, VaultResponse},
    signed_uint::SignedUint,
};

use super::helpers::MockEnv;
use crate::tests::helpers::{default_perp_params, ONE_HOUR_SEC};

#[test]
fn accounting() {
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let user = "jake";

    // credit manager is calling the perps contract, so we need to fund it (funds will be used for closing losing position)
    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000_000u128, &["uosmo", "uatom", "uusdc"]);

    let base_denom_price = Decimal::from_str("0.9").unwrap();
    mock.set_price(&owner, "uusdc", base_denom_price).unwrap();

    // deposit some big number of uusdc to vault
    mock.deposit_to_vault(
        &credit_manager,
        Some(user),
        None,
        &[coin(1_000_000_000_000u128, "uusdc")],
    )
    .unwrap();

    // init denoms
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                closing_fee_rate: Decimal::percent(1),
                opening_fee_rate: Decimal::percent(2),
                max_funding_velocity: Decimal::from_str("32").unwrap(),
                ..default_perp_params("uosmo")
            },
        },
    );
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                closing_fee_rate: Decimal::percent(1),
                opening_fee_rate: Decimal::percent(2),
                max_funding_velocity: Decimal::from_str("30").unwrap(),
                ..default_perp_params("uatom")
            },
        },
    );

    // set entry prices
    mock.set_price(&owner, "uosmo", Decimal::from_str("1.25").unwrap()).unwrap();
    mock.set_price(&owner, "uatom", Decimal::from_str("10.5").unwrap()).unwrap();

    // check accounting in the beginning
    let osmo_accounting = mock.query_market_accounting("uosmo").accounting;
    assert_eq!(osmo_accounting, Accounting::default());
    let atom_accounting = mock.query_market_accounting("uatom").accounting;
    assert_eq!(atom_accounting, Accounting::default());
    let total_accounting = mock.query_total_accounting().accounting;
    assert_eq!(total_accounting, Accounting::default());

    let vault_state_before_opening = mock.query_vault();

    // open few positions for account 1
    let osmo_size = SignedUint::from_str("10000000").unwrap();
    let osmo_opening_fee = mock.query_opening_fee("uosmo", osmo_size).fee;
    mock.execute_perp_order(
        &credit_manager,
        "1",
        "uosmo",
        osmo_size,
        None,
        &[osmo_opening_fee.clone()],
    )
    .unwrap();
    let atom_size = SignedUint::from_str("-260000").unwrap();
    let atom_opening_fee = mock.query_opening_fee("uatom", atom_size).fee;
    mock.execute_perp_order(
        &credit_manager,
        "1",
        "uatom",
        atom_size,
        None,
        &[atom_opening_fee.clone()],
    )
    .unwrap();

    // check vault state after opening positions
    assert_vault(&mock, &vault_state_before_opening);

    // check accounting after opening positions
    let osmo_accounting = mock.query_market_accounting("uosmo").accounting;
    assert_eq!(
        osmo_accounting.cash_flow,
        CashFlow {
            opening_fee: SignedUint::from(osmo_opening_fee.amount),
            ..Default::default()
        }
    );
    let atom_accounting = mock.query_market_accounting("uatom").accounting;
    assert_eq!(
        atom_accounting.cash_flow,
        CashFlow {
            opening_fee: SignedUint::from(atom_opening_fee.amount),
            ..Default::default()
        }
    );
    let total_accounting = mock.query_total_accounting().accounting;
    assert_eq!(
        total_accounting.cash_flow,
        CashFlow {
            opening_fee: SignedUint::from(osmo_opening_fee.amount + atom_opening_fee.amount),
            ..Default::default()
        }
    );
    assert_accounting(&total_accounting, &osmo_accounting, &atom_accounting);

    // move time forward by 12 hour
    mock.increment_by_time(12 * ONE_HOUR_SEC);

    // change only uosmo price
    mock.set_price(&owner, "uosmo", Decimal::from_str("2").unwrap()).unwrap();

    let vault_state_before_closing = mock.query_vault();

    // close uosmo position
    let pos = mock.query_position("1", "uosmo");
    mock.execute_perp_order(
        &credit_manager,
        "1",
        "uosmo",
        SignedUint::zero().checked_sub(osmo_size).unwrap(),
        Some(true),
        &from_position_to_coin(pos.position.unwrap()),
    )
    .unwrap();

    // check vault state after closing uosmo position
    assert_vault(&mock, &vault_state_before_closing);

    // check accounting after closing uosmo position
    let osmo_accounting = mock.query_market_accounting("uosmo").accounting;
    let atom_accounting = mock.query_market_accounting("uatom").accounting;
    let total_accounting = mock.query_total_accounting().accounting;
    assert_accounting(&total_accounting, &osmo_accounting, &atom_accounting);

    // compare realized PnL
    let osmo_realized_pnl = mock.query_realized_pnl_by_account_and_market("1", "uosmo");
    assert_eq!(osmo_realized_pnl.price_pnl.abs, osmo_accounting.cash_flow.price_pnl.abs);
    assert!(!osmo_realized_pnl.price_pnl.negative);
    assert_ne!(osmo_realized_pnl.price_pnl.negative, osmo_accounting.cash_flow.price_pnl.negative);
    assert_eq!(
        osmo_realized_pnl.accrued_funding.abs,
        osmo_accounting.cash_flow.accrued_funding.abs
    );
    assert!(osmo_realized_pnl.accrued_funding.negative);
    assert_ne!(
        osmo_realized_pnl.accrued_funding.negative,
        osmo_accounting.cash_flow.accrued_funding.negative
    );
    assert_eq!(osmo_realized_pnl.opening_fee.abs, osmo_accounting.cash_flow.opening_fee.abs);
    assert!(osmo_realized_pnl.opening_fee.negative);
    assert_ne!(
        osmo_realized_pnl.opening_fee.negative,
        osmo_accounting.cash_flow.opening_fee.negative
    );
    assert_eq!(osmo_realized_pnl.closing_fee.abs, osmo_accounting.cash_flow.closing_fee.abs);
    assert!(osmo_realized_pnl.closing_fee.negative);
    assert_ne!(
        osmo_realized_pnl.closing_fee.negative,
        osmo_accounting.cash_flow.opening_fee.negative
    );

    // move time forward by 12 hour
    mock.increment_by_time(12 * ONE_HOUR_SEC);

    // change only uatom price
    mock.set_price(&owner, "uatom", Decimal::from_str("15").unwrap()).unwrap();

    let vault_state_before_closing = mock.query_vault();

    // close uatom position
    let pos = mock.query_position("1", "uatom");
    mock.execute_perp_order(
        &credit_manager,
        "1",
        "uatom",
        SignedUint::zero().checked_sub(atom_size).unwrap(),
        Some(true),
        &from_position_to_coin(pos.position.unwrap()),
    )
    .unwrap();

    // check vault state after closing uatom position
    assert_vault(&mock, &vault_state_before_closing);

    // check accounting after closing uatom position
    let osmo_accounting = mock.query_market_accounting("uosmo").accounting;
    let atom_accounting = mock.query_market_accounting("uatom").accounting;
    let total_accounting = mock.query_total_accounting().accounting;
    assert_accounting(&total_accounting, &osmo_accounting, &atom_accounting);

    // compare realized PnL
    let atom_realized_pnl = mock.query_realized_pnl_by_account_and_market("1", "uatom");
    assert_eq!(atom_realized_pnl.price_pnl.abs, atom_accounting.cash_flow.price_pnl.abs);
    assert!(atom_realized_pnl.price_pnl.negative);
    assert_ne!(atom_realized_pnl.price_pnl.negative, atom_accounting.cash_flow.price_pnl.negative);
    assert_eq!(
        atom_realized_pnl.accrued_funding.abs,
        atom_accounting.cash_flow.accrued_funding.abs
    );
    assert!(atom_realized_pnl.accrued_funding.negative);
    assert_ne!(
        atom_realized_pnl.accrued_funding.negative,
        atom_accounting.cash_flow.accrued_funding.negative
    );
    assert_eq!(atom_realized_pnl.opening_fee.abs, atom_accounting.cash_flow.opening_fee.abs);
    assert!(atom_realized_pnl.opening_fee.negative);
    assert_ne!(
        atom_realized_pnl.opening_fee.negative,
        atom_accounting.cash_flow.opening_fee.negative
    );
    assert_eq!(atom_realized_pnl.closing_fee.abs, atom_accounting.cash_flow.closing_fee.abs);
    assert!(atom_realized_pnl.closing_fee.negative);
    assert_ne!(
        atom_realized_pnl.closing_fee.negative,
        atom_accounting.cash_flow.opening_fee.negative
    );
}

fn from_position_to_coin(pos: PerpPosition) -> Vec<Coin> {
    if let PnL::Loss(coin) = pos.unrealised_pnl.to_coins(&pos.base_denom).pnl {
        vec![coin]
    } else {
        vec![]
    }
}

fn assert_accounting(
    total_accounting: &Accounting,
    osmo_accounting: &Accounting,
    atom_accounting: &Accounting,
) {
    assert_eq!(
        total_accounting.cash_flow,
        add_cash_flows(&osmo_accounting.cash_flow, &atom_accounting.cash_flow)
    );
    assert_eq!(
        total_accounting.balance,
        add_balances(&osmo_accounting.balance, &atom_accounting.balance)
    );
    assert_eq!(
        total_accounting.withdrawal_balance,
        add_balances(&osmo_accounting.withdrawal_balance, &atom_accounting.withdrawal_balance)
    );
}

fn add_cash_flows(a: &CashFlow, b: &CashFlow) -> CashFlow {
    CashFlow {
        price_pnl: a.price_pnl.checked_add(b.price_pnl).unwrap(),
        opening_fee: a.opening_fee.checked_add(b.opening_fee).unwrap(),
        closing_fee: a.closing_fee.checked_add(b.closing_fee).unwrap(),
        accrued_funding: a.accrued_funding.checked_add(b.accrued_funding).unwrap(),
    }
}

fn add_balances(a: &Balance, b: &Balance) -> Balance {
    Balance {
        price_pnl: a.price_pnl.checked_add(b.price_pnl).unwrap(),
        opening_fee: a.opening_fee.checked_add(b.opening_fee).unwrap(),
        closing_fee: a.closing_fee.checked_add(b.closing_fee).unwrap(),
        accrued_funding: a.accrued_funding.checked_add(b.accrued_funding).unwrap(),
        total: a.total.checked_add(b.total).unwrap(),
    }
}

fn assert_vault(mock: &MockEnv, vault_before: &VaultResponse) {
    let vault = mock.query_vault();
    let total_accounting_res = mock.query_total_accounting();
    let total_accounting = total_accounting_res.accounting;
    let total_pnl_amt = total_accounting_res.unrealized_pnl;
    let total_debt = max(total_pnl_amt.pnl, SignedUint::zero()).abs;
    let total_withdrawal_balance =
        total_accounting.withdrawal_balance.total.checked_add(vault_before.total_balance).unwrap();
    let total_withdrawal_balance = max(total_withdrawal_balance, SignedUint::zero()).abs;
    let total_liquidity = total_accounting
        .cash_flow
        .total()
        .unwrap()
        .checked_add(vault_before.total_balance)
        .unwrap();
    let total_liquidity = max(total_liquidity, SignedUint::zero()).abs;
    let collateralization_ratio = if total_debt.is_zero() {
        None
    } else {
        Some(Decimal::from_ratio(total_liquidity, total_debt))
    };
    assert_eq!(
        vault,
        VaultResponse {
            total_balance: vault_before.total_balance,
            total_shares: vault_before.total_shares,
            total_withdrawal_balance,
            share_price: Some(Decimal::from_ratio(
                total_withdrawal_balance,
                vault_before.total_shares
            )),
            total_liquidity,
            total_debt,
            collateralization_ratio
        }
    );
}

/// This test ensures that the accounting system handles markets where the denom (asset) has a large number of decimals.
/// The test alternates between opening long and short positions until the long open interest (OI) limit is reached.
/// Since the long and short OI limits are symmetric, reaching the long OI limit is sufficient to verify that the system
/// handles both long and short OI correctly.
#[test]
fn accounting_works_up_to_oi_limits() {
    // Initialize the mock environment and build the necessary contracts
    let mut mock = MockEnv::new().build().unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let user = "jake";

    // Fund the credit manager's account with a large amount of tokens.
    // These funds will be used when closing a losing position.
    mock.fund_accounts(&[&credit_manager], u128::MAX, &["untrn", "ueth", "uusdc"]);

    // Set the initial prices for the assets (uusdc, untrn, ueth).
    mock.set_price(&owner, "uusdc", Decimal::from_str("0.9998").unwrap()).unwrap();
    mock.set_price(&owner, "untrn", Decimal::from_str("1.25").unwrap()).unwrap();
    mock.set_price(&owner, "ueth", Decimal::from_str("0.000000002389095541").unwrap()).unwrap();

    // Deposit a large amount of uusdc to the vault on behalf of the user.
    mock.deposit_to_vault(&credit_manager, Some(user), &[coin(1_000_000_000_000_000u128, "uusdc")])
        .unwrap();

    // Initialize perpetual market parameters for ueth (Ethereum).
    // Set the maximum long open interest value (max_long_oi_value) and other parameters for ueth.
    let eth_max_long_oi_value = Uint128::new(19093576000000);
    mock.update_perp_params(
        &owner,
        PerpParamsUpdate::AddOrUpdate {
            params: PerpParams {
                denom: "ueth".to_string(),
                enabled: true,
                max_net_oi_value: Uint128::new(86049000000),
                max_long_oi_value: eth_max_long_oi_value,
                max_short_oi_value: Uint128::new(19093576000000),
                closing_fee_rate: Decimal::from_str("0.00075").unwrap(),
                opening_fee_rate: Decimal::from_str("0.00075").unwrap(),
                liquidation_threshold: Decimal::from_str("0.91").unwrap(),
                max_loan_to_value: Decimal::from_str("0.90").unwrap(),
                max_position_value: None,
                min_position_value: Uint128::zero(),
                max_funding_velocity: Decimal::from_str("36").unwrap(),
                skew_scale: Uint128::new(1186268000000000000000000u128),
            },
        },
    );

    // This loop will open long and short positions alternately until the long open interest limit is reached.
    // Start with a long position, then alternate with short positions.
    #[allow(unused_assignments)]
    let mut contract_err: Option<ContractError> = None;
    let mut acc_id = 1;
    let mut eth_size = SignedUint::from_str("10000000000000000000").unwrap();

    loop {
        // Query the opening fee for the given size of the position (eth_size).
        let eth_opening_fee = mock.query_opening_fee("ueth", eth_size).fee;

        // Attempt to execute a perpetual order using the credit manager for the current position size.
        let res = mock.execute_perp_order(
            &credit_manager,
            acc_id.to_string().as_str(),
            "ueth",
            eth_size,
            None,
            &[eth_opening_fee.clone()],
        );

        // Query the accounting details (positions, open interest, etc.) after opening the position to verify it was successful.
        mock.query_total_accounting();
        mock.query_vault();

        // If the execution of the order fails, capture the error and break the loop.
        if let Err(generic_err) = res {
            let err: ContractError = generic_err.downcast().unwrap();
            println!("Error: {:?}", err);
            contract_err = Some(err);
            break;
        }

        // Advance the time in the mock environment by 5 seconds.
        mock.increment_by_time(5);

        // Increment account ID for the next position and alternate the position size (long/short).
        acc_id += 1;
        eth_size = SignedUint::zero().checked_sub(eth_size).unwrap(); // Alternate between positive and negative sizes.
    }

    // Assert that the final error is due to reaching the long open interest limit.
    assert!(matches!(contract_err, Some(ContractError::LongOpenInterestReached { .. })));
}
