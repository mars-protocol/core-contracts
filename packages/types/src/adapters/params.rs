use std::collections::HashMap;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, QuerierWrapper, StdResult};
use cw_paginate::{PaginationResponse, MAX_LIMIT};

use crate::params::{AssetParams, PerpParams, QueryMsg, TotalDepositResponse, VaultConfig};

#[cw_serde]
pub struct ParamsBase<T>(T);

impl<T> ParamsBase<T> {
    pub fn new(address: T) -> ParamsBase<T> {
        ParamsBase(address)
    }

    pub fn address(&self) -> &T {
        &self.0
    }
}

pub type ParamsUnchecked = ParamsBase<String>;
pub type Params = ParamsBase<Addr>;

impl From<Params> for ParamsUnchecked {
    fn from(mars_params: Params) -> Self {
        Self(mars_params.0.to_string())
    }
}

impl ParamsUnchecked {
    pub fn check(&self, api: &dyn Api) -> StdResult<Params> {
        Ok(ParamsBase(api.addr_validate(self.address())?))
    }
}

impl Params {
    pub fn query_asset_params(
        &self,
        querier: &QuerierWrapper,
        denom: &str,
    ) -> StdResult<Option<AssetParams>> {
        querier.query_wasm_smart(
            self.address().to_string(),
            &QueryMsg::AssetParams {
                denom: denom.to_string(),
            },
        )
    }

    pub fn query_perp_params(
        &self,
        querier: &QuerierWrapper,
        denom: &str,
    ) -> StdResult<PerpParams> {
        querier.query_wasm_smart(
            self.address().to_string(),
            &QueryMsg::PerpParams {
                denom: denom.to_string(),
            },
        )
    }

    pub fn query_total_deposit(
        &self,
        querier: &QuerierWrapper,
        denom: &str,
    ) -> StdResult<TotalDepositResponse> {
        querier.query_wasm_smart(
            self.address().to_string(),
            &QueryMsg::TotalDeposit {
                denom: denom.to_string(),
            },
        )
    }

    pub fn query_vault_config(
        &self,
        querier: &QuerierWrapper,
        vault_address: &Addr,
    ) -> StdResult<VaultConfig> {
        querier.query_wasm_smart(
            self.address().to_string(),
            &QueryMsg::VaultConfig {
                address: vault_address.to_string(),
            },
        )
    }

    pub fn query_all_vault_configs_v2(
        &self,
        querier: &QuerierWrapper,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> StdResult<PaginationResponse<VaultConfig>> {
        querier.query_wasm_smart(
            self.address().to_string(),
            &QueryMsg::AllVaultConfigsV2 {
                start_after,
                limit,
            },
        )
    }

    pub fn query_all_perp_params_v2(
        &self,
        querier: &QuerierWrapper,
    ) -> StdResult<HashMap<String, PerpParams>> {
        let mut start_after = Option::<String>::None;
        let mut has_more = true;
        let mut all_perp_params = HashMap::new();
        while has_more {
            let response: PaginationResponse<PerpParams> = querier.query_wasm_smart(
                self.address().to_string(),
                &QueryMsg::AllPerpParamsV2 {
                    start_after: start_after.clone(),
                    limit: Some(MAX_LIMIT - 1),
                },
            )?;
            for item in response.data {
                let denom = item.denom.clone();
                all_perp_params.insert(denom.clone(), item);
                start_after = Some(denom);
            }
            has_more = response.metadata.has_more;
        }
        Ok(all_perp_params)
    }
}
