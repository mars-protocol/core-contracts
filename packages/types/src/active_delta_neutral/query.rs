use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
use mars_utils::{
    error::ValidationError,
    helpers::{integer_param_gt_zero, validate_native_denom},
};
/// Configuration for the Active Delta Neutral contract.
///
/// This struct contains the addresses and identifiers for all major dependencies
/// and configuration parameters required by the contract.
#[cw_serde]
pub struct Config {
    /// Optional identifier for the associated credit account.
    /// This should be set after contract initialization
    pub credit_account_id: Option<String>,
    /// Address of the Credit Manager contract.
    pub credit_manager_addr: Addr,
    /// Address of the Oracle contract used for price feeds.
    pub oracle_addr: Addr,
    /// Address of the Perps contract.
    pub perps_addr: Addr,
    /// Address of the Health contract for risk checks.
    pub health_addr: Addr,
    /// Address of the Red Bank contract for asset management.
    pub red_bank_addr: Addr,
    /// The base denomination used for this strategy (e.g., "umars").
    pub base_denom: String,
}

/// Query messages supported by the Active Delta Neutral contract.
///
/// These messages allow clients to fetch contract configuration and market parameters.
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns the global configuration of the contract.
    #[returns(Config)]
    Config {},
    /// Returns the configuration for a specific market.
    #[returns(MarketConfig)]
    MarketConfig {
        /// The unique identifier for the market to fetch.
        market_id: String,
    },
    /// Returns a list of all market configurations, optionally paginated.
    #[returns(Vec<MarketConfig>)]
    MarketConfigs {
        /// Optional: return markets after this market_id (pagination).
        start_after: Option<String>,
        /// Optional: maximum number of markets to return.
        limit: Option<u32>,
    },
}

/// Configuration parameters for an individual market.
///
/// This struct defines all the necessary denoms and parameters for a single market
/// managed by the Active Delta Neutral strategy.
#[cw_serde]
pub struct MarketConfig {
    /// Unique identifier for this market.
    pub market_id: String,
    /// Denomination of the USDC token for this market.
    pub usdc_denom: String,
    /// Denomination of the spot asset for this market.
    pub spot_denom: String,
    /// Denomination of the perps asset for this market (must start with "perps/").
    pub perp_denom: String,
    /// Market parameter controlling the AMM curve (must be > 0).
    pub k: u64,
}

impl MarketConfig {
    /// Constructs a new `MarketConfig` instance.
    ///
    /// # Arguments
    /// * `market_id` - Unique identifier for the market.
    /// * `usdc_denom` - Denomination of the USDC token.
    /// * `spot_denom` - Denomination of the spot asset.
    /// * `perp_denom` - Denomination of the perps asset (must start with "perps/").
    /// * `k` - AMM curve parameter (must be > 0).
    pub fn new(
        market_id: String,
        usdc_denom: String,
        spot_denom: String,
        perp_denom: String,
        k: u64,
    ) -> Self {
        Self {
            market_id,
            usdc_denom,
            spot_denom,
            perp_denom,
            k,
        }
    }

    /// Validates the market configuration.
    ///
    /// Ensures all denom fields are valid and `k` is greater than zero.
    /// Returns a `ValidationError` if any field is invalid.
    pub fn validate(&self) -> Result<(), ValidationError> {
        validate_native_denom(&self.usdc_denom)?;
        validate_native_denom(&self.spot_denom)?;

        if !self.perp_denom.starts_with("perps/") {
            return Err(ValidationError::InvalidDenom {
                reason: "Perp denom must start with 'perps/'".to_string(),
            });
        }

        integer_param_gt_zero(self.k, "k")?;

        Ok(())
    }
}
