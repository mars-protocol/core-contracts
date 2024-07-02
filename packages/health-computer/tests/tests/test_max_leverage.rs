use std::{collections::HashMap, str::FromStr};

use cosmwasm_std::{coin, Decimal, Uint128};
use mars_perps::position::{self, PositionExt};
use mars_rover_health_computer::{HealthComputer, PerpsData};
use mars_types::{
    credit_manager::{DebtAmount, Positions},
    health::AccountKind,
    math::SignedDecimal,
    params::{AssetParams, PerpParams},
    perps::{PerpPosition, Position},
    signed_uint::SignedUint,
};

use super::helpers::CoinInfo;
use crate::tests::helpers::{
    create_coin_info, create_default_funding, create_default_perp_info, create_perp_denom_state,
};

#[test]
fn currently_long_max_q_change() {
    // denoms
    let base_denom = "uusdc".to_string();
    let eth_perp_denom = "eth/usd/perp".to_string();

    // market state
    let long_oi = SignedUint::from_str("100000000").unwrap();
    let short_oi = SignedUint::from_str("500000000").unwrap();
    let skew = long_oi.checked_sub(short_oi).unwrap();

    // perp state
    let funding = create_default_funding();

    let eth_perp_params = PerpParams {
        opening_fee_rate: Decimal::from_str("0.2").unwrap(),
        closing_fee_rate: Decimal::from_str("0.003").unwrap(),
        max_long_oi_value: Uint128::new(6000000000000),
        max_short_oi_value: Uint128::new(6000000000000),
        max_net_oi_value: Uint128::new(100000000000000),
        ..produce_eth_perp_params()
    };

    let eth_denom_state = create_perp_denom_state(long_oi.abs, short_oi.abs, funding.clone());

    let perps_data = PerpsData {
        denom_states: HashMap::from([(eth_perp_params.denom.clone(), eth_denom_state)]),
        params: HashMap::from([(eth_perp_params.denom.clone(), eth_perp_params.clone())]),
    };

    // prices
    let base_denom_price = Decimal::one();
    let current_eth_perp_price = Decimal::from_str("2000").unwrap();
    let entry_eth_perp_price = Decimal::from_str("2000").unwrap();

    let mut oracle_prices = produce_default_prices();
    oracle_prices.insert(eth_perp_denom.clone(), current_eth_perp_price);

    let asset_params = produce_default_asset_params();

    // position state
    // let size: SignedDecimal = Decimal::from_str("0.5").unwrap().into();
    let size = SignedUint::from_str("500000").unwrap();

    let entry_accrued_funding_per_unit_in_base_denom = SignedDecimal::from_str("5").unwrap();
    let entry_exec_price = Decimal::from_str("1999").unwrap();
    let position = Position {
        size,
        entry_price: entry_eth_perp_price,
        entry_exec_price,
        entry_accrued_funding_per_unit_in_base_denom,
        initial_skew: long_oi.checked_sub(short_oi).unwrap(),
        realized_pnl: Default::default(),
    };

    let pnl_amounts = position
        .compute_pnl(
            &funding,
            skew,
            current_eth_perp_price,
            base_denom_price,
            eth_perp_params.opening_fee_rate,
            eth_perp_params.closing_fee_rate,
            position::PositionModification::Decrease(size),
        )
        .unwrap();

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(50000000, base_denom.clone()), coin(1000000000, "uosmo")],
            debts: vec![
                DebtAmount {
                    amount: Uint128::new(1000000),
                    denom: base_denom.clone(),
                    shares: Uint128::new(100),
                },
                DebtAmount {
                    amount: Uint128::new(1000000),
                    denom: "uatom".to_string(),
                    shares: Uint128::new(100),
                },
            ],
            lends: vec![],
            vaults: vec![],
            perps: vec![PerpPosition {
                base_denom: base_denom.clone(),
                entry_exec_price,
                current_exec_price: Decimal::from_str("1999.9999").unwrap(),
                denom: eth_perp_params.denom.clone(),
                closing_fee_rate: eth_perp_params.closing_fee_rate,
                current_price: current_eth_perp_price,
                size,
                entry_price: Decimal::from_str("2000").unwrap(),
                realised_pnl: Default::default(),
                unrealised_pnl: pnl_amounts,
            }],
            perp_vault: None,
        },
        oracle_prices,
        asset_params,
        vaults_data: Default::default(),
        perps_data,
    };

    let max_long = h
        .max_perp_size_estimate(
            &eth_perp_denom.clone(),
            &base_denom.clone(),
            long_oi.abs,
            short_oi.abs,
            &mars_rover_health_computer::Direction::Long,
        )
        .unwrap();

    assert_eq!(max_long, SignedUint::from_str("1835738").unwrap());

    // Flip position test
    let max_short = h
        .max_perp_size_estimate(
            &eth_perp_denom.clone(),
            &base_denom.clone(),
            long_oi.abs,
            short_oi.abs,
            &mars_rover_health_computer::Direction::Short,
        )
        .unwrap();

    assert_eq!(max_short, SignedUint::from_str("-1460182").unwrap());
}

