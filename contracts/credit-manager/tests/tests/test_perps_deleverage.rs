use std::{cmp::min, str::FromStr};

use cosmwasm_std::{coin, Addr, Coin, Decimal, Uint128};
use mars_credit_manager::error::ContractError;
use mars_mock_oracle::msg::CoinPrice;
use mars_perps::error::ContractError as PerpsContractError;
use mars_testing::multitest::helpers::{default_perp_params, get_coin, uatom_info, AccountToFund};
use mars_types::{
    credit_manager::{
        Action::{Deposit, ExecutePerpOrder},
        Positions,
    },
    oracle::ActionKind,
    params::{PerpParams, PerpParamsUpdate},
    perps::PnL,
    signed_uint::SignedUint,
};
use test_case::test_case;

use super::helpers::{coin_info, uosmo_info, MockEnv};
use crate::tests::helpers::assert_err;

#[test]
fn unauthorized_update_balance_after_deleverage() {
    let user = Addr::unchecked("random-user");
    let mut mock = MockEnv::new().build().unwrap();

    let res = mock.update_balance_after_deleverage(&user, &[], "1", PnL::Loss(coin(100, "uusdc")));
    assert_err(
        res,
        ContractError::Unauthorized {
            user: "random-user".to_string(),
            action: "update balances after deleverage".to_string(),
        },
    );
}

#[test_case(0, "125", 0; "pnl profit when no usdc in account")]
#[test_case(100, "125", 0; "pnl profit when usdc in account")]
#[test_case(0, "-125", 125; "pnl loss when no usdc in account")]
#[test_case(100, "-125", 25; "pnl loss when not enough usdc in account")]
#[test_case(125, "-125", 0; "pnl loss when usdc in account equal to the loss")]
#[test_case(200, "-125", 0; "pnl loss when more usdc in account than the loss")]
#[test_case(0, "0", 0; "pnl break even when no usdc in account")]
#[test_case(100, "0", 0; "pnl break even when usdc in account")]
fn update_balance_after_deleverage(usdc_deposit_amt: u128, pnl: &str, expected_debt: u128) {
    let pnl_signed_uint = SignedUint::from_str(pnl).unwrap();

    let cm_user = Addr::unchecked("cm_user");
    let contract_owner = Addr::unchecked("owner");

    let osmo_info = uosmo_info();
    let usdc_info = coin_info("uusdc");
    let usdc_vault_deposit = usdc_info.to_coin(150000);

    let osmo_cm_deposit = osmo_info.to_coin(100000);
    let usdc_cm_deposit = usdc_info.to_coin(usdc_deposit_amt);
    let deposits = vec![osmo_cm_deposit.clone(), usdc_cm_deposit.clone()];

    let mut mock = MockEnv::new()
        .owner(contract_owner.as_str())
        .set_params(&[osmo_info, usdc_info.clone()])
        .fund_account(AccountToFund {
            addr: cm_user.clone(),
            funds: deposits.clone(),
        })
        .build()
        .unwrap();

    // fund perps contract to cover pnl profit
    let perps = mock.perps.clone();
    mock.fund_addr(perps.address(), vec![usdc_vault_deposit.clone()]);
    let current_vault_usdc_balance = mock.query_balance(perps.address(), &usdc_info.denom);
    assert_eq!(current_vault_usdc_balance.amount.u128(), usdc_vault_deposit.amount.u128());

    // create credit account
    let acc_id = mock.create_credit_account(&cm_user).unwrap();

    // deposit to credit account
    let actions = deposits.iter().map(|coin| Deposit(coin.clone())).collect::<Vec<_>>();
    mock.update_credit_account(&acc_id, &cm_user, actions, &deposits).unwrap();

    // expected positions data
    let mut expected_usdc_deposit = usdc_deposit_amt;
    let mut expected_deposits = vec![osmo_cm_deposit.clone()];

    // create perp PnL
    let pnl = PnL::from_signed_uint(usdc_info.denom.clone(), pnl_signed_uint);
    let mut pnl_profit = Uint128::zero();
    let mut pnl_loss = Uint128::zero();
    let mut funds = vec![];
    match pnl.clone() {
        PnL::Profit(coin) => {
            pnl_profit = coin.amount;
            funds.push(coin);

            expected_usdc_deposit += pnl_profit.u128();
        }
        PnL::Loss(coin) => {
            pnl_loss = coin.amount;

            if pnl_loss.u128() > usdc_deposit_amt {
                expected_usdc_deposit = 0;
            } else {
                expected_usdc_deposit -= pnl_loss.u128();
            }
        }
        PnL::BreakEven => {}
    };

    // update balances after deleverage, send funds to cover pnl profit
    mock.update_balance_after_deleverage(perps.address(), &funds, &acc_id, pnl).unwrap();

    // // check if perps balance increased by pnl loss and decreased by pnl profit
    let current_vault_usdc_balance = mock.query_balance(perps.address(), &usdc_info.denom);
    let expected_vault_usdc_balance = usdc_vault_deposit.amount + pnl_loss - pnl_profit;
    assert_eq!(current_vault_usdc_balance.amount, expected_vault_usdc_balance);

    // check positions data
    let positions = mock.query_positions(&acc_id);
    if expected_usdc_deposit > 0 {
        expected_deposits.push(usdc_info.to_coin(expected_usdc_deposit));
    }
    let expected_debt = if expected_debt > 0 {
        Some(usdc_info.to_coin(expected_debt))
    } else {
        None
    };
    assert_positions(positions, expected_deposits, expected_debt);
}

