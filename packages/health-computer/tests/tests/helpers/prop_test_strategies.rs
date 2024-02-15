use std::{collections::HashMap, str::FromStr};

use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use mars_perps::{
    position::{PositionExt, PositionModification},
    pricing::opening_execution_price,
};
use mars_rover_health_computer::{DenomsData, HealthComputer, VaultsData};
use mars_types::{
    adapters::vault::{
        CoinValue, LockingVaultAmount, UnlockingPositions, Vault, VaultAmount, VaultPosition,
        VaultPositionAmount, VaultPositionValue,
    },
    credit_manager::{DebtAmount, Positions},
    health::AccountKind,
    math::SignedDecimal,
    params::{AssetParams, CmSettings, HlsParams, LiquidationBonus, RedBankSettings, VaultConfig},
    perps::{Funding, PerpPosition, PnlAmounts, Position, PositionPnl},
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

fn random_coin_info() -> impl Strategy<Value = AssetParams> {
    (random_denom(), 30..70, 2..10, 80..90, random_bool()).prop_map(
        |(denom, max_ltv, liq_thresh_buffer, hls_base, whitelisted)| {
            let max_loan_to_value = Decimal::from_atomics(max_ltv as u128, 2).unwrap();
            let liquidation_threshold =
                max_loan_to_value + Decimal::from_atomics(liq_thresh_buffer as u128, 2).unwrap();
            let hls_max_ltv = Decimal::from_atomics(hls_base as u128, 2).unwrap();
            let hls_liq_threshold =
                hls_max_ltv + Decimal::from_atomics(liq_thresh_buffer as u128, 2).unwrap();

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
            }
        },
    )
}

