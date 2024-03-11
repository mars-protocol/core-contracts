use std::collections::HashMap;

use cosmwasm_std::{coin, Uint128};
use mars_rover_health_computer::{HealthComputer, PerpsData, VaultsData};
use mars_types::{
    credit_manager::Positions,
    health::{AccountKind, BorrowTarget},
};

use super::helpers::{udai_info, umars_info};

#[test]
fn max_borrow_deposit_offset_good() {
    let udai = udai_info();

    let oracle_prices = HashMap::from([(udai.denom.clone(), udai.price)]);
    let asset_params = HashMap::from([(udai.denom.clone(), udai.params.clone())]);

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

    let max_borrow_amount =
        h.max_borrow_amount_estimate(&udai.denom, &BorrowTarget::Deposit).unwrap();
    assert_eq!(Uint128::new(6763), max_borrow_amount);
}

#[test]
fn max_borrow_deposit_offset_margin_of_error() {
    let umars = umars_info();

    let oracle_prices = HashMap::from([(umars.denom.clone(), umars.price)]);
    let asset_params = HashMap::from([(umars.denom.clone(), umars.params.clone())]);

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
            deposits: vec![coin(1200, &umars.denom)],
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

    let max_borrow_amount =
        h.max_borrow_amount_estimate(&umars.denom, &BorrowTarget::Deposit).unwrap();

    // Normally could be 4800, but conservative offset rounding has a margin of error
    assert_eq!(Uint128::new(4795), max_borrow_amount);
}
