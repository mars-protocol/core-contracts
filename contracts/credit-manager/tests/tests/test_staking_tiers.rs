use cosmwasm_std::{Decimal, StdError, Uint128};
use mars_credit_manager::staking::StakingTierManager;
use mars_types::fee_tiers::{FeeTier, FeeTierConfig};
use test_case::test_case;

// Test data based on the tier breakdown provided
fn create_test_fee_tier_config() -> FeeTierConfig {
    FeeTierConfig {
        tiers: vec![
            FeeTier {
                id: "tier_8".to_string(),
                min_voting_power: "1500000000000".to_string(), // 1,500,000 MARS
                discount_pct: Decimal::percent(80),
            },
            FeeTier {
                id: "tier_7".to_string(),
                min_voting_power: "1000000000000".to_string(), // 1,000,000 MARS
                discount_pct: Decimal::percent(70),
            },
            FeeTier {
                id: "tier_6".to_string(),
                min_voting_power: "500000000000".to_string(), // 500,000 MARS
                discount_pct: Decimal::percent(60),
            },
            FeeTier {
                id: "tier_5".to_string(),
                min_voting_power: "250000000000".to_string(), // 250,000 MARS
                discount_pct: Decimal::percent(45),
            },
            FeeTier {
                id: "tier_4".to_string(),
                min_voting_power: "100000000000".to_string(), // 100,000 MARS
                discount_pct: Decimal::percent(30),
            },
            FeeTier {
                id: "tier_3".to_string(),
                min_voting_power: "50000000000".to_string(), // 50,000 MARS
                discount_pct: Decimal::percent(20),
            },
            FeeTier {
                id: "tier_2".to_string(),
                min_voting_power: "10000000000".to_string(), // 10,000 MARS
                discount_pct: Decimal::percent(10),
            },
            FeeTier {
                id: "tier_1".to_string(),
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

    assert_eq!(manager.config.tiers.len(), 8);
    assert_eq!(manager.config.tiers[0].id, "tier_8");
    assert_eq!(manager.config.tiers[7].id, "tier_1");
}

#[test_case(
    Uint128::new(1500000000000),
    "tier_8",
    Decimal::percent(80);
    "exact match tier 8"
)]
#[test_case(
    Uint128::new(1000000000000),
    "tier_7",
    Decimal::percent(70);
    "exact match tier 7"
)]
#[test_case(
    Uint128::new(500000000000),
    "tier_6",
    Decimal::percent(60);
    "exact match tier 6"
)]
#[test_case(
    Uint128::new(250000000000),
    "tier_5",
    Decimal::percent(45);
    "exact match tier 5"
)]
#[test_case(
    Uint128::new(100000000000),
    "tier_4",
    Decimal::percent(30);
    "exact match tier 4"
)]
#[test_case(
    Uint128::new(50000000000),
    "tier_3",
    Decimal::percent(20);
    "exact match tier 3"
)]
#[test_case(
    Uint128::new(10000000000),
    "tier_2",
    Decimal::percent(10);
    "exact match tier 2"
)]
#[test_case(
    Uint128::new(0),
    "tier_1",
    Decimal::percent(0);
    "exact match tier 1"
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
    Uint128::new(1200000000000),
    "tier_7",
    Decimal::percent(70);
    "between tier 6 and tier 7"
)]
#[test_case(
    Uint128::new(750000000000),
    "tier_6",
    Decimal::percent(60);
    "between tier 5 and tier 6"
)]
#[test_case(
    Uint128::new(300000000000),
    "tier_5",
    Decimal::percent(45);
    "between tier 4 and tier 5"
)]
#[test_case(
    Uint128::new(150000000000),
    "tier_4",
    Decimal::percent(30);
    "between tier 3 and tier 4"
)]
#[test_case(
    Uint128::new(75000000000),
    "tier_3",
    Decimal::percent(20);
    "between tier 2 and tier 3"
)]
#[test_case(
    Uint128::new(15000000000),
    "tier_2",
    Decimal::percent(10);
    "between tier 1 and tier 2"
)]
#[test_case(
    Uint128::new(5000000000),
    "tier_1",
    Decimal::percent(0);
    "between tier 0 and tier 1"
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
    Uint128::new(2000000000000),
    "tier_8",
    Decimal::percent(80);
    "above highest tier threshold"
)]
#[test_case(
    Uint128::new(3000000000000),
    "tier_8",
    Decimal::percent(80);
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
    "tier_1",
    Decimal::percent(0);
    "edge case: minimal voting power"
)]
#[test_case(
    Uint128::new(9999000000),
    "tier_1",
    Decimal::percent(0);
    "edge case: just below tier 2"
)]
#[test_case(
    Uint128::new(10001000000),
    "tier_2",
    Decimal::percent(10);
    "edge case: just above tier 1"
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
        StdError::GenericErr {
            msg,
        } => {
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
        StdError::GenericErr {
            msg,
        } => {
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
        StdError::GenericErr {
            msg,
        } => {
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
        StdError::GenericErr {
            msg,
        } => {
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
    assert_eq!(default_tier.id, "tier_1");
    assert_eq!(default_tier.discount_pct, Decimal::percent(0));
}

#[test_case(
    Uint128::new(1500000000000),
    Decimal::percent(80);
    "tier 8: highest discount"
)]
#[test_case(
    Uint128::new(1000000000000),
    Decimal::percent(70);
    "tier 7: high discount"
)]
#[test_case(
    Uint128::new(500000000000),
    Decimal::percent(60);
    "tier 6: medium-high discount"
)]
#[test_case(
    Uint128::new(250000000000),
    Decimal::percent(45);
    "tier 5: medium discount"
)]
#[test_case(
    Uint128::new(100000000000),
    Decimal::percent(30);
    "tier 4: medium-low discount"
)]
#[test_case(
    Uint128::new(50000000000),
    Decimal::percent(20);
    "tier 3: low discount"
)]
#[test_case(
    Uint128::new(10000000000),
    Decimal::percent(10);
    "tier 2: very low discount"
)]
#[test_case(
    Uint128::new(0),
    Decimal::percent(0);
    "tier 1: no discount"
)]
fn test_discount_calculation_examples(voting_power: Uint128, expected_discount: Decimal) {
    let config = create_test_fee_tier_config();
    let manager = StakingTierManager::new(config);

    let tier = manager.find_applicable_tier(voting_power).unwrap();
    assert_eq!(tier.discount_pct, expected_discount, "Failed for voting power: {}", voting_power);
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

// ===== VALIDATION TEST CASES =====

#[test]
fn test_validation_empty_tiers() {
    let config = FeeTierConfig {
        tiers: vec![],
    };
    let manager = StakingTierManager::new(config);

    let result = manager.validate();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), StdError::generic_err("Fee tier config cannot be empty"));
}

