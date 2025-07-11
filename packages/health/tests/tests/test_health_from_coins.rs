use std::vec;

use cosmwasm_std::{
    coin, coins, testing::MockQuerier, Addr, CheckedMultiplyRatioError, Decimal, QuerierWrapper,
    Uint128,
};
use mars_health::{error::HealthError, health::Health};
use mars_testing::MarsMockQuerier;
use mars_types::{
    params::{AssetParams, CmSettings, LiquidationBonus, RedBankSettings},
    red_bank::{InterestRateModel, Market},
};

#[test]
fn health_success_from_coins() {
    let mut mock_querier = MarsMockQuerier::new(MockQuerier::new(&[]));

    // Set Markets
    let osmo_market = Market {
        denom: "osmo".to_string(),
        ..Default::default()
    };
    mock_querier.set_redbank_market(osmo_market);
    mock_querier.set_redbank_params(
        "osmo",
        AssetParams {
            denom: "osmo".to_string(),
            credit_manager: CmSettings {
                whitelisted: false,
                withdraw_enabled: true,
                hls: None,
            },
            red_bank: RedBankSettings {
                deposit_enabled: true,
                withdraw_enabled: true,
                borrow_enabled: true,
            },
            max_loan_to_value: Decimal::from_atomics(50u128, 2).unwrap(),
            liquidation_threshold: Decimal::from_atomics(55u128, 2).unwrap(),
            liquidation_bonus: LiquidationBonus {
                starting_lb: Decimal::percent(0u64),
                slope: Decimal::one(),
                min_lb: Decimal::percent(0u64),
                max_lb: Decimal::percent(5u64),
            },
            protocol_liquidation_fee: Decimal::zero(),
            deposit_cap: Uint128::MAX,
            close_factor: Decimal::percent(80u64),
            reserve_factor: Decimal::percent(10u64),
            interest_rate_model: InterestRateModel {
                optimal_utilization_rate: Decimal::percent(80u64),
                base: Decimal::zero(),
                slope_1: Decimal::percent(7u64),
                slope_2: Decimal::percent(45u64),
            },
        },
    );
    let atom_market = Market {
        denom: "atom".to_string(),
        ..Default::default()
    };
    mock_querier.set_redbank_market(atom_market);
    mock_querier.set_redbank_params(
        "atom",
        AssetParams {
            denom: "atom".to_string(),
            credit_manager: CmSettings {
                whitelisted: false,
                withdraw_enabled: true,
                hls: None,
            },
            red_bank: RedBankSettings {
                withdraw_enabled: true,
                deposit_enabled: true,
                borrow_enabled: true,
            },
            max_loan_to_value: Decimal::from_atomics(70u128, 2).unwrap(),
            liquidation_threshold: Decimal::from_atomics(75u128, 2).unwrap(),
            liquidation_bonus: LiquidationBonus {
                starting_lb: Decimal::percent(0u64),
                slope: Decimal::one(),
                min_lb: Decimal::percent(0u64),
                max_lb: Decimal::percent(5u64),
            },
            protocol_liquidation_fee: Decimal::zero(),
            deposit_cap: Uint128::MAX,
            close_factor: Decimal::percent(80u64),
            reserve_factor: Decimal::percent(10u64),
            interest_rate_model: InterestRateModel {
                optimal_utilization_rate: Decimal::percent(80u64),
                base: Decimal::zero(),
                slope_1: Decimal::percent(7u64),
                slope_2: Decimal::percent(45u64),
            },
        },
    );

    // Set prices in the oracle
    mock_querier.set_oracle_price("osmo", Decimal::from_atomics(23654u128, 4).unwrap());
    mock_querier.set_oracle_price("atom", Decimal::from_atomics(102u128, 1).unwrap());

    let oracle_addr = Addr::unchecked("oracle");
    let red_bank_addr = Addr::unchecked("red_bank");

    let querier_wrapper = QuerierWrapper::new(&mock_querier);

    let collateral = vec![coin(500, "osmo"), coin(200, "atom"), coin(0, "osmo")];
    let debt = vec![coin(200, "atom"), coin(150, "atom"), coin(115, "osmo")];
    let health = Health::compute_health_from_coins(
        &querier_wrapper,
        &oracle_addr,
        &red_bank_addr,
        &collateral,
        &debt,
    )
    .unwrap();
    assert_eq!(health.total_collateral_value, Uint128::new(3222));
    assert_eq!(health.total_debt_value, Uint128::new(3842));
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_atomics(525507548152004164u128, 18).unwrap())
    );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_atomics(567412805830296720u128, 18).unwrap())
    );
    assert!(health.is_liquidatable());
    assert!(health.is_above_max_ltv());
}

#[test]
fn health_error_from_coins() {
    let mut mock_querier = MarsMockQuerier::new(MockQuerier::new(&[]));

    // Set Markets
    let osmo_market = Market {
        denom: "osmo".to_string(),
        ..Default::default()
    };
    mock_querier.set_redbank_market(osmo_market);
    mock_querier.set_redbank_params(
        "osmo",
        AssetParams {
            denom: "osmo".to_string(),
            credit_manager: CmSettings {
                whitelisted: false,
                withdraw_enabled: true,
                hls: None,
            },
            red_bank: RedBankSettings {
                deposit_enabled: false,
                withdraw_enabled: true,
                borrow_enabled: false,
            },
            max_loan_to_value: Decimal::from_atomics(50u128, 2).unwrap(),
            liquidation_threshold: Decimal::from_atomics(55u128, 2).unwrap(),
            liquidation_bonus: LiquidationBonus {
                starting_lb: Decimal::percent(0u64),
                slope: Decimal::one(),
                min_lb: Decimal::percent(0u64),
                max_lb: Decimal::percent(5u64),
            },
            protocol_liquidation_fee: Decimal::zero(),
            deposit_cap: Default::default(),
            close_factor: Decimal::percent(80u64),
            reserve_factor: Decimal::percent(10u64),
            interest_rate_model: InterestRateModel {
                optimal_utilization_rate: Decimal::percent(80u64),
                base: Decimal::zero(),
                slope_1: Decimal::percent(7u64),
                slope_2: Decimal::percent(45u64),
            },
        },
    );

    // Set prices in the oracle
    mock_querier.set_oracle_price("osmo", Decimal::MAX);

    let oracle_addr = Addr::unchecked("oracle");
    let red_bank_addr = Addr::unchecked("red_bank");

    let querier_wrapper = QuerierWrapper::new(&mock_querier);

    let collateral = coins(u128::MAX, "osmo");
    let res_err = Health::compute_health_from_coins(
        &querier_wrapper,
        &oracle_addr,
        &red_bank_addr,
        &collateral,
        &[],
    )
    .unwrap_err();
    assert_eq!(res_err, HealthError::CheckedMultiplyRatio(CheckedMultiplyRatioError::Overflow));
}
