use std::{collections::HashMap, ops::Add, str::FromStr};

use cosmwasm_std::{coin, Addr, Coin, Decimal, Uint128};
use mars_rover_health_computer::{HealthComputer, PerpsData, VaultsData};
use mars_types::{
    adapters::vault::{
        CoinValue, LockingVaultAmount, UnlockingPositions, Vault, VaultAmount, VaultPosition,
        VaultPositionAmount, VaultPositionValue, VaultUnlockingPosition,
    },
    credit_manager::{DebtAmount, Positions},
    health::AccountKind,
    math::SignedDecimal,
    params::VaultConfig,
    perps::{PerpDenomState, PerpPosition, PnlAmounts},
    signed_uint::SignedUint,
};

use super::helpers::{
    atomperp_info, btcperp_info, ethperp_info, udai_info, ujuno_info, uluna_info, umars_info,
    ustars_info, uusdc_info,
};
use crate::tests::helpers::{create_coin_info, create_perp_info};

/// Action: User deposits 300 mars (1 price)
/// Health: assets_value: 300
///         debt value 0
///         liquidatable: false
///         above_max_ltv: false
#[test]
fn only_assets_with_no_debts() {
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

    let deposit_amount = Uint128::new(300);
    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![Coin {
                denom: umars.denom.clone(),
                amount: deposit_amount,
            }],
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

    let health = h.compute_health().unwrap();
    let collateral_value = deposit_amount.checked_mul_floor(umars.price).unwrap();
    assert_eq!(health.total_collateral_value, collateral_value);
    assert_eq!(
        health.max_ltv_adjusted_collateral,
        collateral_value.checked_mul_floor(umars.params.max_loan_to_value).unwrap()
    );
    assert_eq!(
        health.liquidation_threshold_adjusted_collateral,
        collateral_value.checked_mul_floor(umars.params.liquidation_threshold).unwrap()
    );
    assert_eq!(health.total_debt_value, Uint128::zero());
    assert_eq!(health.liquidation_health_factor, None);
    assert_eq!(health.max_ltv_health_factor, None);
    assert!(!health.is_liquidatable());
    assert!(!health.is_above_max_ltv());
}

/// Step 1: User deposits 12 luna (100 price) and borrows 2 luna
/// Health: assets_value: 1400
///         debt value 200
///         liquidatable: false
///         above_max_ltv: false
/// Step 2: luna price goes to zero
/// Health: assets_value: 0
///         debt value 0 (still debt shares outstanding)
///         liquidatable: false
///         above_max_ltv: false
#[test]
fn terra_ragnarok() {
    let mut uluna = uluna_info();
    let oracle_prices = HashMap::from([(uluna.denom.clone(), uluna.price)]);
    let asset_params = HashMap::from([(uluna.denom.clone(), uluna.params.clone())]);

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: Default::default(),
    };

    let perps_data = PerpsData {
        denom_states: Default::default(),
        params: Default::default(),
    };

    let deposit_amount = Uint128::new(12);
    let borrow_amount = Uint128::new(3);

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![Coin {
                denom: uluna.denom.clone(),
                amount: deposit_amount,
            }],
            debts: vec![DebtAmount {
                denom: uluna.denom.clone(),
                amount: borrow_amount,
                shares: Uint128::new(100),
            }],
            lends: vec![],
            vaults: vec![],
            perps: vec![],
            perp_vault: None,
        },
        asset_params,
        oracle_prices,
        vaults_data: vaults_data.clone(),
        perps_data,
    };

    let health = h.compute_health().unwrap();
    let collateral_value = deposit_amount.checked_mul_floor(uluna.price).unwrap();
    let debts_value = borrow_amount.checked_mul_floor(uluna.price).unwrap();

    assert_eq!(health.total_collateral_value, collateral_value);
    assert_eq!(
        health.max_ltv_adjusted_collateral,
        collateral_value.checked_mul_floor(uluna.params.max_loan_to_value).unwrap()
    );
    assert_eq!(
        health.liquidation_threshold_adjusted_collateral,
        collateral_value.checked_mul_floor(uluna.params.liquidation_threshold).unwrap()
    );
    assert_eq!(health.total_debt_value, borrow_amount.checked_mul_floor(uluna.price).unwrap());
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_ratio(
            collateral_value.checked_mul_floor(uluna.params.liquidation_threshold).unwrap(),
            debts_value
        ))
    );
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_ratio(
            collateral_value.checked_mul_floor(uluna.params.max_loan_to_value).unwrap(),
            debts_value,
        ))
    );
    assert!(!health.is_liquidatable());
    assert!(!health.is_above_max_ltv());

    // Terra implosion
    uluna.price = Decimal::zero();

    let oracle_prices = HashMap::from([(uluna.denom.clone(), uluna.price)]);
    let asset_params = HashMap::from([(uluna.denom.clone(), uluna.params.clone())]);

    let perps_data = PerpsData {
        denom_states: Default::default(),
        params: Default::default(),
    };

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![Coin {
                denom: uluna.denom.clone(),
                amount: deposit_amount,
            }],
            debts: vec![DebtAmount {
                denom: uluna.denom,
                amount: borrow_amount,
                shares: Uint128::new(100),
            }],
            lends: vec![],
            vaults: vec![],
            perps: vec![],
            perp_vault: None,
        },
        oracle_prices,
        asset_params,
        vaults_data,
        perps_data,
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::zero());
    assert_eq!(health.total_debt_value, Uint128::zero());
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::zero());
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::zero());
    assert_eq!(health.liquidation_health_factor, None);
    assert_eq!(health.max_ltv_health_factor, None);
    assert!(!health.is_liquidatable());
    assert!(!health.is_above_max_ltv());
}

/// Actions: User deposits 300 stars
///          and borrows 49 juno
/// Health: assets_value: 1569456334491.12991516325
///         debt value 350615100.25
///         liquidatable: false
///         above_max_ltv: false
#[test]
fn ltv_and_lqdt_adjusted_values() {
    let ustars = ustars_info();
    let ujuno = ujuno_info();

    let oracle_prices =
        HashMap::from([(ustars.denom.clone(), ustars.price), (ujuno.denom.clone(), ujuno.price)]);

    let asset_params = HashMap::from([
        (ustars.denom.clone(), ustars.params.clone()),
        (ujuno.denom.clone(), ujuno.params.clone()),
    ]);

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: Default::default(),
    };

    let perps_data = PerpsData {
        denom_states: Default::default(),
        params: Default::default(),
    };

    let deposit_amount = Uint128::new(300);
    let borrow_amount = Uint128::new(49);

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![
                Coin {
                    denom: ustars.denom.clone(),
                    amount: deposit_amount,
                },
                Coin {
                    denom: ujuno.denom.clone(),
                    amount: borrow_amount,
                },
            ],
            debts: vec![DebtAmount {
                denom: ujuno.denom.clone(),
                shares: Uint128::new(12345),
                amount: borrow_amount.add(Uint128::one()), // simulated interest
            }],
            lends: vec![],
            vaults: vec![],
            perps: vec![],
            perp_vault: None,
        },
        oracle_prices,
        asset_params,
        vaults_data,
        perps_data,
    };

    let health = h.compute_health().unwrap();
    assert_eq!(
        health.total_collateral_value,
        deposit_amount
            .checked_mul_floor(ustars.price)
            .unwrap()
            .add(borrow_amount.checked_mul_floor(ujuno.price).unwrap())
    );
    assert_eq!(
        health.total_debt_value,
        Uint128::new(350_615_101) // with simulated interest
    );
    let lqdt_adjusted_assets_value = deposit_amount
        .checked_mul_floor(ustars.price)
        .unwrap()
        .checked_mul_floor(ustars.params.liquidation_threshold)
        .unwrap()
        .add(
            borrow_amount
                .checked_mul_floor(ujuno.price)
                .unwrap()
                .checked_mul_floor(ujuno.params.liquidation_threshold)
                .unwrap(),
        );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_ratio(
            lqdt_adjusted_assets_value,
            (borrow_amount + Uint128::one()).checked_mul_ceil(ujuno.price).unwrap()
        ))
    );
    let ltv_adjusted_assets_value = deposit_amount
        .checked_mul_floor(ustars.price)
        .unwrap()
        .checked_mul_floor(ustars.params.max_loan_to_value)
        .unwrap()
        .add(
            borrow_amount
                .checked_mul_floor(ujuno.price)
                .unwrap()
                .checked_mul_floor(ujuno.params.max_loan_to_value)
                .unwrap(),
        );
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_ratio(
            ltv_adjusted_assets_value,
            (borrow_amount + Uint128::one()).checked_mul_ceil(ujuno.price).unwrap()
        ))
    );
    assert!(!health.is_liquidatable());
    assert!(!health.is_above_max_ltv());
}