fn assert_positions(
    positions: Positions,
    expected_deposits: Vec<Coin>,
    expected_debt: Option<Coin>,
) {
    assert_eq!(positions.deposits, expected_deposits);

    if let Some(expected_debt) = expected_debt {
        assert_eq!(positions.debts.len(), 1);
        let debt = positions.debts.first().unwrap();
        assert_eq!(debt.denom, expected_debt.denom);
        println!("debt.amount: {}, expected_debt.amount: {}", debt.amount, expected_debt.amount);
        let expected_pos_debt = expected_debt.amount + Uint128::new(1); // simulated interest
        assert_eq!(debt.amount, expected_pos_debt);
    } else {
        assert!(positions.debts.is_empty());
    };

    assert!(positions.lends.is_empty());
    assert!(positions.vaults.is_empty());
    assert!(positions.staked_astro_lps.is_empty());
    assert!(positions.perps.is_empty());
}

// TODO: The below tests should be moved to Perps contract once MockEnv from Perps helpers is merged with MockEnv from testing package
#[test_case( "-240000000", "-480000000", "4000000", "6000000", true, "5.0", true, true, 3, Some(false), Some(PerpsContractError::DeleverageDisabled ); "CR below target, Deleverage disabled; close most lossy long position; CR decreased; throw error")]
#[test_case( "240000000", "480000000", "-40000000", "-60000000", false, "15.0", false, false, 1, None, Some(PerpsContractError::DeleverageInvalidPosition { reason: "CR >= TCR and OI <= max OI".to_string()}); "CR greater than or equal to target, OI not exeeded; close a position; throw error")]
#[test_case( "240000000", "480000000", "-40000000", "-60000000", false, "15.0", true, false, 1, None, None; "CR greater than or equal to target, long OI exeeded; close most profitable long position; CR improved, long OI improved")]
#[test_case( "240000000", "480000000", "-40000000", "-60000000", false, "15.0", false, true, 2, None, None; "CR greater than or equal to target, short OI exeeded; close least lossy short position; CR decreased, short OI improved")]
#[test_case( "240000000", "480000000", "-40000000", "-60000000", false, "15.0", false, true, 1, None, Some(PerpsContractError::DeleverageInvalidPosition { reason: "CR >= TCR and OI <= max OI".to_string()}); "CR greater than or equal to target, short OI exeeded; close most profitable long position; CR increased, short OI not improved; throw error")]
#[test_case( "-240000000", "-480000000", "40000000", "60000000", false, "5.0", false, true, 1, None, None; "CR greater than or equal to target, short OI exeeded; close most profitable short position; CR improved, short OI improved")]
#[test_case( "-240000000", "-480000000", "40000000", "60000000", false, "5.0", true, false, 2, None, None; "CR greater than or equal to target, long OI exeeded; close least lossy long position; CR decreased, long OI improved")]
#[test_case( "-240000000", "-480000000", "40000000", "60000000", false, "5.0", true, false, 1, None, Some(PerpsContractError::DeleverageInvalidPosition { reason: "CR >= TCR and OI <= max OI".to_string()}); "CR greater than or equal to target, long OI exeeded; close most profitable short position; CR increased, long OI not improved; throw error")]
#[test_case( "240000000", "480000000", "-4000000", "-6000000", true, "15.0", true, true, 1, None, None; "CR below target, OI exeeded; close most profitable long position; CR improved, long OI improved")]
#[test_case( "240000000", "480000000", "-4000000", "-6000000", true, "15.0", true, true, 0, None, None; "CR below target, OI exeeded; close second most profitable long position; CR improved, long OI improved")]
#[test_case( "240000000", "480000000", "-4000000", "-6000000", true, "15.0", true, true, 3, None, Some(PerpsContractError::DeleverageInvalidPosition { reason: "Position closure did not improve CR".to_string()}); "CR below target, OI exeeded; close most lossy short position; CR decreased; throw error")]
#[test_case( "-240000000", "-480000000", "4000000", "6000000", true, "5.0", true, true, 1, None, None; "CR below target, OI exeeded; close most profitable short position; CR improved, short OI improved")]
#[test_case( "-240000000", "-480000000", "4000000", "6000000", true, "5.0", true, true, 0, None, None; "CR below target, OI exeeded; close second most profitable short position; CR improved, short OI improved")]
#[test_case( "-240000000", "-480000000", "4000000", "6000000", true, "5.0", true, true, 3, None, Some(PerpsContractError::DeleverageInvalidPosition { reason: "Position closure did not improve CR".to_string()}); "CR below target, OI exeeded; close most lossy long position; CR decreased; throw error")]
#[allow(clippy::too_many_arguments)]
fn deleverage(
    acc_1_atom_pos: &str,
    acc_2_atom_pos: &str,
    acc_3_atom_pos: &str,
    acc_4_atom_pos: &str,
    cr_below_threshold: bool,
    atom_price: &str,
    long_oi_above_max: bool,
    short_oi_above_max: bool,
    acc_to_close: usize, // index of account to close (0 idx = acc_1, 1 idx = acc_2, ...)
    deleverage_enabled: Option<bool>,
    exp_error: Option<PerpsContractError>,
) {
    let acc_1_atom_pos = SignedUint::from_str(acc_1_atom_pos).unwrap();
    let acc_2_atom_pos = SignedUint::from_str(acc_2_atom_pos).unwrap();
    let acc_3_atom_pos = SignedUint::from_str(acc_3_atom_pos).unwrap();
    let acc_4_atom_pos = SignedUint::from_str(acc_4_atom_pos).unwrap();

    let atom_price = Decimal::from_str(atom_price).unwrap();

    let atom_pos = vec![acc_1_atom_pos, acc_2_atom_pos, acc_3_atom_pos, acc_4_atom_pos];
    let (atom_max_long_oi, atom_max_short_oi) =
        prepare_max_oi(atom_pos, atom_price, long_oi_above_max, short_oi_above_max);

    let target_collateralization_ratio = Decimal::from_str("3").unwrap();

    let cm_user_1 = Addr::unchecked("user_1");
    let cm_user_2 = Addr::unchecked("user_2");
    let cm_user_3 = Addr::unchecked("user_3");
    let cm_user_4 = Addr::unchecked("user_4");

    let vault_depositor = Addr::unchecked("vault_depositor");
    let contract_owner = Addr::unchecked("owner");

    let mut osmo_info = uosmo_info();
    osmo_info.price = Decimal::from_atomics(5u128, 1).unwrap();
    let mut atom_info = uatom_info();
    atom_info.price = Decimal::from_atomics(10u128, 0).unwrap();
    let mut tia_info = coin_info("utia");
    tia_info.price = Decimal::from_atomics(5u128, 0).unwrap();
    let mut usdc_info = coin_info("uusdc");
    usdc_info.price = Decimal::one();
    let osmo_cm_deposit = osmo_info.to_coin(10_000_000_000);
    let usdc_cm_deposit = usdc_info.to_coin(10_000_000_000);
    let usdc_vault_deposit = usdc_info.to_coin(10_000_000_000);

    let denom_to_close = atom_info.denom.clone();

    let mut mock = MockEnv::new()
        .owner(contract_owner.as_str())
        .target_vault_collaterization_ratio(target_collateralization_ratio)
        .deleverage_enabled(deleverage_enabled.unwrap_or(true))
        .set_params(&[osmo_info.clone(), atom_info.clone(), usdc_info.clone(), tia_info.clone()])
        .fund_accounts(
            vec![cm_user_1.clone(), cm_user_2.clone(), cm_user_3.clone(), cm_user_4.clone()],
            vec![osmo_cm_deposit.clone(), usdc_cm_deposit.clone()],
        )
        .fund_account(AccountToFund {
            addr: vault_depositor.clone(),
            funds: vec![usdc_vault_deposit.clone()],
        })
        .build()
        .unwrap();

    // setup perp params
    // tia perp
    let tia_perp_params = PerpParams {
        max_funding_velocity: Decimal::from_str("36").unwrap(),
        skew_scale: Uint128::new(4504227000000u128),
        ..default_perp_params(&tia_info.denom)
    };
    mock.update_perp_params(PerpParamsUpdate::AddOrUpdate {
        params: tia_perp_params,
    });
    // atom perp
    let mut atom_perp_params = PerpParams {
        max_funding_velocity: Decimal::from_str("36").unwrap(),
        skew_scale: Uint128::new(7227323000000u128),
        ..default_perp_params(&atom_info.denom)
    };
    mock.update_perp_params(PerpParamsUpdate::AddOrUpdate {
        params: atom_perp_params.clone(),
    });

    // create credit accounts
    let vault_depositor_acc = mock.create_credit_account(&vault_depositor).unwrap();
    let acc_1 = mock.create_credit_account(&cm_user_1).unwrap();
    let acc_2 = mock.create_credit_account(&cm_user_2).unwrap();
    let acc_3 = mock.create_credit_account(&cm_user_3).unwrap();
    let acc_4 = mock.create_credit_account(&cm_user_4).unwrap();
    let cm_perps_users =
        [cm_user_1.clone(), cm_user_2.clone(), cm_user_3.clone(), cm_user_4.clone()];
    let cm_perps_accs = [acc_1.clone(), acc_2.clone(), acc_3.clone(), acc_4.clone()];
    let acc_to_close = cm_perps_accs.get(acc_to_close).unwrap();

    // fund perps contract to cover pnl profit
    mock.update_credit_account(
        &vault_depositor_acc,
        &vault_depositor,
        vec![Deposit(usdc_vault_deposit.clone())],
        &[usdc_vault_deposit.clone()],
    )
    .unwrap();
    mock.deposit_to_perp_vault(&vault_depositor_acc, &usdc_vault_deposit, None).unwrap();

    // fund credit accounts
    for (user, acc) in cm_perps_users.iter().zip(cm_perps_accs.iter()) {
        mock.update_credit_account(
            acc,
            user,
            vec![Deposit(osmo_cm_deposit.clone()), Deposit(usdc_cm_deposit.clone())],
            &[osmo_cm_deposit.clone(), usdc_cm_deposit.clone()],
        )
        .unwrap();
    }

    // open tia perp positions:
    // - acc_1 short, profit
    // - acc_2 long, loss
    open_perp(
        &mut mock,
        &cm_user_1,
        &acc_1,
        &tia_info.denom,
        SignedUint::from_str("-100000000").unwrap(),
    );
    open_perp(
        &mut mock,
        &cm_user_2,
        &acc_2,
        &tia_info.denom,
        SignedUint::from_str("100000000").unwrap(),
    );

    // open perp positions
    open_perp(&mut mock, &cm_user_1, &acc_1, &atom_info.denom, acc_1_atom_pos);
    open_perp(&mut mock, &cm_user_2, &acc_2, &atom_info.denom, acc_2_atom_pos);
    open_perp(&mut mock, &cm_user_3, &acc_3, &atom_info.denom, acc_3_atom_pos);
    open_perp(&mut mock, &cm_user_4, &acc_4, &atom_info.denom, acc_4_atom_pos);

    // change OI values for atom
    let max_net_oi_value = min(atom_max_long_oi, atom_max_short_oi);
    atom_perp_params.max_net_oi_value = max_net_oi_value;
    atom_perp_params.max_long_oi_value = atom_max_long_oi;
    atom_perp_params.max_short_oi_value = atom_max_short_oi;
    mock.update_perp_params(PerpParamsUpdate::AddOrUpdate {
        params: atom_perp_params,
    });

    // move time forward
    mock.increment_by_time(86400); // 24 hours

    // 50% decrease in price for tia
    change_price(&mut mock, &tia_info.denom, Decimal::from_atomics(25u128, 1).unwrap());

    // increase price for atom
    change_price(&mut mock, &atom_info.denom, atom_price);

    // check perp vault balance
    let vault_usdc_balance_before = mock.query_balance(mock.perps.address(), &usdc_info.denom);

    // check account usdc balance
    let position = mock.query_positions(acc_to_close);
    let acc_usdc_deposit = get_coin(&usdc_info.denom, &position.deposits).amount;

    // check unrealized pnl
    let perp_position = mock.query_perp_position(acc_to_close, &denom_to_close).position.unwrap();
    let pnl = perp_position.unrealised_pnl.to_coins(&perp_position.base_denom).pnl;
    let mut pnl_profit = Uint128::zero();
    let mut pnl_loss = Uint128::zero();
    match pnl.clone() {
        PnL::Profit(coin) => {
            pnl_profit = coin.amount;
        }
        PnL::Loss(coin) => {
            pnl_loss = coin.amount;
        }
        PnL::BreakEven => {}
    };

    // check CR before deleverage
    let vault = mock.query_perp_vault(Some(ActionKind::Default)).unwrap();
    let cr_before = vault.collateralization_ratio.unwrap_or(Decimal::MAX);
    assert!(
        (cr_below_threshold && cr_before < target_collateralization_ratio)
            || (!cr_below_threshold && cr_before >= target_collateralization_ratio)
    );

    // remove Default price for all coins
    mock.remove_price(&osmo_info.denom, ActionKind::Default);
    mock.remove_price(&atom_info.denom, ActionKind::Default);
    mock.remove_price(&usdc_info.denom, ActionKind::Default);
    mock.remove_price(&tia_info.denom, ActionKind::Default);

    // deleverage
    let result = mock.deleverage(acc_to_close, &denom_to_close);

    // check result
    match (result, exp_error) {
        (Err(err), Some(exp_err)) => {
            let err: PerpsContractError = err.downcast().unwrap();
            assert_eq!(err, exp_err);
            return;
        }
        (Err(err), None) => {
            panic!("unexpected error: {:?}", err);
        }
        (Ok(_), Some(_)) => panic!("expected error, but got success"),
        (Ok(_), None) => {}
    }

    // check perp vault balance
    let vault_usdc_balance = mock.query_balance(mock.perps.address(), &usdc_info.denom);
    assert_eq!(vault_usdc_balance.amount, vault_usdc_balance_before.amount + pnl_loss - pnl_profit);

    // query the liquidatee's position with Default pricing should fail
    let res = mock.query_positions_with_action(acc_to_close, Some(ActionKind::Default));
    match res {
        Ok(positions) if !positions.perps.is_empty() => {
            // query should fail because Default pricing is removed
            panic!("expected error, but got success");
        }
        Ok(_) => {
            // no perps, no pricing needed
        }
        _ => {}
    }

    // check account usdc balance
    let position =
        mock.query_positions_with_action(acc_to_close, Some(ActionKind::Liquidation)).unwrap();
    assert_present(&position, &usdc_info.denom, acc_usdc_deposit + pnl_profit - pnl_loss);

    // query the vault with Default pricing should fail
    let res = mock.query_perp_vault(Some(ActionKind::Default));
    assert!(res.is_err());

    // Check CR after deleverage.
    // CR after deleverage should be greater than or equal to target CR or improved if CR was less than target CR before deleverage.
    let vault = mock.query_perp_vault(Some(ActionKind::Liquidation)).unwrap();
    let cr_after = vault.collateralization_ratio.unwrap_or(Decimal::MAX);
    let cr_after_ge_threshold = cr_after >= target_collateralization_ratio;
    let cr_improved = cr_after >= cr_before;
    assert!(cr_after_ge_threshold || cr_improved);
}

