use cosmwasm_std::{Decimal, StdError, Uint128};
use mars_credit_manager::staking::StakingTierManager;
use mars_types::fee_tiers::{FeeTier, FeeTierConfig};
use test_case::test_case;

// Test data based on the tier breakdown provided
fn create_test_fee_tier_config() -> FeeTierConfig {
    FeeTierConfig {
        tiers: vec![
            FeeTier {
                id: "tier_1".to_string(),
                min_voting_power: "350000".to_string(),
                discount_pct: Decimal::percent(75),
            },
            FeeTier {
                id: "tier_2".to_string(),
                min_voting_power: "200000".to_string(),
                discount_pct: Decimal::percent(60),
            },
            FeeTier {
                id: "tier_3".to_string(),
                min_voting_power: "100000".to_string(),
                discount_pct: Decimal::percent(45),
            },
            FeeTier {
                id: "tier_4".to_string(),
                min_voting_power: "50000".to_string(),
                discount_pct: Decimal::percent(35),
            },
            FeeTier {
                id: "tier_5".to_string(),
                min_voting_power: "25000".to_string(),
                discount_pct: Decimal::percent(25),
            },
            FeeTier {
                id: "tier_6".to_string(),
                min_voting_power: "10000".to_string(),
                discount_pct: Decimal::percent(15),
            },
            FeeTier {
                id: "tier_7".to_string(),
                min_voting_power: "5000".to_string(),
                discount_pct: Decimal::percent(10),
            },
            FeeTier {
                id: "tier_8".to_string(),
                min_voting_power: "1000".to_string(),
                discount_pct: Decimal::percent(5),
            },
            FeeTier {
                id: "tier_9".to_string(),
                min_voting_power: "100".to_string(),
                discount_pct: Decimal::percent(1),
            },
            FeeTier {
                id: "tier_10".to_string(),
                min_voting_power: "0".to_string(),
                discount_pct: Decimal::percent(0),
            },
        ],
    }
}

#[test]
fn test_staking_tier_manager_creation() {
    let config = create_test_fee_tier_config();
    let manager = StakingTierManager::new(config);

    assert_eq!(manager.config.tiers.len(), 10);
    assert_eq!(manager.config.tiers[0].id, "tier_1");
    assert_eq!(manager.config.tiers[9].id, "tier_10");
}

#[test_case(
    Uint128::new(350000),
    "tier_1",
    Decimal::percent(75);
    "exact match tier 1"
)]
#[test_case(
    Uint128::new(200000),
    "tier_2",
    Decimal::percent(60);
    "exact match tier 2"
)]
#[test_case(
    Uint128::new(100000),
    "tier_3",
    Decimal::percent(45);
    "exact match tier 3"
)]
#[test_case(
    Uint128::new(50000),
    "tier_4",
    Decimal::percent(35);
    "exact match tier 4"
)]
#[test_case(
    Uint128::new(25000),
    "tier_5",
    Decimal::percent(25);
    "exact match tier 5"
)]
#[test_case(
    Uint128::new(10000),
    "tier_6",
    Decimal::percent(15);
    "exact match tier 6"
)]
#[test_case(
    Uint128::new(5000),
    "tier_7",
    Decimal::percent(10);
    "exact match tier 7"
)]
#[test_case(
    Uint128::new(1000),
    "tier_8",
    Decimal::percent(5);
    "exact match tier 8"
)]
#[test_case(
    Uint128::new(100),
    "tier_9",
    Decimal::percent(1);
    "exact match tier 9"
)]
#[test_case(
    Uint128::new(0),
    "tier_10",
    Decimal::percent(0);
    "exact match tier 10"
)]
fn test_find_applicable_tier_exact_matches(
    voting_power: Uint128,
    expected_tier_id: &str,
    expected_discount: Decimal,
) {
    let config = create_test_fee_tier_config();
    let manager = StakingTierManager::new(config);

    let tier = manager.find_applicable_tier(voting_power).unwrap();
    assert_eq!(tier.id, expected_tier_id);
    assert_eq!(tier.discount_pct, expected_discount);
}

