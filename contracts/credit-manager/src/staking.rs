use cosmwasm_std::{Decimal, Deps, Uint128};
use mars_types::{
    adapters::governance::Governance,
    fee_tiers::{FeeTier, FeeTierConfig},
};

const MAX_TIER_SIZE: usize = 20;

use crate::{
    error::{ContractError, ContractResult},
    state::{FEE_TIER_CONFIG, GOVERNANCE},
    utils::{
        assert_discount_pct, assert_tiers_max_size, assert_tiers_not_empty,
        assert_tiers_sorted_descending, query_nft_token_owner,
    },
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
    pub fn find_applicable_tier(&self, voting_power: Uint128) -> ContractResult<&FeeTier> {
        // Ensure tiers are sorted in descending order of min_voting_power
        assert_tiers_not_empty(&self.config.tiers)?;

        // Binary search for the applicable tier
        let mut left = 0;
        let mut right = self.config.tiers.len() - 1;
        let mut result = 0; // Default to first tier (highest threshold)

        while left <= right {
            let mid = left + (right - left) / 2;
            let tier = &self.config.tiers[mid];

            let min_power = tier.min_voting_power;

            if voting_power >= min_power {
                // User qualifies for this tier, but there might be a better one
                result = mid;
                // Look for higher tiers (lower indices) but don't go below 0
                if mid == 0 {
                    break;
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
    pub fn validate(&self) -> ContractResult<()> {
        assert_tiers_not_empty(&self.config.tiers)?;
        assert_tiers_max_size(&self.config.tiers, MAX_TIER_SIZE)?;

        // Check duplicates, descending order, and discount percentages
        let mut voting_powers = Vec::new();
        for (i, tier) in self.config.tiers.iter().enumerate() {
            // Validate discount percentage
            assert_discount_pct(tier.discount_pct)?;

            // Collect voting power for later validation
            voting_powers.push(tier.min_voting_power);

            // Check for duplicates (compare with previous tier)
            if i > 0 && voting_powers[i] == voting_powers[i - 1] {
                return Err(ContractError::DuplicateVotingPowerThresholds);
            }
        }

        // Check for descending order
        assert_tiers_sorted_descending(&voting_powers)?;

        Ok(())
    }

    /// Get the default tier (tier with lowest min_voting_power)
    /// the default tier (lowest voting power requirement) is the last element.
    pub fn get_default_tier(&self) -> ContractResult<&FeeTier> {
        self.config.tiers.last().ok_or(ContractError::NoTiersPresent)
    }
}

/// Get tier, discount percentage, and voting power for an account based on their staked MARS balance
pub fn get_account_tier_and_discount(
    deps: Deps,
    account_id: &str,
) -> ContractResult<(FeeTier, Decimal, Uint128)> {
    // Get account owner from account_id
    let account_owner = query_nft_token_owner(deps, account_id)?;

    // Get governance contract address from state
    let governance_addr =
        GOVERNANCE.load(deps.storage).map_err(|_| ContractError::FailedToLoadGovernanceAddress)?;
    let governance = Governance::new(governance_addr);

    // Query voting power for the account owner
    let voting_power_response = governance
        .query_voting_power_at_height(&deps.querier, &account_owner)
        .map_err(|e| ContractError::FailedToQueryVotingPower {
            error: e.to_string(),
        })?;

    // Get fee tier config and find applicable tier
    let fee_tier_config =
        FEE_TIER_CONFIG.load(deps.storage).map_err(|_| ContractError::FailedToLoadFeeTierConfig)?;
    let manager = StakingTierManager::new(fee_tier_config);
    let tier = manager.find_applicable_tier(voting_power_response.power)?;

    Ok((tier.clone(), tier.discount_pct, voting_power_response.power))
}
