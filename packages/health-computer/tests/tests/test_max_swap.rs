use std::collections::HashMap;

use cosmwasm_std::{coin, Decimal, Uint128};
use mars_rover_health_computer::{HealthComputer, PerpsData, VaultsData};
use mars_types::{
    credit_manager::{DebtAmount, Positions},
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
        params: Default::default(),
    };

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            account_kind: AccountKind::Default,

            deposits: vec![coin(1200, &udai.denom)],
            debts: vec![],
            lends: vec![],
            vaults: vec![],
            staked_astro_lps: vec![],
            perps: vec![],
            perp_vault: None,
        },
        asset_params,
        oracle_prices,
        vaults_data,
        perps_data,
    };

    let max_borrow_amount = h
        .max_swap_amount_estimate(
            &udai.denom,
            &umars.denom,
            &SwapKind::Default,
            Decimal::zero(),
            false,
        )
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
        params: Default::default(),
    };

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            account_kind: AccountKind::Default,

            deposits: vec![coin(5000, &udai.denom), coin(500, &umars.denom)],
            debts: vec![],
            lends: vec![],
            vaults: vec![],
            staked_astro_lps: vec![],
            perps: vec![],
            perp_vault: None,
        },
        asset_params,
        oracle_prices,
        vaults_data,
        perps_data,
    };

    let max_borrow_amount = h
        .max_swap_amount_estimate(
            &udai.denom,
            &umars.denom,
            &SwapKind::Margin,
            Decimal::zero(),
            false,
        )
        .unwrap();
    assert_eq!(Uint128::new(31351), max_borrow_amount);
}

#[test]
fn max_swap_repaying_debt() {
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
        params: Default::default(),
    };

    // Create unhealthy position
    // Deposits = 0.313451 * 5000 = 1567.55
    // Debts = 1 * 20000 = 2000
    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            account_kind: AccountKind::Default,

            deposits: vec![coin(5000, &udai.denom)],
            debts: vec![DebtAmount {
                denom: umars.denom.to_string(),
                amount: Uint128::new(2000u128),
                shares: Uint128::new(1u128),
            }],
            lends: vec![],
            vaults: vec![],
            staked_astro_lps: vec![],
            perps: vec![],
            perp_vault: None,
        },
        asset_params,
        oracle_prices,
        vaults_data,
        perps_data,
    };

    // If not repaying debt, expect 0.
    let max_borrow_amount = h
        .max_swap_amount_estimate(
            &udai.denom,
            &umars.denom,
            &SwapKind::Default,
            Decimal::zero(),
            false,
        )
        .unwrap();
    assert_eq!(Uint128::zero(), max_borrow_amount);

    // When repaying debt, expect max borrow amount.
    let max_borrow_amount = h
        .max_swap_amount_estimate(
            &udai.denom,
            &umars.denom,
            &SwapKind::Default,
            Decimal::zero(),
            true,
        )
        .unwrap();

    assert_eq!(Uint128::new(5000), max_borrow_amount);
}

#[test]
fn max_swap_from_ltv_zero() {
    let udai = udai_info();
    let umars = umars_info();

    let oracle_prices = HashMap::from([(umars.denom.clone(), umars.price)]);

    let asset_params = HashMap::from([(umars.denom.clone(), umars.params.clone())]);

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: Default::default(),
    };

    let perps_data = PerpsData {
        params: Default::default(),
    };

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            account_kind: AccountKind::Default,
            deposits: vec![coin(5000, &udai.denom)],
            debts: vec![],
            lends: vec![],
            vaults: vec![],
            staked_astro_lps: vec![],
            perps: vec![],
            perp_vault: None,
        },
        asset_params,
        oracle_prices,
        vaults_data,
        perps_data,
    };

    let max_swap_amount = h
        .max_swap_amount_estimate(
            &udai.denom,
            &umars.denom,
            &SwapKind::Default,
            Decimal::zero(),
            false,
        )
        .unwrap();
    assert_eq!(Uint128::new(5000), max_swap_amount);
}

#[test]
fn max_swap_both_ltv_zero() {
    let udai = udai_info();
    let umars = umars_info();

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: Default::default(),
    };

    let perps_data = PerpsData {
        params: Default::default(),
    };

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            account_kind: AccountKind::Default,
            deposits: vec![coin(5000, &udai.denom)],
            debts: vec![],
            lends: vec![],
            vaults: vec![],
            staked_astro_lps: vec![],
            perps: vec![],
            perp_vault: None,
        },
        asset_params: HashMap::new(),
        oracle_prices: HashMap::new(),
        vaults_data,
        perps_data,
    };

    let max_swap_amount = h
        .max_swap_amount_estimate(
            &udai.denom,
            &umars.denom,
            &SwapKind::Default,
            Decimal::zero(),
            false,
        )
        .unwrap();
    assert_eq!(Uint128::new(5000), max_swap_amount);
}
