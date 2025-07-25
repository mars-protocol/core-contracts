use std::{collections::HashMap, ops::RangeInclusive, str::FromStr};

use cosmwasm_std::{Addr, Coin, Decimal, Int128, SignedDecimal, Uint128};
use mars_perps::position::{PositionExt, PositionModification};
use mars_perps_common::pricing::opening_execution_price;
use mars_rover_health_computer::{HealthComputer, PerpsData, VaultsData};
use mars_types::{
    adapters::vault::{
        CoinValue, LockingVaultAmount, UnlockingPositions, Vault, VaultAmount, VaultPosition,
        VaultPositionAmount, VaultPositionValue,
    },
    credit_manager::{DebtAmount, Positions},
    health::AccountKind,
    params::{
        AssetParams, CmSettings, HlsAssetType, HlsParams, LiquidationBonus, PerpParams,
        RedBankSettings, VaultConfig,
    },
    perps::{Funding, PerpPosition, PnlAmounts, Position},
    red_bank::InterestRateModel,
};
use proptest::{
    collection::vec,
    prelude::{Just, Strategy},
    prop_oneof,
};

use super::uusdc_info;

fn random_account_kind() -> impl Strategy<Value = AccountKind> {
    prop_oneof![
        Just(AccountKind::Default),
        Just(AccountKind::HighLeveredStrategy),
        Just(AccountKind::FundManager {
            vault_addr: "vault_addr".to_string()
        })
    ]
}

fn random_denom() -> impl Strategy<Value = String> {
    (5..=20)
        .prop_flat_map(|len| proptest::string::string_regex(&format!("[a-z]{{{},}}", len)).unwrap())
}

fn random_bool() -> impl Strategy<Value = bool> {
    proptest::bool::ANY
}

fn random_price() -> impl Strategy<Value = Decimal> {
    (1..=10000000, 1..8)
        .prop_map(|(price, offset)| Decimal::from_atomics(price as u128, offset as u32).unwrap())
}

fn random_perp_size() -> impl Strategy<Value = Int128> {
    (1000..=10000000000000000i128, random_bool()).prop_map(|(size, negative)| {
        if negative {
            -Int128::new(size)
        } else {
            Int128::new(size)
        }
    })
}

fn random_decimal(
    range: RangeInclusive<i32>,
    decimal_range: RangeInclusive<i32>,
) -> impl Strategy<Value = Decimal> {
    if decimal_range.end().gt(&18i32) {
        panic!("Decimal range must be between 0 and 18")
    }
    (range, decimal_range)
        .prop_map(|(price, offset)| Decimal::from_atomics(price as u128, offset as u32).unwrap())
}

fn random_uint128(range: RangeInclusive<i128>) -> impl Strategy<Value = Uint128> {
    range.prop_map(|val| Uint128::new(val as u128))
}

fn random_signed_uint(range: RangeInclusive<i128>) -> impl Strategy<Value = Int128> {
    (range, random_bool()).prop_map(|(num, negative)| {
        if negative {
            -Int128::new(num)
        } else {
            Int128::new(num)
        }
    })
}

fn random_signed_decimal(
    range: RangeInclusive<i32>,
    decimal_range: RangeInclusive<i32>,
) -> impl Strategy<Value = SignedDecimal> {
    (random_decimal(range, decimal_range), random_bool()).prop_map(|(price, negative)| {
        let s = if negative {
            format!("-{}", price)
        } else {
            format!("{}", price)
        };
        SignedDecimal::from_str(&s).unwrap()
    })
}

