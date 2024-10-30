use std::{collections::HashMap, str::FromStr};

use cosmwasm_std::{coin, Decimal, Int128, SignedDecimal, Uint128};
use mars_perps::position::{self, PositionExt};
use mars_rover_health_computer::{HealthComputer, PerpsData};
use mars_types::{
    credit_manager::{DebtAmount, Positions},
    health::AccountKind,
    params::{AssetParams, PerpParams},
    perps::{PerpPosition, Position},
};
use test_case::test_case;

use super::helpers::CoinInfo;
use crate::tests::helpers::{create_coin_info, create_default_funding, create_default_perp_info};

#[test_case(
    "1465698",
    "-1465698",
    vec![],
    None,
    "100000000",
    "500000000";
    "No existing perp position"
)]
#[test_case(
    "1338102",
    "-500000",
    vec![Int128::from_str("500000").unwrap()],
    Some(PerpParams {
        max_long_oi_value: Uint128::new(600000000000),
        max_short_oi_value: Uint128::new(600000000000),
        max_net_oi_value: Uint128::new(40000000),
        ..produce_eth_perp_params()
    }),
    "100000000",
    "500000000";
    "Max short size limited to position size if NET OI exceeded on short side"
)]
#[test_case(
    "0",
    "-1962546",
    vec![Int128::from_str("500000").unwrap()],
    Some(PerpParams {
        max_long_oi_value: Uint128::new(0),
        max_short_oi_value: Uint128::new(8000000000000),
        max_net_oi_value: Uint128::new(12000000000000),
        ..produce_eth_perp_params()
    }),
    "100000000",
    "500000000";
    "Max LONG size 0 if LONG OI exceeded"
)]
#[test_case(
    "1338102",
    "-500000",
    vec![Int128::from_str("500000").unwrap()],
    Some(PerpParams {
        max_long_oi_value: Uint128::new(8000000000000),
        max_short_oi_value: Uint128::new(0),
        max_net_oi_value: Uint128::new(12000000000000),
        ..produce_eth_perp_params()
    }),
    "100000000",
    "500000000";
    "Max SHORT size limited to position size if SHORT OI exceeded"
)]
#[test_case(
    "1200000",
    "-1300000",
    vec![],
    Some(PerpParams {
        max_long_oi_value: Uint128::new(202400000000), // 2400000000 OI left, divided by price 2000, max LONG = 1200000
        max_short_oi_value: Uint128::new(1002600000000), // 2600000000 OI left, divided by price 2000, max SHORT = 1300000
        max_net_oi_value: Uint128::new(120000000000000),
        ..produce_eth_perp_params()
    }),
    "100000000",
    "500000000";
    "Max size up to max LONG and SHORT OI"
)]
#[test_case(
    "1000000",
    "-1120000",
    vec![Int128::from_str("-1000000").unwrap()],
    Some(PerpParams {
        max_long_oi_value: Uint128::new(0),
        max_short_oi_value: Uint128::new(2500000000*2000),
        max_net_oi_value: Uint128::new(1120000*2000),
        ..produce_eth_perp_params()
    }),
    "0",
    "0";
    "Can close short position when max oi for long is 0"
)]
#[test_case(
    "1120000",
    "-1000000",
    vec![Int128::from_str("1000000").unwrap()],
    Some(PerpParams {
        max_long_oi_value: Uint128::new(2500000000*2000),
        max_short_oi_value: Uint128::new(0),
        max_net_oi_value: Uint128::new(1120000*2000),
        ..produce_eth_perp_params()
    }),
    "0",
    "0";
    "Can close long position when max oi for short is 0"
)]
#[test_case(
    "1001000",
    "-1120000",
    vec![Int128::from_str("-1000000").unwrap()],
    Some(PerpParams {
        max_long_oi_value: Uint128::new(1000*2000),
        max_short_oi_value: Uint128::new(2500000000*2000),
        max_net_oi_value: Uint128::new(1120000*2000),
        ..produce_eth_perp_params()
    }),
    "0",
    "0";
    "Can flip position Short to Long when max oi for closing direction is limited"
)]
#[test_case(
    "1120000",
    "-1120000",
    vec![],
    Some(PerpParams {
        max_long_oi_value: Uint128::new(5000000000000),
        max_short_oi_value: Uint128::new(5000000000000),
        max_net_oi_value: Uint128::new(2240000000), // 2240000000 OI left, divided by price 2000, max LONG = 1120000, max SHORT = 1120000
        ..produce_eth_perp_params()
    }),
    "200000000",
    "200000000";
    "Max size up to max NET OI"
)]
#[test_case(
    "2453092",
    "-1204204",
    vec![Int128::from_str("-1000000").unwrap()],
    None,
    "100000000",
    "500000000";
    "Existing short position"
)]
#[test_case(
    "1338102",
    "-1962546",
    vec![Int128::from_str("500000").unwrap()],
    None,
    "100000000",
    "500000000";
    "Existing long position"
)]