/// Borrows 30 stars
/// Borrows 49 juno
/// Deposits 298 stars
/// Test validates debt calculation results
#[test]
fn debt_value() {
    let ustars = ustars_info();
    let ujuno = ujuno_info();

    let asset_params = HashMap::from([
        (ustars.denom.clone(), ustars.params.clone()),
        (ujuno.denom.clone(), ujuno.params.clone()),
    ]);

    let oracle_prices =
        HashMap::from([(ustars.denom.clone(), ustars.price), (ujuno.denom.clone(), ujuno.price)]);

    let vaults_data = VaultsData {
        vault_values: Default::default(),
        vault_configs: Default::default(),
    };

    let perps_data = PerpsData {
        denom_states: Default::default(),
        params: Default::default(),
    };

    let deposit_amount_stars = Uint128::new(298);
    let borrowed_amount_juno = Uint128::new(49);
    let borrowed_amount_stars = Uint128::new(30);

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![
                Coin {
                    denom: ustars.denom.clone(),
                    amount: deposit_amount_stars,
                },
                Coin {
                    denom: ujuno.denom.clone(),
                    amount: borrowed_amount_juno,
                },
                Coin {
                    denom: ustars.denom.clone(),
                    amount: borrowed_amount_stars,
                },
            ],
            debts: vec![
                DebtAmount {
                    denom: ujuno.denom.clone(),
                    shares: Uint128::new(12345),
                    amount: borrowed_amount_juno.add(Uint128::one()), // simulated interest
                },
                DebtAmount {
                    denom: ustars.denom.clone(),
                    shares: Uint128::new(12345),
                    amount: borrowed_amount_stars.add(Uint128::one()), // simulated interest
                },
            ],
            lends: vec![],
            vaults: vec![],
            perps: vec![],
            perp_vault: None,
        },
        oracle_prices,
        asset_params,
        vaults_data,
        perps_data,
    };

    let health = h.compute_health().unwrap();

    assert!(!health.is_above_max_ltv());
    assert!(!health.is_liquidatable());

    let juno_debt_value =
        borrowed_amount_juno.add(Uint128::one()).checked_mul_ceil(ujuno.price).unwrap();

    let stars_debt_value =
        borrowed_amount_stars.add(Uint128::one()).checked_mul_ceil(ustars.price).unwrap();

    let total_debt_value = juno_debt_value.add(stars_debt_value);
    assert_eq!(health.total_debt_value, total_debt_value);

    let lqdt_adjusted_assets_value = deposit_amount_stars
        .checked_mul_floor(ustars.price)
        .unwrap()
        .checked_mul_floor(ustars.params.liquidation_threshold)
        .unwrap()
        .add(
            borrowed_amount_stars
                .checked_mul_floor(ustars.price)
                .unwrap()
                .checked_mul_floor(ustars.params.liquidation_threshold)
                .unwrap(),
        )
        .add(
            borrowed_amount_juno
                .checked_mul_floor(ujuno.price)
                .unwrap()
                .checked_mul_floor(ujuno.params.liquidation_threshold)
                .unwrap(),
        );

    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_ratio(lqdt_adjusted_assets_value, total_debt_value))
    );

    let max_ltv_adjusted_assets_value = deposit_amount_stars
        .checked_mul_floor(ustars.price)
        .unwrap()
        .checked_mul_floor(ustars.params.max_loan_to_value)
        .unwrap()
        .add(
            borrowed_amount_stars
                .checked_mul_floor(ustars.price)
                .unwrap()
                .checked_mul_floor(ustars.params.max_loan_to_value)
                .unwrap(),
        )
        .add(
            borrowed_amount_juno
                .checked_mul_floor(ujuno.price)
                .unwrap()
                .checked_mul_floor(ujuno.params.max_loan_to_value)
                .unwrap(),
        );
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_ratio(max_ltv_adjusted_assets_value, total_debt_value))
    );
}

#[test]
fn above_max_ltv_below_liq_threshold() {
    let umars = umars_info();
    let udai = udai_info();

    let oracle_prices =
        HashMap::from([(umars.denom.clone(), umars.price), (udai.denom.clone(), udai.price)]);

    let asset_params = HashMap::from([
        (umars.denom.clone(), umars.params.clone()),
        (udai.denom.clone(), udai.params.clone()),
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
            deposits: vec![coin(1200, &umars.denom), coin(33, &udai.denom)],
            debts: vec![DebtAmount {
                denom: udai.denom,
                shares: Default::default(),
                amount: Uint128::new(3100),
            }],
            lends: vec![],
            vaults: vec![],
            perps: vec![],
            perp_vault: None,
        },
        oracle_prices,
        asset_params,
        vaults_data,
        perps_data,
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(1210));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(968));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(1017));
    assert_eq!(health.total_debt_value, Uint128::new(972));
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("0.99588477366255144").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("1.046296296296296296").unwrap())
    );
    assert!(health.is_above_max_ltv());
    assert!(!health.is_liquidatable());
}

#[test]
fn liquidatable() {
    let umars = umars_info();
    let udai = udai_info();

    let asset_params = HashMap::from([
        (umars.denom.clone(), umars.params.clone()),
        (udai.denom.clone(), udai.params.clone()),
    ]);

    let oracle_prices =
        HashMap::from([(umars.denom.clone(), umars.price), (udai.denom.clone(), udai.price)]);

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
            deposits: vec![coin(1200, &umars.denom), coin(33, &udai.denom)],
            debts: vec![
                DebtAmount {
                    denom: udai.denom,
                    shares: Default::default(),
                    amount: Uint128::new(3100),
                },
                DebtAmount {
                    denom: umars.denom,
                    shares: Default::default(),
                    amount: Uint128::new(200),
                },
            ],
            lends: vec![],
            vaults: vec![],
            perps: vec![],
            perp_vault: None,
        },
        oracle_prices,
        asset_params,
        vaults_data,
        perps_data,
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(1210));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(968));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(1017));
    assert_eq!(health.total_debt_value, Uint128::new(1172));
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("0.825938566552901023").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("0.867747440273037542").unwrap())
    );
    assert!(health.is_above_max_ltv());
    assert!(health.is_liquidatable());
}

#[test]
fn rover_whitelist_influences_max_ltv() {
    let umars = umars_info();
    let mut udai = udai_info();

    udai.params.credit_manager.whitelisted = false;

    let oracle_prices =
        HashMap::from([(umars.denom.clone(), umars.price), (udai.denom.clone(), udai.price)]);

    let asset_params = HashMap::from([
        (umars.denom.clone(), umars.params.clone()),
        (udai.denom.clone(), udai.params.clone()),
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
            deposits: vec![coin(1200, &umars.denom), coin(33, &udai.denom)],
            debts: vec![
                DebtAmount {
                    denom: udai.denom,
                    shares: Default::default(),
                    amount: Uint128::new(3100),
                },
                DebtAmount {
                    denom: umars.denom,
                    shares: Default::default(),
                    amount: Uint128::new(200),
                },
            ],
            lends: vec![],
            vaults: vec![],
            perps: vec![],
            perp_vault: None,
        },
        oracle_prices,
        asset_params,
        vaults_data,
        perps_data,
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(1210));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(960));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(1017));
    assert_eq!(health.total_debt_value, Uint128::new(1172));
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("0.819112627986348122").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("0.867747440273037542").unwrap())
    );
    assert!(health.is_above_max_ltv());
    assert!(health.is_liquidatable());
}

