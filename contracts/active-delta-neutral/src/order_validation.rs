use cosmwasm_std::{Decimal, Deps, Env, SignedDecimal};
use mars_delta_neutral_position::types::Position;
use mars_types::{active_delta_neutral::execute::Direction, position::Side};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    error::{ContractError, ContractResult},
    traits::Validator,
};

// TODO validate profitabity correctly here

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Validation {
    Fixed,
    Dynamic,
}

impl Validator for Validation {
    fn validate_order_execution(&self, deps: Deps, env: &Env) -> ContractResult<()> {
        match self {
            Validation::Fixed => FixedValidator.validate_order_execution(deps, env),
            Validation::Dynamic => DynamicValidator.validate_order_execution(deps, env),
        }
    }
}

pub struct FixedValidator;

impl Validator for FixedValidator {
    fn validate_order_execution(&self, _deps: Deps, _env: &Env) -> ContractResult<()> {
        // Placeholder implementation for fixed validation
        // This will not be used initially but is here for future extension.
        Ok(())
    }
}

pub struct DynamicValidator;

impl Validator for DynamicValidator {
    /// Dynamic validator asserts that the position will meet the formula.
    /// Under this model, we determine that an entry is valid by the formula
    ///     
    ///     `base < entry_cost < net_yield / k`
    ///
    ///  Entry cost is the cost of entering a position, which can be determined by the formula
    ///
    ///     `entry_cost = ((spot_execution_price - perp_execution_price) / perp_price) + perp_trading_fee_rate`
    ///
    fn validate_order_execution(
        &self,
        perp_funding_rate: SignedDecimal,
        net_spot_yield: SignedDecimal,
        spot_execution_price: Decimal,
        perp_execution_price: Decimal,
        perp_trading_fee_rate: Decimal,
        direction: Direction,
    ) -> ContractResult<bool> {

        let net_yield = net_spot_yield.checked_add(perp_funding_rate)?;

        // The limit as defined by our model
        let cost_limit = net_yield.checked_div(self.k)?;
        
        // The cost of our order
        let cost = perp_execution_price
            .checked_sub(spot_execution_price)?
            .checked_div(spot_execution_price)?
            .checked_add(perp_trading_fee_rate.checked_mul(direction.sign()))?;

        let valid_entry = match direction {
            Direction::Buy => cost.lt(cost_limit),
            Direction::Sell => cost.gt(cost_limit),
        };

        Ok(valid_entry)
    }
}
