use std::str::FromStr;

use cosmwasm_std::{coin, Addr, Coin, Decimal, Int128, Uint128};
use mars_credit_manager::error::ContractError;
use mars_mock_oracle::msg::CoinPrice;
use mars_types::{
    credit_manager::{
        Action::{Deposit, ExecutePerpOrder, Lend, Withdraw},
        ActionAmount, ActionCoin, ExecutePerpOrderType, Positions,
    },
    oracle::ActionKind,
    params::PerpParamsUpdate,
    perps::PnL,
};
use test_case::test_case;

use super::helpers::{coin_info, uatom_info, uosmo_info, AccountToFund, CoinInfo, MockEnv};
use crate::tests::helpers::{assert_err, default_perp_params, get_coin};

// Expected USDC position is calculated based on Opening Fee which is 49.
// usdc_lend is deducted from usdc_deposit to calculate expected initial deposit in the account.
#[test_case(
    100,
    0,
    UsdcPosition { deposit: 51, lend: 0, debt: 0 };
    "open position; deposit, no lend; expected from deposit"
)]
#[test_case(
    20,
    0,
    UsdcPosition { deposit: 0, lend: 0, debt: 30 }; // 29 + 1 (simulated interest)
    "open position; not enough deposit, no lend; expected from deposit and debt"
)]
#[test_case(
    0,
    0,
    UsdcPosition { deposit: 0, lend: 0, debt: 50 }; // 49 + 1 (simulated interest)
    "open position; no deposit, no lend; expected from debt"
)]
#[test_case(
    100,
    100,
    UsdcPosition { deposit: 0, lend: 52, debt: 0 }; // 51 + 1 (simulated interest)
    "open position; no deposit, lend; expected from lend"
)]
#[test_case(
    40,
    40,
    UsdcPosition { deposit: 0, lend: 0, debt: 9 };
    "open position; no deposit, not enough lend; expected from lend and debt"
)]
#[test_case(
    40,
    20,
    UsdcPosition { deposit: 0, lend: 0, debt: 9 };
    "open position; not enough deposit and lend; expected from deposit, lend and debt"
)]
fn open_position_with_correct_payment(
    usdc_deposit: u128,
    usdc_lend: u128,
    expected_usdc_position: UsdcPosition,
) {
    let osmo_info = uosmo_info();
    let atom_info = uatom_info();
    let usdc_info = coin_info("uusdc");

    let osmo_coin_deposited = osmo_info.to_coin(1000000);
    let usdc_coin_deposited = usdc_info.to_coin(usdc_deposit);

    let cm_user = Addr::unchecked("user");

    let (mut mock, account_id) = setup(
        &osmo_info,
        &atom_info,
        &usdc_info,
        &osmo_coin_deposited,
        &usdc_coin_deposited,
        &cm_user,
    );

    // Lend usdc
    if usdc_lend > 0 {
        mock.update_credit_account(
            &account_id,
            &cm_user,
            vec![Lend(ActionCoin {
                denom: usdc_coin_deposited.denom.clone(),
                amount: ActionAmount::Exact(Uint128::new(usdc_lend)),
            })],
            &[],
        )
        .unwrap();
    }

    let perp_size = Int128::from_str("1200").unwrap();

    // Check perp data before any action
    let vault_usdc_balance = mock.query_balance(mock.perps.address(), &usdc_info.denom);
    let opening_fee = mock.query_perp_opening_fee(&atom_info.denom, perp_size);
    assert_eq!(opening_fee.fee.amount.u128(), 49);

    // Open perp position
    mock.update_credit_account(
        &account_id,
        &cm_user,
        vec![ExecutePerpOrder {
            denom: atom_info.denom.clone(),
            order_size: perp_size,
            reduce_only: None,
            order_type: Some(ExecutePerpOrderType::Default),
        }],
        &[],
    )
    .unwrap();

    // Check if perp balance increased by opening fee
    let current_vault_usdc_balance = mock.query_balance(mock.perps.address(), &usdc_info.denom);
    let expected_vault_usdc_balance_after_opening_perp =
        vault_usdc_balance.amount + opening_fee.fee.amount;
    assert_eq!(current_vault_usdc_balance.amount, expected_vault_usdc_balance_after_opening_perp);

    // Check position data
    let position = mock.query_positions(&account_id);
    assert_present(&position, &osmo_coin_deposited.denom, osmo_coin_deposited.amount);
    expected_usdc_position.assert_eq(&usdc_coin_deposited.denom, &position);
    assert_eq!(position.perps.len(), 1);
}

