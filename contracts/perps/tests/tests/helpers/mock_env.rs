#![allow(dead_code)] // TODO: remove once functions are used
use std::mem::take;

use anyhow::Result as AnyResult;
use cosmwasm_std::{coin, Addr, Coin, Decimal, Empty, Uint128};
use cw_multi_test::{App, AppResponse, BankSudo, BasicApp, Executor, SudoMsg};
use mars_oracle_osmosis::OsmosisPriceSourceUnchecked;
use mars_owner::{OwnerResponse, OwnerUpdate};
use mars_types::{
    adapters::oracle::OracleBase,
    math::SignedDecimal,
    oracle,
    perps::{
        self, Config, DenomStateResponse, DepositResponse, PerpDenomState, PnlValues,
        PositionResponse, PositionsByAccountResponse, VaultState,
    },
};

use super::{
    contracts::{mock_oracle_contract, mock_perps_contract},
    mock_credit_manager_contract,
};

pub struct MockEnv {
    app: BasicApp,
    pub owner: Addr,
    pub perps: Addr,
    pub oracle: Addr,
    pub credit_manager: Addr,
}

pub struct MockEnvBuilder {
    app: BasicApp,
    deployer: Addr,
    oracle_base_denom: String,
    perps_base_denom: String,
    min_position_value: Uint128,
}

#[allow(clippy::new_ret_no_self)]
impl MockEnv {
    pub fn new() -> MockEnvBuilder {
        MockEnvBuilder {
            app: App::default(),
            deployer: Addr::unchecked("deployer"),
            oracle_base_denom: "uusd".to_string(),
            perps_base_denom: "uusdc".to_string(),
            min_position_value: Uint128::one(),
        }
    }

    pub fn fund_accounts(&mut self, addrs: &[&Addr], amount: u128, denoms: &[&str]) {
        for addr in addrs {
            let coins: Vec<_> = denoms.iter().map(|&d| coin(amount, d)).collect();
            self.fund_account(addr, &coins);
        }
    }

    pub fn fund_account(&mut self, addr: &Addr, coins: &[Coin]) {
        self.app
            .sudo(SudoMsg::Bank(BankSudo::Mint {
                to_address: addr.to_string(),
                amount: coins.to_vec(),
            }))
            .unwrap();
    }

    pub fn increment_by_blocks(&mut self, num_of_blocks: u64) {
        self.app.update_block(|block| {
            block.height += num_of_blocks;
            // assume block time = 6 sec
            block.time = block.time.plus_seconds(num_of_blocks * 6);
        })
    }

    pub fn increment_by_time(&mut self, seconds: u64) {
        self.app.update_block(|block| {
            block.height += seconds / 6;
            // assume block time = 6 sec
            block.time = block.time.plus_seconds(seconds);
        })
    }

    pub fn query_block_time(&self) -> u64 {
        self.app.block_info().time.seconds()
    }

    //--------------------------------------------------------------------------------------------------
    // Execute Msgs
    //--------------------------------------------------------------------------------------------------