fn prepare_max_oi(
    positions: Vec<SignedUint>,
    atom_price: Decimal,
    long_oi_above_max: bool,
    short_oi_above_max: bool,
) -> (Uint128, Uint128) {
    let (shorts, longs): (Vec<_>, Vec<_>) =
        positions.into_iter().partition(|pos| pos.is_negative());
    let short_total_size = shorts.iter().map(|pos| pos.abs).sum::<Uint128>();
    let long_total_size = longs.iter().map(|pos| pos.abs).sum::<Uint128>();
    let short_oi = short_total_size * atom_price;
    let long_oi = long_total_size * atom_price;

    let max_long_oi = if long_oi_above_max {
        long_oi - Uint128::new(10)
    } else {
        long_oi + Uint128::new(10)
    };

    let max_short_oi = if short_oi_above_max {
        short_oi - Uint128::new(10)
    } else {
        short_oi + Uint128::new(10)
    };

    (max_long_oi, max_short_oi)
}

fn open_perp(mock: &mut MockEnv, user: &Addr, acc_id: &str, denom: &str, size: SignedUint) {
    mock.update_credit_account(
        acc_id,
        user,
        vec![ExecutePerpOrder {
            denom: denom.to_string(),
            order_size: size,
            reduce_only: None,
        }],
        &[],
    )
    .unwrap();
}

fn change_price(mock: &mut MockEnv, denom: &str, price: Decimal) {
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: denom.to_string(),
        price,
    });
    mock.price_change(CoinPrice {
        pricing: ActionKind::Liquidation,
        denom: denom.to_string(),
        price,
    });
}

fn assert_present(res: &Positions, denom: &str, amount: Uint128) {
    res.deposits.iter().find(|item| item.denom == denom && item.amount == amount).unwrap();
}