// Expected USDC position is calculated based on PnL loss which is 524.
// usdc_lend is deducted from usdc_deposit to calculate expected initial deposit in the account.
#[test_case(
    100000,
    0,
    false,
    UsdcPosition { deposit: 99476, lend: 0, debt: 0 };
    "close negative pnl; deposit, no lend; expected from deposit"
)]
#[test_case(
    20,
    0,
    false,
    UsdcPosition { deposit: 0, lend: 0, debt: 505 }; // 504 + 1 (simulated interest)
    "close negative pnl; not enough deposit, no lend; expected from deposit and debt"
)]
#[test_case(
    0,
    0,
    false,
    UsdcPosition { deposit: 0, lend: 0, debt: 525 }; // 524 + 1 (simulated interest)
    "close negative pnl; no deposit, no lend; expected from debt"
)]
#[test_case(
    1000,
    1000,
    false,
    UsdcPosition { deposit: 0, lend: 477, debt: 0 }; // 476 + 1 (simulated interest)
    "close negative pnl; no deposit, lend; expected from lend"
)]
#[test_case(
    500,
    500,
    false,
    UsdcPosition { deposit: 0, lend: 0, debt: 24 };
    "close negative pnl; no deposit, not enough lend; expected from lend and debt"
)]
#[test_case(
    500,
    200,
    false,
    UsdcPosition { deposit: 0, lend: 0, debt: 24 };
    "close negative pnl; not enough deposit and lend; expected from deposit, lend and debt"
)]
#[test_case(
    100000,
    0,
    true,
    UsdcPosition { deposit: 99476, lend: 0, debt: 0 };
    "close negative pnl; reduce more than position size; deposit, no lend; expected from deposit"
)]
fn close_losing_position_with_correct_payment(
    usdc_deposit: u128,
    usdc_lend: u128,
    reduce_more_than_pos_size: bool,
    expected_usdc_position: UsdcPosition,
) {
    let osmo_info = uosmo_info();
    let atom_info = uatom_info();
    let usdc_info = coin_info("uusdc");

    let osmo_coin_deposited = osmo_info.to_coin(1000000);
    let usdc_coin_deposited = usdc_info.to_coin(usdc_deposit);

    let cm_user = Addr::unchecked("user");

    let (mut mock, account_id) = setup(
        &osmo_info,
        &atom_info,
        &usdc_info,
        &osmo_coin_deposited,
        &usdc_coin_deposited,
        &cm_user,
    );

    // Lend usdc
    if usdc_lend > 0 {
        mock.update_credit_account(
            &account_id,
            &cm_user,
            vec![Lend(ActionCoin {
                denom: usdc_coin_deposited.denom.clone(),
                amount: ActionAmount::Exact(Uint128::new(usdc_lend)),
            })],
            &[],
        )
        .unwrap();
    }

    // Reset opening_fee to 0
    let mut atom_params = default_perp_params(&atom_info.denom);
    atom_params.opening_fee_rate = Decimal::zero();
    mock.update_perp_params(PerpParamsUpdate::AddOrUpdate {
        params: atom_params,
    });

    let perp_size = Int128::from_str("1200").unwrap();

    // Check perp data before any action
    let vault_usdc_balance_before = mock.query_balance(mock.perps.address(), &usdc_info.denom);

    // Open perp position
    mock.update_credit_account(
        &account_id,
        &cm_user,
        vec![ExecutePerpOrder {
            denom: atom_info.denom.clone(),
            order_size: perp_size,
            reduce_only: None,
            order_type: Some(ExecutePerpOrderType::Default),
        }],
        &[],
    )
    .unwrap();

    // Simulate loss in perp position
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: atom_info.denom.clone(),
        price: atom_info.price * Decimal::percent(90u64), // 10% loss in price
    });

    // Update opening_fee to previous value
    let atom_params = default_perp_params(&atom_info.denom);
    mock.update_perp_params(PerpParamsUpdate::AddOrUpdate {
        params: atom_params,
    });

    // Check perp position pnl
    let perp_position = mock.query_perp_position(&account_id, &atom_info.denom).position.unwrap();
    let loss_amt = pnl_loss(perp_position.unrealized_pnl.to_coins(&perp_position.base_denom).pnl);
    assert_eq!(loss_amt.u128(), 524);

    // Close perp position
    let mut order_size = Int128::zero() - perp_size;
    if reduce_more_than_pos_size {
        // Reduce more than the original size
        order_size -= Int128::new(120);
    }
    mock.update_credit_account(
        &account_id,
        &cm_user,
        vec![ExecutePerpOrder {
            denom: atom_info.denom,
            order_size,
            reduce_only: Some(true),
            order_type: Some(ExecutePerpOrderType::Default),
        }],
        &[],
    )
    .unwrap();

    // Check position data
    let position = mock.query_positions(&account_id);
    assert_present(&position, &osmo_coin_deposited.denom, osmo_coin_deposited.amount);
    expected_usdc_position.assert_eq(&usdc_coin_deposited.denom, &position);
    assert_eq!(position.perps.len(), 0);

    // Check if perp balance increased by position loss
    let current_vault_usdc_balance = mock.query_balance(mock.perps.address(), &usdc_info.denom);
    assert_eq!(current_vault_usdc_balance.amount, vault_usdc_balance_before.amount + loss_amt);
}