#[test]
fn test_validation_single_tier_valid() {
    let config = FeeTierConfig {
        tiers: vec![FeeTier {
            id: "single".to_string(),
            min_voting_power: "1000".to_string(),
            discount_pct: Decimal::percent(25),
        }],
    };
    let manager = StakingTierManager::new(config);

    let result = manager.validate();
    assert!(result.is_ok());
}

#[test]
fn test_validation_multiple_tiers_valid() {
    let config = create_test_fee_tier_config();
    let manager = StakingTierManager::new(config);

    let result = manager.validate();
    assert!(result.is_ok());
}

#[test]
fn test_validation_duplicate_voting_power() {
    let config = FeeTierConfig {
        tiers: vec![
            FeeTier {
                id: "tier_1".to_string(),
                min_voting_power: "1000".to_string(),
                discount_pct: Decimal::percent(50),
            },
            FeeTier {
                id: "tier_2".to_string(),
                min_voting_power: "1000".to_string(), // Duplicate!
                discount_pct: Decimal::percent(25),
            },
        ],
    };
    let manager = StakingTierManager::new(config);

    let result = manager.validate();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), StdError::generic_err("Duplicate voting power thresholds"));
}

#[test]
fn test_validation_not_descending_order() {
    let config = FeeTierConfig {
        tiers: vec![
            FeeTier {
                id: "tier_1".to_string(),
                min_voting_power: "1000".to_string(),
                discount_pct: Decimal::percent(50),
            },
            FeeTier {
                id: "tier_2".to_string(),
                min_voting_power: "2000".to_string(), // Higher than previous!
                discount_pct: Decimal::percent(25),
            },
        ],
    };
    let manager = StakingTierManager::new(config);

    let result = manager.validate();
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        StdError::generic_err("Tiers must be sorted in descending order")
    );
}

