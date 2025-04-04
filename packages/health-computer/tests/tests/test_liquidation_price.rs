use std::{collections::HashMap, str::FromStr};

use cosmwasm_std::{coin, Decimal, Int128, Uint128};
use mars_rover_health_computer::{HealthComputer, PerpsData, VaultsData};
use mars_types::{
    credit_manager::{DebtAmount, Positions},
    health::{AccountKind, LiquidationPriceKind},
    perps::{PerpPosition, PnlAmounts},
};

use crate::tests::helpers::{create_coin_info, create_perp_info, udai_info};

#[test]
fn liquidation_price_when_debt_larger_than_collateral() {
    let udai = udai_info();

    let uusdc = create_coin_info(
        "uusdc".to_string(),
        Decimal::from_atomics(1u32, 0).unwrap(),
        Decimal::percent(84),
        Decimal::percent(85),
    );

    let perps_data = PerpsData {
        params: HashMap::new(),
    };

    let oracle_prices =
        HashMap::from([(udai.denom.clone(), udai.price), (uusdc.denom.clone(), uusdc.price)]);
    let asset_params = HashMap::from([
        (udai.denom.clone(), udai.params.clone()),
        (uusdc.denom.clone(), uusdc.params.clone()),
    ]);

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: Default::default(),
    };

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            account_kind: AccountKind::Default,
            deposits: vec![coin(1200, &udai.denom)],
            debts: vec![DebtAmount {
                amount: Uint128::from(2000u32),
                denom: uusdc.denom.clone(),
                shares: Uint128::zero(),
            }],
            lends: vec![],
            vaults: vec![],
            staked_astro_lps: vec![],
            perps: vec![],
        },
        asset_params,
        oracle_prices,
        vaults_data,
        perps_data,
    };

    let liq_price = h.liquidation_price(&udai.denom, &LiquidationPriceKind::Asset).unwrap();
    assert_eq!(udai.price, liq_price);
}

#[test]
fn liquidation_price_no_debt() {
    let udai = udai_info();

    let oracle_prices = HashMap::from([(udai.denom.clone(), udai.price)]);
    let asset_params = HashMap::from([(udai.denom.clone(), udai.params.clone())]);

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
        },
        asset_params,
        oracle_prices,
        vaults_data,
        perps_data,
    };

    let liq_price = h.liquidation_price(&udai.denom, &LiquidationPriceKind::Asset).unwrap();
    assert_eq!(Decimal::zero(), liq_price);
}

#[test]
fn liquidation_price_debt_lt_collateral() {
    let udai = udai_info();

    let oracle_prices = HashMap::from([(udai.denom.clone(), udai.price)]);
    let asset_params = HashMap::from([(udai.denom.clone(), udai.params.clone())]);

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
            debts: vec![DebtAmount {
                denom: udai.denom.clone(),
                amount: Uint128::from(1200u32),
                shares: Uint128::zero(),
            }],
            lends: vec![],
            vaults: vec![],
            staked_astro_lps: vec![],
            perps: vec![],
        },
        asset_params,
        oracle_prices,
        vaults_data,
        perps_data,
    };

    let liq_price = h.liquidation_price(&udai.denom, &LiquidationPriceKind::Asset).unwrap();

    assert_eq!(udai.price, liq_price)
}