#[test]
fn close_perp_position_with_profit() {
    let osmo_info = uosmo_info();
    let atom_info = uatom_info();
    let usdc_info = coin_info("uusdc");

    let osmo_coin_deposited = osmo_info.to_coin(10000);
    let usdc_coin_deposited = usdc_info.to_coin(1000);

    let cm_user = Addr::unchecked("user");

    let (mut mock, account_id) = setup(
        &osmo_info,
        &atom_info,
        &usdc_info,
        &osmo_coin_deposited,
        &usdc_coin_deposited,
        &cm_user,
    );

    let perp_size = Int128::from_str("200").unwrap();

    // check perp data before any action
    let vault_usdc_balance = mock.query_balance(mock.perps.address(), &usdc_info.denom);
    let opening_fee = mock.query_perp_opening_fee(&atom_info.denom, perp_size);

    // open perp position
    mock.update_credit_account(
        &account_id,
        &cm_user,
        vec![ExecutePerpOrder {
            denom: atom_info.denom.clone(),
            order_size: perp_size,
            reduce_only: None,
            order_type: Some(ExecutePerpOrderType::Default),
        }],
        &[],
    )
    .unwrap();

    // check position data
    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 2);
    assert_present(&position, &osmo_coin_deposited.denom, osmo_coin_deposited.amount);
    let expected_pos_usdc_amt_after_opening_perp =
        usdc_coin_deposited.amount - opening_fee.fee.amount; // opening fee deducted from deposit
    assert_present(&position, &usdc_coin_deposited.denom, expected_pos_usdc_amt_after_opening_perp);
    assert_eq!(position.lends.len(), 0);
    assert_eq!(position.debts.len(), 0);
    assert_eq!(position.perps.len(), 1);
    let perp_position = position.perps.first().unwrap().clone();
    let expected_perp_position =
        mock.query_perp_position(&account_id, &atom_info.denom).position.unwrap();
    assert_eq!(perp_position, expected_perp_position);

    // check if perp balance increased by opening fee
    let current_vault_usdc_balance = mock.query_balance(mock.perps.address(), &usdc_info.denom);
    let expected_vault_usdc_balance_after_opening_perp =
        vault_usdc_balance.amount + opening_fee.fee.amount;
    assert_eq!(current_vault_usdc_balance.amount, expected_vault_usdc_balance_after_opening_perp);

    // simulate profit in perp position
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: atom_info.denom.clone(),
        price: atom_info.price * Decimal::percent(120u64), // 20% profit in price
    });

    // check perp position pnl
    let perp_position = mock.query_perp_position(&account_id, &atom_info.denom).position.unwrap();
    let profit_amt =
        pnl_profit(perp_position.unrealized_pnl.to_coins(&perp_position.base_denom).pnl);

    // close perp position
    mock.update_credit_account(
        &account_id,
        &cm_user,
        vec![ExecutePerpOrder {
            denom: atom_info.denom.clone(),
            order_size: Int128::zero() - perp_size,
            reduce_only: Some(true),
            order_type: Some(ExecutePerpOrderType::Default),
        }],
        &[],
    )
    .unwrap();

    // check position data
    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 2);
    assert_present(&position, &osmo_coin_deposited.denom, osmo_coin_deposited.amount);
    assert_present(
        &position,
        &usdc_coin_deposited.denom,
        expected_pos_usdc_amt_after_opening_perp + profit_amt, // deposit increased by perp profit
    );
    assert_eq!(position.lends.len(), 0);
    assert_eq!(position.debts.len(), 0);
    assert_eq!(position.perps.len(), 0);

    // check if perp balance decreased by position profit
    let current_vault_usdc_balance = mock.query_balance(mock.perps.address(), &usdc_info.denom);
    assert_eq!(
        current_vault_usdc_balance.amount,
        expected_vault_usdc_balance_after_opening_perp - profit_amt
    );
}

