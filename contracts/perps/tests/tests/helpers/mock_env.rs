#![allow(dead_code)]
use std::mem::take;

use anyhow::Result as AnyResult;
use cosmwasm_std::{coin, Addr, Coin, Decimal, Empty, Timestamp, Uint128};
use cw_multi_test::{App, AppResponse, BankSudo, BasicApp, Executor, SudoMsg};
use cw_paginate::PaginationResponse;
use mars_oracle_osmosis::OsmosisPriceSourceUnchecked;
use mars_owner::{OwnerResponse, OwnerUpdate};
use mars_types::{
    adapters::{oracle::OracleBase, params::ParamsBase},
    address_provider,
    oracle::{self, ActionKind},
    params::{self, ExecuteMsg::UpdatePerpParams, PerpParamsUpdate},
    perps::{
        self, Accounting, Config, DenomStateResponse, PerpDenomState, PerpVaultDeposit,
        PerpVaultPosition, PerpVaultUnlock, PnlAmounts, PnlValues, PositionFeesResponse,
        PositionResponse, PositionsByAccountResponse, TradingFee, VaultResponse,
    },
    signed_uint::SignedUint,
};

use super::{
    contracts::{mock_oracle_contract, mock_perps_contract},
    mock_address_provider_contract, mock_credit_manager_contract, mock_params_contract,
};

pub const ONE_HOUR_SEC: u64 = 3600u64;

pub struct MockEnv {
    app: BasicApp,
    pub owner: Addr,
    pub perps: Addr,
    pub oracle: Addr,
    pub params: Addr,
    pub credit_manager: Addr,
}

pub struct MockEnvBuilder {
    app: BasicApp,
    deployer: Addr,
    oracle_base_denom: String,
    perps_base_denom: String,
    cooldown_period: u64,
    max_positions: u8,
}