#[test]
fn max_position_size_zero_if_net_oi_exceeded() {
    // inputs
    let base_denom = "uusdc".to_string();
    let eth_perp_denom = "eth/usd/perp".to_string();

    // prices
    let base_denom_price = Decimal::one();
    let current_eth_perp_price: Decimal = Decimal::from_str("2000").unwrap();
    let entry_eth_perp_price = Decimal::from_str("2000").unwrap();

    // market state
    let long_oi = SignedUint::from_str("100000000").unwrap();
    let short_oi = SignedUint::from_str("500000000").unwrap();
    let skew = long_oi.checked_sub(short_oi).unwrap();

    // perp state
    let funding = create_default_funding();
    let eth_perp_params = PerpParams {
        opening_fee_rate: Decimal::from_str("0.2").unwrap(),
        closing_fee_rate: Decimal::from_str("0.003").unwrap(),
        max_long_oi_value: Uint128::new(60000000),
        max_short_oi_value: Uint128::new(60000000),
        max_net_oi_value: Uint128::new(400000000),
        ..produce_eth_perp_params()
    };

    let eth_denom_state = create_perp_denom_state(long_oi.abs, short_oi.abs, funding.clone());

    let perps_data = PerpsData {
        denom_states: HashMap::from([(eth_perp_params.denom.clone(), eth_denom_state)]),
        params: HashMap::from([(eth_perp_params.denom.clone(), eth_perp_params.clone())]),
    };

    let mut oracle_prices = produce_default_prices();
    oracle_prices.insert(eth_perp_denom.clone(), current_eth_perp_price);

    let asset_params = produce_default_asset_params();

    // position state
    let size = SignedUint::from_str("500000").unwrap();

    let entry_accrued_funding_per_unit_in_base_denom = SignedDecimal::from_str("100").unwrap();
    let entry_exec_price = Decimal::from_str("1999").unwrap();
    let current_exec_price = Decimal::from_str("1199.5").unwrap();
    let position = Position {
        size,
        entry_price: entry_eth_perp_price,
        entry_exec_price,
        entry_accrued_funding_per_unit_in_base_denom,
        initial_skew: SignedUint::zero(),
        realized_pnl: Default::default(),
    };

    let pnl_amounts = position
        .compute_pnl(
            &funding,
            skew,
            current_eth_perp_price,
            base_denom_price,
            eth_perp_params.opening_fee_rate,
            eth_perp_params.closing_fee_rate,
            position::PositionModification::Decrease(size),
        )
        .unwrap();

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(50, base_denom.clone()), coin(1000, "uosmo")],
            debts: vec![
                DebtAmount {
                    amount: Uint128::new(1),
                    denom: base_denom.clone(),
                    shares: Uint128::new(100),
                },
                DebtAmount {
                    amount: Uint128::new(1),
                    denom: "uatom".to_string(),
                    shares: Uint128::new(100),
                },
            ],
            lends: vec![],
            vaults: vec![],
            perps: vec![PerpPosition {
                base_denom: base_denom.clone(),
                entry_exec_price,
                current_exec_price,
                denom: eth_perp_params.denom.clone(),
                closing_fee_rate: eth_perp_params.closing_fee_rate,
                current_price: current_eth_perp_price,
                size,
                entry_price: Decimal::from_str("2000").unwrap(),
                realised_pnl: Default::default(),
                unrealised_pnl: pnl_amounts,
            }],
            perp_vault: None,
        },
        oracle_prices,
        asset_params,
        vaults_data: Default::default(),
        perps_data,
    };

    let result = h
        .max_perp_size_estimate(
            &eth_perp_denom.clone(),
            &base_denom.clone(),
            long_oi.abs,
            short_oi.abs,
            &mars_rover_health_computer::Direction::Long,
        )
        .unwrap();

    assert_eq!(result, SignedUint::zero());
}