#[test]
fn increase_position_with_realized_pnl() {
    let osmo_info = uosmo_info();
    let atom_info = uatom_info();
    let usdc_info = coin_info("uusdc");

    let osmo_coin_deposited = osmo_info.to_coin(10000);
    let usdc_coin_deposited = usdc_info.to_coin(1000);

    let cm_user = Addr::unchecked("user");

    let (mut mock, account_id) = setup(
        &osmo_info,
        &atom_info,
        &usdc_info,
        &osmo_coin_deposited,
        &usdc_coin_deposited,
        &cm_user,
    );

    let perp_size = Int128::from_str("200").unwrap();

    // open perp position
    mock.update_credit_account(
        &account_id,
        &cm_user,
        vec![ExecutePerpOrder {
            denom: atom_info.denom.clone(),
            order_size: perp_size,
            reduce_only: None,
            order_type: Some(ExecutePerpOrderType::Default),
        }],
        &[],
    )
    .unwrap();

    // check data before modification
    let position = mock.query_positions(&account_id);
    let user_usdc_balance_before_modification =
        get_coin(&usdc_info.denom, &position.deposits).amount;
    let vault_usdc_balance_before_modification =
        mock.query_balance(mock.perps.address(), &usdc_info.denom).amount;

    // simulate profit in perp position
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: atom_info.denom.clone(),
        price: atom_info.price * Decimal::percent(120u64), // 20% profit in price
    });

    // increase perp position size
    let new_size = Int128::from_str("350").unwrap();
    let delta_change = new_size.checked_sub(perp_size).unwrap();

    // check perp position pnl
    let perp_position = mock
        .query_perp_position_with_modification_size(
            &account_id,
            &atom_info.denom,
            Some(delta_change),
        )
        .position
        .unwrap();
    let profit_amt =
        pnl_profit(perp_position.unrealized_pnl.to_coins(&perp_position.base_denom).pnl);

    // modify perp position
    mock.update_credit_account(
        &account_id,
        &cm_user,
        vec![ExecutePerpOrder {
            denom: atom_info.denom.clone(),
            order_size: delta_change,
            reduce_only: None,
            order_type: Some(ExecutePerpOrderType::Default),
        }],
        &[],
    )
    .unwrap();

    // check position data
    let position = mock.query_positions(&account_id);
    let user_usdc_balance_after_modification =
        get_coin(&usdc_info.denom, &position.deposits).amount;
    assert_eq!(position.deposits.len(), 2);
    assert_present(&position, &osmo_coin_deposited.denom, osmo_coin_deposited.amount);
    assert_present(
        &position,
        &usdc_coin_deposited.denom,
        user_usdc_balance_before_modification + profit_amt, // deposit increased by perp profit
    );
    assert_eq!(position.lends.len(), 0);
    assert_eq!(position.debts.len(), 0);
    assert_eq!(position.perps.len(), 1);

    // check if perp balance decreased by position profit
    let vault_usdc_balance_after_modification =
        mock.query_balance(mock.perps.address(), &usdc_info.denom).amount;
    assert_eq!(
        vault_usdc_balance_after_modification,
        vault_usdc_balance_before_modification - profit_amt
    );

    // simulate loss in perp position
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: atom_info.denom.clone(),
        price: atom_info.price * Decimal::percent(90u64), // 10% loss in price
    });

    // increase perp position size
    let new_size = Int128::from_str("460").unwrap();
    let delta_change = new_size.checked_sub(perp_size).unwrap();

    // check perp position pnl
    let perp_position = mock
        .query_perp_position_with_modification_size(
            &account_id,
            &atom_info.denom,
            Some(delta_change),
        )
        .position
        .unwrap();
    let loss_amt = pnl_loss(perp_position.unrealized_pnl.to_coins(&perp_position.base_denom).pnl);

    // modify perp position
    mock.update_credit_account(
        &account_id,
        &cm_user,
        vec![ExecutePerpOrder {
            denom: atom_info.denom.clone(),
            order_size: delta_change,
            reduce_only: None,
            order_type: Some(ExecutePerpOrderType::Default),
        }],
        &[],
    )
    .unwrap();

    // check position data
    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 2);
    assert_present(&position, &osmo_coin_deposited.denom, osmo_coin_deposited.amount);
    assert_present(
        &position,
        &usdc_coin_deposited.denom,
        user_usdc_balance_after_modification - loss_amt, // loss deducted from deposit
    );
    assert_eq!(position.lends.len(), 0);
    assert_eq!(position.debts.len(), 0);
    assert_eq!(position.perps.len(), 1);

    // check if perp balance increased by position loss
    let current_vault_usdc_balance = mock.query_balance(mock.perps.address(), &usdc_info.denom);
    assert_eq!(current_vault_usdc_balance.amount, vault_usdc_balance_after_modification + loss_amt);
}