#[test]
fn unlocked_vault() {
    let umars = umars_info();
    let udai = udai_info();

    let asset_params = HashMap::from([
        (umars.denom.clone(), umars.params.clone()),
        (udai.denom.clone(), udai.params.clone()),
    ]);

    let oracle_prices =
        HashMap::from([(umars.denom.clone(), umars.price), (udai.denom.clone(), udai.price)]);

    let vault = Vault::new(Addr::unchecked("vault_addr_123".to_string()));

    let vaults_data = VaultsData {
        vault_values: HashMap::from([(
            vault.address.clone(),
            VaultPositionValue {
                vault_coin: CoinValue {
                    denom: "leverage_vault_123".to_string(),
                    amount: Default::default(),
                    value: Uint128::new(5264),
                },
                base_coin: CoinValue {
                    denom: udai.denom.clone(),
                    amount: Default::default(),
                    value: Default::default(),
                },
            },
        )]),
        vault_configs: HashMap::from([(
            vault.address.clone(),
            VaultConfig {
                addr: vault.address.clone(),
                deposit_cap: Default::default(),
                max_loan_to_value: Decimal::from_str("0.4").unwrap(),
                liquidation_threshold: Decimal::from_str("0.5").unwrap(),
                whitelisted: true,
                hls: None,
            },
        )]),
    };

    let perps_data = PerpsData {
        denom_states: Default::default(),
        params: Default::default(),
    };

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(1200, &umars.denom), coin(33, &udai.denom)],
            debts: vec![
                DebtAmount {
                    denom: udai.denom,
                    shares: Default::default(),
                    amount: Uint128::new(3100),
                },
                DebtAmount {
                    denom: umars.denom,
                    shares: Default::default(),
                    amount: Uint128::new(200),
                },
            ],
            lends: vec![],
            vaults: vec![VaultPosition {
                vault,
                amount: VaultPositionAmount::Unlocked(VaultAmount::new(Uint128::new(5264))),
            }],
            perps: vec![],
            perp_vault: None,
        },
        asset_params,
        oracle_prices,
        vaults_data,
        perps_data,
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(6474));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(3073));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(3649));
    assert_eq!(health.total_debt_value, Uint128::new(1172));
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("2.622013651877133105").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("3.113481228668941979").unwrap())
    );
    assert!(!health.is_above_max_ltv());
    assert!(!health.is_liquidatable());
}

#[test]
fn locked_vault() {
    let umars = umars_info();
    let udai = udai_info();

    let asset_params = HashMap::from([
        (umars.denom.clone(), umars.params.clone()),
        (udai.denom.clone(), udai.params.clone()),
    ]);

    let oracle_prices =
        HashMap::from([(umars.denom.clone(), umars.price), (udai.denom.clone(), udai.price)]);

    let vault = Vault::new(Addr::unchecked("vault_addr_123".to_string()));

    let vaults_data = VaultsData {
        vault_values: HashMap::from([(
            vault.address.clone(),
            VaultPositionValue {
                vault_coin: CoinValue {
                    denom: "leverage_vault_123".to_string(),
                    amount: Default::default(),
                    value: Uint128::new(5264),
                },
                base_coin: CoinValue {
                    denom: udai.denom.clone(),
                    amount: Default::default(),
                    value: Default::default(),
                },
            },
        )]),
        vault_configs: HashMap::from([(
            vault.address.clone(),
            VaultConfig {
                addr: vault.address.clone(),
                deposit_cap: Default::default(),
                max_loan_to_value: Decimal::from_str("0.4").unwrap(),
                liquidation_threshold: Decimal::from_str("0.5").unwrap(),
                whitelisted: true,
                hls: None,
            },
        )]),
    };

    let perps_data = PerpsData {
        denom_states: Default::default(),
        params: Default::default(),
    };

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(1200, &umars.denom), coin(33, &udai.denom)],
            debts: vec![
                DebtAmount {
                    denom: udai.denom,
                    shares: Default::default(),
                    amount: Uint128::new(3100),
                },
                DebtAmount {
                    denom: umars.denom,
                    shares: Default::default(),
                    amount: Uint128::new(200),
                },
            ],
            lends: vec![],
            vaults: vec![VaultPosition {
                vault,
                amount: VaultPositionAmount::Locking(LockingVaultAmount {
                    locked: VaultAmount::new(Uint128::new(42451613)),
                    unlocking: UnlockingPositions::new(vec![]),
                }),
            }],
            perps: vec![],
            perp_vault: None,
        },
        oracle_prices,
        asset_params,
        vaults_data,
        perps_data,
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(6474));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(3073));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(3649));
    assert_eq!(health.total_debt_value, Uint128::new(1172));
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("2.622013651877133105").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("3.113481228668941979").unwrap())
    );
    assert!(!health.is_above_max_ltv());
    assert!(!health.is_liquidatable());
}

#[test]
fn locked_vault_with_unlocking_positions() {
    let umars = umars_info();
    let udai = udai_info();

    let asset_params = HashMap::from([
        (umars.denom.clone(), umars.params.clone()),
        (udai.denom.clone(), udai.params.clone()),
    ]);

    let oracle_prices =
        HashMap::from([(umars.denom.clone(), umars.price), (udai.denom.clone(), udai.price)]);

    let vault = Vault::new(Addr::unchecked("vault_addr_123".to_string()));

    let vaults_data = VaultsData {
        vault_values: HashMap::from([(
            vault.address.clone(),
            VaultPositionValue {
                vault_coin: CoinValue {
                    denom: "leverage_vault_123".to_string(),
                    amount: Default::default(),
                    value: Uint128::new(5000),
                },
                base_coin: CoinValue {
                    denom: udai.denom.clone(),
                    amount: Default::default(),
                    value: Uint128::new(264),
                },
            },
        )]),
        vault_configs: HashMap::from([(
            vault.address.clone(),
            VaultConfig {
                addr: vault.address.clone(),
                deposit_cap: Default::default(),
                max_loan_to_value: Decimal::from_str("0.4").unwrap(),
                liquidation_threshold: Decimal::from_str("0.5").unwrap(),
                whitelisted: true,
                hls: None,
            },
        )]),
    };

    let perps_data = PerpsData {
        denom_states: Default::default(),
        params: Default::default(),
    };

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(1200, &umars.denom), coin(33, &udai.denom)],
            debts: vec![
                DebtAmount {
                    denom: udai.denom.clone(),
                    shares: Default::default(),
                    amount: Uint128::new(3100),
                },
                DebtAmount {
                    denom: umars.denom,
                    shares: Default::default(),
                    amount: Uint128::new(200),
                },
            ],
            lends: vec![],
            vaults: vec![VaultPosition {
                vault,
                amount: VaultPositionAmount::Locking(LockingVaultAmount {
                    locked: VaultAmount::new(Uint128::new(40330000)),
                    unlocking: UnlockingPositions::new(vec![
                        VaultUnlockingPosition {
                            id: 0,
                            coin: coin(840, udai.denom.clone()),
                        },
                        VaultUnlockingPosition {
                            id: 1,
                            coin: coin(3, udai.denom),
                        },
                    ]),
                }),
            }],
            perps: vec![],
            perp_vault: None,
        },
        asset_params,
        oracle_prices,
        vaults_data,
        perps_data,
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(6474));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(3192));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(3754));
    assert_eq!(health.total_debt_value, Uint128::new(1172));
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("2.723549488054607508").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("3.203071672354948805").unwrap())
    );
    assert!(!health.is_above_max_ltv());
    assert!(!health.is_liquidatable());
}

#[test]
fn vault_is_not_whitelisted() {
    let umars = umars_info();
    let udai = udai_info();

    let asset_params = HashMap::from([
        (umars.denom.clone(), umars.params.clone()),
        (udai.denom.clone(), udai.params.clone()),
    ]);

    let oracle_prices =
        HashMap::from([(umars.denom.clone(), umars.price), (udai.denom.clone(), udai.price)]);

    let vault = Vault::new(Addr::unchecked("vault_addr_123".to_string()));

    let vaults_data = VaultsData {
        vault_values: HashMap::from([(
            vault.address.clone(),
            VaultPositionValue {
                vault_coin: CoinValue {
                    denom: "leverage_vault_123".to_string(),
                    amount: Default::default(),
                    value: Uint128::new(5264),
                },
                base_coin: CoinValue {
                    denom: udai.denom.clone(),
                    amount: Default::default(),
                    value: Default::default(),
                },
            },
        )]),
        vault_configs: HashMap::from([(
            vault.address.clone(),
            VaultConfig {
                addr: vault.address.clone(),
                deposit_cap: Default::default(),
                max_loan_to_value: Decimal::from_str("0.4").unwrap(),
                liquidation_threshold: Decimal::from_str("0.5").unwrap(),
                whitelisted: false,
                hls: None,
            },
        )]),
    };

    let perps_data = PerpsData {
        denom_states: Default::default(),
        params: Default::default(),
    };

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(1200, &umars.denom), coin(33, &udai.denom)],
            debts: vec![
                DebtAmount {
                    denom: udai.denom,
                    shares: Default::default(),
                    amount: Uint128::new(3100),
                },
                DebtAmount {
                    denom: umars.denom,
                    shares: Default::default(),
                    amount: Uint128::new(200),
                },
            ],
            lends: vec![],
            vaults: vec![VaultPosition {
                vault,
                amount: VaultPositionAmount::Unlocked(VaultAmount::new(Uint128::new(5264))),
            }],
            perps: vec![],
            perp_vault: None,
        },
        oracle_prices,
        asset_params,
        vaults_data,
        perps_data,
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(6474));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(968));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(3649));
    assert_eq!(health.total_debt_value, Uint128::new(1172));
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("0.825938566552901023").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("3.113481228668941979").unwrap())
    );
    assert!(health.is_above_max_ltv());
    assert!(!health.is_liquidatable());
}