#[test_case(
    Uint128::new(300000),
    "tier_2",
    Decimal::percent(60);
    "between tier 1 and tier 2"
)]
#[test_case(
    Uint128::new(150000),
    "tier_3",
    Decimal::percent(45);
    "between tier 2 and tier 3"
)]
#[test_case(
    Uint128::new(75000),
    "tier_4",
    Decimal::percent(35);
    "between tier 3 and tier 4"
)]
#[test_case(
    Uint128::new(30000),
    "tier_5",
    Decimal::percent(25);
    "between tier 4 and tier 5"
)]
#[test_case(
    Uint128::new(15000),
    "tier_6",
    Decimal::percent(15);
    "between tier 5 and tier 6"
)]
#[test_case(
    Uint128::new(7500),
    "tier_7",
    Decimal::percent(10);
    "between tier 6 and tier 7"
)]
#[test_case(
    Uint128::new(1500),
    "tier_8",
    Decimal::percent(5);
    "between tier 7 and tier 8"
)]
#[test_case(
    Uint128::new(500),
    "tier_9",
    Decimal::percent(1);
    "between tier 8 and tier 9"
)]
#[test_case(
    Uint128::new(50),
    "tier_10",
    Decimal::percent(0);
    "between tier 9 and tier 10"
)]
fn test_find_applicable_tier_between_thresholds(
    voting_power: Uint128,
    expected_tier_id: &str,
    expected_discount: Decimal,
) {
    let config = create_test_fee_tier_config();
    let manager = StakingTierManager::new(config);

    let tier = manager.find_applicable_tier(voting_power).unwrap();
    assert_eq!(tier.id, expected_tier_id);
    assert_eq!(tier.discount_pct, expected_discount);
}

#[test_case(
    Uint128::new(500000),
    "tier_1",
    Decimal::percent(75);
    "above highest tier threshold"
)]
#[test_case(
    Uint128::new(1000000),
    "tier_1",
    Decimal::percent(75);
    "well above highest tier threshold"
)]
fn test_find_applicable_tier_above_highest(
    voting_power: Uint128,
    expected_tier_id: &str,
    expected_discount: Decimal,
) {
    let config = create_test_fee_tier_config();
    let manager = StakingTierManager::new(config);

    let tier = manager.find_applicable_tier(voting_power).unwrap();
    assert_eq!(tier.id, expected_tier_id);
    assert_eq!(tier.discount_pct, expected_discount);
}

#[test_case(
    Uint128::new(1),
    "tier_10",
    Decimal::percent(0);
    "edge case: minimal voting power"
)]
#[test_case(
    Uint128::new(99),
    "tier_10",
    Decimal::percent(0);
    "edge case: just below tier 9"
)]
#[test_case(
    Uint128::new(101),
    "tier_9",
    Decimal::percent(1);
    "edge case: just above tier 10"
)]
fn test_find_applicable_tier_edge_cases(
    voting_power: Uint128,
    expected_tier_id: &str,
    expected_discount: Decimal,
) {
    let config = create_test_fee_tier_config();
    let manager = StakingTierManager::new(config);

    let tier = manager.find_applicable_tier(voting_power).unwrap();
    assert_eq!(tier.id, expected_tier_id);
    assert_eq!(tier.discount_pct, expected_discount);
}

#[test]
fn test_validate_fee_tier_config_valid() {
    let config = create_test_fee_tier_config();
    let manager = StakingTierManager::new(config);

    // Should not panic for valid config
    let result = manager.validate();
    assert!(result.is_ok());
}

#[test]
fn test_validate_fee_tier_config_empty() {
    let config = FeeTierConfig {
        tiers: vec![],
    };
    let manager = StakingTierManager::new(config);

    let result = manager.validate();
    assert!(result.is_err());
    match result.unwrap_err() {
        StdError::GenericErr { msg } => {
            assert!(msg.contains("Fee tier config cannot be empty"));
        }
        _ => panic!("Expected StdError::GenericErr"),
    }
}

#[test]
fn test_validate_fee_tier_config_unsorted() {
    let config = FeeTierConfig {
        tiers: vec![
            FeeTier {
                id: "tier_2".to_string(),
                min_voting_power: "200000".to_string(),
                discount_pct: Decimal::percent(60),
            },
            FeeTier {
                id: "tier_1".to_string(),
                min_voting_power: "350000".to_string(),
                discount_pct: Decimal::percent(75),
            },
        ],
    };
    let manager = StakingTierManager::new(config);

    let result = manager.validate();
    assert!(result.is_err());
    match result.unwrap_err() {
        StdError::GenericErr { msg } => {
            assert!(msg.contains("Tiers must be sorted in descending order"));
        }
        _ => panic!("Expected StdError::GenericErr"),
    }
}

#[test]
fn test_validate_fee_tier_config_duplicate_thresholds() {
    let config = FeeTierConfig {
        tiers: vec![
            FeeTier {
                id: "tier_1".to_string(),
                min_voting_power: "350000".to_string(),
                discount_pct: Decimal::percent(75),
            },
            FeeTier {
                id: "tier_2".to_string(),
                min_voting_power: "350000".to_string(),
                discount_pct: Decimal::percent(60),
            },
        ],
    };
    let manager = StakingTierManager::new(config);

    let result = manager.validate();
    assert!(result.is_err());
    match result.unwrap_err() {
        StdError::GenericErr { msg } => {
            assert!(msg.contains("Duplicate voting power thresholds"));
        }
        _ => panic!("Expected StdError::GenericErr"),
    }
}