#[test]
fn max_position_size_zero_if_long_oi_exceeded() {
    // inputs
    let base_denom = "uusdc".to_string();
    let eth_perp_denom = "eth/usd/perp".to_string();

    // prices
    let base_denom_price = Decimal::one();
    let current_eth_perp_price = Decimal::from_str("2000").unwrap();
    let entry_eth_perp_price = Decimal::from_str("2000").unwrap();

    // market state
    let long_oi = SignedUint::from_str("100000000").unwrap();
    let short_oi = SignedUint::from_str("500000000").unwrap();
    let skew = long_oi.checked_sub(short_oi).unwrap();

    // perp state
    let funding = create_default_funding();
    let eth_perp_params = PerpParams {
        opening_fee_rate: Decimal::from_str("0.2").unwrap(),
        closing_fee_rate: Decimal::from_str("0.003").unwrap(),
        // Only selling :)
        max_long_oi_value: Uint128::new(0),
        max_short_oi_value: Uint128::new(60000000000),
        max_net_oi_value: Uint128::new(10000000000),
        ..produce_eth_perp_params()
    };

    let eth_denom_state = create_perp_denom_state(long_oi.abs, short_oi.abs, funding.clone());

    let perps_data = PerpsData {
        denom_states: HashMap::from([(eth_perp_params.denom.clone(), eth_denom_state)]),
        params: HashMap::from([(eth_perp_params.denom.clone(), eth_perp_params.clone())]),
    };

    let mut oracle_prices = produce_default_prices();
    oracle_prices.insert(eth_perp_denom.clone(), current_eth_perp_price);

    let asset_params = produce_default_asset_params();

    // position state
    let size = SignedUint::from_str("500000").unwrap();

    let entry_accrued_funding_per_unit_in_base_denom = SignedDecimal::from_str("100").unwrap();
    let entry_exec_price = Decimal::from_str("1999").unwrap();
    let current_exec_price = Decimal::from_str("1199.5").unwrap();
    let position = Position {
        size,
        entry_price: entry_eth_perp_price,
        entry_exec_price,
        entry_accrued_funding_per_unit_in_base_denom,
        initial_skew: SignedUint::zero(),
        realized_pnl: Default::default(),
    };

    let pnl_amounts = position
        .compute_pnl(
            &funding,
            skew,
            current_eth_perp_price,
            base_denom_price,
            eth_perp_params.opening_fee_rate,
            eth_perp_params.closing_fee_rate,
            position::PositionModification::Decrease(size),
        )
        .unwrap();

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(50, base_denom.clone()), coin(1000, "uosmo")],
            debts: vec![
                DebtAmount {
                    amount: Uint128::new(1),
                    denom: base_denom.clone(),
                    shares: Uint128::new(100),
                },
                DebtAmount {
                    amount: Uint128::new(1),
                    denom: "uatom".to_string(),
                    shares: Uint128::new(100),
                },
            ],
            lends: vec![],
            vaults: vec![],
            perps: vec![PerpPosition {
                base_denom: base_denom.clone(),
                entry_exec_price,
                current_exec_price,
                denom: eth_perp_params.denom.clone(),
                closing_fee_rate: eth_perp_params.closing_fee_rate,
                current_price: current_eth_perp_price,
                size,
                entry_price: Decimal::from_str("2000").unwrap(),
                realised_pnl: Default::default(),
                unrealised_pnl: pnl_amounts,
            }],
            perp_vault: None,
        },
        oracle_prices,
        asset_params,
        vaults_data: Default::default(),
        perps_data,
    };

    let result = h
        .max_perp_size_estimate(
            &eth_perp_denom.clone(),
            &base_denom.clone(),
            long_oi.abs,
            short_oi.abs,
            &mars_rover_health_computer::Direction::Long,
        )
        .unwrap();

    assert_eq!(result, SignedUint::zero());
}

