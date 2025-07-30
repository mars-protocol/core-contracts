use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
use mars_utils::{
    error::ValidationError,
    helpers::{integer_param_gt_zero, validate_native_denom},
};

#[cw_serde]
pub struct Config {
    pub owner: Addr,
    pub credit_account_id: String,
    pub credit_manager_addr: Addr,
    pub oracle_addr: Addr,
    pub perps_addr: Addr,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Config)]
    Config {},
    #[returns(MarketConfig)]
    MarketConfig {
        market_id: String,
    },
    #[returns(Vec<MarketConfig>)]
    MarketConfigs {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct MarketConfig {
    pub market_id: String,
    pub usdc_denom: String,
    pub spot_denom: String,
    pub perp_denom: String,
    pub k: u64,
}

impl MarketConfig {
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

    pub fn validate(&self) -> Result<(), ValidationError> {
        validate_native_denom(&self.usdc_denom)?;
        validate_native_denom(&self.spot_denom)?;

        if self.spot_denom == self.perp_denom {
            return Err(ValidationError::InvalidDenom {
                reason: "Spot and perp denoms must be different".to_string(),
            });
        }

        if !self.perp_denom.starts_with("perps/") {
            return Err(ValidationError::InvalidDenom {
                reason: "Perp denom must start with 'perps/'".to_string(),
            });
        }

        integer_param_gt_zero(self.k, "k")?;

        Ok(())
    }
}
