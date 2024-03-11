use std::collections::HashMap;

use cosmwasm_std::{coin, Decimal, Uint128};
use mars_rover_health_computer::{HealthComputer, PerpsData, VaultsData};
use mars_types::{
    credit_manager::Positions,
    health::{AccountKind, SwapKind},
};

use super::helpers::{udai_info, umars_info};

#[test]
fn max_swap_default() {
    let udai = udai_info();
    let umars = umars_info();

    let oracle_prices =
        HashMap::from([(udai.denom.clone(), udai.price), (umars.denom.clone(), umars.price)]);

    let asset_params = HashMap::from([
        (udai.denom.clone(), udai.params.clone()),
        (umars.denom.clone(), umars.params.clone()),
    ]);

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: Default::default(),
    };

    let perps_data = PerpsData {
        denom_states: Default::default(),
        params: Default::default(),
    };

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(1200, &udai.denom)],
            debts: vec![],
            lends: vec![],
            vaults: vec![],
            perps: vec![],
            perp_vault: None,
        },
        asset_params,
        oracle_prices,
        vaults_data,
        perps_data,
    };

    let max_borrow_amount = h
        .max_swap_amount_estimate(&udai.denom, &umars.denom, &SwapKind::Default, Decimal::zero())
        .unwrap();
    assert_eq!(Uint128::new(1200), max_borrow_amount);
}

#[test]
fn max_swap_margin() {
    let udai = udai_info();
    let umars = umars_info();

    let oracle_prices =
        HashMap::from([(udai.denom.clone(), udai.price), (umars.denom.clone(), umars.price)]);

    let asset_params = HashMap::from([
        (udai.denom.clone(), udai.params.clone()),
        (umars.denom.clone(), umars.params.clone()),
    ]);

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: Default::default(),
    };

    let perps_data = PerpsData {
        denom_states: Default::default(),
        params: Default::default(),
    };

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(5000, &udai.denom), coin(500, &umars.denom)],
            debts: vec![],
            lends: vec![],
            vaults: vec![],
            perps: vec![],
            perp_vault: None,
        },
        asset_params,
        oracle_prices,
        vaults_data,
        perps_data,
    };

    let max_borrow_amount = h
        .max_swap_amount_estimate(&udai.denom, &umars.denom, &SwapKind::Margin, Decimal::zero())
        .unwrap();
    assert_eq!(Uint128::new(31351), max_borrow_amount);
}