    pub fn update_owner(&mut self, sender: &Addr, update: OwnerUpdate) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.perps.clone(),
            &perps::ExecuteMsg::UpdateOwner(update),
            &[],
        )
    }

    pub fn init_denom(
        &mut self,
        sender: &Addr,
        denom: &str,
        max_funding_velocity: Decimal,
        skew_scale: Decimal,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.perps.clone(),
            &perps::ExecuteMsg::InitDenom {
                denom: denom.to_string(),
                max_funding_velocity,
                skew_scale,
            },
            &[],
        )
    }

    pub fn enable_denom(&mut self, sender: &Addr, denom: &str) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.perps.clone(),
            &perps::ExecuteMsg::EnableDenom {
                denom: denom.to_string(),
            },
            &[],
        )
    }

    pub fn disable_denom(&mut self, sender: &Addr, denom: &str) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.perps.clone(),
            &perps::ExecuteMsg::DisableDenom {
                denom: denom.to_string(),
            },
            &[],
        )
    }

    pub fn deposit_to_vault(&mut self, sender: &Addr, funds: &[Coin]) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.perps.clone(),
            &perps::ExecuteMsg::Deposit {},
            funds,
        )
    }

    pub fn withdraw_from_vault(
        &mut self,
        sender: &Addr,
        shares: Uint128,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.perps.clone(),
            &perps::ExecuteMsg::Withdraw {
                shares,
            },
            &[],
        )
    }

    pub fn open_position(
        &mut self,
        sender: &Addr,
        account_id: &str,
        denom: &str,
        size: SignedDecimal,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.perps.clone(),
            &perps::ExecuteMsg::OpenPosition {
                account_id: account_id.to_string(),
                denom: denom.to_string(),
                size,
            },
            &[],
        )
    }

    pub fn close_position(
        &mut self,
        sender: &Addr,
        account_id: &str,
        denom: &str,
        funds: &[Coin],
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.perps.clone(),
            &perps::ExecuteMsg::ClosePosition {
                account_id: account_id.to_string(),
                denom: denom.to_string(),
            },
            funds,
        )
    }

    pub fn set_price(
        &mut self,
        sender: &Addr,
        denom: &str,
        price: Decimal,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.oracle.clone(),
            &oracle::ExecuteMsg::<OsmosisPriceSourceUnchecked>::SetPriceSource {
                denom: denom.to_string(),
                price_source: OsmosisPriceSourceUnchecked::Fixed {
                    price,
                },
            },
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
        self.app.wrap().query_wasm_smart(self.perps.clone(), &perps::QueryMsg::Owner {}).unwrap()
    }

    pub fn query_config(&self) -> Config<Addr> {
        self.app.wrap().query_wasm_smart(self.perps.clone(), &perps::QueryMsg::Config {}).unwrap()
    }

    pub fn query_vault_state(&self) -> VaultState {
        self.app
            .wrap()
            .query_wasm_smart(self.perps.clone(), &perps::QueryMsg::VaultState {})
            .unwrap()
    }

    pub fn query_denom_state(&self, denom: &str) -> DenomStateResponse {
        self.app
            .wrap()
            .query_wasm_smart(
                self.perps.clone(),
                &perps::QueryMsg::DenomState {
                    denom: denom.to_string(),
                },
            )
            .unwrap()
    }

    pub fn query_perp_denom_state(&self, denom: &str) -> PerpDenomState {
        self.app
            .wrap()
            .query_wasm_smart(
                self.perps.clone(),
                &perps::QueryMsg::PerpDenomState {
                    denom: denom.to_string(),
                },
            )
            .unwrap()
    }

    pub fn query_denom_states(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Vec<DenomStateResponse> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.perps.clone(),
                &perps::QueryMsg::DenomStates {
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn query_deposit(&self, depositor: &str) -> DepositResponse {
        self.app
            .wrap()
            .query_wasm_smart(
                self.perps.clone(),
                &perps::QueryMsg::Deposit {
                    depositor: depositor.to_string(),
                },
            )
            .unwrap()
    }

    pub fn query_position(&self, account_id: &str, denom: &str) -> PositionResponse {
        self.app
            .wrap()
            .query_wasm_smart(
                self.perps.clone(),
                &perps::QueryMsg::Position {
                    account_id: account_id.to_string(),
                    denom: denom.to_string(),
                },
            )
            .unwrap()
    }

    pub fn query_positions(
        &self,
        start_after: Option<(String, String)>,
        limit: Option<u32>,
    ) -> Vec<PositionResponse> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.perps.clone(),
                &perps::QueryMsg::Positions {
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn query_positions_by_account_id(&self, account_id: &str) -> PositionsByAccountResponse {
        self.app
            .wrap()
            .query_wasm_smart(
                self.perps.clone(),
                &perps::QueryMsg::PositionsByAccount {
                    account_id: account_id.to_string(),
                },
            )
            .unwrap()
    }

    pub fn query_total_pnl(&self) -> PnlValues {
        self.app.wrap().query_wasm_smart(self.perps.clone(), &perps::QueryMsg::TotalPnl {}).unwrap()
    }
}

impl MockEnvBuilder {
    pub fn build(&mut self) -> AnyResult<MockEnv> {
        let oracle_contract = self.deploy_oracle();
        let credit_manager_contract = self.deploy_credit_manager();

        let code_id = self.app.store_code(mock_perps_contract());
        let perps_contract = self.app.instantiate_contract(
            code_id,
            self.deployer.clone(),
            &perps::InstantiateMsg {
                credit_manager: credit_manager_contract.to_string(),
                oracle: OracleBase::new(oracle_contract.to_string()),
                base_denom: self.perps_base_denom.clone(),
                min_position_value: self.min_position_value,
            },
            &[],
            "mock-perps",
            None,
        )?;

        Ok(MockEnv {
            app: take(&mut self.app),
            owner: self.deployer.clone(),
            perps: perps_contract,
            oracle: oracle_contract,
            credit_manager: credit_manager_contract,
        })
    }

    fn deploy_oracle(&mut self) -> Addr {
        let contract = mock_oracle_contract();
        let code_id = self.app.store_code(contract);

        self.app
            .instantiate_contract(
                code_id,
                self.deployer.clone(),
                &oracle::InstantiateMsg::<Empty> {
                    owner: self.deployer.clone().to_string(),
                    base_denom: self.oracle_base_denom.clone(),
                    custom_init: None,
                },
                &[],
                "mock-oracle",
                None,
            )
            .unwrap()
    }

    fn deploy_credit_manager(&mut self) -> Addr {
        let contract = mock_credit_manager_contract();
        let code_id = self.app.store_code(contract);

        self.app
            .instantiate_contract(
                code_id,
                self.deployer.clone(),
                &Empty {},
                &[],
                "mock-credit-manager",
                None,
            )
            .unwrap()
    }

    //--------------------------------------------------------------------------------------------------
    // Setter functions
    //--------------------------------------------------------------------------------------------------

    pub fn oracle_base_denom(&mut self, denom: &str) -> &mut Self {
        self.oracle_base_denom = denom.to_string();
        self
    }

    pub fn perps_base_denom(&mut self, denom: &str) -> &mut Self {
        self.perps_base_denom = denom.to_string();
        self
    }

    pub fn min_position_value(&mut self, mpv: Uint128) -> &mut Self {
        self.min_position_value = mpv;
        self
    }
}
