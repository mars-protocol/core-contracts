use cosmwasm_std::{Coin, Uint128, Int128, Decimal};
use mars_active_delta_neutral::helpers::calculate_deltas;
use mars_types::{
    active_delta_neutral::query::MarketConfig,
    credit_manager::{Positions, DebtAmount},
    health::AccountKind,
};
use mars_delta_neutral_position::types::Position;

#[test]
fn test_calculate_deltas_basic() {
    let mars_positions = Positions {
        account_id: "acct1".to_string(),
        account_kind: AccountKind::Default,
        deposits: vec![Coin { denom: "ATOM".to_string(), amount: Uint128::new(1000) }],
        debts: vec![DebtAmount { denom: "USDC".to_string(), amount: Uint128::new(500), shares: Uint128::zero() }],
        lends: vec![],
        vaults: vec![],
        staked_astro_lps: vec![],
        perps: vec![],
    };
    let market_config = MarketConfig {
        spot_denom: "ATOM".to_string(),
        usdc_denom: "USDC".to_string(),
        perp_denom: "perps/ATOM".to_string(),
        k: 100,
        market_id: "atom".to_string(),
    };
    let position_state = Position::default();
    let result = calculate_deltas(&mars_positions, &market_config, &position_state).unwrap();
    assert_eq!(result.spot_delta, Int128::new(1000));
    assert_eq!(result.borrow_delta, Uint128::new(500));
    assert_eq!(result.funding_delta, Int128::zero());
}

#[test]
fn test_calculate_deltas_missing_debt() {
    let mars_positions = Positions {
        account_id: "acct2".to_string(),
        account_kind: AccountKind::Default,
        deposits: vec![Coin { denom: "ATOM".to_string(), amount: Uint128::new(1000) }],
        debts: vec![],
        lends: vec![],
        vaults: vec![],
        staked_astro_lps: vec![],
        perps: vec![],
    };
    let market_config = MarketConfig {
        spot_denom: "ATOM".to_string(),
        usdc_denom: "USDC".to_string(),
        perp_denom: "perps/ATOM".to_string(),
        k: 100,
        market_id: "atom".to_string(),
    };
    let position_state = Position::default();
    let result = calculate_deltas(&mars_positions, &market_config, &position_state).unwrap();
    assert_eq!(result.borrow_delta, Uint128::zero());
    assert_eq!(result.spot_delta, Int128::zero());
    assert_eq!(result.funding_delta, Int128::zero());
}

#[test]
fn test_calculate_deltas_empty_positions() {
    let mars_positions = Positions {
        account_id: "acct3".to_string(),
        account_kind: AccountKind::Default,
        deposits: vec![],
        debts: vec![],
        lends: vec![],
        vaults: vec![],
        staked_astro_lps: vec![],
        perps: vec![],
    };
    let market_config = MarketConfig {
        spot_denom: "ATOM".to_string(),
        usdc_denom: "USDC".to_string(),
        perp_denom: "perps/ATOM".to_string(),
        k: 100,
        market_id: "atom".to_string(),
    };
    let position_state = Position::default();
    let result = calculate_deltas(&mars_positions, &market_config, &position_state).unwrap();
    assert_eq!(result.funding_delta, Int128::zero());
    assert_eq!(result.borrow_delta, Uint128::zero());
    assert_eq!(result.spot_delta, Int128::zero());
}
