use std::collections::HashMap;

use cosmwasm_std::{coin, Uint128};
use mars_rover_health_computer::{HealthComputer, PerpsData, VaultsData};
use mars_types::{
    credit_manager::Positions,
    health::{AccountKind, BorrowTarget},
};

use super::helpers::{osmo_atom_1_config, udai_info, umars_info};

#[test]
fn max_borrow_vault_offset_good() {
    let udai = udai_info();
    let osmo_atom_1_config = osmo_atom_1_config();

    let params = HashMap::from([(udai.denom.clone(), udai.params.clone())]);
    let prices = HashMap::from([(udai.denom.clone(), udai.price)]);

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: HashMap::from([(
            osmo_atom_1_config.addr.clone(),
            osmo_atom_1_config.clone(),
        )]),
    };

    let perps_data = Default::default();

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
        },
        asset_params: params,
        oracle_prices: prices,
        vaults_data,
        perps_data,
    };

    let max_borrow_amount = h
        .max_borrow_amount_estimate(
            &udai.denom,
            &BorrowTarget::Vault {
                address: osmo_atom_1_config.addr,
            },
        )
        .unwrap();

    assert_eq!(Uint128::new(3381), max_borrow_amount);
}

#[test]
fn max_borrow_vault_offset_margin_of_error() {
    let umars = umars_info();
    let osmo_atom_1_config = osmo_atom_1_config();

    let prices = HashMap::from([(umars.denom.clone(), umars.price)]);
    let params = HashMap::from([(umars.denom.clone(), umars.params.clone())]);

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: HashMap::from([(
            osmo_atom_1_config.addr.clone(),
            osmo_atom_1_config.clone(),
        )]),
    };

    let perps_data = PerpsData {
        params: Default::default(),
    };

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            account_kind: AccountKind::Default,

            deposits: vec![coin(1200, &umars.denom)],
            debts: vec![],
            lends: vec![],
            vaults: vec![],
            staked_astro_lps: vec![],
            perps: vec![],
        },
        asset_params: params,
        oracle_prices: prices,
        vaults_data,
        perps_data,
    };

    let max_borrow_amount = h
        .max_borrow_amount_estimate(
            &umars.denom,
            &BorrowTarget::Vault {
                address: osmo_atom_1_config.addr,
            },
        )
        .unwrap();

    // Normally could be 3200, but conservative offset rounding has a margin of error
    assert_eq!(Uint128::new(3196), max_borrow_amount);
}
