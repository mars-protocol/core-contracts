use crate::error::{ContractResult, ContractError};
use crate::traits::Validator;
use cosmwasm_std::{Deps, Env};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// TODO validate profitabity correctly here

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Validation {
    Fixed, // Placeholder for fixed validation
    Dynamic,
}

impl Validator for Validation {
    fn validate_entry(&self, deps: Deps, env: &Env) -> ContractResult<()> {
        match self {
            Validation::Fixed => FixedValidator.validate_entry(deps, env),
            Validation::Dynamic => DynamicValidator.validate_entry(deps, env),
        }
    }
}

pub struct FixedValidator;

impl Validator for FixedValidator {
    fn validate_entry(&self, _deps: Deps, _env: &Env) -> ContractResult<()> {
        // Placeholder implementation for fixed validation
        // This will not be used initially but is here for future extension.
        Ok(())
    }
}

pub struct DynamicValidator;

impl Validator for DynamicValidator {
    fn validate_entry(&self, _deps: Deps, _env: &Env) -> ContractResult<()> {
        // TODO: Implement the dynamic validation logic as described in order_validation_plan.md
        // This includes model-based and risk-based checks.
        // For now, we return a placeholder error to indicate it's not implemented.
        Err(ContractError::NotImplemented)
    }
}