fn asserting_health_factor(
    max_size_long: &str,
    max_size_short: &str,
    perp_position_sizes: Vec<Int128>,
    perp_params: Option<PerpParams>,
    market_long_oi: &str,
    market_short_oi: &str,
) {
    // inputs
    let base_denom = "uusdc".to_string();
    let eth_perp_denom = "eth/usd/perp".to_string();

    // prices
    let current_eth_perp_price = Decimal::from_str("2000").unwrap();
    let entry_exec_price = Decimal::from_str("1999").unwrap();
    let current_exec_price = Decimal::from_str("1199.5").unwrap();
    let base_denom_price = Decimal::one();

    // market state
    let long_oi = Int128::from_str(market_long_oi).unwrap();
    let short_oi = Int128::from_str(market_short_oi).unwrap();
    let skew = long_oi.checked_sub(short_oi).unwrap();

    // perp state
    let mut funding = create_default_funding();
    let entry_accrued_funding_per_unit_in_base_denom = SignedDecimal::from_str("2000").unwrap();
    funding.last_funding_accrued_per_unit_in_base_denom = SignedDecimal::from_str("2001").unwrap();
    let eth_perp_params = PerpParams {
        opening_fee_rate: Decimal::from_str("0.2").unwrap(),
        closing_fee_rate: Decimal::from_str("0.003").unwrap(),
        max_long_oi_value: Uint128::new(6000000000000),
        max_short_oi_value: Uint128::new(6000000000000),
        max_net_oi_value: Uint128::new(40000000000000),
        ..produce_eth_perp_params()
    };

    let perps_data = PerpsData {
        params: HashMap::from(
            perp_params
                .map(|p| {
                    [(
                        eth_perp_denom.clone(),
                        PerpParams {
                            opening_fee_rate: Decimal::from_str("0.2").unwrap(),
                            closing_fee_rate: Decimal::from_str("0.003").unwrap(),
                            ..p
                        },
                    )]
                })
                .unwrap_or([(eth_perp_denom.clone(), eth_perp_params.clone())]),
        ),
    };

    let mut oracle_prices = produce_default_prices();
    oracle_prices.insert(eth_perp_denom.clone(), current_eth_perp_price);

    let asset_params = produce_default_asset_params();

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            account_kind: AccountKind::Default,
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
            staked_astro_lps: vec![],
            perps: perp_position_sizes
                .into_iter()
                .map(|size| {
                    let position = Position {
                        size,
                        entry_price: entry_exec_price,
                        entry_exec_price,
                        entry_accrued_funding_per_unit_in_base_denom,
                        initial_skew: Int128::zero(),
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

                    PerpPosition {
                        base_denom: base_denom.clone(),
                        entry_exec_price,
                        current_exec_price,
                        denom: eth_perp_params.denom.clone(),
                        current_price: current_eth_perp_price,
                        size,
                        entry_price: Decimal::from_str("2000").unwrap(),
                        realized_pnl: Default::default(),
                        unrealized_pnl: pnl_amounts,
                    }
                })
                .collect(),
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
            long_oi.unsigned_abs(),
            short_oi.unsigned_abs(),
            &mars_rover_health_computer::Direction::Long,
        )
        .unwrap();

    assert_eq!(result, Int128::from_str(max_size_long).unwrap());

    let result = h
        .max_perp_size_estimate(
            &eth_perp_denom.clone(),
            &base_denom.clone(),
            long_oi.unsigned_abs(),
            short_oi.unsigned_abs(),
            &mars_rover_health_computer::Direction::Short,
        )
        .unwrap();

    assert_eq!(result, Int128::from_str(max_size_short).unwrap());
}

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