#[test]
fn test_validation_equal_voting_power() {
    let config = FeeTierConfig {
        tiers: vec![
            FeeTier {
                id: "tier_1".to_string(),
                min_voting_power: "1000".to_string(),
                discount_pct: Decimal::percent(50),
            },
            FeeTier {
                id: "tier_2".to_string(),
                min_voting_power: "1000".to_string(), // Equal to previous!
                discount_pct: Decimal::percent(25),
            },
        ],
    };
    let manager = StakingTierManager::new(config);

    let result = manager.validate();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), StdError::generic_err("Duplicate voting power thresholds"));
}

#[test]
fn test_validation_invalid_voting_power_format() {
    let config = FeeTierConfig {
        tiers: vec![FeeTier {
            id: "tier_1".to_string(),
            min_voting_power: "invalid_number".to_string(), // Invalid format!
            discount_pct: Decimal::percent(50),
        }],
    };
    let manager = StakingTierManager::new(config);

    let result = manager.validate();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), StdError::generic_err("Invalid min_voting_power in tier"));
}

#[test]
fn test_validation_discount_100_percent() {
    let config = FeeTierConfig {
        tiers: vec![FeeTier {
            id: "tier_1".to_string(),
            min_voting_power: "1000".to_string(),
            discount_pct: Decimal::one(), // 100% discount!
        }],
    };
    let manager = StakingTierManager::new(config);

    let result = manager.validate();
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        StdError::generic_err("Discount percentage must be less than 100%")
    );
}

#[test]
fn test_validation_discount_over_100_percent() {
    let config = FeeTierConfig {
        tiers: vec![FeeTier {
            id: "tier_1".to_string(),
            min_voting_power: "1000".to_string(),
            discount_pct: Decimal::percent(150), // 150% discount!
        }],
    };
    let manager = StakingTierManager::new(config);

    let result = manager.validate();
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        StdError::generic_err("Discount percentage must be less than 100%")
    );
}

#[test]
fn test_validation_discount_99_percent_valid() {
    let config = FeeTierConfig {
        tiers: vec![FeeTier {
            id: "tier_1".to_string(),
            min_voting_power: "1000".to_string(),
            discount_pct: Decimal::percent(99), // 99% discount - valid!
        }],
    };
    let manager = StakingTierManager::new(config);

    let result = manager.validate();
    assert!(result.is_ok());
}

#[test]
fn test_validation_zero_discount_valid() {
    let config = FeeTierConfig {
        tiers: vec![FeeTier {
            id: "tier_1".to_string(),
            min_voting_power: "1000".to_string(),
            discount_pct: Decimal::zero(), // 0% discount - valid!
        }],
    };
    let manager = StakingTierManager::new(config);

    let result = manager.validate();
    assert!(result.is_ok());
}

#[test]
fn test_validation_complex_scenario() {
    let config = FeeTierConfig {
        tiers: vec![
            FeeTier {
                id: "platinum".to_string(),
                min_voting_power: "1000000".to_string(),
                discount_pct: Decimal::percent(90),
            },
            FeeTier {
                id: "gold".to_string(),
                min_voting_power: "500000".to_string(),
                discount_pct: Decimal::percent(75),
            },
            FeeTier {
                id: "silver".to_string(),
                min_voting_power: "100000".to_string(),
                discount_pct: Decimal::percent(50),
            },
            FeeTier {
                id: "bronze".to_string(),
                min_voting_power: "10000".to_string(),
                discount_pct: Decimal::percent(25),
            },
            FeeTier {
                id: "basic".to_string(),
                min_voting_power: "0".to_string(),
                discount_pct: Decimal::zero(),
            },
        ],
    };
    let manager = StakingTierManager::new(config);

    let result = manager.validate();
    assert!(result.is_ok());
}

#[test]
fn test_validation_edge_case_single_digit() {
    let config = FeeTierConfig {
        tiers: vec![
            FeeTier {
                id: "tier_1".to_string(),
                min_voting_power: "1".to_string(),
                discount_pct: Decimal::percent(10),
            },
            FeeTier {
                id: "tier_2".to_string(),
                min_voting_power: "0".to_string(),
                discount_pct: Decimal::zero(),
            },
        ],
    };
    let manager = StakingTierManager::new(config);

    let result = manager.validate();
    assert!(result.is_ok());
}