/// Delisting base token will make even vault token maxLTV to drop
#[test]
fn vault_base_token_is_not_whitelisted() {
    let umars = umars_info();
    let udai = udai_info();
    let mut ujuno = ujuno_info();

    ujuno.params.credit_manager.whitelisted = false;

    let asset_params = HashMap::from([
        (umars.denom.clone(), umars.params.clone()),
        (udai.denom.clone(), udai.params.clone()),
        (ujuno.denom.clone(), ujuno.params.clone()),
    ]);

    let oracle_prices = HashMap::from([
        (umars.denom.clone(), umars.price),
        (udai.denom.clone(), udai.price),
        (ujuno.denom.clone(), ujuno.price),
    ]);

    let vault = Vault::new(Addr::unchecked("vault_addr_123".to_string()));

    let vaults_data = VaultsData {
        vault_values: HashMap::from([(
            vault.address.clone(),
            VaultPositionValue {
                vault_coin: CoinValue {
                    denom: "leverage_vault_123".to_string(),
                    amount: Uint128::new(40330000),
                    value: Uint128::new(5000),
                },
                base_coin: CoinValue {
                    denom: ujuno.denom.clone(),
                    amount: Uint128::new(71),
                    value: Uint128::new(497873442),
                },
            },
        )]),
        vault_configs: HashMap::from([(
            vault.address.clone(),
            VaultConfig {
                addr: vault.address.clone(),
                deposit_cap: Default::default(),
                max_loan_to_value: Decimal::from_str("0.4").unwrap(),
                liquidation_threshold: Decimal::from_str("0.5").unwrap(),
                whitelisted: true,
                hls: None,
            },
        )]),
    };

    let perps_data = PerpsData {
        denom_states: Default::default(),
        params: Default::default(),
    };

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(1200, &umars.denom), coin(33, &udai.denom)],
            debts: vec![
                DebtAmount {
                    denom: udai.denom,
                    shares: Default::default(),
                    amount: Uint128::new(3100),
                },
                DebtAmount {
                    denom: umars.denom,
                    shares: Default::default(),
                    amount: Uint128::new(200),
                },
            ],
            lends: vec![],
            vaults: vec![VaultPosition {
                vault,
                amount: VaultPositionAmount::Locking(LockingVaultAmount {
                    locked: VaultAmount::new(Uint128::new(40330000)),
                    unlocking: UnlockingPositions::new(vec![
                        VaultUnlockingPosition {
                            id: 0,
                            coin: coin(60, ujuno.denom.clone()),
                        },
                        VaultUnlockingPosition {
                            id: 1,
                            coin: coin(11, ujuno.denom),
                        },
                    ]),
                }),
            }],
            perps: vec![],
            perp_vault: None,
        },
        oracle_prices,
        asset_params,
        vaults_data,
        perps_data,
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(497879652));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(968)); // Lower due to vault blacklisted
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(448089614));
    assert_eq!(health.total_debt_value, Uint128::new(1172));
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("0.825938566552901023").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("382329.022184300341296928").unwrap())
    );
    assert!(health.is_above_max_ltv());
    assert!(!health.is_liquidatable());
}

#[test]
fn lent_coins_used_as_collateral() {
    let umars = umars_info();
    let udai = udai_info();
    let uluna = uluna_info();

    let asset_params = HashMap::from([
        (umars.denom.clone(), umars.params.clone()),
        (udai.denom.clone(), udai.params.clone()),
        (uluna.denom.clone(), uluna.params.clone()),
    ]);

    let oracle_prices = HashMap::from([
        (umars.denom.clone(), umars.price),
        (udai.denom.clone(), udai.price),
        (uluna.denom.clone(), uluna.price),
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
            deposits: vec![coin(1200, &umars.denom), coin(23, &udai.denom)],
            debts: vec![DebtAmount {
                denom: udai.denom.clone(),
                shares: Default::default(),
                amount: Uint128::new(3100),
            }],
            lends: vec![coin(10, udai.denom), coin(2, uluna.denom)],
            vaults: vec![],
            perps: vec![],
            perp_vault: None,
        },
        asset_params,
        oracle_prices,
        vaults_data,
        perps_data,
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(1230));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(981));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(1031));
    assert_eq!(health.total_debt_value, Uint128::new(972));
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("1.009259259259259259").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("1.060699588477366255").unwrap())
    );
    assert!(!health.is_above_max_ltv());
    assert!(!health.is_liquidatable());
}

#[test]
fn allowed_lent_coins_influence_max_ltv() {
    let umars = umars_info();
    let udai = udai_info();
    let mut uluna = uluna_info();

    uluna.params.credit_manager.whitelisted = false;

    let asset_params = HashMap::from([
        (umars.denom.clone(), umars.params.clone()),
        (udai.denom.clone(), udai.params.clone()),
        (uluna.denom.clone(), uluna.params.clone()),
    ]);

    let oracle_prices = HashMap::from([
        (umars.denom.clone(), umars.price),
        (udai.denom.clone(), udai.price),
        (uluna.denom.clone(), uluna.price),
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
            deposits: vec![coin(1200, &umars.denom), coin(23, &udai.denom)],
            debts: vec![DebtAmount {
                denom: udai.denom.clone(),
                shares: Default::default(),
                amount: Uint128::new(3100),
            }],
            lends: vec![coin(10, udai.denom), coin(2, uluna.denom)],
            vaults: vec![],
            perps: vec![],
            perp_vault: None,
        },
        oracle_prices,
        asset_params,
        vaults_data,
        perps_data,
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(1230));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(967));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(1031));
    assert_eq!(health.total_debt_value, Uint128::new(972));
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("0.9948559670781893").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("1.060699588477366255").unwrap())
    );
    assert!(health.is_above_max_ltv());
    assert!(!health.is_liquidatable());
}

// DOC: Health Factor underwater - longs
#[test]
fn long_one_negative_pnl_perp_no_spot_debt() {
    let uusd = uusdc_info();
    let entry_price = Decimal::from_str("100").unwrap();
    let current_price = Decimal::from_str("92").unwrap();
    let max_ltv = Decimal::from_str("0.9").unwrap();
    let liquidation_threshold = Decimal::from_str("0.95").unwrap();
    let size = SignedUint::from_str("10000000").unwrap();
    let btcperp =
        create_perp_info("btc/usd/perp".to_string(), current_price, max_ltv, liquidation_threshold);

    let asset_params = HashMap::from([(uusd.denom.clone(), uusd.params.clone())]);

    let oracle_prices =
        HashMap::from([(uusd.denom.clone(), uusd.price), (btcperp.denom.clone(), btcperp.price)]);

    let vaults_data = Default::default();
    let closing_fee_rate = Decimal::from_str("0.0002").unwrap();
    let unrealised_funding_accrued = SignedUint::from_str("-25210000").unwrap();

    let perps_data = PerpsData {
        denom_states: HashMap::from([(
            btcperp.denom.clone(),
            PerpDenomState {
                enabled: true,
                ..PerpDenomState::default()
            },
        )]),
        params: HashMap::from([(btcperp.denom.clone(), btcperp.perp_params.clone())]),
    };
    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(152000000, &uusd.denom)],
            debts: vec![],
            lends: vec![],
            vaults: vec![],
            perps: vec![PerpPosition {
                denom: btcperp.denom,
                base_denom: uusd.denom,
                current_price,
                entry_price,
                entry_exec_price: entry_price,
                current_exec_price: current_price,
                size,
                unrealised_pnl: PnlAmounts {
                    accrued_funding: unrealised_funding_accrued,
                    pnl: SignedUint::from_str("-24790000").unwrap(),
                    ..Default::default()
                },
                realised_pnl: PnlAmounts::default(),
                closing_fee_rate,
            }],
            perp_vault: None,
        },
        oracle_prices,
        asset_params,
        vaults_data,
        perps_data,
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(152000000));
    assert_eq!(health.total_debt_value, Uint128::new(0));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(136800000));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(144400000));

    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("0.940896011548853405").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("0.993177983047375659").unwrap())
    );
    assert!(health.is_above_max_ltv());
    assert!(health.is_liquidatable());
}

