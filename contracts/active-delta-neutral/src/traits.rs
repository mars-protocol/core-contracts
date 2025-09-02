use cosmwasm_std::{Deps, Env};
use crate::error::ContractResult;

pub trait Validator {
    fn validate_entry(&self, deps: Deps, env: &Env) -> ContractResult<()>;
}
