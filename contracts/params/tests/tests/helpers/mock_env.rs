use std::mem::take;

use anyhow::Result as AnyResult;
use cosmwasm_std::{Addr, Decimal, Empty};
use cw_multi_test::{App, AppResponse, BasicApp, Executor};
use cw_paginate::PaginationResponse;
use mars_owner::{OwnerResponse, OwnerUpdate};
use mars_types::{
    address_provider::{self, AddressResponseItem, MarsAddressType},
    oracle,
    params::{
        AssetParams, AssetParamsUpdate, ConfigResponse, EmergencyUpdate, ExecuteMsg,
        InstantiateMsg, ManagedVaultConfigResponse, ManagedVaultUpdate, PerpParams,
        PerpParamsUpdate, QueryMsg, VaultConfig, VaultConfigUpdate,
    },
    perps::{self, Config},
};

use super::contracts::{
    mock_address_provider_contract, mock_oracle_contract, mock_params_contract, mock_perps_contract,
};

pub struct MockEnv {
    pub app: BasicApp,
    pub params_contract: Addr,
    pub address_provider_contract: Addr,
}

pub struct MockEnvBuilder {
    pub app: BasicApp,
    pub deployer: Addr,
    pub target_health_factor: Option<Decimal>,
    pub emergency_owner: Option<String>,
    pub address_provider: Option<Addr>,
    pub max_perp_params: Option<u8>,
}

#[allow(clippy::new_ret_no_self)]
impl MockEnv {
    pub fn new() -> MockEnvBuilder {
        MockEnvBuilder {
            app: App::default(),
            deployer: Addr::unchecked("owner"),
            target_health_factor: None,
            emergency_owner: None,
            address_provider: None,
            max_perp_params: None,
        }
    }

    //--------------------------------------------------------------------------------------------------
    // Execute Msgs
    //--------------------------------------------------------------------------------------------------

