use cosmwasm_std::{Decimal, Int128, SignedDecimal};
use mars_utils::{error::ValidationError, helpers::integer_param_gt_zero};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    active_delta_neutral::error::ContractResult, position::Direction
};


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct DynamicValidator {
    pub k: u64,
}

#[derive(PartialEq, Debug)]
pub struct ValidationResult {
    pub valid: bool,
    pub cost_limit: SignedDecimal,
    pub cost: SignedDecimal,
}

impl DynamicValidator {
    /// Dynamic validator asserts that the position will meet the formula.
    /// Under this model, we determine that an entry is valid by the formula
    ///     
    ///     `base < entry_cost < net_yield / k`
    ///
    ///  Entry cost is the cost of entering a position, which can be determined by the formula
    ///
    ///     `entry_cost = ((spot_execution_price - perp_execution_price) / perp_price) + perp_trading_fee_rate`
    ///
    pub fn validate_order_execution(    
        &self,
        perp_funding_rate: SignedDecimal,
        net_spot_yield: SignedDecimal,
        spot_execution_price: SignedDecimal,
        perp_execution_price: SignedDecimal,
        perp_trading_fee_rate: Decimal,
        direction: Direction,
    ) -> ContractResult<ValidationResult> {
        let net_yield = match direction {
            Direction::Long => net_spot_yield.checked_sub(perp_funding_rate)?,
            Direction::Short => net_spot_yield.checked_add(perp_funding_rate)?,
        };

        // Convert values to SignedDecimal to support our operation
        let perp_trading_fee_sd: SignedDecimal = perp_trading_fee_rate.try_into()?;

        // The limit as defined by our model
        let cost_limit =
            net_yield.checked_div(SignedDecimal::from_atomics(Int128::from(self.k), 0)?)?;

        let price_diff = spot_execution_price.checked_sub(perp_execution_price)?;
        let price_diff_percent = price_diff.checked_div(perp_execution_price)?;
        
        // The cost of our order
        let cost = price_diff_percent.checked_add(perp_trading_fee_sd)?;

        let valid_entry = match direction {
            Direction::Long => cost.lt(&cost_limit),
            Direction::Short => cost.gt(&cost_limit),
        };

        Ok(ValidationResult {
            valid: valid_entry,
            cost_limit,
            cost,
        })
    }

    pub fn validate(&self) -> Result<(), ValidationError> {
        integer_param_gt_zero(self.k, "k")?;
        Ok(())
    }
}

