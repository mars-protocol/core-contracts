use cosmwasm_schema::cw_serde;
use cosmwasm_std::Api;
use mars_swapper_base::{Config, ContractResult};

#[cw_serde]
pub struct DualityConfig {}

impl Config for DualityConfig {
    fn validate(&self, _api: &dyn Api) -> ContractResult<()> {
        // Nothing to validate
        Ok(())
    }
}
