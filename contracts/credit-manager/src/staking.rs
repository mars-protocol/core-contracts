use std::str::FromStr;

use cosmwasm_std::{Decimal, Deps, StdError, StdResult, Uint128};
use mars_types::{
    adapters::dao_staking::DaoStaking,
    fee_tiers::{FeeTier, FeeTierConfig},
};

use crate::{
    state::{DAO_STAKING_ADDRESS, FEE_TIER_CONFIG},
    utils::query_nft_token_owner,
};

pub struct StakingTierManager {
    pub config: FeeTierConfig,
}

impl StakingTierManager {
    pub fn new(config: FeeTierConfig) -> Self {
        Self {
            config,
        }
    }

    /// Find the applicable tier for a given voting power
    /// Returns the tier with the highest min_voting_power that the user qualifies for
    pub fn find_applicable_tier(&self, voting_power: Uint128) -> StdResult<&FeeTier> {
        // Ensure tiers are sorted in descending order of min_voting_power
        if self.config.tiers.is_empty() {
            return Err(StdError::generic_err("No tiers configured"));
        }

        // Binary search for the applicable tier
        let mut left = 0;
        let mut right = self.config.tiers.len() - 1;
        let mut result = 0; // Default to first tier (highest threshold)

        while left <= right {
            let mid = left + (right - left) / 2;
            let tier = &self.config.tiers[mid];

            // Parse min_voting_power once per tier
            let min_power = Uint128::from_str(&tier.min_voting_power)
                .map_err(|_| StdError::generic_err("Invalid min_voting_power in tier"))?;

            if voting_power >= min_power {
                // User qualifies for this tier, but there might be a better one
                result = mid;
                // Look for higher tiers (lower indices) but don't go below 0
                if mid == 0 {
                    break; // We found the highest tier
                }
                right = mid - 1;
            } else {
                // User doesn't qualify for this tier, look at lower tiers
                left = mid + 1;
            }
        }

        Ok(&self.config.tiers[result])
    }

    /// Validate that tiers are properly ordered by min_voting_power (descending)
    pub fn validate(&self) -> StdResult<()> {
        if self.config.tiers.is_empty() {
            return Err(StdError::generic_err("Fee tier config cannot be empty"));
        }

        // Parse first tier once
        let mut prev_power = Uint128::from_str(&self.config.tiers[0].min_voting_power)
            .map_err(|_| StdError::generic_err("Invalid min_voting_power in tier"))?;

        // Check for descending order and duplicates in one pass
        for i in 1..self.config.tiers.len() {
            let curr_power = Uint128::from_str(&self.config.tiers[i].min_voting_power)
                .map_err(|_| StdError::generic_err("Invalid min_voting_power in tier"))?;

            if curr_power == prev_power {
                return Err(StdError::generic_err("Duplicate voting power thresholds"));
            }

            if curr_power >= prev_power {
                return Err(StdError::generic_err("Tiers must be sorted in descending order"));
            }

            prev_power = curr_power;
        }

        // Validate discount percentages are reasonable (0-100%)
        for tier in &self.config.tiers {
            if tier.discount_pct >= Decimal::one() {
                return Err(StdError::generic_err("Discount percentage must be less than 100%"));
            }
        }

        Ok(())
    }

    /// Get the default tier (tier with lowest min_voting_power)
    pub fn get_default_tier(&self) -> StdResult<&FeeTier> {
        let mut default_tier: Option<&FeeTier> = None;
        let mut lowest_power = Uint128::MAX;

        for tier in &self.config.tiers {
            let min_power = Uint128::from_str(&tier.min_voting_power)
                .map_err(|_| StdError::generic_err("Invalid min_voting_power in tier"))?;

            if min_power < lowest_power {
                default_tier = Some(tier);
                lowest_power = min_power;
            }
        }

        default_tier.ok_or_else(|| StdError::generic_err("No tiers configured"))
    }
}

/// Get tier, discount percentage, and voting power for an account based on their staked MARS balance
pub fn get_account_tier_and_discount(
    deps: Deps,
    account_id: &str,
) -> StdResult<(FeeTier, Decimal, Uint128)> {
    // Get account owner from account_id
    let account_owner = query_nft_token_owner(deps, account_id)
        .map_err(|e| StdError::generic_err(e.to_string()))?;

    // Get DAO staking contract address from state
    let dao_staking_addr = DAO_STAKING_ADDRESS.load(deps.storage)?;
    let dao_staking = DaoStaking::new(dao_staking_addr);

    // Query voting power for the account owner
    let voting_power_response =
        dao_staking.query_voting_power_at_height(&deps.querier, &account_owner)?;

    // Get fee tier config and find applicable tier
    let fee_tier_config = FEE_TIER_CONFIG.load(deps.storage)?;
    let manager = StakingTierManager::new(fee_tier_config);
    let tier = manager.find_applicable_tier(voting_power_response.power)?;

    Ok((tier.clone(), tier.discount_pct, voting_power_response.power))
}
