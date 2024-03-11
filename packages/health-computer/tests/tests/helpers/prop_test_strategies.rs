use std::{collections::HashMap, ops::RangeInclusive};

use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use mars_rover_health_computer::{HealthComputer, PerpsData, VaultsData};
use mars_types::{
    adapters::vault::{
        CoinValue, LockingVaultAmount, UnlockingPositions, Vault, VaultAmount, VaultPosition,
        VaultPositionAmount, VaultPositionValue,
    },
    credit_manager::{DebtAmount, Positions},
    health::AccountKind,
    math::SignedDecimal,
    params::{
        AssetParams, CmSettings, HlsParams, LiquidationBonus, PerpParams, RedBankSettings,
        VaultConfig,
    },
    perps::PerpDenomState,
};
use proptest::{
    collection::vec,
    prelude::{Just, Strategy},
    prop_oneof,
};

use super::uusdc_info;

fn random_account_kind() -> impl Strategy<Value = AccountKind> {
    prop_oneof![Just(AccountKind::Default), Just(AccountKind::HighLeveredStrategy)]
}

fn random_denom() -> impl Strategy<Value = String> {
    (5..=20)
        .prop_flat_map(|len| proptest::string::string_regex(&format!("[a-z]{{{},}}", len)).unwrap())
}

fn random_bool() -> impl Strategy<Value = bool> {
    proptest::bool::ANY
}

fn random_price() -> impl Strategy<Value = Decimal> {
    (1..=10000, 1..6)
        .prop_map(|(price, offset)| Decimal::from_atomics(price as u128, offset as u32).unwrap())
}

fn random_signed_decimal(range: RangeInclusive<u128>) -> impl Strategy<Value = SignedDecimal> {
    (range, 1..6, random_bool()).prop_map(|(price, offset, negative)| SignedDecimal {
        abs: Decimal::from_atomics(price, offset as u32).unwrap(),
        negative,
    })
}

fn random_decimal(range: RangeInclusive<i32>) -> impl Strategy<Value = Decimal> {
    (range, 1..6)
        .prop_map(|(price, offset)| Decimal::from_atomics(price as u128, offset as u32).unwrap())
}

fn random_coin_info() -> impl Strategy<Value = AssetParams> {
    (random_denom(), 30..70, 2..10, 80..90, 50..80, random_bool()).prop_map(
        |(denom, max_ltv, liq_thresh_buffer, hls_base, close_factor, whitelisted)| {
            let max_loan_to_value = Decimal::from_atomics(max_ltv as u128, 2).unwrap();
            let liquidation_threshold =
                max_loan_to_value + Decimal::from_atomics(liq_thresh_buffer as u128, 2).unwrap();
            let hls_max_ltv = Decimal::from_atomics(hls_base as u128, 2).unwrap();
            let hls_liq_threshold =
                hls_max_ltv + Decimal::from_atomics(liq_thresh_buffer as u128, 2).unwrap();
            let close_factor = Decimal::from_atomics(close_factor as u128, 2).unwrap();

            AssetParams {
                denom,
                credit_manager: CmSettings {
                    whitelisted,
                    hls: Some(HlsParams {
                        max_loan_to_value: hls_max_ltv,
                        liquidation_threshold: hls_liq_threshold,
                        correlations: vec![],
                    }),
                },
                red_bank: RedBankSettings {
                    deposit_enabled: true,
                    borrow_enabled: true,
                },
                max_loan_to_value,
                liquidation_threshold,
                liquidation_bonus: LiquidationBonus {
                    starting_lb: Default::default(),
                    slope: Default::default(),
                    min_lb: Default::default(),
                    max_lb: Default::default(),
                },
                protocol_liquidation_fee: Default::default(),
                deposit_cap: Default::default(),
                close_factor,
            }
        },
    )
}

fn random_denoms_data(
) -> impl Strategy<Value = (HashMap<String, AssetParams>, PerpsData, HashMap<String, Decimal>)> {
    // Construct prices, perp_params, asset_params
    vec(
        (
            random_coin_info(),
            random_price(),
            random_price(),
            random_perp_info(),
            random_denom_state(),
        ),
        2..=8,
    )
    .prop_map(|info| {
        let mut asset_params = HashMap::new();
        let mut prices = HashMap::new();
        let mut perp_params: HashMap<String, PerpParams> = HashMap::new();
        let mut denom_states: HashMap<String, PerpDenomState> = HashMap::new();

        // Base denom
        let usdc = uusdc_info();
        prices.insert(usdc.denom.clone(), usdc.price);
        asset_params.insert(usdc.denom.clone(), usdc.params);

        for (coin_info, coin_price, perp_price, perp_info, denom_state) in info {
            // Coins
            asset_params.insert(coin_info.denom.clone(), coin_info.clone());
            prices.insert(coin_info.denom.clone(), coin_price);

            // Perps
            perp_params.insert(perp_info.denom.clone(), perp_info.clone());
            prices.insert(perp_info.denom.clone(), perp_price);
            denom_states.insert(perp_info.denom.clone(), denom_state);
        }

        (
            asset_params,
            PerpsData {
                params: perp_params,
                denom_states,
            },
            prices,
        )
    })
}