// DOC: Health Factor positive - longs
#[test]
fn long_one_positive_pnl_perp_no_spot_debt() {
    let uusd = uusdc_info();
    let entry_price = Decimal::from_str("100").unwrap();
    let current_price = Decimal::from_str("104").unwrap();
    let max_ltv = Decimal::from_str("0.9").unwrap();
    let liquidation_threshold = Decimal::from_str("0.95").unwrap();
    let size = SignedUint::from_str("10000000").unwrap();
    let btcperp =
        create_perp_info("btc/usd/perp".to_string(), current_price, max_ltv, liquidation_threshold);

    let asset_params = HashMap::from([(uusd.denom.clone(), uusd.params.clone())]);

    let oracle_prices =
        HashMap::from([(uusd.denom.clone(), uusd.price), (btcperp.denom.clone(), btcperp.price)]);

    let perps_data = PerpsData {
        denom_states: HashMap::from([(
            btcperp.denom.clone(),
            PerpDenomState {
                enabled: true,
                ..PerpDenomState::default()
            },
        )]),
        params: HashMap::from([(btcperp.denom.clone(), btcperp.perp_params.clone())]),
    };
    let vaults_data = Default::default();
    let closing_fee_rate = Decimal::from_str("0.0002").unwrap();
    let unrealised_funding_accrued = SignedUint::from_str("15210000").unwrap();
    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(152000000, &uusd.denom)],
            debts: vec![],
            lends: vec![],
            vaults: vec![],
            perps: vec![PerpPosition {
                denom: btcperp.denom,
                base_denom: uusd.denom,
                current_price,
                entry_price,
                entry_exec_price: entry_price,
                current_exec_price: current_price,
                size,
                unrealised_pnl: PnlAmounts {
                    accrued_funding: unrealised_funding_accrued,
                    pnl: SignedUint::from_str("-24790000").unwrap(),
                    ..Default::default()
                },
                realised_pnl: PnlAmounts::default(),
                closing_fee_rate,
            }],
            perp_vault: None,
        },
        asset_params,
        oracle_prices,
        vaults_data,
        perps_data,
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(152000000));
    assert_eq!(health.total_debt_value, Uint128::new(0));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(136800000));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(144400000));

    assert_eq!(health.max_ltv_health_factor, Some(Decimal::from_str("1.086281").unwrap()));
    assert_eq!(health.liquidation_health_factor, Some(Decimal::from_str("1.1466415").unwrap()));
    assert!(!health.is_above_max_ltv());
    assert!(!health.is_liquidatable());
}

// DOC - Health Factor Underwater (short)
#[test]
fn one_short_negative_pnl_perp_no_spot_debt() {
    let uusd = uusdc_info();
    let entry_price = Decimal::from_str("100").unwrap();
    let current_price = Decimal::from_str("105").unwrap();
    let max_ltv = Decimal::from_str("0.9").unwrap();
    let liquidation_threshold = Decimal::from_str("0.95").unwrap();
    let size = SignedUint::from_str("-10000000").unwrap();
    let btcperp =
        create_perp_info("btc/usd/perp".to_string(), current_price, max_ltv, liquidation_threshold);

    let asset_params = HashMap::from([(uusd.denom.clone(), uusd.params.clone())]);

    let oracle_prices =
        HashMap::from([(uusd.denom.clone(), uusd.price), (btcperp.denom.clone(), btcperp.price)]);

    let vaults_data = Default::default();

    let perps_data = PerpsData {
        denom_states: HashMap::from([(
            btcperp.denom.clone(),
            PerpDenomState {
                enabled: true,
                ..PerpDenomState::default()
            },
        )]),
        params: HashMap::from([(btcperp.denom.clone(), btcperp.perp_params.clone())]),
    };
    let closing_fee_rate = Decimal::from_str("0.0002").unwrap();
    let unrealised_funding_accrued = SignedUint::from_str("15210000").unwrap();
    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(152000000, &uusd.denom)],
            debts: vec![],
            lends: vec![],
            vaults: vec![],
            perps: vec![PerpPosition {
                denom: btcperp.denom,
                base_denom: uusd.denom,
                current_price,
                entry_price,
                entry_exec_price: entry_price,
                current_exec_price: current_price,
                size,
                unrealised_pnl: PnlAmounts {
                    accrued_funding: unrealised_funding_accrued,
                    pnl: SignedUint::from_str("-55000000").unwrap(),
                    ..Default::default()
                },
                closing_fee_rate,
                realised_pnl: PnlAmounts::default(),
            }],
            perp_vault: None,
        },
        oracle_prices,
        asset_params,
        vaults_data,
        perps_data,
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(152000000));
    assert_eq!(health.total_debt_value, Uint128::new(0));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(136800000));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(144400000));

    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("0.995913297149436033").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("1.050910484170815536").unwrap())
    );
    assert!(health.is_above_max_ltv());
    assert!(!health.is_liquidatable());
}

#[test]
fn one_short_negative_pnl_perp_vault_collateral_no_spot_debt() {
    let uusd = uusdc_info();

    let entry_price = Decimal::from_str("100").unwrap();
    let current_price = Decimal::from_str("105.50").unwrap();
    let max_ltv = Decimal::from_str("0.9").unwrap();
    let liquidation_threshold = Decimal::from_str("0.95").unwrap();
    let btcperp =
        create_perp_info("btc/usd/perp".to_string(), current_price, max_ltv, liquidation_threshold);

    let asset_params = HashMap::from([(uusd.denom.clone(), uusd.params.clone())]);

    let oracle_prices =
        HashMap::from([(uusd.denom.clone(), uusd.price), (btcperp.denom.clone(), current_price)]);

    let vault = Vault::new(Addr::unchecked("vault_addr_123".to_string()));

    let vaults_data = VaultsData {
        vault_values: HashMap::from([(
            vault.address.clone(),
            VaultPositionValue {
                vault_coin: CoinValue {
                    denom: "leverage_vault_123".to_string(),
                    amount: Default::default(),
                    value: Uint128::new(112000000),
                },
                base_coin: CoinValue {
                    denom: uusd.denom.clone(),
                    amount: Default::default(),
                    value: Default::default(),
                },
            },
        )]),
        vault_configs: HashMap::from([(
            vault.address.clone(),
            VaultConfig {
                addr: vault.address.clone(),
                deposit_cap: Default::default(),
                max_loan_to_value: Decimal::from_str("0.9").unwrap(),
                liquidation_threshold: Decimal::from_str("0.95").unwrap(),
                whitelisted: true,
                hls: None,
            },
        )]),
    };

    let perps_data = PerpsData {
        denom_states: HashMap::from([(
            btcperp.denom.clone(),
            PerpDenomState {
                enabled: true,
                ..PerpDenomState::default()
            },
        )]),
        params: HashMap::from([(btcperp.denom.clone(), btcperp.perp_params.clone())]),
    };

    let closing_fee_rate = Decimal::from_str("0.000").unwrap();
    let unrealised_funding_accrued = SignedUint::zero();

    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![],
            debts: vec![],
            lends: vec![],
            vaults: vec![VaultPosition {
                vault,
                amount: VaultPositionAmount::Unlocked(VaultAmount::new(Uint128::new(5264))),
            }],

            perps: vec![PerpPosition {
                denom: btcperp.denom,
                base_denom: uusd.denom,
                current_price,
                entry_price,
                entry_exec_price: entry_price,
                current_exec_price: current_price,
                size: SignedUint::from_str("-10000000").unwrap(),
                unrealised_pnl: PnlAmounts {
                    accrued_funding: unrealised_funding_accrued,
                    pnl: SignedUint::from_str("-55000000").unwrap(),
                    ..Default::default()
                },
                realised_pnl: PnlAmounts::default(),
                closing_fee_rate,
            }],
            perp_vault: None,
        },
        oracle_prices,
        asset_params,
        vaults_data,
        perps_data,
    };

    let health = h.compute_health().unwrap();

    assert_eq!(health.total_collateral_value, Uint128::new(112000000));
    assert_eq!(health.total_debt_value, Uint128::new(0));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(100800000));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(106400000));

    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("0.948556656613528651").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("0.998781313473256601").unwrap())
    );
    assert!(health.is_above_max_ltv());
    assert!(health.is_liquidatable());
}