#[test]
fn decrease_position_with_realized_pnl() {
    let osmo_info = uosmo_info();
    let atom_info = uatom_info();
    let usdc_info = coin_info("uusdc");

    let osmo_coin_deposited = osmo_info.to_coin(10000);
    let usdc_coin_deposited = usdc_info.to_coin(1000);

    let cm_user = Addr::unchecked("user");

    let (mut mock, account_id) = setup(
        &osmo_info,
        &atom_info,
        &usdc_info,
        &osmo_coin_deposited,
        &usdc_coin_deposited,
        &cm_user,
    );

    let perp_size = Int128::from_str("-400").unwrap();

    // open perp position
    mock.update_credit_account(
        &account_id,
        &cm_user,
        vec![ExecutePerpOrder {
            denom: atom_info.denom.clone(),
            order_size: perp_size,
            reduce_only: None,
            order_type: Some(ExecutePerpOrderType::Default),
        }],
        &[],
    )
    .unwrap();

    // check data before modification
    let position = mock.query_positions(&account_id);
    let user_usdc_balance_before_modification =
        get_coin(&usdc_info.denom, &position.deposits).amount;
    let vault_usdc_balance_before_modification =
        mock.query_balance(mock.perps.address(), &usdc_info.denom).amount;

    // simulate profit in perp position
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: atom_info.denom.clone(),
        price: atom_info.price * Decimal::percent(90u64), // 10% loss in price
    });

    // decrease perp position size
    let new_size = Int128::from_str("-320").unwrap();
    let delta_change = new_size.checked_sub(perp_size).unwrap();
    // check perp position pnl
    let perp_position = mock
        .query_perp_position_with_modification_size(
            &account_id,
            &atom_info.denom,
            Some(delta_change),
        )
        .position
        .unwrap();
    let profit_amt =
        pnl_profit(perp_position.unrealized_pnl.to_coins(&perp_position.base_denom).pnl);

    // modify perp position
    mock.update_credit_account(
        &account_id,
        &cm_user,
        vec![ExecutePerpOrder {
            denom: atom_info.denom.clone(),
            order_size: delta_change,
            reduce_only: None,
            order_type: Some(ExecutePerpOrderType::Default),
        }],
        &[],
    )
    .unwrap();

    // check position data
    let position = mock.query_positions(&account_id);
    let user_usdc_balance_after_modification =
        get_coin(&usdc_info.denom, &position.deposits).amount;
    assert_eq!(position.deposits.len(), 2);
    assert_present(&position, &osmo_coin_deposited.denom, osmo_coin_deposited.amount);
    assert_present(
        &position,
        &usdc_coin_deposited.denom,
        user_usdc_balance_before_modification + profit_amt, // deposit increased by perp profit
    );
    assert_eq!(position.lends.len(), 0);
    assert_eq!(position.debts.len(), 0);
    assert_eq!(position.perps.len(), 1);

    // check if perp balance decreased by position profit
    let vault_usdc_balance_after_modification =
        mock.query_balance(mock.perps.address(), &usdc_info.denom).amount;
    assert_eq!(
        vault_usdc_balance_after_modification,
        vault_usdc_balance_before_modification - profit_amt
    );

    // simulate loss in perp position
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: atom_info.denom.clone(),
        price: atom_info.price * Decimal::percent(120u64), // 20% profit in price
    });

    // increase perp position size
    let new_size = Int128::from_str("-250").unwrap();
    let delta_change = new_size.checked_sub(perp_size).unwrap();

    // check perp position pnl
    let perp_position = mock
        .query_perp_position_with_modification_size(
            &account_id,
            &atom_info.denom,
            Some(delta_change),
        )
        .position
        .unwrap();
    let loss_amt = pnl_loss(perp_position.unrealized_pnl.to_coins(&perp_position.base_denom).pnl);

    // modify perp position
    mock.update_credit_account(
        &account_id,
        &cm_user,
        vec![ExecutePerpOrder {
            denom: atom_info.denom,
            order_size: delta_change,
            reduce_only: None,
            order_type: Some(ExecutePerpOrderType::Default),
        }],
        &[],
    )
    .unwrap();

    // check position data
    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 2);
    assert_present(&position, &osmo_coin_deposited.denom, osmo_coin_deposited.amount);
    assert_present(
        &position,
        &usdc_coin_deposited.denom,
        user_usdc_balance_after_modification - loss_amt, // loss deducted from deposit
    );
    assert_eq!(position.lends.len(), 0);
    assert_eq!(position.debts.len(), 0);
    assert_eq!(position.perps.len(), 1);

    // check if perp balance increased by position loss
    let current_vault_usdc_balance = mock.query_balance(mock.perps.address(), &usdc_info.denom);
    assert_eq!(current_vault_usdc_balance.amount, vault_usdc_balance_after_modification + loss_amt);
}