fn random_coin_info() -> impl Strategy<Value = AssetParams> {
    (random_denom(), 30..70, 2..10, 80..90, 50..80, random_bool(), 10..25, 50..90, 2..15, 45..300)
        .prop_map(
            |(
                denom,
                max_ltv,
                liq_thresh_buffer,
                hls_base,
                close_factor,
                whitelisted,
                reserve_factor,
                optimal_utilization_rate,
                slope_1,
                slope_2,
            )| {
                let max_loan_to_value = Decimal::from_atomics(max_ltv as u128, 2).unwrap();
                let liquidation_threshold = max_loan_to_value
                    + Decimal::from_atomics(liq_thresh_buffer as u128, 2).unwrap();
                let hls_max_ltv = Decimal::from_atomics(hls_base as u128, 2).unwrap();
                let hls_liq_threshold =
                    hls_max_ltv + Decimal::from_atomics(liq_thresh_buffer as u128, 2).unwrap();
                let close_factor = Decimal::from_atomics(close_factor as u128, 2).unwrap();
                let reserve_factor = Decimal::from_atomics(reserve_factor as u128, 2).unwrap();
                let optimal_utilization_rate =
                    Decimal::from_atomics(optimal_utilization_rate as u128, 2).unwrap();
                let base = Decimal::zero();
                let slope_1 = Decimal::from_atomics(slope_1 as u128, 2).unwrap();
                let slope_2 = Decimal::from_atomics(slope_2 as u128, 2).unwrap();

                AssetParams {
                    denom: denom.clone(),
                    credit_manager: CmSettings {
                        whitelisted,
                        withdraw_enabled: true,
                        hls: Some(HlsParams {
                            max_loan_to_value: hls_max_ltv,
                            liquidation_threshold: hls_liq_threshold,
                            correlations: vec![],
                        }),
                    },
                    red_bank: RedBankSettings {
                        withdraw_enabled: true,
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
                    reserve_factor,
                    interest_rate_model: InterestRateModel {
                        optimal_utilization_rate,
                        base,
                        slope_1,
                        slope_2,
                    },
                }
            },
        )
}

fn random_denoms_data(
) -> impl Strategy<Value = (HashMap<String, AssetParams>, PerpsData, HashMap<String, Decimal>)> {
    // Construct prices, perp_params, asset_params
    vec((random_coin_info(), random_price(), random_price(), random_perp_info()), 2..=8).prop_map(
        |info| {
            let mut asset_params = HashMap::new();
            let mut prices = HashMap::new();
            let mut perp_params: HashMap<String, PerpParams> = HashMap::new();

            // Base denom
            let usdc = uusdc_info();
            prices.insert(usdc.denom.clone(), usdc.price);
            asset_params.insert(usdc.denom.clone(), usdc.params);

            for (coin_info, coin_price, perp_price, perp_info) in info {
                // Coins
                asset_params.insert(coin_info.denom.clone(), coin_info.clone());
                prices.insert(coin_info.denom.clone(), coin_price);

                // Perps
                perp_params.insert(perp_info.denom.clone(), perp_info.clone());
                prices.insert(perp_info.denom.clone(), perp_price);
            }

            (
                asset_params,
                PerpsData {
                    params: perp_params,
                },
                prices,
            )
        },
    )
}

fn random_perp_info() -> impl Strategy<Value = PerpParams> {
    (
        random_bool(),
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
                enabled,
                denom,
                max_net_oi_value,
                max_long_oi_value,
                max_short_oi_value,
                closing_fee_rate_denominator,
                opening_rate_fee_denominator,
                min_position_value,
                max_position_value,
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
                let min_position_value = Uint128::new(min_position_value as u128);
                let max_position_value = Uint128::new(max_position_value as u128);
                let max_loan_to_value = Decimal::from_atomics(max_ltv_base as u128, 2).unwrap();
                let liquidation_threshold = max_loan_to_value
                    + Decimal::from_atomics(liq_thresh_buffer as u128, 2).unwrap();

                PerpParams {
                    denom,
                    enabled,
                    max_net_oi_value,
                    max_long_oi_value,
                    max_short_oi_value,
                    closing_fee_rate,
                    opening_fee_rate,
                    min_position_value,
                    max_position_value: Some(max_position_value),
                    max_loan_to_value,
                    liquidation_threshold,
                    ..Default::default()
                }
            },
        )
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
    // random denoms can be (asset_params, perp_params, oracle_prices)
    // (asset_params, perp_data, vaults_data, oracle_prices)
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

fn random_astro_lp_coins(
    asset_params: HashMap<String, AssetParams>,
) -> impl Strategy<Value = Vec<Coin>> {
    let denoms = asset_params.keys().cloned().collect::<Vec<String>>();
    let denoms_len = denoms.len();
    vec(
        (0..denoms_len, 1..=10000000).prop_map(move |(index, amount)| {
            let denom = denoms.get(index).unwrap().clone();
            let amount = Uint128::new(amount as u128);

            Coin {
                denom: format!("factory/{}/astroport/share", denom),
                amount,
            }
        }),
        0..denoms_len,
    )
}

fn random_perps(perp_denoms_data: PerpsData) -> impl Strategy<Value = Vec<PerpPosition>> {
    let perp_denoms = perp_denoms_data.params.keys().cloned().collect::<Vec<String>>();
    let perp_denoms_len = perp_denoms.len();
    let usdc = uusdc_info();
    vec(
        (
            0..perp_denoms_len,
            random_perp_size(),
            random_price(),
            random_price(),
            random_uint128(1000000000000..=i128::MAX / 100),
            1..=10000,
            random_decimal(1..=100, 2..=3),
            random_signed_uint(0..=100000000000),
            random_signed_uint(0..=100000000000),
            random_signed_decimal(0..=10000000, 2..=8),
            random_signed_decimal(0..=10000000, 2..=8),
            random_decimal(0..=100, 2..=3),
        )
            .prop_map(
                move |(
                    index,
                    size,
                    entry_price,
                    market_price,
                    skew_scale,
                    rate,
                    base_denom_price,
                    initial_skew,
                    current_skew,
                    entry_accrued_funding_per_unit_in_base_denom,
                    exit_funding_diff,
                    opening_fee_rate,
                )| {
                    let perp_denom = perp_denoms.get(index).unwrap().clone();
                    let base_denom = usdc.denom.clone();

                    let position: Position = Position {
                        size,
                        entry_price,
                        entry_exec_price: opening_execution_price(
                            initial_skew,
                            skew_scale,
                            size,
                            entry_price,
                        )
                        .unwrap(),
                        entry_accrued_funding_per_unit_in_base_denom,
                        initial_skew,
                        realized_pnl: PnlAmounts::default(),
                    };

                    // This gives us a max of 10
                    let rate = Decimal::from_atomics(Uint128::new(rate as u128), 3).unwrap();

                    // Rate is between 0 and 10, so our closing fee will be between 0 and 1%
                    let closing_fee_rate =
                        rate.checked_div(Decimal::from_str("1000").unwrap()).unwrap();

                    let funding = Funding {
                        max_funding_velocity: Decimal::from_str("3").unwrap(),
                        skew_scale,
                        last_funding_rate: rate.try_into().unwrap(),
                        last_funding_accrued_per_unit_in_base_denom:
                            entry_accrued_funding_per_unit_in_base_denom
                                .checked_add(exit_funding_diff)
                                .unwrap(),
                    };

                    let pnl_amounts = position
                        .compute_pnl(
                            &funding,
                            current_skew,
                            market_price,
                            base_denom_price,
                            opening_fee_rate,
                            closing_fee_rate,
                            PositionModification::Decrease(position.size),
                        )
                        .unwrap();

                    PerpPosition {
                        denom: perp_denom,
                        base_denom,
                        size,
                        current_price: market_price,
                        entry_price,
                        entry_exec_price: entry_price,
                        current_exec_price: market_price,
                        unrealized_pnl: pnl_amounts,
                        realized_pnl: PnlAmounts::default(),
                    }
                },
            ),
        0..perp_denoms_len,
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
    (random_param_maps()).prop_flat_map(
        |(mut asset_params, oracle_prices, mut vaults_data, perps_data)| {
            update_hls_correlations(&mut asset_params, &mut vaults_data);

            (
                // Get prices
                random_account_kind(),
                random_coins(asset_params.clone()),
                random_debts(asset_params.clone()),
                random_coins(asset_params.clone()),
                random_vault_positions(vaults_data.clone()),
                random_astro_lp_coins(asset_params.clone()),
                random_perps(perps_data.clone()),
            )
                .prop_map(
                    move |(kind, deposits, debts, lends, vaults, staked_astro_lps, perps)| {
                        HealthComputer {
                            kind: kind.clone(),
                            positions: Positions {
                                account_id: "123".to_string(),
                                account_kind: kind,
                                deposits,
                                debts,
                                lends,
                                vaults,
                                staked_astro_lps,
                                perps,
                            },
                            vaults_data: vaults_data.clone(),
                            oracle_prices: oracle_prices.clone(),
                            asset_params: asset_params.clone(),
                            perps_data: perps_data.clone(),
                        }
                    },
                )
        },
    )
}

fn update_hls_correlations(
    asset_params: &mut HashMap<String, AssetParams>,
    vaults_data: &mut VaultsData,
) {
    // Add correlations to the denoms and vaults. This is necessary for the HealthComputer to be able to compute the health for HLS accounts.
    let denoms = asset_params
        .keys()
        .map(|denom| HlsAssetType::Coin {
            denom: denom.clone(),
        })
        .collect::<Vec<HlsAssetType<Addr>>>();
    let vaults = vaults_data
        .vault_configs
        .keys()
        .map(|addr| HlsAssetType::Vault {
            addr: addr.clone(),
        })
        .collect::<Vec<HlsAssetType<Addr>>>();
    let correlations = denoms.into_iter().chain(vaults).collect::<Vec<HlsAssetType<Addr>>>();

    for (_, params) in asset_params.iter_mut() {
        params.credit_manager.hls.as_mut().unwrap().correlations.clone_from(&correlations);
    }

    for (_, config) in vaults_data.vault_configs.iter_mut() {
        config.hls.as_mut().unwrap().correlations.clone_from(&correlations);
    }
}