// DOC : Health factor positive - (short)
#[test]
fn one_short_positive_pnl_perp_no_spot_debt() {
    let uusd = uusdc_info();
    let entry_price = Decimal::from_str("100").unwrap();
    let current_price = Decimal::from_str("95").unwrap();
    let max_ltv = Decimal::from_str("0.9").unwrap();
    let liquidation_threshold = Decimal::from_str("0.95").unwrap();
    let size = SignedUint::from_str("-10000000").unwrap();
    let btcperp =
        create_perp_info("btc/usd/perp".to_string(), current_price, max_ltv, liquidation_threshold);

    let asset_params = HashMap::from([(uusd.denom.clone(), uusd.params.clone())]);

    let oracle_prices =
        HashMap::from([(uusd.denom.clone(), uusd.price), (btcperp.denom.clone(), btcperp.price)]);

    let vaults_data = Default::default();

    let perps_data = PerpsData {
        denom_states: HashMap::from([(
            btcperp.denom.clone(),
            PerpDenomState {
                enabled: true,
                ..PerpDenomState::default()
            },
        )]),
        params: HashMap::from([(btcperp.denom.clone(), btcperp.perp_params.clone())]),
    };
    let closing_fee_rate = Decimal::from_str("0.0002").unwrap();
    let unrealised_funding_accrued = SignedUint::from_str("15210000").unwrap();
    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(152000000, &uusd.denom)],
            debts: vec![],
            lends: vec![],
            vaults: vec![],
            perps: vec![PerpPosition {
                denom: btcperp.denom,
                base_denom: uusd.denom,
                current_price,
                entry_price,
                entry_exec_price: entry_price,
                current_exec_price: current_price,
                size,
                unrealised_pnl: PnlAmounts {
                    accrued_funding: unrealised_funding_accrued,
                    pnl: SignedUint::from_str("-55000000").unwrap(),
                    ..Default::default()
                },
                realised_pnl: PnlAmounts::default(),
                closing_fee_rate,
            }],
            perp_vault: None,
        },
        oracle_prices,
        asset_params,
        vaults_data,
        perps_data,
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(152000000));
    assert_eq!(health.total_debt_value, Uint128::new(0));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(136800000));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(144400000));

    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("1.100746275796745089").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("1.161532640399322434").unwrap())
    );
    assert!(!health.is_above_max_ltv());
    assert!(!health.is_liquidatable());
}

// DOC - Health Factor - Long & Short
#[test]
fn perps_one_short_negative_pnl_one_long_negative_pnl_no_spot_debt() {
    let uusd = uusdc_info();
    let entry_price_btc = Decimal::from_str("100").unwrap();
    let current_price_btc = Decimal::from_str("95").unwrap();
    let max_ltv_btc = Decimal::from_str("0.9").unwrap();
    let liquidation_threshold_btc = Decimal::from_str("0.95").unwrap();
    let size_btc = SignedUint::from_str("10000000").unwrap();
    let btcperp = create_perp_info(
        "btc/usd/perp".to_string(),
        current_price_btc,
        max_ltv_btc,
        liquidation_threshold_btc,
    );

    let entry_price_eth = Decimal::from_str("10").unwrap();
    let current_price_eth = Decimal::from_str("10.72").unwrap();
    let max_ltv_eth = Decimal::from_str("0.85").unwrap();
    let liquidation_threshold_eth = Decimal::from_str("0.9").unwrap();
    let size_eth = SignedUint::from_str("-80000000").unwrap();
    let ethperp = create_perp_info(
        "eth/usd/perp".to_string(),
        current_price_eth,
        max_ltv_eth,
        liquidation_threshold_eth,
    );

    let oracle_prices = HashMap::from([
        (uusd.denom.clone(), uusd.price),
        (btcperp.denom.clone(), btcperp.price),
        (ethperp.denom.clone(), ethperp.price),
    ]);

    let asset_params = HashMap::from([(uusd.denom.clone(), uusd.params.clone())]);

    let vaults_data = Default::default();
    let default_perp_denom_state = PerpDenomState {
        enabled: true,
        ..PerpDenomState::default()
    };
    let perps_data = PerpsData {
        denom_states: HashMap::from([
            (btcperp.denom.clone(), default_perp_denom_state.clone()),
            (ethperp.denom.clone(), default_perp_denom_state),
        ]),
        params: HashMap::from([
            (btcperp.denom.clone(), btcperp.perp_params.clone()),
            (ethperp.denom.clone(), ethperp.perp_params.clone()),
        ]),
    };
    let closing_fee_rate = Decimal::from_str("0.0002").unwrap();
    let unrealised_funding_accrued_btc = SignedUint::from_str("15210000").unwrap();
    let unrealised_funding_accrued_eth = SignedUint::from_str("-12650000").unwrap();
    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(152000000, &uusd.denom)],
            debts: vec![],
            lends: vec![],
            vaults: vec![],
            perps: vec![
                PerpPosition {
                    denom: btcperp.denom,
                    base_denom: uusd.denom.clone(),
                    current_price: current_price_btc,
                    entry_price: entry_price_btc,
                    entry_exec_price: entry_price_btc,
                    current_exec_price: current_price_btc,
                    size: size_btc,
                    // pnl is not used for calculating
                    unrealised_pnl: PnlAmounts {
                        accrued_funding: unrealised_funding_accrued_btc,
                        pnl: SignedUint::zero(),
                        ..Default::default()
                    },
                    realised_pnl: PnlAmounts::default(),
                    closing_fee_rate,
                },
                PerpPosition {
                    denom: ethperp.denom,
                    base_denom: uusd.denom,
                    current_price: current_price_eth,
                    entry_price: entry_price_eth,
                    entry_exec_price: entry_price_eth,
                    current_exec_price: current_price_eth,
                    size: size_eth,
                    // pnl is not used for calculating
                    unrealised_pnl: PnlAmounts {
                        accrued_funding: unrealised_funding_accrued_eth,
                        pnl: SignedUint::zero(),
                        ..Default::default()
                    },
                    realised_pnl: PnlAmounts::default(),
                    closing_fee_rate,
                },
            ],
            perp_vault: None,
        },
        oracle_prices,
        asset_params,
        vaults_data,
        perps_data,
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(152000000));
    assert_eq!(health.total_debt_value, Uint128::new(0));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(136800000));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(144400000));

    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("0.903073258095628792").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("0.951424743037139007").unwrap())
    );
    assert!(health.is_above_max_ltv());
    assert!(health.is_liquidatable());
}

// DOC - Health Factor - (Long  & Short) 2
#[test]
fn perps_one_short_negative_pnl_one_long_positive_pnl_no_spot_debt() {
    let uusd = uusdc_info();
    let entry_price_btc = Decimal::from_str("100").unwrap();
    let current_price_btc = Decimal::from_str("115").unwrap();
    let max_ltv_btc = Decimal::from_str("0.9").unwrap();
    let liquidation_threshold_btc = Decimal::from_str("0.95").unwrap();
    let size_btc = SignedUint::from_str("10000000").unwrap();
    let btcperp = create_perp_info(
        "btc/usd/perp".to_string(),
        current_price_btc,
        max_ltv_btc,
        liquidation_threshold_btc,
    );

    let entry_price_eth = Decimal::from_str("10").unwrap();
    let current_price_eth = Decimal::from_str("10.72").unwrap();
    let max_ltv_eth = Decimal::from_str("0.85").unwrap();
    let liquidation_threshold_eth = Decimal::from_str("0.9").unwrap();
    let size_eth = SignedUint::from_str("-80000000").unwrap();
    let ethperp = create_perp_info(
        "eth/usd/perp".to_string(),
        current_price_eth,
        max_ltv_eth,
        liquidation_threshold_eth,
    );

    let oracle_prices = HashMap::from([
        (uusd.denom.clone(), uusd.price),
        (btcperp.denom.clone(), btcperp.price),
        (ethperp.denom.clone(), ethperp.price),
    ]);

    let asset_params = HashMap::from([(uusd.denom.clone(), uusd.params.clone())]);

    let vaults_data = Default::default();

    let default_perp_denom_state = PerpDenomState {
        enabled: true,
        ..PerpDenomState::default()
    };
    let perps_data = PerpsData {
        denom_states: HashMap::from([
            (btcperp.denom.clone(), default_perp_denom_state.clone()),
            (ethperp.denom.clone(), default_perp_denom_state),
        ]),
        params: HashMap::from([
            (btcperp.denom.clone(), btcperp.perp_params.clone()),
            (ethperp.denom.clone(), ethperp.perp_params.clone()),
        ]),
    };
    let closing_fee_rate = Decimal::from_str("0.0002").unwrap();
    let unrealised_funding_accrued_btc = SignedUint::from_str("15210000").unwrap();
    let unrealised_funding_accrued_eth = SignedUint::from_str("-12650000").unwrap();
    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(152000000, &uusd.denom)],
            debts: vec![],
            lends: vec![],
            vaults: vec![],
            perps: vec![
                PerpPosition {
                    denom: btcperp.denom,
                    base_denom: uusd.denom.clone(),
                    current_price: current_price_btc,
                    entry_price: entry_price_btc,
                    entry_exec_price: entry_price_btc,
                    current_exec_price: current_price_btc,
                    size: size_btc,
                    // pnl is not used for calculating
                    unrealised_pnl: PnlAmounts {
                        accrued_funding: unrealised_funding_accrued_btc,
                        pnl: SignedUint::zero(),
                        ..Default::default()
                    },
                    realised_pnl: PnlAmounts::default(),
                    closing_fee_rate,
                },
                PerpPosition {
                    denom: ethperp.denom,
                    base_denom: uusd.denom,
                    current_price: current_price_eth,
                    entry_price: entry_price_eth,
                    entry_exec_price: entry_price_eth,
                    current_exec_price: current_price_eth,
                    size: size_eth,
                    // pnl is not used for calculating
                    unrealised_pnl: PnlAmounts {
                        accrued_funding: unrealised_funding_accrued_eth,
                        pnl: SignedUint::zero(),
                        ..Default::default()
                    },
                    realised_pnl: PnlAmounts::default(),
                    closing_fee_rate,
                },
            ],
            perp_vault: None,
        },
        oracle_prices,
        asset_params,
        vaults_data,
        perps_data,
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(152000000));
    assert_eq!(health.total_debt_value, Uint128::new(0));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(136800000));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(144400000));

    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("0.993095500132482165").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("1.048532295714561294").unwrap())
    );
    assert!(health.is_above_max_ltv());
    assert!(!health.is_liquidatable());
}