#[test]
fn flip_position_with_realized_pnl() {
    let osmo_info = uosmo_info();
    let atom_info = uatom_info();
    let usdc_info = coin_info("uusdc");

    let osmo_coin_deposited = osmo_info.to_coin(10000);
    let usdc_coin_deposited = usdc_info.to_coin(1000);

    let cm_user = Addr::unchecked("user");

    let (mut mock, account_id) = setup(
        &osmo_info,
        &atom_info,
        &usdc_info,
        &osmo_coin_deposited,
        &usdc_coin_deposited,
        &cm_user,
    );

    // start in short position
    let perp_size = Int128::from_str("-400").unwrap();

    // open the perp position
    mock.update_credit_account(
        &account_id,
        &cm_user,
        vec![ExecutePerpOrder {
            denom: atom_info.denom.clone(),
            order_size: perp_size,
            reduce_only: None,
            order_type: Some(ExecutePerpOrderType::Default),
        }],
        &[],
    )
    .unwrap();

    // check data before modification
    let position = mock.query_positions(&account_id);
    let user_usdc_balance_before_modification =
        get_coin(&usdc_info.denom, &position.deposits).amount;

    let vault_usdc_balance_before_modification =
        mock.query_balance(mock.perps.address(), &usdc_info.denom).amount;

    // simulate profit in short perp position (price decrease)
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: atom_info.denom.clone(),
        price: atom_info.price * Decimal::percent(90u64), // 10% loss in price
    });

    // flip perp position to long
    let new_size = Int128::from_str("100").unwrap();
    let delta_change = new_size.checked_sub(perp_size).unwrap();

    // check perp position pnl
    let perp_position = mock
        .query_perp_position_with_modification_size(
            &account_id,
            &atom_info.denom,
            Some(delta_change),
        )
        .position
        .unwrap();
    let profit_amt =
        pnl_profit(perp_position.unrealized_pnl.to_coins(&perp_position.base_denom).pnl);

    // modify perp position
    mock.update_credit_account(
        &account_id,
        &cm_user,
        vec![ExecutePerpOrder {
            denom: atom_info.denom.clone(),
            order_size: delta_change,
            reduce_only: None,
            order_type: Some(ExecutePerpOrderType::Default),
        }],
        &[],
    )
    .unwrap();

    // check position data
    let position = mock.query_positions(&account_id);
    let user_usdc_balance_after_modification =
        get_coin(&usdc_info.denom, &position.deposits).amount;
    assert_eq!(position.deposits.len(), 2);
    assert_present(&position, &osmo_coin_deposited.denom, osmo_coin_deposited.amount);
    assert_present(
        &position,
        &usdc_coin_deposited.denom,
        user_usdc_balance_before_modification + profit_amt, // deposit increased by perp profit
    );
    assert_eq!(position.lends.len(), 0);
    assert_eq!(position.debts.len(), 0);
    assert_eq!(position.perps.len(), 1);

    // check if perp balance decreased by position profit
    let vault_usdc_balance_after_modification =
        mock.query_balance(mock.perps.address(), &usdc_info.denom).amount;
    assert_eq!(
        vault_usdc_balance_after_modification,
        vault_usdc_balance_before_modification - profit_amt
    );

    // simulate loss in perp position (position = long, so decrease in price)
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: atom_info.denom.clone(),
        price: atom_info.price * Decimal::percent(80u64), // 20% profit in price
    });

    // flip position to short again
    let new_size = Int128::from_str("-250").unwrap();
    let delta_change = new_size.checked_sub(perp_size).unwrap();

    // check perp position pnl
    let perp_position = mock
        .query_perp_position_with_modification_size(
            &account_id,
            &atom_info.denom,
            Some(delta_change),
        )
        .position
        .unwrap();
    let loss_amt = pnl_loss(perp_position.unrealized_pnl.to_coins(&perp_position.base_denom).pnl);

    // modify perp position
    mock.update_credit_account(
        &account_id,
        &cm_user,
        vec![ExecutePerpOrder {
            denom: atom_info.denom.clone(),
            order_size: delta_change,
            reduce_only: None,
            order_type: Some(ExecutePerpOrderType::Default),
        }],
        &[],
    )
    .unwrap();

    // check position data
    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 2);
    assert_present(&position, &osmo_coin_deposited.denom, osmo_coin_deposited.amount);
    assert_present(
        &position,
        &usdc_coin_deposited.denom,
        user_usdc_balance_after_modification - loss_amt, // loss deducted from deposit
    );
    assert_eq!(position.lends.len(), 0);
    assert_eq!(position.debts.len(), 0);
    assert_eq!(position.perps.len(), 1);

    // check if perp balance increased by position loss
    let current_vault_usdc_balance = mock.query_balance(mock.perps.address(), &usdc_info.denom);
    assert_eq!(current_vault_usdc_balance.amount, vault_usdc_balance_after_modification + loss_amt);
}

