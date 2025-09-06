use cosmwasm_std::{Addr, Decimal, Uint128};
use mars_types::{
    credit_manager::{MarketType, TradingFeeResponse},
    params::PerpParamsUpdate,
};
use test_case::test_case;

use super::helpers::{default_perp_params, uosmo_info, MockEnv};

#[test_case(
    Uint128::new(100_000_000_000),
    "tier_4",
    Decimal::percent(30),
    Decimal::percent(1);
    "spot market tier 4: 30% discount on 1% base fee"
)]
#[test_case(
    Uint128::new(50_000_000_000),
    "tier_3",
    Decimal::percent(20),
    Decimal::percent(1);
    "spot market tier 3: 20% discount on 1% base fee"
)]
#[test_case(
    Uint128::new(10_000_000_000),
    "tier_2",
    Decimal::percent(10),
    Decimal::percent(1);
    "spot market tier 2: 10% discount on 1% base fee"
)]
fn test_trading_fee_query_spot(
    voting_power: Uint128,
    expected_tier_id: &str,
    expected_discount: Decimal,
    expected_base_fee: Decimal,
) {
    let mut mock =
        MockEnv::new().set_params(&[uosmo_info()]).swap_fee(Decimal::percent(1)).build().unwrap();

    // Create a credit account
    let user = Addr::unchecked("user");
    let account_id = mock.create_credit_account(&user).unwrap();

    // Set voting power for the specified tier
    mock.set_voting_power(&user, voting_power);

    // Query trading fee for spot market
    let response: TradingFeeResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            mock.rover.clone(),
            &mars_types::credit_manager::QueryMsg::TradingFee {
                account_id: account_id.clone(),
                market_type: MarketType::Spot,
            },
        )
        .unwrap();

    // Verify the response
    assert_eq!(response.base_fee_pct, expected_base_fee);
    assert_eq!(response.discount_pct, expected_discount);

    // Calculate the expected effective fee based on the actual response
    let calculated_effective =
        response.base_fee_pct.checked_mul(Decimal::one() - expected_discount).unwrap();
    assert_eq!(response.effective_fee_pct, calculated_effective);
    assert_eq!(response.tier_id, expected_tier_id);
}

#[test_case(
    Uint128::new(250_000_000_000),
    "tier_5",
    Decimal::percent(45),
    "uosmo";
    "perp market tier 5: 45% discount on uosmo"
)]
#[test_case(
    Uint128::new(500_000_000_000),
    "tier_6",
    Decimal::percent(60),
    "uosmo";
    "perp market tier 6: 60% discount on uosmo"
)]
#[test_case(
    Uint128::new(1_000_000_000_000),
    "tier_7",
    Decimal::percent(70),
    "uosmo";
    "perp market tier 7: 70% discount on uosmo"
)]
fn test_trading_fee_query_perp(
    voting_power: Uint128,
    expected_tier_id: &str,
    expected_discount: Decimal,
    denom: &str,
) {
    let mut mock =
        MockEnv::new().set_params(&[uosmo_info()]).swap_fee(Decimal::percent(1)).build().unwrap();

    // Create a credit account
    let user = Addr::unchecked("user");
    let account_id = mock.create_credit_account(&user).unwrap();

    // Set voting power for the specified tier
    mock.set_voting_power(&user, voting_power);

    // Set up perp params for the specified denom
    mock.update_perp_params(PerpParamsUpdate::AddOrUpdate {
        params: default_perp_params(denom),
    });

    // Query trading fee for perp market
    let response: TradingFeeResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            mock.rover.clone(),
            &mars_types::credit_manager::QueryMsg::TradingFee {
                account_id: account_id.clone(),
                market_type: MarketType::Perp {
                    denom: denom.to_string(),
                },
            },
        )
        .unwrap();

    // Verify the response
    assert_eq!(response.discount_pct, expected_discount);
    assert_eq!(response.tier_id, expected_tier_id);

    // The effective fee should be base_fee * (1 - discount)
    let expected_effective =
        response.base_fee_pct.checked_mul(Decimal::one() - expected_discount).unwrap();
    assert_eq!(response.effective_fee_pct, expected_effective);
}

#[test]
fn test_trading_fee_query_edge_cases() {
    let mut mock =
        MockEnv::new().set_params(&[uosmo_info()]).swap_fee(Decimal::percent(1)).build().unwrap();

    // Create a credit account
    let user = Addr::unchecked("user");
    let account_id = mock.create_credit_account(&user).unwrap();

    // Test tier 8 (highest discount - 80%)
    mock.set_voting_power(&user, Uint128::new(1_500_000_000_000));

    let response: TradingFeeResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            mock.rover.clone(),
            &mars_types::credit_manager::QueryMsg::TradingFee {
                account_id: account_id.clone(),
                market_type: MarketType::Spot,
            },
        )
        .unwrap();

    assert_eq!(response.tier_id, "tier_8");
    assert_eq!(response.discount_pct, Decimal::percent(80));

    // Calculate the expected effective fee based on the actual response
    let calculated_effective =
        response.base_fee_pct.checked_mul(Decimal::one() - Decimal::percent(80)).unwrap();
    assert_eq!(response.effective_fee_pct, calculated_effective);

    // Test tier 1 (no discount - 0%)
    mock.set_voting_power(&user, Uint128::new(0));

    let response: TradingFeeResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            mock.rover.clone(),
            &mars_types::credit_manager::QueryMsg::TradingFee {
                account_id: account_id.clone(),
                market_type: MarketType::Spot,
            },
        )
        .unwrap();

    assert_eq!(response.tier_id, "tier_1");
    assert_eq!(response.discount_pct, Decimal::percent(0));

    // Calculate the expected effective fee based on the actual response
    let calculated_effective =
        response.base_fee_pct.checked_mul(Decimal::one() - Decimal::percent(0)).unwrap();
    assert_eq!(response.effective_fee_pct, calculated_effective);
}