// DOC - Delta neutral BTC - Short Perp
#[test]
fn perp_short_delta_neutral_with_btc_collateral() {
    let uusd = uusdc_info();
    let entry_price = Decimal::from_str("100").unwrap();
    let current_price = Decimal::from_str("95").unwrap();
    let max_ltv = Decimal::from_str("0.8").unwrap();
    let liquidation_threshold = Decimal::from_str("0.85").unwrap();
    let size = SignedUint::from_str("-10000000").unwrap();
    let btcperp =
        create_perp_info("btc/usd/perp".to_string(), current_price, max_ltv, liquidation_threshold);
    let btc_coin = create_coin_info(
        "btc".to_string(),
        current_price,
        Decimal::from_str("0.7").unwrap(),
        Decimal::from_str("0.72").unwrap(),
    );

    let oracle_prices = HashMap::from([
        (uusd.denom.clone(), uusd.price),
        (btcperp.denom.clone(), btcperp.price),
        (btc_coin.denom.clone(), btc_coin.price),
    ]);

    let asset_params = HashMap::from([
        (uusd.denom.clone(), uusd.params.clone()),
        (btc_coin.denom.clone(), btc_coin.params.clone()),
    ]);

    let vaults_data = Default::default();
    let default_perp_denom_state = PerpDenomState {
        enabled: true,
        ..PerpDenomState::default()
    };
    let perps_data = PerpsData {
        denom_states: HashMap::from([(btcperp.denom.clone(), default_perp_denom_state)]),
        params: HashMap::from([(btcperp.denom.clone(), btcperp.perp_params.clone())]),
    };
    let closing_fee_rate = Decimal::from_str("0.0002").unwrap();
    let unrealised_funding_accrued = SignedUint::from_str("15210000").unwrap();
    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(size.abs.into(), &btc_coin.denom)],
            debts: vec![],
            lends: vec![],
            vaults: vec![],
            perps: vec![PerpPosition {
                denom: btcperp.denom,
                base_denom: uusd.denom,
                current_price,
                entry_price,
                entry_exec_price: entry_price,
                current_exec_price: current_price,
                size,
                unrealised_pnl: PnlAmounts {
                    accrued_funding: unrealised_funding_accrued,
                    pnl: SignedUint::zero(),
                    ..Default::default()
                },
                realised_pnl: PnlAmounts::default(),
                closing_fee_rate,
            }],
            perp_vault: None,
        },
        oracle_prices,
        asset_params,
        vaults_data,
        perps_data,
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(950000000));
    assert_eq!(health.total_debt_value, Uint128::new(0));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(665000000));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(684000000));

    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("1.472288829054806655").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("1.554374525254189202").unwrap())
    );
    assert!(!health.is_above_max_ltv());
    assert!(!health.is_liquidatable());
}

// DOC - Delta neutral btc short spot with leverage
#[test]
fn spot_short_delta_neutral_with_leverage() {
    let uusd = uusdc_info();
    let entry_price = Decimal::from_str("100").unwrap();
    let current_price = Decimal::from_str("104").unwrap();
    let max_ltv = Decimal::from_str("0.9").unwrap();
    let liquidation_threshold = Decimal::from_str("0.95").unwrap();
    let size = SignedUint::from_str("10000000").unwrap();
    let btcperp =
        create_perp_info("btc/usd/perp".to_string(), current_price, max_ltv, liquidation_threshold);
    let btc_coin = create_coin_info(
        "btc".to_string(),
        current_price,
        Decimal::from_str("0.7").unwrap(),
        Decimal::from_str("0.72").unwrap(),
    );

    let asset_params = HashMap::from([
        (uusd.denom.clone(), uusd.params.clone()),
        (btc_coin.denom.clone(), btc_coin.params.clone()),
    ]);

    let oracle_prices = HashMap::from([
        (uusd.denom.clone(), uusd.price),
        (btcperp.denom.clone(), btcperp.price),
        (btc_coin.denom.clone(), btc_coin.price),
    ]);

    let vaults_data = Default::default();
    let default_perp_denom_state = PerpDenomState {
        enabled: true,
        ..PerpDenomState::default()
    };
    let perps_data = PerpsData {
        denom_states: HashMap::from([(btcperp.denom.clone(), default_perp_denom_state)]),
        params: HashMap::from([(btcperp.denom.clone(), btcperp.perp_params.clone())]),
    };

    let closing_fee_rate = Decimal::from_str("0.0002").unwrap();
    let unrealised_funding_accrued = SignedUint::from_str("15210000").unwrap();
    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(1300000000, &uusd.denom)],
            debts: vec![DebtAmount {
                denom: btc_coin.denom,
                amount: Uint128::new(10000000),
                shares: Uint128::new(100),
            }],
            lends: vec![],
            vaults: vec![],
            perps: vec![PerpPosition {
                denom: btcperp.denom,
                base_denom: uusd.denom,
                current_price,
                entry_price,
                entry_exec_price: entry_price,
                current_exec_price: current_price,
                size,
                unrealised_pnl: PnlAmounts {
                    accrued_funding: unrealised_funding_accrued,
                    pnl: SignedUint::zero(),
                    ..Default::default()
                },
                realised_pnl: PnlAmounts::default(),
                closing_fee_rate,
            }],
            perp_vault: None,
        },
        oracle_prices,
        asset_params,
        vaults_data,
        perps_data,
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(1300000000));
    assert_eq!(health.total_debt_value, Uint128::new(1040000000));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(1170000000));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(1235000000));

    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("1.038961274509803921").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("1.096687009803921568").unwrap())
    );
    assert!(!health.is_above_max_ltv());
    assert!(!health.is_liquidatable());
}

