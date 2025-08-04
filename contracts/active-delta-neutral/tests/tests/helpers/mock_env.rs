use cosmwasm_std::{Addr, Uint128};
use cw_multi_test::{error::AnyResult, no_init, AppResponse, BasicAppBuilder, Executor};
use cw_paginate::PaginationResponse;
use mars_testing::multitest::{helpers::{active_delta_neutral_contract, MockEnv as BaseMockEnv, MockEnvBuilder as BaseMockEnvBuilder}, modules::token_factory::CustomApp};   
use mars_types::{active_delta_neutral::{execute::ExecuteMsg, instantiate::InstantiateMsg, query::{Config, MarketConfig, QueryMsg}}, adapters::active_delta_neutral::ActiveDeltaNeutral, address_provider::MarsAddressType, swapper::SwapperRoute};


pub struct MockEnv {
    app: CustomApp,
    active_delta_neutral: ActiveDeltaNeutral,
    address_provider: Addr,
}

pub struct MockEnvBuilder {
    base_builder: BaseMockEnvBuilder,
    app: CustomApp,
    owner: Option<Addr>,
    active_delta_neutral: Option<ActiveDeltaNeutral>,
    address_provider: Option<Addr>,
}

impl MockEnv {
    pub fn new() -> MockEnvBuilder {
        MockEnvBuilder::new()
    }

    pub fn query_active_delta_neutral_market(&self, market_id: &str) -> MarketConfig {
        self.app
            .wrap()
            .query_wasm_smart(
                self.active_delta_neutral.address(),
                &QueryMsg::MarketConfig {
                    market_id: market_id.to_string(),
                },
            )
            .unwrap()
    }

    pub fn query_active_delta_neutral_config(&self) -> Config {
        self.app
            .wrap()
            .query_wasm_smart(
                self.active_delta_neutral.address(),
                &QueryMsg::Config {},
            )
            .unwrap()
    }

    pub fn query_all_active_delta_neutral_markets(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> PaginationResponse<MarketConfig> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.active_delta_neutral.address(),
                &QueryMsg::MarketConfigs {
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn add_active_delta_neutral_market(
        &mut self,
        sender: &Addr,
        market_config: MarketConfig,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.active_delta_neutral.address().clone(),
            &ExecuteMsg::AddMarket {
                config: market_config,
            },
            &[],
        )
    }
    
    pub fn buy_delta_neutral_market(
        &mut self,
        sender: &Addr,
        market_id: &str,
        amount: Uint128,
        swapper_route: SwapperRoute,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.active_delta_neutral.address().clone(),
            &ExecuteMsg::Buy {
                amount,
                market_id: market_id.to_string(),
                swapper_route,
            },
            &[],
        )
    }
    
    pub fn sell_delta_neutral_market(
        &mut self,
        sender: &Addr,
        market_id: &str,
        amount: Uint128,
        swapper_route: SwapperRoute,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.active_delta_neutral.address().clone(),
            &ExecuteMsg::Sell {
                market_id: market_id.to_string(),
                amount,
                swapper_route,
            },
            &[],
        )
    }

}

impl MockEnvBuilder {

    pub fn new() -> MockEnvBuilder {


        let tf_default = TokenFactory::default();
        let app = BasicAppBuilder::new().with_stargate(tf_default).build(no_init);

        let base_moc_env_builder = BaseMockEnvBuilder {
            app,
            owner: None,
            emergency_owner: None,
            vault_configs: None,
            coin_params: None,
            address_provider: None,
            oracle: None,
            params: None,
            red_bank: None,
            incentives: None,
            deploy_nft_contract: true,
            set_nft_contract_minter: true,
            accounts_to_fund: vec![],
            max_trigger_orders: None,
            max_unlocking_positions: None,
            max_slippage: None,
            health_contract: None,
            evil_vault: None,
            target_vault_collateralization_ratio: None,
            deleverage_enabled: None,
            withdraw_enabled: None,
            keeper_fee_config: None,
            perps_liquidation_bonus_ratio: None,
            perps_protocol_fee_ratio: None,
        };

        MockEnvBuilder {
            base_builder: base_mock_env_builder,
            app,
            owner: None,
            active_delta_neutral: None,
            address_provider: None,
        }
    }

    pub fn build(&mut self) -> MockEnv {

        let address_provider = self.address_provider.unwrap();
        let active_delta_neutral = self.deploy_active_delta_neutral_contract();
        MockEnv {
            app: self.app,
            active_delta_neutral,
            address_provider,
        }
    }
    
    pub fn deploy_active_delta_neutral_contract(&mut self) -> ActiveDeltaNeutral {
        let contract_code_id = self.app.store_code(active_delta_neutral_contract());
        let owner = self.owner.clone().unwrap();
        let address_provider = self.address_provider.clone().unwrap();
    
        let addr = self
            .app
            .instantiate_contract(
                contract_code_id,
                owner.clone(),
                &InstantiateMsg {
                    address_provider: address_provider.into(),
                },
                &[],
                "mock-active-delta-neutral-contract",
                Some(owner.to_string()),
            )
            .unwrap();
    
        self.base_builder.set_address(MarsAddressType::ActiveDeltaNeutral, addr.clone());
    
        ActiveDeltaNeutral::new(addr)
    }

}