#[allow(clippy::new_ret_no_self)]
impl MockEnv {
    pub fn new() -> MockEnvBuilder {
        MockEnvBuilder {
            app: App::default(),
            deployer: Addr::unchecked("deployer"),
            oracle_base_denom: "uusd".to_string(),
            perps_base_denom: "uusdc".to_string(),
            cooldown_period: 3600,
            max_positions: 4,
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

    pub fn query_balance(&self, addr: &Addr, denom: &str) -> Coin {
        self.app.wrap().query_balance(addr.clone(), denom).unwrap()
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

    pub fn set_block_time(&mut self, seconds: u64) {
        self.app.update_block(|block| {
            block.time = Timestamp::from_seconds(seconds);
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
        skew_scale: Uint128,
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

    pub fn deposit_to_vault(
        &mut self,
        sender: &Addr,
        account_id: Option<&str>,
        funds: &[Coin],
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.perps.clone(),
            &perps::ExecuteMsg::Deposit {
                account_id: account_id.map(|s| s.to_string()),
            },
            funds,
        )
    }

    pub fn unlock_from_vault(
        &mut self,
        sender: &Addr,
        account_id: Option<&str>,
        shares: Uint128,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.perps.clone(),
            &perps::ExecuteMsg::Unlock {
                account_id: account_id.map(|s| s.to_string()),
                shares,
            },
            &[],
        )
    }

    pub fn withdraw_from_vault(
        &mut self,
        sender: &Addr,
        account_id: Option<&str>,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.perps.clone(),
            &perps::ExecuteMsg::Withdraw {
                account_id: account_id.map(|s| s.to_string()),
            },
            &[],
        )
    }

    pub fn execute_perp_order(
        &mut self,
        sender: &Addr,
        account_id: &str,
        denom: &str,
        size: SignedUint,
        reduce_only: Option<bool>,
        funds: &[Coin],
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.perps.clone(),
            &perps::ExecuteMsg::ExecutePerpOrder {
                account_id: account_id.to_string(),
                denom: denom.to_string(),
                size,
                reduce_only,
            },
            funds,
        )
    }

    pub fn close_all_positions(
        &mut self,
        sender: &Addr,
        account_id: &str,
        funds: &[Coin],
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.perps.clone(),
            &perps::ExecuteMsg::CloseAllPositions {
                account_id: account_id.to_string(),
                action: None,
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

    pub fn update_perp_params(&mut self, sender: &Addr, update: PerpParamsUpdate) {
        self.app
            .execute_contract(sender.clone(), self.params.clone(), &UpdatePerpParams(update), &[])
            .unwrap();
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

    pub fn query_vault(&self) -> VaultResponse {
        self.app
            .wrap()
            .query_wasm_smart(
                self.perps.clone(),
                &perps::QueryMsg::Vault {
                    action: None,
                },
            )
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

    pub fn query_perp_denom_state(&self, denom: &str, action: ActionKind) -> PerpDenomState {
        self.app
            .wrap()
            .query_wasm_smart(
                self.perps.clone(),
                &perps::QueryMsg::PerpDenomState {
                    denom: denom.to_string(),
                    action,
                },
            )
            .unwrap()
    }

    pub fn query_perp_denom_states(
        &self,
        action: ActionKind,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> PaginationResponse<PerpDenomState> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.perps.clone(),
                &perps::QueryMsg::PerpDenomStates {
                    action,
                    start_after,
                    limit,
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

    pub fn query_cm_deposit(&self, account_id: &str) -> PerpVaultDeposit {
        self.query_deposit(self.credit_manager.as_str(), Some(account_id))
    }

    pub fn query_deposit(&self, user_address: &str, account_id: Option<&str>) -> PerpVaultDeposit {
        self.app
            .wrap()
            .query_wasm_smart(
                self.perps.clone(),
                &perps::QueryMsg::Deposit {
                    user_address: user_address.to_string(),
                    account_id: account_id.map(|s| s.to_string()),
                },
            )
            .unwrap()
    }

    pub fn query_cm_unlocks(&self, account_id: &str) -> Vec<PerpVaultUnlock> {
        self.query_unlocks(self.credit_manager.as_str(), Some(account_id))
    }

    pub fn query_unlocks(
        &self,
        user_address: &str,
        account_id: Option<&str>,
    ) -> Vec<PerpVaultUnlock> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.perps.clone(),
                &perps::QueryMsg::Unlocks {
                    user_address: user_address.to_string(),
                    account_id: account_id.map(|s| s.to_string()),
                },
            )
            .unwrap()
    }

    pub fn query_cm_vault_position(
        &self,
        account_id: &str,
        action: ActionKind,
    ) -> Option<PerpVaultPosition> {
        self.query_vault_position(self.credit_manager.as_str(), Some(account_id), action)
    }

    pub fn query_vault_position(
        &self,
        user_address: &str,
        account_id: Option<&str>,
        action: ActionKind,
    ) -> Option<PerpVaultPosition> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.perps.clone(),
                &perps::QueryMsg::PerpVaultPosition {
                    user_address: user_address.to_string(),
                    account_id: account_id.map(|s| s.to_string()),
                    action: Some(action),
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
                    order_size: None,
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

    pub fn query_positions_by_account_id(
        &self,
        account_id: &str,
        action: ActionKind,
    ) -> PositionsByAccountResponse {
        self.app
            .wrap()
            .query_wasm_smart(
                self.perps.clone(),
                &perps::QueryMsg::PositionsByAccount {
                    account_id: account_id.to_string(),
                    action: Some(action),
                },
            )
            .unwrap()
    }

    pub fn query_total_pnl(&self) -> PnlValues {
        self.app.wrap().query_wasm_smart(self.perps.clone(), &perps::QueryMsg::TotalPnl {}).unwrap()
    }

    pub fn query_denom_accounting(&self, denom: &str) -> Accounting {
        self.app
            .wrap()
            .query_wasm_smart(
                self.perps.clone(),
                &perps::QueryMsg::DenomAccounting {
                    denom: denom.to_string(),
                },
            )
            .unwrap()
    }

    pub fn query_total_accounting(&self) -> Accounting {
        self.app
            .wrap()
            .query_wasm_smart(self.perps.clone(), &perps::QueryMsg::TotalAccounting {})
            .unwrap()
    }

    pub fn query_denom_realized_pnl_for_account(
        &self,
        account_id: &str,
        denom: &str,
    ) -> PnlAmounts {
        self.app
            .wrap()
            .query_wasm_smart(
                self.perps.clone(),
                &perps::QueryMsg::DenomRealizedPnlForAccount {
                    account_id: account_id.to_string(),
                    denom: denom.to_string(),
                },
            )
            .unwrap()
    }

    pub fn query_opening_fee(&self, denom: &str, size: SignedUint) -> TradingFee {
        self.app
            .wrap()
            .query_wasm_smart(
                self.perps.clone(),
                &perps::QueryMsg::OpeningFee {
                    denom: denom.to_string(),
                    size,
                },
            )
            .unwrap()
    }

    pub fn query_position_fees(
        &self,
        account_id: &str,
        denom: &str,
        new_size: SignedUint,
    ) -> PositionFeesResponse {
        self.app
            .wrap()
            .query_wasm_smart(
                self.perps.clone(),
                &perps::QueryMsg::PositionFees {
                    account_id: account_id.to_string(),
                    denom: denom.to_string(),
                    new_size,
                },
            )
            .unwrap()
    }
}

impl MockEnvBuilder {
    pub fn build(&mut self) -> AnyResult<MockEnv> {
        let address_provider_contract = self.deploy_address_provider();
        let oracle_contract = self.deploy_oracle();
        let params_contract = self.deploy_params(address_provider_contract.as_str());
        let credit_manager_contract = self.deploy_credit_manager();

        let code_id = self.app.store_code(mock_perps_contract());
        let perps_contract = self.app.instantiate_contract(
            code_id,
            self.deployer.clone(),
            &perps::InstantiateMsg {
                credit_manager: credit_manager_contract.to_string(),
                oracle: OracleBase::new(oracle_contract.to_string()),
                params: ParamsBase::new(params_contract.to_string()),
                base_denom: self.perps_base_denom.clone(),
                cooldown_period: self.cooldown_period,
                max_positions: self.max_positions,
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
            params: params_contract,
            credit_manager: credit_manager_contract,
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

    fn deploy_params(&mut self, address_provider: &str) -> Addr {
        let contract = mock_params_contract();
        let code_id = self.app.store_code(contract);

        self.app
            .instantiate_contract(
                code_id,
                self.deployer.clone(),
                &params::InstantiateMsg {
                    owner: self.deployer.clone().to_string(),
                    address_provider: address_provider.to_string(),
                },
                &[],
                "mock-params",
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

    pub fn cooldown_period(&mut self, cp: u64) -> &mut Self {
        self.cooldown_period = cp;
        self
    }

    pub fn max_positions(&mut self, max_positions: u8) -> &mut Self {
        self.max_positions = max_positions;
        self
    }
}