#[test]
fn existing_short_max_q_change() {
    // inputs
    let base_denom = "uusdc".to_string();
    let eth_perp_denom = "eth/usd/perp".to_string();

    // prices
    let base_denom_price = Decimal::one();
    let current_eth_perp_price = Decimal::from_str("2000").unwrap();
    let entry_eth_perp_price = Decimal::from_str("2000").unwrap();

    // market state
    let long_oi = SignedUint::from_str("100000000").unwrap();
    let short_oi = SignedUint::from_str("500000000").unwrap();
    let skew = long_oi.checked_sub(short_oi).unwrap();

    // perp state
    let mut funding = create_default_funding();
    funding.last_funding_accrued_per_unit_in_base_denom = SignedDecimal::from_str("3").unwrap();
    let eth_perp_params = PerpParams {
        opening_fee_rate: Decimal::from_str("0.2").unwrap(),
        closing_fee_rate: Decimal::from_str("0.003").unwrap(),
        max_long_oi_value: Uint128::new(6000000000000),
        max_short_oi_value: Uint128::new(6000000000000),
        max_net_oi_value: Uint128::new(50000000000000),
        ..produce_eth_perp_params()
    };

    let eth_denom_state = create_perp_denom_state(long_oi.abs, short_oi.abs, funding.clone());

    let perps_data = PerpsData {
        denom_states: HashMap::from([(eth_perp_params.denom.clone(), eth_denom_state)]),
        params: HashMap::from([(eth_perp_params.denom.clone(), eth_perp_params.clone())]),
    };

    let mut oracle_prices = produce_default_prices();
    oracle_prices.insert(eth_perp_denom.clone(), current_eth_perp_price);

    let asset_params = produce_default_asset_params();

    // position state
    let size = SignedUint::from_str("-1000000").unwrap();

    let entry_accrued_funding_per_unit_in_base_denom = SignedDecimal::from_str("2").unwrap();
    let entry_exec_price = Decimal::from_str("1999").unwrap();
    let current_exec_price = Decimal::from_str("1201").unwrap();

    let position = Position {
        size,
        entry_price: entry_eth_perp_price,
        entry_exec_price,
        entry_accrued_funding_per_unit_in_base_denom,
        initial_skew: SignedUint::zero(),
        realized_pnl: Default::default(),
    };

    let pnl_amounts = position
        .compute_pnl(
            &funding,
            skew,
            current_eth_perp_price,
            base_denom_price,
            eth_perp_params.opening_fee_rate,
            eth_perp_params.closing_fee_rate,
            position::PositionModification::Decrease(size),
        )
        .unwrap();

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(50000000, base_denom.clone()), coin(1000000000, "uosmo")],
            debts: vec![
                DebtAmount {
                    amount: Uint128::new(1000000),
                    denom: base_denom.clone(),
                    shares: Uint128::new(100),
                },
                DebtAmount {
                    amount: Uint128::new(1000000),
                    denom: "uatom".to_string(),
                    shares: Uint128::new(100),
                },
            ],
            lends: vec![],
            vaults: vec![],
            perps: vec![PerpPosition {
                base_denom: base_denom.clone(),
                entry_exec_price,
                current_exec_price,
                denom: eth_perp_params.denom.clone(),
                closing_fee_rate: eth_perp_params.closing_fee_rate,
                current_price: current_eth_perp_price,
                size,
                entry_price: Decimal::from_str("2000").unwrap(),
                realised_pnl: Default::default(),
                unrealised_pnl: pnl_amounts,
            }],
            perp_vault: None,
        },
        asset_params,
        oracle_prices,
        vaults_data: Default::default(),
        perps_data,
    };

    let max_short = h
        .max_perp_size_estimate(
            &eth_perp_denom.clone(),
            &base_denom.clone(),
            long_oi.abs,
            short_oi.abs,
            &mars_rover_health_computer::Direction::Short,
        )
        .unwrap();

    assert_eq!(max_short, SignedUint::from_str("-2204204").unwrap(),);

    let max_long = h
        .max_perp_size_estimate(
            &eth_perp_denom.clone(),
            &base_denom.clone(),
            long_oi.abs,
            short_oi.abs,
            &mars_rover_health_computer::Direction::Long,
        )
        .unwrap();

    assert_eq!(max_long, SignedUint::from_str("1453092").unwrap(),);
}