#[test]
fn cannot_open_perp_above_max_ltv() {
    let osmo_info = uosmo_info();
    let atom_info = uatom_info();
    let usdc_info = coin_info("uusdc");

    let osmo_coin_deposited = osmo_info.to_coin(10000);
    let usdc_coin_deposited = usdc_info.to_coin(1000);

    let cm_user = Addr::unchecked("user");

    let (mut mock, account_id) = setup(
        &osmo_info,
        &atom_info,
        &usdc_info,
        &osmo_coin_deposited,
        &usdc_coin_deposited,
        &cm_user,
    );

    let perp_size = Int128::from_str("100000").unwrap();

    let health = mock.query_health(&account_id, ActionKind::Default);
    assert!(!health.above_max_ltv);
    assert!(!health.liquidatable);

    // open perp position
    let res = mock.update_credit_account(
        &account_id,
        &cm_user,
        vec![ExecutePerpOrder {
            denom: atom_info.denom.clone(),
            order_size: perp_size,
            reduce_only: None,
            order_type: Some(ExecutePerpOrderType::Default),
        }],
        &[],
    );
    assert_err(
        res,
        ContractError::AboveMaxLTV {
            account_id,
            max_ltv_health_factor: "0.820408124686912222".to_string(),
        },
    );
}

#[test]
fn health_check_works_if_no_spot_base_denom() {
    let osmo_info = uosmo_info();
    let atom_info = uatom_info();
    let usdc_info = coin_info("uusdc");

    let osmo_coin_deposited = osmo_info.to_coin(1000000);
    let usdc_coin_deposited = usdc_info.to_coin(100000);

    let cm_user = Addr::unchecked("user");

    let (mut mock, account_id) = setup(
        &osmo_info,
        &atom_info,
        &usdc_info,
        &osmo_coin_deposited,
        &usdc_coin_deposited,
        &cm_user,
    );

    let perp_size = Int128::from_str("20000").unwrap();

    // open perp position
    mock.update_credit_account(
        &account_id,
        &cm_user,
        vec![ExecutePerpOrder {
            denom: atom_info.denom.clone(),
            order_size: perp_size,
            reduce_only: None,
            order_type: Some(ExecutePerpOrderType::Default),
        }],
        &[],
    )
    .unwrap();

    // withdraw usdc (base denom) from the account
    mock.update_credit_account(
        &account_id,
        &cm_user,
        vec![Withdraw(ActionCoin {
            denom: usdc_coin_deposited.denom,
            amount: ActionAmount::AccountBalance,
        })],
        &[],
    )
    .unwrap();

    // Check position data.
    // There should be no spot base denom (deposits, lends, debts etc).
    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 1);
    assert_present(&position, &osmo_coin_deposited.denom, osmo_coin_deposited.amount); // only osmo left
    assert_eq!(position.lends.len(), 0);
    assert_eq!(position.debts.len(), 0);
    assert_eq!(position.perps.len(), 1);

    // Close the perp position.
    // This should not fail even if the account has no spot base denom (deposits, lends, debts).
    // Prices and params for base denom should be fetched correctly for Health check.
    mock.update_credit_account(
        &account_id,
        &cm_user,
        vec![ExecutePerpOrder {
            denom: atom_info.denom,
            order_size: Int128::zero().checked_sub(perp_size).unwrap(),
            reduce_only: Some(true),
            order_type: Some(ExecutePerpOrderType::Default),
        }],
        &[],
    )
    .unwrap();

    // there should be no perp position
    let position = mock.query_positions(&account_id);
    assert!(position.perps.is_empty());
}