    pub fn update_asset_params(
        &mut self,
        sender: &Addr,
        update: AssetParamsUpdate,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.params_contract.clone(),
            &ExecuteMsg::UpdateAssetParams(update),
            &[],
        )
    }

    pub fn update_vault_config(
        &mut self,
        sender: &Addr,
        update: VaultConfigUpdate,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.params_contract.clone(),
            &ExecuteMsg::UpdateVaultConfig(update),
            &[],
        )
    }

    pub fn update_perp_params(
        &mut self,
        sender: &Addr,
        update: PerpParamsUpdate,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.params_contract.clone(),
            &ExecuteMsg::UpdatePerpParams(update),
            &[],
        )
    }

    pub fn update_owner(&mut self, sender: &Addr, update: OwnerUpdate) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.params_contract.clone(),
            &ExecuteMsg::UpdateOwner(update),
            &[],
        )
    }

    pub fn update_risk_manager(
        &mut self,
        sender: &Addr,
        update: OwnerUpdate,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.params_contract.clone(),
            &ExecuteMsg::UpdateRiskManager(update),
            &[],
        )
    }

    pub fn reset_risk_manager(&mut self, sender: &Addr) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.params_contract.clone(),
            &ExecuteMsg::ResetRiskManager(),
            &[],
        )
    }

    pub fn update_config(
        &mut self,
        sender: &Addr,
        address_provider: Option<String>,
        max_perp_params: Option<u8>,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.params_contract.clone(),
            &ExecuteMsg::UpdateConfig {
                address_provider,
                max_perp_params,
            },
            &[],
        )
    }

    pub fn update_managed_vault_config(
        &mut self,
        sender: &Addr,
        update: ManagedVaultUpdate,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.params_contract.clone(),
            &ExecuteMsg::UpdateManagedVaultConfig(update),
            &[],
        )
    }

    pub fn emergency_update(
        &mut self,
        sender: &Addr,
        update: EmergencyUpdate,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.params_contract.clone(),
            &ExecuteMsg::EmergencyUpdate(update),
            &[],
        )
    }

    //--------------------------------------------------------------------------------------------------
    // Queries
    //--------------------------------------------------------------------------------------------------

    pub fn query_owner(&self) -> Addr {
        let res = self.query_ownership();
        Addr::unchecked(res.owner.unwrap())
    }

    pub fn query_ownership(&self) -> OwnerResponse {
        self.app.wrap().query_wasm_smart(self.params_contract.clone(), &QueryMsg::Owner {}).unwrap()
    }

    pub fn query_risk_manager(&self) -> Addr {
        let risk_manager: OwnerResponse = self
            .app
            .wrap()
            .query_wasm_smart(self.params_contract.clone(), &QueryMsg::RiskManager {})
            .unwrap();
        Addr::unchecked(risk_manager.owner.unwrap())
    }

    pub fn query_asset_params(&self, denom: &str) -> AssetParams {
        self.app
            .wrap()
            .query_wasm_smart(
                self.params_contract.clone(),
                &QueryMsg::AssetParams {
                    denom: denom.to_string(),
                },
            )
            .unwrap()
    }

    pub fn query_all_asset_params(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Vec<AssetParams> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.params_contract.clone(),
                &QueryMsg::AllAssetParams {
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn query_all_asset_params_v2(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> PaginationResponse<AssetParams> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.params_contract.clone(),
                &QueryMsg::AllAssetParamsV2 {
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn query_vault_config(&self, addr: &str) -> VaultConfig {
        self.app
            .wrap()
            .query_wasm_smart(
                self.params_contract.clone(),
                &QueryMsg::VaultConfig {
                    address: addr.to_string(),
                },
            )
            .unwrap()
    }

    pub fn query_all_vault_configs(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Vec<VaultConfig> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.params_contract.clone(),
                &QueryMsg::AllVaultConfigs {
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn query_all_vault_configs_v2(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> PaginationResponse<VaultConfig> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.params_contract.clone(),
                &QueryMsg::AllVaultConfigsV2 {
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn query_perp_params(&self, denom: &str) -> PerpParams {
        self.app
            .wrap()
            .query_wasm_smart(
                self.params_contract.clone(),
                &QueryMsg::PerpParams {
                    denom: denom.to_string(),
                },
            )
            .unwrap()
    }

    pub fn query_all_perp_params(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Vec<PerpParams> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.params_contract.clone(),
                &QueryMsg::AllPerpParams {
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn query_all_perp_params_v2(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> PaginationResponse<PerpParams> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.params_contract.clone(),
                &QueryMsg::AllPerpParamsV2 {
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn query_config(&self) -> ConfigResponse {
        self.app
            .wrap()
            .query_wasm_smart(self.params_contract.clone(), &QueryMsg::Config {})
            .unwrap()
    }

    pub fn query_managed_vault_config(&self) -> ManagedVaultConfigResponse {
        self.app
            .wrap()
            .query_wasm_smart(self.params_contract.clone(), &QueryMsg::ManagedVaultConfig {})
            .unwrap()
    }

    pub fn query_perp_config(&self) -> Config<Addr> {
        let perps_address: AddressResponseItem = self
            .app
            .wrap()
            .query_wasm_smart(
                self.address_provider_contract.clone(),
                &address_provider::QueryMsg::Address(MarsAddressType::Perps),
            )
            .unwrap();

        self.app
            .wrap()
            .query_wasm_smart(perps_address.address, &perps::QueryMsg::Config {})
            .unwrap()
    }
}

impl MockEnvBuilder {
    pub fn build(&mut self) -> AnyResult<MockEnv> {
        self.build_with_risk_manager(None)
    }

    pub fn build_with_risk_manager(&mut self, risk_manager: Option<String>) -> AnyResult<MockEnv> {
        let address_provider_contract = self.get_address_provider();
        self.deploy_perps(address_provider_contract.as_str());
        self.deploy_oracle();

        let code_id = self.app.store_code(mock_params_contract());

        let params_contract = self.app.instantiate_contract(
            code_id,
            self.deployer.clone(),
            &InstantiateMsg {
                owner: self.deployer.clone().to_string(),
                risk_manager,
                address_provider: address_provider_contract.to_string(),
                max_perp_params: self.max_perp_params.unwrap_or(40),
            },
            &[],
            "mock-params-contract",
            None,
        )?;

        self.set_address(MarsAddressType::Params, params_contract.clone());

        if self.emergency_owner.is_some() {
            self.set_emergency_owner(&params_contract, &self.emergency_owner.clone().unwrap());
        }

        Ok(MockEnv {
            app: take(&mut self.app),
            params_contract,
            address_provider_contract,
        })
    }

    fn deploy_address_provider(&mut self) -> Addr {
        let contract = mock_address_provider_contract();
        let code_id = self.app.store_code(contract);

        self.app
            .instantiate_contract(
                code_id,
                self.deployer.clone(),
                &address_provider::InstantiateMsg {
                    owner: self.deployer.clone().to_string(),
                    prefix: "".to_string(),
                },
                &[],
                "mock-address-provider",
                None,
            )
            .unwrap()
    }

    fn deploy_perps(&mut self, address_provider: &str) -> Addr {
        let code_id = self.app.store_code(mock_perps_contract());

        let addr = self
            .app
            .instantiate_contract(
                code_id,
                self.deployer.clone(),
                &perps::InstantiateMsg {
                    address_provider: address_provider.to_string(),
                    base_denom: "uusdc".to_string(),
                    cooldown_period: 0,
                    max_positions: 4,
                    protocol_fee_rate: Decimal::from_ratio(1u128, 100u128),
                    target_vault_collateralization_ratio: Decimal::from_ratio(125u128, 100u128),
                    deleverage_enabled: true,
                    vault_withdraw_enabled: true,
                    max_unlocks: 5,
                },
                &[],
                "mock-perps",
                None,
            )
            .unwrap();

        self.set_address(MarsAddressType::Perps, addr.clone());

        addr
    }

    fn deploy_oracle(&mut self) -> Addr {
        let code_id = self.app.store_code(mock_oracle_contract());

        let addr = self
            .app
            .instantiate_contract(
                code_id,
                self.deployer.clone(),
                &oracle::InstantiateMsg::<Empty> {
                    owner: self.deployer.to_string(),
                    base_denom: "uusd".to_string(),
                    custom_init: None,
                },
                &[],
                "oracle",
                None,
            )
            .unwrap();

        self.set_address(MarsAddressType::Oracle, addr.clone());

        addr
    }

    fn set_address(&mut self, address_type: MarsAddressType, address: Addr) {
        let address_provider_addr = self.get_address_provider();

        self.app
            .execute_contract(
                self.deployer.clone(),
                address_provider_addr,
                &address_provider::ExecuteMsg::SetAddress {
                    address_type,
                    address: address.into(),
                },
                &[],
            )
            .unwrap();
    }

    fn get_address_provider(&mut self) -> Addr {
        if self.address_provider.is_none() {
            let addr = self.deploy_address_provider();

            self.address_provider = Some(addr);
        }
        self.address_provider.clone().unwrap()
    }

    fn set_emergency_owner(&mut self, params_contract: &Addr, eo: &str) {
        self.app
            .execute_contract(
                self.deployer.clone(),
                params_contract.clone(),
                &ExecuteMsg::UpdateOwner(OwnerUpdate::SetEmergencyOwner {
                    emergency_owner: eo.to_string(),
                }),
                &[],
            )
            .unwrap();
    }

    //--------------------------------------------------------------------------------------------------
    // Setter functions
    //--------------------------------------------------------------------------------------------------
    pub fn emergency_owner(&mut self, eo: &str) -> &mut Self {
        self.emergency_owner = Some(eo.to_string());
        self
    }

    pub fn max_perp_params(&mut self, max: u8) -> &mut Self {
        self.max_perp_params = Some(max);
        self
    }
}
