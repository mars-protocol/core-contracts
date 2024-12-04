use cosmwasm_std::{Addr, Deps, QuerierWrapper, StdError, StdResult};
use mars_types::{
    adapters::{oracle::Oracle, params::Params, perps::Perps, vault::Vault},
    credit_manager::{ConfigResponse, Positions, QueryMsg as CmQueryMsg},
    health::HealthResult,
    oracle::ActionKind,
    params::VaultConfig,
};

use crate::state::CREDIT_MANAGER;

pub struct HealthQuerier<'a> {
    querier: &'a QuerierWrapper<'a>,
    credit_manager: Addr,
    pub params: Params,
    pub oracle: Oracle,
    pub perps: Perps,
}

impl<'a> HealthQuerier<'a> {
    pub fn new(deps: &'a Deps) -> StdResult<Self> {
        let credit_manager = CREDIT_MANAGER.load(deps.storage).map_err(|_| {
            StdError::generic_err(
                "Credit Manager contract is currently not set up in the health contract",
            )
        })?;
        let config: ConfigResponse =
            deps.querier.query_wasm_smart(credit_manager.to_string(), &CmQueryMsg::Config {})?;

        Self::new_with_config(deps, credit_manager, config)
    }

    pub fn new_with_config(
        deps: &'a Deps,
        credit_manager: Addr,
        config: ConfigResponse,
    ) -> StdResult<Self> {
        Ok(Self {
            querier: &deps.querier,
            credit_manager,
            params: Params::new(Addr::unchecked(config.params)),
            oracle: Oracle::new(Addr::unchecked(config.oracle)),
            perps: Perps::new(Addr::unchecked(config.perps)),
        })
    }

    pub fn query_positions(&self, account_id: &str, action: ActionKind) -> HealthResult<Positions> {
        Ok(self.querier.query_wasm_smart(
            self.credit_manager.to_string(),
            &CmQueryMsg::Positions {
                account_id: account_id.to_string(),
                action: Some(action),
            },
        )?)
    }

    pub fn query_vault_config(&self, vault: &Vault) -> HealthResult<VaultConfig> {
        Ok(self
            .params
            .query_vault_config(self.querier, &Addr::unchecked(vault.address.to_string()))?)
    }
}