#[test]
fn liquidation_price_asset() {
    let uusd = create_coin_info(
        "uusd".to_string(),
        Decimal::from_atomics(1u32, 0).unwrap(),
        Decimal::percent(84),
        Decimal::percent(85),
    );
    let uusdc = create_coin_info(
        "uusdc".to_string(),
        Decimal::from_atomics(1u32, 0).unwrap(),
        Decimal::percent(84),
        Decimal::percent(85),
    );
    let utia = create_coin_info(
        "utia".to_string(),
        Decimal::from_atomics(16u32, 12).unwrap(),
        Decimal::percent(34),
        Decimal::percent(35),
    );
    let uatom = create_coin_info(
        "uatom".to_string(),
        Decimal::from_atomics(11u32, 0).unwrap(),
        Decimal::percent(84),
        Decimal::percent(85),
    );
    let udydx = create_coin_info(
        "udydx".to_string(),
        Decimal::from_atomics(3u32, 12).unwrap(),
        Decimal::percent(79),
        Decimal::percent(80),
    );
    let uosmo = create_coin_info(
        "uosmo".to_string(),
        Decimal::from_atomics(135u32, 2).unwrap(),
        Decimal::percent(79),
        Decimal::percent(80),
    );

    let oracle_prices = HashMap::from([
        (uusd.denom.clone(), uusd.price),
        (uusdc.denom.clone(), uusdc.price),
        (utia.denom.clone(), utia.price),
        (uatom.denom.clone(), uatom.price),
        (udydx.denom.clone(), udydx.price),
        (uosmo.denom.clone(), uosmo.price),
    ]);

    let asset_params = HashMap::from([
        (uusd.denom.clone(), uusd.params.clone()),
        (uusdc.denom.clone(), uusdc.params.clone()),
        (utia.denom.clone(), utia.params.clone()),
        (uatom.denom.clone(), uatom.params.clone()),
        (udydx.denom.clone(), udydx.params.clone()),
        (uosmo.denom.clone(), uosmo.params.clone()),
    ]);

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: Default::default(),
    };

    let uatom_perp = create_perp_info(
        uatom.denom.clone(),
        Decimal::from_atomics(12u32, 6).unwrap(),
        Decimal::percent(84),
        Decimal::percent(85),
    );
    let udydx_perp = create_perp_info(
        udydx.denom.clone(),
        Decimal::from_atomics(33u32, 19).unwrap(),
        Decimal::percent(79),
        Decimal::percent(80),
    );

    let perps_data = PerpsData {
        params: HashMap::from([
            (uatom.denom.clone(), uatom_perp.perp_params),
            (udydx.denom.clone(), udydx_perp.perp_params),
        ]),
    };

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            account_kind: AccountKind::Default,
            deposits: vec![
                coin(1_000_000_000, &uusdc.denom),
                coin(150_000_000_000_000_000_000, &utia.denom),
            ],
            debts: vec![DebtAmount {
                denom: uosmo.denom.clone(),
                amount: Uint128::from(285_000_000u32),
                shares: Uint128::zero(),
            }],
            lends: vec![],
            vaults: vec![],
            staked_astro_lps: vec![],
            perps: vec![
                PerpPosition {
                    denom: "uatom".to_string(),
                    size: Int128::from_str("300000000").unwrap(),
                    current_exec_price: Decimal::from_atomics(12u32, 0).unwrap(),
                    entry_exec_price: Decimal::from_atomics(10u32, 0).unwrap(),
                    current_price: Decimal::MAX,
                    entry_price: Decimal::MAX,
                    base_denom: "uusdc".to_string(),
                    unrealized_pnl: PnlAmounts {
                        accrued_funding: Int128::from_str("-725000000").unwrap(),
                        closing_fee: Int128::from_str("-2700000").unwrap(),
                        opening_fee: Int128::zero(),
                        price_pnl: Int128::from_str("600000000").unwrap(),
                        pnl: Int128::from_str("-127700000").unwrap(),
                    },
                    realized_pnl: PnlAmounts::default(),
                },
                PerpPosition {
                    denom: "udydx".to_string(),
                    size: Int128::from_str("-500000000000000000000").unwrap(),
                    current_exec_price: Decimal::from_atomics(33u32, 13).unwrap(),
                    entry_exec_price: Decimal::from_atomics(27u32, 13).unwrap(),
                    current_price: Decimal::MAX,
                    entry_price: Decimal::MAX,
                    base_denom: "uusdc".to_string(),
                    unrealized_pnl: PnlAmounts {
                        accrued_funding: Int128::from_str("425000000").unwrap(),
                        closing_fee: Int128::from_str("-1237500").unwrap(),
                        opening_fee: Int128::zero(),
                        price_pnl: Int128::from_str("-300000000").unwrap(),
                        pnl: Int128::from_str("123760000").unwrap(),
                    },
                    realized_pnl: PnlAmounts::default(),
                },
            ],
        },
        asset_params,
        oracle_prices,
        vaults_data,
        perps_data,
    };

    // Check the liquidation prices for every asset and type
    // The expected values are calculated in the `liquidation_price.xls` file

    // USDC
    let liq_price = h.liquidation_price(&uusdc.denom, &LiquidationPriceKind::Asset).unwrap();
    assert_eq!(Decimal::from_str("0.589176470588235294").unwrap(), liq_price);

    // TIA
    let liq_price = h.liquidation_price(&utia.denom, &LiquidationPriceKind::Asset).unwrap();
    assert_eq!(Decimal::from_str("0.000000000009348571").unwrap(), liq_price);

    // OSMO
    let liq_price = h.liquidation_price(&uosmo.denom, &LiquidationPriceKind::Debt).unwrap();
    assert_eq!(Decimal::from_str("2.575263157894736842").unwrap(), liq_price);

    // ATOM
    let liq_price = h.liquidation_price(&uatom.denom, &LiquidationPriceKind::Perp).unwrap();
    assert_eq!(Decimal::from_str("10.630265944928218404").unwrap(), liq_price);

    // DYDX
    let liq_price = h.liquidation_price(&udydx.denom, &LiquidationPriceKind::Perp).unwrap();
    assert_eq!(Decimal::from_str("0.000000000003881903").unwrap(), liq_price);
}

#[test]
fn liquidation_price_for_staked_astro_lp() {
    let staked_lp_coin_info = create_coin_info(
        "factory/contractaddress/astroport/share".to_string(),
        Decimal::from_atomics(1u32, 0).unwrap(),
        Decimal::percent(84),
        Decimal::percent(85),
    );

    let dai_info = udai_info();

    let oracle_prices = HashMap::from([
        (staked_lp_coin_info.denom.clone(), staked_lp_coin_info.price),
        (dai_info.denom.clone(), dai_info.price),
    ]);
    let asset_params = HashMap::from([
        (staked_lp_coin_info.denom.clone(), staked_lp_coin_info.params.clone()),
        (dai_info.denom.clone(), dai_info.params.clone()),
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
            deposits: vec![],
            debts: vec![DebtAmount {
                denom: dai_info.denom.clone(),
                amount: Uint128::from(1200000000u32),
                shares: Uint128::zero(),
            }],
            lends: vec![],
            vaults: vec![],
            staked_astro_lps: vec![coin(1800000000, &staked_lp_coin_info.denom)],
            perps: vec![],
        },
        asset_params,
        oracle_prices,
        vaults_data,
        perps_data,
    };

    let liq_price =
        h.liquidation_price(&staked_lp_coin_info.denom, &LiquidationPriceKind::Asset).unwrap();

    assert_eq!(Decimal::from_str("0.24584392156862745").unwrap(), liq_price)
}