fn random_perp_info() -> impl Strategy<Value = PerpParams> {
    (
        random_denom(),
        0..1000000000,
        0..1000000000,
        0..1000000000,
        1..100,
        1..100,
        1..1000,
        10..1000000000,
        20..90,
        1..5,
    )
        .prop_map(
            |(
                denom,
                max_net_oi_value,
                max_long_oi_value,
                max_short_oi_value,
                closing_fee_rate_denominator,
                opening_rate_fee_denominator,
                min_position_size_in_base_denom,
                max_position_size_in_base_denom,
                max_ltv_base,
                liq_thresh_buffer,
            )| {
                let max_net_oi_value = Uint128::new(max_net_oi_value as u128);
                let max_long_oi_value = Uint128::new(max_long_oi_value as u128);
                let max_short_oi_value = Uint128::new(max_short_oi_value as u128);
                let opening_fee_rate =
                    Decimal::from_atomics(opening_rate_fee_denominator as u128, 3).unwrap();
                let closing_fee_rate =
                    Decimal::from_atomics(closing_fee_rate_denominator as u128, 3).unwrap();
                let min_position_in_base_denom =
                    Uint128::new(min_position_size_in_base_denom as u128);
                let max_position_in_base_denom =
                    Uint128::new(max_position_size_in_base_denom as u128);
                let max_loan_to_value = Decimal::from_atomics(max_ltv_base as u128, 2).unwrap();
                let liquidation_threshold = max_loan_to_value
                    + Decimal::from_atomics(liq_thresh_buffer as u128, 2).unwrap();

                PerpParams {
                    denom,
                    max_net_oi_value,
                    max_long_oi_value,
                    max_short_oi_value,
                    closing_fee_rate,
                    opening_fee_rate,
                    min_position_value: min_position_in_base_denom,
                    max_position_value: Some(max_position_in_base_denom),
                    max_loan_to_value,
                    liquidation_threshold,
                }
            },
        )
}

fn random_denom_state() -> impl Strategy<Value = PerpDenomState> {
    (
        random_bool(),
        random_decimal(0..=1000000),
        random_decimal(0..=1000000),
        random_signed_decimal(0..=100000),
        random_signed_decimal(0..=100000),
    )
        .prop_map(|(enabled, long_oi, short_oi, total_entry_cost, total_entry_funding)| {
            PerpDenomState {
                enabled,
                long_oi,
                short_oi,
                total_entry_cost,
                total_entry_funding,
                ..Default::default()
            }
        })
}

fn random_address() -> impl Strategy<Value = String> {
    proptest::string::string_regex("cosmos1[a-zA-Z0-9]{38}").unwrap()
}

fn random_vault_denom() -> impl Strategy<Value = String> {
    (random_denom()).prop_map(|denom| format!("vault_{denom}"))
}

fn random_vault(
    asset_params: HashMap<String, AssetParams>,
) -> impl Strategy<Value = (String, VaultPositionValue, VaultConfig)> {
    (
        random_address(),
        random_vault_denom(),
        20..10_000,
        0..1000,
        30..70,
        2..10,
        80..90,
        random_bool(),
    )
        .prop_map(
            move |(
                addr,
                vault_denom,
                vault_val,
                base_val,
                max_ltv,
                liq_thresh_buffer,
                hls_base,
                whitelisted,
            )| {
                let denoms =
                    asset_params.values().map(|params| params.denom.clone()).collect::<Vec<_>>();
                let base_denom = denoms.first().unwrap();
                let position_val = VaultPositionValue {
                    vault_coin: CoinValue {
                        denom: vault_denom,
                        amount: Default::default(),
                        value: Uint128::new(vault_val as u128),
                    },
                    // The base coin denom should only be from a denom generated from random_denoms_data()
                    base_coin: CoinValue {
                        denom: base_denom.clone(),
                        amount: Default::default(),
                        value: Uint128::new(base_val as u128),
                    },
                };
                let max_loan_to_value = Decimal::from_atomics(max_ltv as u128, 2).unwrap();
                let liquidation_threshold = max_loan_to_value
                    + Decimal::from_atomics(liq_thresh_buffer as u128, 2).unwrap();
                let hls_max_ltv = Decimal::from_atomics(hls_base as u128, 2).unwrap();
                let hls_liq_threshold =
                    hls_max_ltv + Decimal::from_atomics(liq_thresh_buffer as u128, 2).unwrap();

                let config = VaultConfig {
                    addr: Addr::unchecked(addr.clone()),
                    deposit_cap: Default::default(),
                    max_loan_to_value,
                    liquidation_threshold,
                    whitelisted,
                    hls: Some(HlsParams {
                        max_loan_to_value: hls_max_ltv,
                        liquidation_threshold: hls_liq_threshold,
                        correlations: vec![],
                    }),
                };
                (addr, position_val, config)
            },
        )
}