fn random_denoms_data() -> impl Strategy<Value = DenomsData> {
    vec((random_coin_info(), random_price()), 2..=5).prop_map(|info| {
        let mut prices = HashMap::new();
        let mut params = HashMap::new();
        let usdc = uusdc_info();

        prices.insert(usdc.denom.clone(), usdc.price);
        params.insert(usdc.denom.clone(), usdc.params);

        for (coin_info, price) in info {
            prices.insert(coin_info.denom.clone(), price);
            params.insert(coin_info.denom.clone(), coin_info);
        }

        DenomsData {
            prices,
            params,
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
    denoms_data: DenomsData,
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
                let denoms = denoms_data
                    .params
                    .values()
                    .map(|params| params.denom.clone())
                    .collect::<Vec<_>>();
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

fn random_param_maps() -> impl Strategy<Value = (DenomsData, VaultsData)> {
    random_denoms_data().prop_flat_map(|denoms_data| {
        vec(random_vault(denoms_data.clone()), 0..=3).prop_map(move |vaults| {
            let mut vault_values = HashMap::new();
            let mut vault_configs: HashMap<Addr, VaultConfig> = HashMap::new();

            for (addr, position_val, config) in vaults {
                let addr = Addr::unchecked(addr.clone());
                vault_values.insert(addr.clone(), position_val);
                vault_configs.insert(addr, config);
            }

            (
                denoms_data.clone(),
                VaultsData {
                    vault_values,
                    vault_configs,
                },
            )
        })
    })
}

fn random_coins(denoms_data: DenomsData) -> impl Strategy<Value = Vec<Coin>> {
    let denoms = denoms_data.params.keys().cloned().collect::<Vec<String>>();
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

fn random_perps(perp_denoms_data: DenomsData) -> impl Strategy<Value = Vec<PerpPosition>> {
    let perp_denoms = perp_denoms_data.params.keys().cloned().collect::<Vec<String>>();
    let perp_denoms_len = perp_denoms.len();
    let usdc = uusdc_info();
    vec(
        (
            0..perp_denoms_len,
            1..=10000,
            1..=10000,
            1..=10000,
            1..=10000,
            1..=10000,
            1..=10000,
            80..=120,
            -1000000..=1000000,
            -1000000..=1000000,
        )
            .prop_map(
                move |(
                    index,
                    size,
                    entry_price,
                    current_price,
                    skew_scale,
                    rate,
                    funding_index,
                    usdc_price,
                    inital_skew,
                    current_skew,
                )| {
                    let perp_denom = perp_denoms.get(index).unwrap().clone();
                    let base_denom = usdc.denom.clone();
                    let amount = Uint128::new(size as u128);
                    let current_price =
                        Decimal::from_atomics(Uint128::new(current_price as u128), 2).unwrap();
                    let entry_price =
                        Decimal::from_atomics(Uint128::new(entry_price as u128), 2).unwrap();
                    let usdc_price =
                        Decimal::from_atomics(Uint128::new(usdc_price as u128), 2).unwrap();
                    let size = SignedDecimal::from(amount)
                        .checked_sub(
                            // Size can be negative. Subtracing 5000 means we range from -5000 : 5000
                            SignedDecimal::from(Uint128::new(5000)),
                        )
                        .unwrap();
                    let initial_skew = SignedDecimal {
                        negative: inital_skew < 0,
                        abs: Decimal::from_atomics(i32::abs(inital_skew) as u128, 0).unwrap(),
                    };
                    let current_skew = SignedDecimal {
                        negative: current_skew < 0,
                        abs: Decimal::from_atomics(i32::abs(current_skew) as u128, 0).unwrap(),
                    };
                    // We randomize the skew scale, the rate and the index
                    let skew_scale = Decimal::from_atomics(Uint128::new(skew_scale as u128), 0)
                        .unwrap()
                        .checked_mul(Decimal::from_str("1000000").unwrap())
                        .unwrap();
                    let position = Position {
                        size,
                        entry_price,
                        entry_exec_price: opening_execution_price(
                            initial_skew,
                            skew_scale,
                            size,
                            entry_price,
                        )
                        .unwrap()
                        .abs,
                        entry_accrued_funding_per_unit_in_base_denom: SignedDecimal::zero(),
                        initial_skew,
                        realized_pnl: PnlAmounts::default(),
                    };

                    // This gives us a max of 10
                    let rate = Decimal::from_atomics(Uint128::new(rate as u128), 3).unwrap();

                    // Rate is between 0 and 10, so our closing fee will be between 0 and 1%
                    let closing_fee_rate =
                        rate.checked_div(Decimal::from_str("1000").unwrap()).unwrap();

                    let funding_index_dec =
                        Decimal::from_atomics(Uint128::new(funding_index as u128), 6)
                            .unwrap()
                            .checked_add(Decimal::one())
                            .unwrap();

                    let funding = Funding {
                        max_funding_velocity: Decimal::from_str("3").unwrap(),
                        skew_scale,
                        last_funding_rate: rate.into(),
                        last_funding_accrued_per_unit_in_base_denom: funding_index_dec.into(),
                    };

                    let (pnl_values, pnl_amounts) = position
                        .compute_pnl(
                            &funding,
                            current_skew,
                            current_price,
                            usdc_price,
                            Decimal::zero(), // TODO: provide a real value
                            closing_fee_rate,
                            PositionModification::None,
                        )
                        .unwrap();

                    let pnl_coins = pnl_amounts.to_coins(&base_denom);
                    PerpPosition {
                        denom: perp_denom,
                        base_denom,
                        size,
                        current_price,
                        entry_price,
                        entry_exec_price: entry_price,
                        current_exec_price: current_price,
                        unrealised_pnl: PositionPnl {
                            values: pnl_values,
                            amounts: pnl_amounts,
                            coins: pnl_coins,
                        },
                        realised_pnl: PnlAmounts::default(),
                        closing_fee_rate,
                    }
                },
            ),
        0..perp_denoms_len,
    )
}

fn random_debts(denoms_data: DenomsData) -> impl Strategy<Value = Vec<DebtAmount>> {
    let denoms = denoms_data.params.keys().cloned().collect::<Vec<String>>();
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
    (random_param_maps()).prop_flat_map(|(denoms_data, vaults_data)| {
        (
            random_account_kind(),
            random_coins(denoms_data.clone()),
            random_debts(denoms_data.clone()),
            random_coins(denoms_data.clone()),
            random_vault_positions(vaults_data.clone()),
            random_perps(denoms_data.clone()),
        )
            .prop_map(move |(kind, deposits, debts, lends, vaults, perps)| {
                HealthComputer {
                    kind,
                    positions: Positions {
                        account_id: "123".to_string(),
                        deposits,
                        debts,
                        lends,
                        vaults,
                        perps,
                    },
                    denoms_data: denoms_data.clone(),
                    vaults_data: vaults_data.clone(),
                }
            })
    })
}