fn setup(
    osmo_info: &CoinInfo,
    atom_info: &CoinInfo,
    usdc_info: &CoinInfo,
    osmo_coin_deposited: &Coin,
    usdc_coin_deposited: &Coin,
    cm_user: &Addr,
) -> (MockEnv, String) {
    let contract_owner = Addr::unchecked("owner");
    let vault_depositor = Addr::unchecked("vault_depositor");

    let vault_coin_deposited = coin(100000, usdc_info.denom.clone());

    let mut mock = MockEnv::new()
        .owner(contract_owner.as_str())
        .set_params(&[osmo_info.clone(), atom_info.clone(), usdc_info.clone()])
        .fund_account(AccountToFund {
            addr: cm_user.clone(),
            funds: vec![
                osmo_coin_deposited.clone(),
                usdc_coin_deposited.clone(),
                //usdc_coin_deposited.clone(),
            ],
        })
        .fund_account(AccountToFund {
            addr: vault_depositor.clone(),
            funds: vec![vault_coin_deposited.clone()],
        })
        .build()
        .unwrap();

    let trader_account_id = mock.create_credit_account(cm_user).unwrap();
    let vault_depositor_account_id = mock.create_credit_account(&vault_depositor).unwrap();

    // setup params contract
    mock.update_perp_params(PerpParamsUpdate::AddOrUpdate {
        params: default_perp_params(&atom_info.denom),
    });

    // setup perp contract
    mock.update_credit_account(
        &vault_depositor_account_id,
        &vault_depositor,
        vec![Deposit(vault_coin_deposited.clone())],
        &[vault_coin_deposited.clone()],
    )
    .unwrap();
    mock.deposit_to_perp_vault(&vault_depositor_account_id, &vault_coin_deposited, None).unwrap();

    if !usdc_coin_deposited.amount.is_zero() {
        mock.update_credit_account(
            &trader_account_id,
            cm_user,
            vec![Deposit(osmo_coin_deposited.clone()), Deposit(usdc_coin_deposited.clone())],
            &[osmo_coin_deposited.clone(), usdc_coin_deposited.clone()],
        )
        .unwrap();
    } else {
        mock.update_credit_account(
            &trader_account_id,
            cm_user,
            vec![Deposit(osmo_coin_deposited.clone())],
            &[osmo_coin_deposited.clone()],
        )
        .unwrap();
    }

    (mock, trader_account_id)
}

fn pnl_profit(pnl: PnL) -> Uint128 {
    match pnl {
        PnL::Profit(coin) => coin.amount,
        _ => panic!("expected profit"),
    }
}

fn pnl_loss(pnl: PnL) -> Uint128 {
    match pnl {
        PnL::Loss(coin) => coin.amount,
        _ => panic!("expected loss"),
    }
}

fn assert_present(res: &Positions, denom: &str, amount: Uint128) {
    res.deposits.iter().find(|item| item.denom == denom && item.amount == amount).unwrap();
}

struct UsdcPosition {
    deposit: u128,
    lend: u128,
    debt: u128,
}

impl UsdcPosition {
    fn assert_eq(&self, usdc_denom: &str, position: &Positions) {
        if self.deposit > 0 {
            assert_eq!(position.deposits.len(), 2);
            assert_present(position, usdc_denom, Uint128::new(self.deposit));
        } else {
            assert_eq!(position.deposits.len(), 1);
        }

        if self.lend > 0 {
            assert_eq!(position.lends.len(), 1);
            let lend = position.lends.first().unwrap();
            assert_eq!(lend.amount.u128(), self.lend);
        } else {
            assert_eq!(position.lends.len(), 0);
        }

        if self.debt > 0 {
            assert_eq!(position.debts.len(), 1);
            let debt = position.debts.first().unwrap();
            assert_eq!(debt.amount.u128(), self.debt);
        } else {
            assert_eq!(position.debts.len(), 0);
        }
    }
}