#[test]
fn no_existing_perp_position() {
    // inputs
    let base_denom = "uusdc".to_string();
    let eth_perp_denom = "eth/usd/perp".to_string();

    // prices
    let current_eth_perp_price = Decimal::from_str("2000").unwrap();

    // market state
    let long_oi: SignedUint = SignedUint::from_str("100000000").unwrap();
    let short_oi: SignedUint = SignedUint::from_str("500000000").unwrap();

    // perp state
    let mut funding = create_default_funding();
    funding.last_funding_accrued_per_unit_in_base_denom = SignedDecimal::from_str("200").unwrap();
    let eth_perp_params = PerpParams {
        opening_fee_rate: Decimal::from_str("0.2").unwrap(),
        closing_fee_rate: Decimal::from_str("0.003").unwrap(),
        max_long_oi_value: Uint128::new(6000000000000),
        max_short_oi_value: Uint128::new(6000000000000),
        max_net_oi_value: Uint128::new(40000000000000),
        ..produce_eth_perp_params()
    };

    let eth_denom_state = create_perp_denom_state(long_oi.abs, short_oi.abs, funding);
    let perps_data = PerpsData {
        denom_states: HashMap::from([(eth_perp_params.denom.clone(), eth_denom_state)]),
        params: HashMap::from([(eth_perp_params.denom.clone(), eth_perp_params.clone())]),
    };

    let mut oracle_prices = produce_default_prices();
    oracle_prices.insert(eth_perp_denom.clone(), current_eth_perp_price);

    let asset_params = produce_default_asset_params();

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![
                coin(50000000, base_denom.clone()),
                coin(1000000000, "uosmo".to_string()),
            ],
            debts: vec![
                DebtAmount {
                    amount: Uint128::new(1000000),
                    denom: base_denom.clone(),
                    shares: Uint128::new(100),
                },
                DebtAmount {
                    amount: Uint128::new(1000000),
                    denom: "uatom".to_string(),
                    shares: Uint128::new(100),
                },
            ],
            lends: vec![],
            vaults: vec![],
            perps: vec![],
            perp_vault: None,
        },
        oracle_prices,
        asset_params,
        vaults_data: Default::default(),
        perps_data,
    };

    let result = h
        .max_perp_size_estimate(
            &eth_perp_denom.clone(),
            &base_denom.clone(),
            long_oi.abs,
            short_oi.abs,
            &mars_rover_health_computer::Direction::Long,
        )
        .unwrap();

    assert_eq!(result, SignedUint::from_str("1465698").unwrap());
}

// TODO add test setup function to generate and manage state for tests to reduce repition.
// COINS
fn produce_usdc_coin_info() -> CoinInfo {
    create_coin_info(
        "uusdc".to_string(),
        Decimal::one(),
        Decimal::from_ratio(Uint128::new(85), Uint128::new(100)),
        Decimal::from_ratio(Uint128::new(87), Uint128::new(100)),
    )
}

fn produce_eth_coin_info() -> CoinInfo {
    create_coin_info(
        "ueth".to_string(),
        Decimal::one(),
        Decimal::from_ratio(Uint128::new(80), Uint128::new(100)),
        Decimal::from_ratio(Uint128::new(82), Uint128::new(100)),
    )
}

fn produce_osmo_coin_info() -> CoinInfo {
    create_coin_info(
        "uosmo".to_string(),
        Decimal::one(),
        Decimal::from_ratio(Uint128::new(75), Uint128::new(100)),
        Decimal::from_ratio(Uint128::new(77), Uint128::new(100)),
    )
}

fn produce_atom_coin_info() -> CoinInfo {
    create_coin_info(
        "uatom".to_string(),
        Decimal::one(),
        Decimal::from_ratio(Uint128::new(75), Uint128::new(100)),
        Decimal::from_ratio(Uint128::new(77), Uint128::new(100)),
    )
}

fn produce_default_prices() -> HashMap<String, Decimal> {
    let usdc_coin_info = produce_usdc_coin_info();
    let eth_coin_info = produce_eth_coin_info();
    let osmo_coin_info = produce_osmo_coin_info();
    let atom_coin_info = produce_atom_coin_info();

    HashMap::from([
        (eth_coin_info.denom.clone(), eth_coin_info.price),
        (usdc_coin_info.denom.clone(), usdc_coin_info.price),
        (osmo_coin_info.denom.clone(), osmo_coin_info.price),
        (atom_coin_info.denom.clone(), atom_coin_info.price),
    ])
}

fn produce_default_asset_params() -> HashMap<String, AssetParams> {
    let usdc_coin_info = produce_usdc_coin_info();
    let eth_coin_info = produce_eth_coin_info();
    let osmo_coin_info = produce_osmo_coin_info();
    let atom_coin_info = produce_atom_coin_info();

    HashMap::from([
        (eth_coin_info.denom.clone(), eth_coin_info.params),
        (osmo_coin_info.denom.clone(), osmo_coin_info.params.clone()),
        (usdc_coin_info.denom.clone(), usdc_coin_info.params.clone()),
        (atom_coin_info.denom.clone(), atom_coin_info.params.clone()),
    ])
}

fn produce_eth_perp_params() -> PerpParams {
    let default_perp_info = create_default_perp_info();

    PerpParams {
        denom: "eth/usd/perp".to_string(),
        max_loan_to_value: Decimal::from_str("0.93333333").unwrap(),
        liquidation_threshold: Decimal::from_str("0.95").unwrap(),
        ..default_perp_info
    }
}