#[test]
fn test_validate_fee_tier_config_invalid_discount() {
    let config = FeeTierConfig {
        tiers: vec![FeeTier {
            id: "tier_1".to_string(),
            min_voting_power: "350000".to_string(),
            discount_pct: Decimal::percent(100), // 100% discount (invalid)
        }],
    };
    let manager = StakingTierManager::new(config);

    let result = manager.validate();
    assert!(result.is_err());
    match result.unwrap_err() {
        StdError::GenericErr { msg } => {
            assert!(msg.contains("Discount percentage must be less than 100%"));
        }
        _ => panic!("Expected StdError::GenericErr"),
    }
}

#[test]
fn test_get_default_tier() {
    let config = create_test_fee_tier_config();
    let manager = StakingTierManager::new(config);

    let default_tier = manager.get_default_tier().unwrap();
    assert_eq!(default_tier.id, "tier_10");
    assert_eq!(default_tier.discount_pct, Decimal::percent(0));
}

#[test_case(
    Uint128::new(400000),
    Decimal::percent(75);
    "tier 1: highest discount"
)]
#[test_case(
    Uint128::new(250000),
    Decimal::percent(60);
    "tier 2: high discount"
)]
#[test_case(
    Uint128::new(120000),
    Decimal::percent(45);
    "tier 3: medium-high discount"
)]
#[test_case(
    Uint128::new(60000),
    Decimal::percent(35);
    "tier 4: medium discount"
)]
#[test_case(
    Uint128::new(30000),
    Decimal::percent(25);
    "tier 5: medium-low discount"
)]
#[test_case(
    Uint128::new(12000),
    Decimal::percent(15);
    "tier 6: low discount"
)]
#[test_case(
    Uint128::new(6000),
    Decimal::percent(10);
    "tier 7: very low discount"
)]
#[test_case(
    Uint128::new(1500),
    Decimal::percent(5);
    "tier 8: minimal discount"
)]
#[test_case(
    Uint128::new(500),
    Decimal::percent(1);
    "tier 9: tiny discount"
)]
#[test_case(
    Uint128::new(50),
    Decimal::percent(0);
    "tier 10: no discount"
)]
fn test_discount_calculation_examples(
    voting_power: Uint128,
    expected_discount: Decimal,
) {
    let config = create_test_fee_tier_config();
    let manager = StakingTierManager::new(config);

    let tier = manager.find_applicable_tier(voting_power).unwrap();
    assert_eq!(
        tier.discount_pct,
        expected_discount,
        "Failed for voting power: {}",
        voting_power
    );
}

#[test]
fn test_fee_tier_config_with_single_tier() {
    let config = FeeTierConfig {
        tiers: vec![FeeTier {
            id: "single_tier".to_string(),
            min_voting_power: "0".to_string(),
            discount_pct: Decimal::percent(25),
        }],
    };
    let manager = StakingTierManager::new(config);

    // Should always return the single tier
    let tier = manager.find_applicable_tier(Uint128::new(1000)).unwrap();
    assert_eq!(tier.id, "single_tier");
    assert_eq!(tier.discount_pct, Decimal::percent(25));

    let tier = manager.find_applicable_tier(Uint128::new(0)).unwrap();
    assert_eq!(tier.id, "single_tier");
    assert_eq!(tier.discount_pct, Decimal::percent(25));
}

#[test]
fn test_fee_tier_config_with_two_tiers() {
    let config = FeeTierConfig {
        tiers: vec![
            FeeTier {
                id: "high_tier".to_string(),
                min_voting_power: "1000".to_string(),
                discount_pct: Decimal::percent(50),
            },
            FeeTier {
                id: "low_tier".to_string(),
                min_voting_power: "0".to_string(),
                discount_pct: Decimal::percent(10),
            },
        ],
    };
    let manager = StakingTierManager::new(config);

    // Test high tier
    let tier = manager.find_applicable_tier(Uint128::new(1500)).unwrap();
    assert_eq!(tier.id, "high_tier");
    assert_eq!(tier.discount_pct, Decimal::percent(50));

    // Test low tier
    let tier = manager.find_applicable_tier(Uint128::new(500)).unwrap();
    assert_eq!(tier.id, "low_tier");
    assert_eq!(tier.discount_pct, Decimal::percent(10));

    // Test boundary
    let tier = manager.find_applicable_tier(Uint128::new(1000)).unwrap();
    assert_eq!(tier.id, "high_tier");
    assert_eq!(tier.discount_pct, Decimal::percent(50));
}