fn random_param_maps(
) -> impl Strategy<Value = (HashMap<String, AssetParams>, HashMap<String, Decimal>, VaultsData, PerpsData)>
{
    random_denoms_data().prop_flat_map(|(asset_params, perps_data, prices)| {
        vec(random_vault(asset_params.clone()), 0..=3).prop_map(move |result| {
            let mut vault_values = HashMap::new();
            let mut vault_configs: HashMap<Addr, VaultConfig> = HashMap::new();

            for (addr, position_val, config) in result {
                let addr = Addr::unchecked(addr.clone());
                vault_values.insert(addr.clone(), position_val);
                vault_configs.insert(addr, config);
            }

            (
                asset_params.clone(),
                prices.clone(),
                VaultsData {
                    vault_values,
                    vault_configs,
                },
                perps_data.clone(),
            )
        })
    })
}

fn random_coins(asset_params: HashMap<String, AssetParams>) -> impl Strategy<Value = Vec<Coin>> {
    let denoms = asset_params.keys().cloned().collect::<Vec<String>>();
    let denoms_len = denoms.len();
    vec(
        (0..denoms_len, 1..=10000).prop_map(move |(index, amount)| {
            let denom = denoms.get(index).unwrap().clone();
            let amount = Uint128::new(amount as u128);

            Coin {
                denom,
                amount,
            }
        }),
        0..denoms_len,
    )
}

fn random_debts(
    asset_params: HashMap<String, AssetParams>,
) -> impl Strategy<Value = Vec<DebtAmount>> {
    let denoms = asset_params.keys().cloned().collect::<Vec<String>>();
    let denoms_len = denoms.len();
    vec(
        (0..denoms_len, 1..=10000).prop_map(move |(index, amount)| {
            let denom = denoms.get(index).unwrap().clone();
            let amount = Uint128::new(amount as u128);

            DebtAmount {
                denom,
                shares: amount * Uint128::new(10),
                amount,
            }
        }),
        0..denoms_len,
    )
}

fn random_vault_pos_amount() -> impl Strategy<Value = VaultPositionAmount> {
    prop_oneof![
        random_vault_amount().prop_map(VaultPositionAmount::Unlocked),
        random_locking_vault_amount().prop_map(VaultPositionAmount::Locking),
    ]
}

fn random_vault_amount() -> impl Strategy<Value = VaultAmount> {
    (10..=100000).prop_map(|amount| VaultAmount::new(Uint128::new(amount as u128)))
}

fn random_locking_vault_amount() -> impl Strategy<Value = LockingVaultAmount> {
    (random_vault_amount()).prop_map(|locked| LockingVaultAmount {
        locked,
        unlocking: UnlockingPositions::new(vec![]),
    })
}

fn random_vault_positions(vd: VaultsData) -> impl Strategy<Value = Vec<VaultPosition>> {
    let vault_addrs = vd.vault_configs.keys().cloned().collect::<Vec<Addr>>();
    let addrs_len = vault_addrs.len();

    vec(
        (0..addrs_len, random_vault_pos_amount()).prop_map(move |(index, amount)| {
            let addr = vault_addrs.get(index).unwrap().clone();

            VaultPosition {
                vault: Vault::new(addr),
                amount,
            }
        }),
        addrs_len,
    )
}

pub fn random_health_computer() -> impl Strategy<Value = HealthComputer> {
    (random_param_maps()).prop_flat_map(|(asset_params, oracle_prices, vaults_data, perps_data)| {
        (
            // Get prices
            random_account_kind(),
            random_coins(asset_params.clone()),
            random_debts(asset_params.clone()),
            random_coins(asset_params.clone()),
            random_vault_positions(vaults_data.clone()),
        )
            .prop_map(move |(kind, deposits, debts, lends, vaults)| HealthComputer {
                kind,
                positions: Positions {
                    account_id: "123".to_string(),
                    deposits,
                    debts,
                    lends,
                    vaults,
                    perps: vec![],
                },
                vaults_data: vaults_data.clone(),
                oracle_prices: oracle_prices.clone(),
                asset_params: asset_params.clone(),
                perps_data: perps_data.clone(),
            })
    })
}