#[test]
fn perps_two_short_positive_pnl_one_long_negative_pnl_with_spot_debt() {
    let uusd = uusdc_info();
    let btcperp = btcperp_info();
    let ethperp = ethperp_info();
    let atomperp = atomperp_info();
    let uluna = uluna_info();

    let eth_price_change = SignedDecimal::from_str("-0.035").unwrap();
    let btc_price_change = SignedDecimal::from_str("-0.02").unwrap();
    let atom_price_change = SignedDecimal::from_str("-0.08").unwrap();

    let entry_price_btc = SignedDecimal::from_str("100").unwrap();
    let current_price_btc = entry_price_btc
        .checked_mul(SignedDecimal::from_str("1").unwrap().checked_add(btc_price_change).unwrap())
        .unwrap();
    let entry_price_eth = SignedDecimal::from_str("10").unwrap();
    let current_price_eth = entry_price_eth
        .checked_mul(SignedDecimal::from_str("1").unwrap().checked_add(eth_price_change).unwrap())
        .unwrap();
    let entry_price_atom = SignedDecimal::from_str("5").unwrap();
    let current_price_atom = entry_price_atom
        .checked_mul(SignedDecimal::from_str("1").unwrap().checked_add(atom_price_change).unwrap())
        .unwrap();
    let price_luna = SignedDecimal::from_str("1").unwrap();

    let btc_size = SignedUint::from_str("-5000000").unwrap();
    let eth_size = SignedUint::from_str("50000000").unwrap();
    let atom_size = SignedUint::from_str("-100000000").unwrap();

    let notional_eth_entry = eth_size.checked_mul_floor(entry_price_eth).unwrap();
    let notional_btc_entry = btc_size.checked_mul_floor(entry_price_btc).unwrap();
    let notional_atom_entry = atom_size.checked_mul_floor(entry_price_atom).unwrap();

    let notional_eth_current = eth_size.checked_mul_floor(current_price_eth).unwrap();
    let notional_btc_current = btc_size.checked_mul_floor(current_price_btc).unwrap();
    let notional_atom_current = atom_size.checked_mul_floor(current_price_atom).unwrap();

    let raw_pnl_eth = notional_eth_current.checked_sub(notional_eth_entry).unwrap();
    let raw_pnl_btc = notional_btc_current.checked_sub(notional_btc_entry).unwrap();
    let raw_pnl_atom = notional_atom_current.checked_sub(notional_atom_entry).unwrap();

    let asset_params = HashMap::from([
        (uusd.denom.clone(), uusd.params.clone()),
        (uluna.denom.clone(), uluna.params.clone()),
    ]);

    let oracle_prices = HashMap::from([
        (uusd.denom.clone(), uusd.price),
        (btcperp.denom.clone(), Decimal::from_str(current_price_btc.to_string().as_str()).unwrap()),
        (ethperp.denom.clone(), Decimal::from_str(current_price_eth.to_string().as_str()).unwrap()),
        (
            atomperp.denom.clone(),
            Decimal::from_str(current_price_atom.to_string().as_str()).unwrap(),
        ),
        (uluna.denom.clone(), Decimal::from_str(price_luna.to_string().as_str()).unwrap()),
    ]);
    let vaults_data = Default::default();
    let default_perp_denom_state = PerpDenomState {
        enabled: true,
        ..PerpDenomState::default()
    };
    let perps_data = PerpsData {
        denom_states: HashMap::from([
            (btcperp.denom.clone(), default_perp_denom_state.clone()),
            (ethperp.denom.clone(), default_perp_denom_state.clone()),
            (atomperp.denom.clone(), default_perp_denom_state.clone()),
        ]),
        params: HashMap::from([
            (btcperp.denom.clone(), btcperp.perp_params.clone()),
            (ethperp.denom.clone(), ethperp.perp_params.clone()),
            (atomperp.denom.clone(), atomperp.perp_params.clone()),
        ]),
    };

    let unrealised_funding_accrued = SignedUint::from_str("1234").unwrap();
    let base_denom = uusd.denom.clone();
    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(300000000, &uusd.denom)],
            debts: vec![DebtAmount {
                denom: uluna.denom,
                amount: Uint128::new(100000000),
                shares: Uint128::new(100),
            }],
            lends: vec![],
            vaults: vec![],
            perps: vec![
                PerpPosition {
                    denom: btcperp.denom,
                    base_denom: base_denom.clone(),
                    current_price: current_price_btc.abs,
                    entry_price: entry_price_btc.abs,
                    entry_exec_price: entry_price_btc.abs,
                    current_exec_price: current_price_btc.abs,
                    size: btc_size,
                    unrealised_pnl: PnlAmounts {
                        accrued_funding: unrealised_funding_accrued,
                        pnl: raw_pnl_btc,
                        ..Default::default()
                    },
                    realised_pnl: PnlAmounts::default(),
                    closing_fee_rate: Decimal::from_str("0.000").unwrap(),
                },
                PerpPosition {
                    denom: ethperp.denom,
                    base_denom: base_denom.clone(),
                    current_price: current_price_eth.abs,
                    entry_price: entry_price_eth.abs,
                    entry_exec_price: entry_price_eth.abs,
                    current_exec_price: current_price_eth.abs,
                    size: eth_size,
                    unrealised_pnl: PnlAmounts {
                        accrued_funding: unrealised_funding_accrued,
                        pnl: raw_pnl_eth,
                        ..Default::default()
                    },
                    realised_pnl: PnlAmounts::default(),
                    closing_fee_rate: Decimal::from_str("0.0002").unwrap(),
                },
                PerpPosition {
                    denom: atomperp.denom,
                    base_denom: uusd.denom.clone(),
                    current_price: current_price_atom.abs,
                    entry_price: entry_price_atom.abs,
                    entry_exec_price: entry_price_atom.abs,
                    current_exec_price: current_price_atom.abs,
                    size: atom_size,
                    unrealised_pnl: PnlAmounts {
                        accrued_funding: unrealised_funding_accrued,
                        pnl: raw_pnl_atom,
                        ..Default::default()
                    },
                    realised_pnl: PnlAmounts::default(),
                    closing_fee_rate: Decimal::from_str("0.0002").unwrap(),
                },
            ],
            perp_vault: None,
        },
        oracle_prices,
        asset_params,
        vaults_data,
        perps_data,
    };

    let health = h.compute_health().unwrap();

    assert_eq!(health.total_collateral_value, Uint128::new(300000000));
    assert_eq!(health.total_debt_value, Uint128::new(100000000));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(270000000));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(285000000));

    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_str("0.993459746719870947").unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_str("1.045975531640455782").unwrap())
    );
    assert!(health.is_above_max_ltv());
    assert!(!health.is_liquidatable());
}

// DOC - Long perp, funding greater than pnl
#[test]
fn single_perp_funding_greater_than_pnl() {
    let uusd = uusdc_info();
    let entry_price = Decimal::from_str("100").unwrap();
    let current_price = Decimal::from_str("92").unwrap();
    let max_ltv = Decimal::from_str("0.9").unwrap();
    let liquidation_threshold = Decimal::from_str("0.95").unwrap();
    let size = SignedUint::from_str("10000000").unwrap();
    let btcperp =
        create_perp_info("btc/usd/perp".to_string(), current_price, max_ltv, liquidation_threshold);

    let asset_params = HashMap::from([(uusd.denom.clone(), uusd.params.clone())]);

    let oracle_prices =
        HashMap::from([(uusd.denom.clone(), uusd.price), (btcperp.denom.clone(), btcperp.price)]);
    let default_perp_denom_state = PerpDenomState {
        enabled: true,
        ..PerpDenomState::default()
    };
    let perps_data = PerpsData {
        denom_states: HashMap::from([(btcperp.denom.clone(), default_perp_denom_state.clone())]),
        params: HashMap::from([(btcperp.denom.clone(), btcperp.perp_params.clone())]),
    };

    let vaults_data = Default::default();
    let closing_fee_rate = Decimal::from_str("0.0002").unwrap();
    let unrealised_funding_accrued = SignedUint::from_str("1225210000").unwrap();
    let h = HealthComputer {
        kind: AccountKind::Default,
        positions: Positions {
            account_id: "123".to_string(),
            deposits: vec![coin(152000000, &uusd.denom)],
            debts: vec![],
            lends: vec![],
            vaults: vec![],
            perps: vec![PerpPosition {
                denom: btcperp.denom,
                base_denom: uusd.denom,
                current_price,
                entry_price,
                entry_exec_price: entry_price,
                current_exec_price: current_price,
                size,
                unrealised_pnl: PnlAmounts {
                    accrued_funding: unrealised_funding_accrued,
                    pnl: SignedUint::from_str("-24790000").unwrap(),
                    ..Default::default()
                },
                realised_pnl: PnlAmounts::default(),
                closing_fee_rate,
            }],
            perp_vault: None,
        },
        oracle_prices,
        asset_params,
        vaults_data,
        perps_data,
    };

    let health = h.compute_health().unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(152000000));
    assert_eq!(health.total_debt_value, Uint128::new(0));
    assert_eq!(health.max_ltv_adjusted_collateral, Uint128::new(136800000));
    assert_eq!(health.liquidation_threshold_adjusted_collateral, Uint128::new(144400000));

    assert_eq!(health.max_ltv_health_factor, Some(Decimal::from_str("2.067305").unwrap()));
    assert_eq!(health.liquidation_health_factor, Some(Decimal::from_str("2.1821655").unwrap()));
    assert!(!health.is_above_max_ltv());
    assert!(!health.is_liquidatable());
}
