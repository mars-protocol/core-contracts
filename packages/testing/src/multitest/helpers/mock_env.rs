use std::default::Default;

use anyhow::Result as AnyResult;
use cosmwasm_std::{
    coin, coins, testing::MockApi, Addr, Coin, ContractInfoResponse, Decimal, Empty, Int128,
    StdError, StdResult, Timestamp, Uint128,
};
use cw721::TokensResponse;
use cw721_base::{Action::TransferOwnership, Ownership};
use cw_multi_test::{no_init, AppResponse, BankSudo, BasicAppBuilder, Executor, SudoMsg};
use cw_paginate::PaginationResponse;
use cw_vault_standard::{
    extensions::lockup::{LockupQueryMsg, UnlockingPosition},
    msg::{ExtensionQueryMsg, VaultStandardQueryMsg::VaultExtension},
};
use mars_mock_oracle::msg::{
    CoinPrice, ExecuteMsg as OracleExecuteMsg, InstantiateMsg as OracleInstantiateMsg,
};
use mars_mock_vault::{
    contract::DEFAULT_VAULT_TOKEN_PREFUND, msg::InstantiateMsg as VaultInstantiateMsg,
};
use mars_owner::OwnerUpdate;
use mars_types::{
    account_nft::{
        ExecuteMsg as NftExecuteMsg, InstantiateMsg as NftInstantiateMsg, NftConfigUpdates,
        QueryMsg as NftQueryMsg, UncheckedNftConfig,
    },
    adapters::{
        account_nft::AccountNftUnchecked,
        health::HealthContract,
        incentives::{Incentives, IncentivesUnchecked},
        oracle::{Oracle, OracleBase, OracleUnchecked},
        params::Params,
        perps::Perps,
        red_bank::RedBankUnchecked,
        swapper::{Swapper, SwapperBase},
        vault::{Vault, VaultPosition, VaultPositionValue as VPositionValue, VaultUnchecked},
        zapper::{Zapper, ZapperBase},
    },
    address_provider::{self, AddressResponseItem, MarsAddressType},
    credit_manager::{
        Account, Action, CallbackMsg, CoinBalanceResponseItem, ConfigResponse, ConfigUpdates,
        DebtShares, ExecuteMsg, InstantiateMsg, KeeperFeeConfig, Positions,
        QueryMsg::{self, EstimateProvideLiquidity, VaultPositionValue},
        SharesResponseItem, TriggerOrderResponse, VaultBinding, VaultPositionResponseItem,
        VaultUtilizationResponse,
    },
    health::{
        AccountKind, ExecuteMsg::UpdateConfig, HealthValuesResponse,
        InstantiateMsg as HealthInstantiateMsg, QueryMsg::HealthValues,
    },
    incentives::{
        ExecuteMsg::{BalanceChange, SetAssetIncentive},
        IncentiveKind,
        QueryMsg::{StakedAstroLpPosition, StakedAstroLpRewards, UserUnclaimedRewards},
        StakedLpPositionResponse,
    },
    oracle::{ActionKind, PriceResponse, QueryMsg::Price as OraclePrice},
    params::{
        AssetParams,
        AssetParamsUpdate::{self, AddOrUpdate},
        ExecuteMsg::{
            UpdateAssetParams, UpdateManagedVaultConfig, UpdatePerpParams, UpdateVaultConfig,
        },
        InstantiateMsg as ParamsInstantiateMsg, ManagedVaultConfigResponse,
        ManagedVaultConfigUpdate, PerpParamsUpdate, QueryMsg as ParamsQueryMsg, VaultConfig,
        VaultConfigUnchecked, VaultConfigUpdate,
    },
    perps::{
        self, Config, InstantiateMsg as PerpsInstantiateMsg, PnL, PositionResponse, TradingFee,
        VaultPositionResponse, VaultResponse,
    },
    red_bank::{
        self, InitOrUpdateAssetParams, InterestRateModel,
        QueryMsg::{UserCollateral, UserDebt},
        UserCollateralResponse, UserDebtResponse,
    },
    rewards_collector::{self, RewardConfig, TransferType},
    swapper::{
        EstimateExactInSwapResponse, InstantiateMsg as SwapperInstantiateMsg,
        QueryMsg::EstimateExactInSwap, SwapperRoute,
    },
};
use mars_vault::{
    msg::InstantiateMsg as ManagedVaultInstantiateMsg, performance_fee::PerformanceFeeConfig,
};
use mars_zapper_mock::msg::{InstantiateMsg as ZapperInstantiateMsg, LpConfig};

use super::{
    lp_token_info, mock_account_nft_contract, mock_address_provider_contract,
    mock_astro_incentives_contract, mock_health_contract, mock_incentives_contract,
    mock_managed_vault_contract, mock_oracle_contract, mock_params_contract, mock_perps_contract,
    mock_red_bank_contract, mock_rover_contract, mock_swapper_contract, mock_v2_zapper_contract,
    mock_vault_contract, AccountToFund, CoinInfo, VaultTestInfo, ASTRO_LP_DENOM,
};
use crate::{
    integration::mock_contracts::mock_rewards_collector_osmosis_contract,
    multitest::modules::token_factory::{CustomApp, TokenFactory},
};

pub const DEFAULT_RED_BANK_COIN_BALANCE: Uint128 = Uint128::new(100_000_000);

pub struct MockEnv {
    pub app: CustomApp,
    pub rover: Addr,
    pub mars_oracle: Addr,
    pub health_contract: HealthContract,
    pub incentives: Incentives,
    pub params: Params,
    pub perps: Perps,
    pub address_provider: Addr,
}

pub struct MockEnvBuilder {
    pub app: CustomApp,
    pub owner: Option<Addr>,
    pub emergency_owner: Option<Addr>,
    pub vault_configs: Option<Vec<VaultTestInfo>>,
    pub coin_params: Option<Vec<CoinInfo>>,
    pub address_provider: Option<Addr>,
    pub oracle: Option<Oracle>,
    pub params: Option<Params>,
    pub red_bank: Option<RedBankUnchecked>,
    pub incentives: Option<IncentivesUnchecked>,
    pub deploy_nft_contract: bool,
    pub set_nft_contract_minter: bool,
    pub accounts_to_fund: Vec<AccountToFund>,
    pub max_unlocking_positions: Option<Uint128>,
    pub max_slippage: Option<Decimal>,
    pub health_contract: Option<HealthContract>,
    pub evil_vault: Option<String>,
    pub target_vault_collateralization_ratio: Option<Decimal>,
    pub deleverage_enabled: Option<bool>,
    pub withdraw_enabled: Option<bool>,
    pub keeper_fee_config: Option<KeeperFeeConfig>,
    pub perps_liquidation_bonus_ratio: Option<Decimal>,
    pub perps_protocol_fee_ratio: Option<Decimal>,
}

#[allow(clippy::new_ret_no_self)]
impl MockEnv {
    pub fn new() -> MockEnvBuilder {
        let tf_default = TokenFactory::default();
        let app = BasicAppBuilder::new().with_stargate(tf_default).build(no_init);

        MockEnvBuilder {
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
        }
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

    pub fn fund_addr(&mut self, addr: &Addr, funds: Vec<Coin>) {
        self.app
            .sudo(SudoMsg::Bank(BankSudo::Mint {
                to_address: addr.to_string(),
                amount: funds,
            }))
            .unwrap();
    }

    pub fn query_code_id(&self, addr: &Addr) -> u64 {
        let res: ContractInfoResponse = self.app.wrap().query_wasm_contract_info(addr).unwrap();
        res.code_id
    }

    //--------------------------------------------------------------------------------------------------
    // Execute Msgs
    //--------------------------------------------------------------------------------------------------

    pub fn update_credit_account(
        &mut self,
        account_id: &str,
        sender: &Addr,
        actions: Vec<Action>,
        send_funds: &[Coin],
    ) -> AnyResult<AppResponse> {
        self.update_credit_account_with_new_acc(
            Some(account_id.to_string()),
            None,
            sender,
            actions,
            send_funds,
        )
    }

    pub fn update_credit_account_with_new_acc(
        &mut self,
        account_id: Option<String>,
        account_kind: Option<AccountKind>,
        sender: &Addr,
        actions: Vec<Action>,
        send_funds: &[Coin],
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.rover.clone(),
            &ExecuteMsg::UpdateCreditAccount {
                account_id,
                account_kind,
                actions,
            },
            send_funds,
        )
    }

    pub fn invoke_callback(
        &mut self,
        sender: &Addr,
        callback: CallbackMsg,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.rover.clone(),
            &ExecuteMsg::Callback(callback),
            &[],
        )
    }

    pub fn repay_from_wallet(
        &mut self,
        sender: &Addr,
        account_id: &str,
        funds: &[Coin],
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.rover.clone(),
            &ExecuteMsg::RepayFromWallet {
                account_id: account_id.to_string(),
            },
            funds,
        )
    }

    pub fn execute_trigger_order(
        &mut self,
        sender: &Addr,
        account_id: &str,
        trigger_order_id: &str,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.rover.clone(),
            &ExecuteMsg::ExecuteTriggerOrder {
                account_id: account_id.to_string(),
                trigger_order_id: trigger_order_id.to_string(),
            },
            &[],
        )
    }

    pub fn update_config(
        &mut self,
        sender: &Addr,
        updates: ConfigUpdates,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.rover.clone(),
            &ExecuteMsg::UpdateConfig {
                updates,
            },
            &[],
        )
    }

    pub fn update_balance_after_deleverage(
        &mut self,
        sender: &Addr,
        funds: &[Coin],
        account_id: &str,
        pnl: PnL,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.rover.clone(),
            &ExecuteMsg::UpdateBalanceAfterDeleverage {
                account_id: account_id.to_string(),
                pnl,
            },
            funds,
        )
    }

    pub fn update_asset_params(&mut self, update: AssetParamsUpdate) {
        let config = self.query_config();
        self.app
            .execute_contract(
                Addr::unchecked(config.ownership.owner.unwrap()),
                Addr::unchecked(config.params),
                &UpdateAssetParams(update),
                &[],
            )
            .unwrap();
    }

    pub fn update_vault_params(&mut self, update: VaultConfigUpdate) {
        let config = self.query_config();
        self.app
            .execute_contract(
                Addr::unchecked(config.ownership.owner.unwrap()),
                Addr::unchecked(config.params),
                &UpdateVaultConfig(update),
                &[],
            )
            .unwrap();
    }

    pub fn update_perp_params(&mut self, update: PerpParamsUpdate) {
        let config = self.query_config();
        self.app
            .execute_contract(
                Addr::unchecked(config.ownership.owner.unwrap()),
                Addr::unchecked(config.params),
                &UpdatePerpParams(update),
                &[],
            )
            .unwrap();
    }

    pub fn update_managed_vault_config(&mut self, update: ManagedVaultConfigUpdate) {
        let config = self.query_config();
        self.app
            .execute_contract(
                Addr::unchecked(config.ownership.owner.unwrap()),
                Addr::unchecked(config.params),
                &UpdateManagedVaultConfig(update),
                &[],
            )
            .unwrap();
    }

    pub fn update_nft_config(
        &mut self,
        sender: &Addr,
        config: Option<NftConfigUpdates>,
        ownership: Option<cw721_base::Action>,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.rover.clone(),
            &ExecuteMsg::UpdateNftConfig {
                config,
                ownership,
            },
            &[],
        )
    }

    pub fn deploy_new_nft_contract(&mut self) -> AnyResult<AccountNftUnchecked> {
        let nft_minter = Addr::unchecked("original_nft_minter");
        let nft_contract = deploy_nft_contract(&mut self.app, &nft_minter);
        propose_new_nft_minter(
            &mut self.app,
            nft_contract.clone(),
            &nft_minter.clone(),
            &self.rover.clone(),
        );
        Ok(AccountNftUnchecked::new(nft_contract.to_string()))
    }

    pub fn create_credit_account(&mut self, sender: &Addr) -> AnyResult<String> {
        self._create_credit_account(sender, AccountKind::Default)
    }

    pub fn create_hls_account(&mut self, sender: &Addr) -> String {
        self._create_credit_account(sender, AccountKind::HighLeveredStrategy).unwrap()
    }

    pub fn create_fund_manager_account(&mut self, sender: &Addr, vault: &Addr) -> String {
        self._create_credit_account(
            sender,
            AccountKind::FundManager {
                vault_addr: vault.to_string(),
            },
        )
        .unwrap()
    }

    fn _create_credit_account(&mut self, sender: &Addr, kind: AccountKind) -> AnyResult<String> {
        let res = self.app.execute_contract(
            sender.clone(),
            self.rover.clone(),
            &ExecuteMsg::CreateCreditAccount(kind),
            &[],
        )?;
        Ok(self.get_account_id(res))
    }

    pub fn create_fund_manager_account_with_error(
        &mut self,
        sender: &Addr,
        vault: &Addr,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.rover.clone(),
            &ExecuteMsg::CreateCreditAccount(AccountKind::FundManager {
                vault_addr: vault.to_string(),
            }),
            &[],
        )
    }

    pub fn get_account_id(&mut self, res: AppResponse) -> String {
        let attr: Vec<&String> = res
            .events
            .iter()
            .flat_map(|event| &event.attributes)
            .filter(|attr| attr.key == "token_id")
            .map(|attr| &attr.value)
            .collect();

        assert_eq!(attr.len(), 1);
        attr.first().unwrap().to_string()
    }

    pub fn price_change(&mut self, coin: CoinPrice) {
        self.app
            .execute_contract(
                Addr::unchecked("anyone"),
                self.mars_oracle.clone(),
                &OracleExecuteMsg::ChangePrice(coin),
                &[],
            )
            .unwrap();
    }

    pub fn remove_price(&mut self, denom: &str, pricing: ActionKind) {
        self.app
            .execute_contract(
                Addr::unchecked("anyone"),
                self.mars_oracle.clone(),
                &OracleExecuteMsg::RemovePrice {
                    denom: denom.to_string(),
                    pricing,
                },
                &[],
            )
            .unwrap();
    }

    pub fn execute_callback(&mut self, sender: &Addr, msg: CallbackMsg) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.rover.clone(),
            &ExecuteMsg::Callback(msg),
            &[],
        )
    }

    pub fn update_owner(&mut self, sender: &Addr, update: OwnerUpdate) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.rover.clone(),
            &ExecuteMsg::UpdateOwner(update),
            &[],
        )
    }

    pub fn add_incentive_reward(&mut self, account_id: &str, kind: &IncentiveKind, coin: Coin) {
        // Register reward in mock contract
        self.app
            .execute_contract(
                self.rover.clone(),
                self.incentives.addr.clone(),
                &BalanceChange {
                    user_addr: self.rover.clone(),
                    account_id: Some(account_id.to_string()),
                    kind: kind.clone(),
                    denom: coin.denom.clone(),
                    user_amount: coin.amount,
                    total_amount: Default::default(),
                },
                &[],
            )
            .unwrap();

        // Mint token for incentives contract so it can be claimed
        self.app
            .sudo(SudoMsg::Bank(BankSudo::Mint {
                to_address: self.incentives.addr.to_string(),
                amount: vec![coin],
            }))
            .unwrap();
    }

    pub fn add_astro_incentive_reward(&mut self, account_id: &str, lp_denom: &str, coin: Coin) {
        // This is a bit of a hack to set up astroport lp rewards in our mock contract, using the existing API.
        self.app
            .execute_contract(
                self.rover.clone(),
                self.incentives.addr.clone(),
                &SetAssetIncentive {
                    kind: IncentiveKind::RedBank,
                    denom: lp_denom.to_string(),
                    incentive_denom: coin.denom.clone(),
                    // Emision per second is used for amount
                    emission_per_second: coin.amount,
                    // Start time is used for account_id. The account id is parsed as a u64
                    start_time: account_id.parse().unwrap(),
                    duration: Default::default(),
                },
                &[],
            )
            .unwrap();

        // Mint token for incentives contract so it can be claimed
        self.app
            .sudo(SudoMsg::Bank(BankSudo::Mint {
                to_address: self.incentives.addr.to_string(),
                amount: vec![coin],
            }))
            .unwrap();
    }

    pub fn deposit_to_perp_vault(
        &mut self,
        account_id: &str,
        coin: &Coin,
        max_shares_receivable: Option<Uint128>,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            self.rover.clone(),
            self.perps.address().clone(),
            &perps::ExecuteMsg::Deposit {
                account_id: Some(account_id.to_string()),
                max_shares_receivable,
            },
            &[coin.clone()],
        )
    }

    pub fn deleverage(&mut self, account_id: &str, denom: &str) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            self.rover.clone(),
            self.perps.address().clone(),
            &perps::ExecuteMsg::Deleverage {
                account_id: account_id.to_string(),
                denom: denom.to_string(),
            },
            &[],
        )
    }

    //--------------------------------------------------------------------------------------------------
    // Queries
    //--------------------------------------------------------------------------------------------------

    pub fn query_positions(&self, account_id: &str) -> Positions {
        self.query_positions_with_action(account_id, None).unwrap()
    }

    pub fn query_positions_with_action(
        &self,
        account_id: &str,
        action: Option<ActionKind>,
    ) -> Result<Positions, StdError> {
        self.app.wrap().query_wasm_smart(
            self.rover.clone(),
            &QueryMsg::Positions {
                account_id: account_id.to_string(),
                action,
            },
        )
    }

    pub fn query_health(&self, account_id: &str, action: ActionKind) -> HealthValuesResponse {
        self.app
            .wrap()
            .query_wasm_smart(
                self.health_contract.clone().address(),
                &HealthValues {
                    account_id: account_id.to_string(),
                    action,
                },
            )
            .unwrap()
    }

    pub fn query_unclaimed_astroport_rewards_for_lp(
        &self,
        account_id: &str,
        lp_denom: &str,
    ) -> Vec<Coin> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.incentives.clone().addr,
                &StakedAstroLpRewards {
                    account_id: account_id.to_string(),
                    lp_denom: lp_denom.to_string(),
                },
            )
            .unwrap()
    }

    pub fn query_unclaimed_rewards(&self, account_id: &str) -> Vec<Coin> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.incentives.clone().addr,
                &UserUnclaimedRewards {
                    user: self.rover.to_string(),
                    account_id: Some(account_id.to_string()),
                    start_after_kind: None,
                    start_after_denom: None,
                    start_after_incentive_denom: None,
                    limit: None,
                },
            )
            .unwrap()
    }

    pub fn query_staked_astro_lp_rewards(&self, account_id: &str, lp_denom: &str) -> Vec<Coin> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.incentives.clone().addr,
                &StakedAstroLpRewards {
                    account_id: account_id.to_string(),
                    lp_denom: lp_denom.to_string(),
                },
            )
            .unwrap()
    }

    pub fn query_staked_lp_position(
        &self,
        account_id: &str,
        lp_denom: &str,
    ) -> StakedLpPositionResponse {
        self.app
            .wrap()
            .query_wasm_smart(
                self.incentives.clone().addr,
                &StakedAstroLpPosition {
                    account_id: account_id.to_string(),
                    lp_denom: lp_denom.to_string(),
                },
            )
            .unwrap()
    }

    pub fn query_balance(&self, addr: &Addr, denom: &str) -> Coin {
        self.app.wrap().query_balance(addr.clone(), denom).unwrap()
    }

    pub fn query_config(&self) -> ConfigResponse {
        self.app.wrap().query_wasm_smart(self.rover.clone(), &QueryMsg::Config {}).unwrap()
    }

    pub fn query_accounts(
        &self,
        owner: &str,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Vec<Account> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.rover.clone(),
                &QueryMsg::Accounts {
                    owner: owner.to_string(),
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn query_account_kind(&self, account_id: &str) -> AccountKind {
        self.app
            .wrap()
            .query_wasm_smart(
                self.rover.clone(),
                &QueryMsg::AccountKind {
                    account_id: account_id.to_string(),
                },
            )
            .unwrap()
    }

    pub fn query_nft_config(&self) -> UncheckedNftConfig {
        let config = self.query_config();
        self.app
            .wrap()
            .query_wasm_smart(config.account_nft.unwrap(), &NftQueryMsg::Config {})
            .unwrap()
    }

    pub fn query_nft_ownership(&self) -> Ownership<Addr> {
        let config = self.query_config();
        self.app
            .wrap()
            .query_wasm_smart(config.account_nft.unwrap(), &NftQueryMsg::Ownership {})
            .unwrap()
    }

    pub fn query_rewards_collector_account(&self) -> String {
        let config = self.query_config();
        let response: TokensResponse = self
            .app
            .wrap()
            .query_wasm_smart(
                config.account_nft.unwrap(),
                &NftQueryMsg::Tokens {
                    owner: config.rewards_collector.unwrap().address,
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();
        response.tokens.first().unwrap().to_string()
    }

    pub fn query_vault_params(&self, vault_addr: &str) -> VaultConfig {
        self.app
            .wrap()
            .query_wasm_smart(
                self.params.address(),
                &ParamsQueryMsg::VaultConfig {
                    address: vault_addr.to_string(),
                },
            )
            .unwrap()
    }

    pub fn query_asset_params(&self, denom: &str) -> AssetParams {
        self.app
            .wrap()
            .query_wasm_smart(
                self.params.address(),
                &ParamsQueryMsg::AssetParams {
                    denom: denom.to_string(),
                },
            )
            .unwrap()
    }

    pub fn query_managed_vault_config(&self) -> ManagedVaultConfigResponse {
        self.app
            .wrap()
            .query_wasm_smart(self.params.address(), &ParamsQueryMsg::ManagedVaultConfig {})
            .unwrap()
    }

    pub fn query_all_vault_params(&self) -> Vec<VaultConfig> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.params.address(),
                &ParamsQueryMsg::AllVaultConfigs {
                    start_after: None,
                    limit: Some(30), // Max limit
                },
            )
            .unwrap()
    }

    pub fn get_vault(&self, vault: &VaultTestInfo) -> VaultUnchecked {
        let vault_params = self.query_all_vault_params();
        let matched_vault = vault_params
            .iter()
            .find(|v| {
                let info = Vault::new(v.addr.clone()).query_info(&self.app.wrap()).unwrap();
                vault.vault_token_denom == info.vault_token
            })
            .unwrap();
        VaultUnchecked::new(matched_vault.addr.to_string())
    }

    pub fn query_vault_utilization(
        &self,
        vault: &VaultUnchecked,
    ) -> StdResult<VaultUtilizationResponse> {
        self.app.wrap().query_wasm_smart(
            self.rover.clone(),
            &QueryMsg::VaultUtilization {
                vault: vault.clone(),
            },
        )
    }

    pub fn query_all_vault_utilizations(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> StdResult<PaginationResponse<VaultUtilizationResponse>> {
        self.app.wrap().query_wasm_smart(
            self.rover.clone(),
            &QueryMsg::AllVaultUtilizations {
                start_after,
                limit,
            },
        )
    }

    pub fn query_all_coin_balances(
        &self,
        start_after: Option<(String, String)>,
        limit: Option<u32>,
    ) -> Vec<CoinBalanceResponseItem> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.rover.clone(),
                &QueryMsg::AllCoinBalances {
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn query_all_debt_shares(
        &self,
        start_after: Option<(String, String)>,
        limit: Option<u32>,
    ) -> Vec<SharesResponseItem> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.rover.clone(),
                &QueryMsg::AllDebtShares {
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn query_all_total_debt_shares(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Vec<DebtShares> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.rover.clone(),
                &QueryMsg::AllTotalDebtShares {
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn query_total_debt_shares(&self, denom: &str) -> DebtShares {
        self.app
            .wrap()
            .query_wasm_smart(self.rover.clone(), &QueryMsg::TotalDebtShares(denom.to_string()))
            .unwrap()
    }

    pub fn query_red_bank_debt(&self, denom: &str) -> UserDebtResponse {
        let config = self.query_config();
        self.app
            .wrap()
            .query_wasm_smart(
                config.red_bank,
                &UserDebt {
                    user: self.rover.to_string(),
                    denom: denom.into(),
                },
            )
            .unwrap()
    }

    pub fn query_red_bank_collateral(
        &self,
        account_id: &str,
        denom: &str,
    ) -> UserCollateralResponse {
        let config = self.query_config();
        self.app
            .wrap()
            .query_wasm_smart(
                config.red_bank,
                &UserCollateral {
                    user: self.rover.to_string(),
                    account_id: Some(account_id.to_string()),
                    denom: denom.into(),
                },
            )
            .unwrap()
    }

    pub fn query_preview_redeem(&self, vault: &VaultUnchecked, shares: Uint128) -> Uint128 {
        vault
            .check(&MockApi::default())
            .unwrap()
            .query_preview_redeem(&self.app.wrap(), shares)
            .unwrap()
    }

    pub fn query_unlocking_position(&self, vault: &VaultUnchecked, id: u64) -> UnlockingPosition {
        vault
            .check(&MockApi::default())
            .unwrap()
            .query_unlocking_position(&self.app.wrap(), id)
            .unwrap()
    }

    pub fn query_unlocking_positions(
        &self,
        vault: &VaultUnchecked,
        addr: &Addr,
    ) -> Vec<UnlockingPosition> {
        self.app
            .wrap()
            .query_wasm_smart(
                vault.address.to_string(),
                &VaultExtension(ExtensionQueryMsg::Lockup(LockupQueryMsg::UnlockingPositions {
                    owner: addr.to_string(),
                    start_after: None,
                    limit: None,
                })),
            )
            .unwrap()
    }

    pub fn query_total_vault_coin_balance(&self, vault: &VaultUnchecked) -> Uint128 {
        let info = Vault::new(Addr::unchecked(vault.address.clone()))
            .query_info(&self.app.wrap())
            .unwrap();
        self.app.wrap().query_balance(self.rover.clone(), info.vault_token).unwrap().amount
    }

    pub fn query_all_vault_positions(
        &self,
        start_after: Option<(String, String)>,
        limit: Option<u32>,
    ) -> Vec<VaultPositionResponseItem> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.rover.clone(),
                &QueryMsg::AllVaultPositions {
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn query_trigger_orders_for_account(
        &self,
        account_id: String,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> PaginationResponse<TriggerOrderResponse> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.rover.clone(),
                &QueryMsg::AllAccountTriggerOrders {
                    account_id,
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn query_all_trigger_orders(
        &self,
        start_after: Option<(String, String)>,
        limit: Option<u32>,
    ) -> PaginationResponse<TriggerOrderResponse> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.rover.clone(),
                &QueryMsg::AllTriggerOrders {
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn query_swap_estimate(
        &self,
        coin_in: &Coin,
        denom_out: &str,
        route: SwapperRoute,
    ) -> EstimateExactInSwapResponse {
        self.query_swap_estimate_with_optional_route(coin_in, denom_out, Some(route))
    }

    pub fn query_swap_estimate_with_optional_route(
        &self,
        coin_in: &Coin,
        denom_out: &str,
        route: Option<SwapperRoute>,
    ) -> EstimateExactInSwapResponse {
        let config = self.query_config();
        self.app
            .wrap()
            .query_wasm_smart(
                config.swapper,
                &EstimateExactInSwap {
                    coin_in: coin_in.clone(),
                    denom_out: denom_out.to_string(),
                    route,
                },
            )
            .unwrap()
    }

    pub fn estimate_provide_liquidity(&self, lp_token_out: &str, coins_in: &[Coin]) -> Uint128 {
        let config = self.query_config();
        self.app
            .wrap()
            .query_wasm_smart(
                config.zapper,
                &EstimateProvideLiquidity {
                    lp_token_out: lp_token_out.to_string(),
                    coins_in: coins_in.to_vec(),
                },
            )
            .unwrap()
    }

    pub fn query_vault_position_value(
        &self,
        position: &VaultPosition,
    ) -> StdResult<VPositionValue> {
        self.app.wrap().query_wasm_smart(
            self.rover.clone(),
            &VaultPositionValue {
                vault_position: position.clone(),
            },
        )
    }

    pub fn query_price(&self, denom: &str) -> PriceResponse {
        self.app
            .wrap()
            .query_wasm_smart(
                self.mars_oracle.clone(),
                &OraclePrice {
                    kind: None,
                    denom: denom.to_string(),
                },
            )
            .unwrap()
    }

    pub fn query_vault_bindings(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> StdResult<PaginationResponse<VaultBinding>> {
        self.app.wrap().query_wasm_smart(
            self.rover.clone(),
            &QueryMsg::VaultBindings {
                start_after,
                limit,
            },
        )
    }

    pub fn query_perp_position(&self, account_id: &str, denom: &str) -> PositionResponse {
        self.query_perp_position_with_modification_size(account_id, denom, None)
    }

    pub fn query_perp_position_with_modification_size(
        &self,
        account_id: &str,
        denom: &str,
        modification_size: Option<Int128>,
    ) -> PositionResponse {
        self.app
            .wrap()
            .query_wasm_smart(
                self.perps.address(),
                &perps::QueryMsg::Position {
                    account_id: account_id.to_string(),
                    denom: denom.to_string(),
                    order_size: modification_size,
                    reduce_only: None,
                },
            )
            .unwrap()
    }

    pub fn query_perp_opening_fee(&self, denom: &str, size: Int128) -> TradingFee {
        self.app
            .wrap()
            .query_wasm_smart(
                self.perps.address(),
                &perps::QueryMsg::OpeningFee {
                    denom: denom.to_string(),
                    size,
                },
            )
            .unwrap()
    }

    pub fn query_perp_config(&self) -> Config<String> {
        self.app.wrap().query_wasm_smart(self.perps.address(), &perps::QueryMsg::Config {}).unwrap()
    }

    pub fn query_perp_vault(&self, action: Option<ActionKind>) -> Result<VaultResponse, StdError> {
        self.app.wrap().query_wasm_smart(
            self.perps.address(),
            &perps::QueryMsg::Vault {
                action,
            },
        )
    }

    pub fn query_perp_vault_position(&self, acc_id: &str) -> Option<VaultPositionResponse> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.perps.address(),
                &perps::QueryMsg::VaultPosition {
                    user_address: self.rover.to_string(),
                    account_id: Some(acc_id.to_string()),
                },
            )
            .unwrap()
    }

    pub fn query_address_provider(&self, address_type: MarsAddressType) -> Addr {
        let res: AddressResponseItem = self
            .app
            .wrap()
            .query_wasm_smart(
                self.address_provider.to_string(),
                &address_provider::QueryMsg::Address(address_type),
            )
            .unwrap();

        Addr::unchecked(res.address)
    }
}

impl MockEnvBuilder {
    pub fn build(mut self) -> AnyResult<MockEnv> {
        let rover = self.get_rover()?;
        self.set_emergency_owner(&rover);

        let addr_provider = self.get_address_provider();

        let mars_oracle = self.get_oracle();
        let incentives =
            Incentives::new(Addr::unchecked(self.get_incentives().address()), rover.clone());

        let params: mars_types::adapters::params::ParamsBase<Addr> = self.get_params_contract();
        self.add_params_to_contract();

        let health_contract = self.get_health_contract();
        self.update_health_contract_config(&rover);

        self.deploy_nft_contract(&rover);

        if self.deploy_nft_contract && self.set_nft_contract_minter {
            self.update_config(
                &rover,
                ConfigUpdates {
                    rewards_collector: Some("rewards_collector_contract".to_string()),
                    ..Default::default()
                },
            );
        }

        let perps = self.deploy_perps_contract();
        self.update_config(
            &rover,
            ConfigUpdates {
                perps: Some(perps.clone().into()),
                keeper_fee_config: Some(self.get_keeper_fee_config()),
                ..Default::default()
            },
        );

        self.fund_users();

        self.deploy_vaults();

        Ok(MockEnv {
            app: self.app,
            rover,
            mars_oracle: mars_oracle.address().clone(),
            health_contract,
            incentives,
            params,
            perps,
            address_provider: addr_provider,
        })
    }

    //--------------------------------------------------------------------------------------------------
    // Execute Msgs
    //--------------------------------------------------------------------------------------------------

    fn fund_users(&mut self) {
        for account in &self.accounts_to_fund {
            self.app
                .sudo(SudoMsg::Bank(BankSudo::Mint {
                    to_address: account.addr.to_string(),
                    amount: account.funds.clone(),
                }))
                .unwrap();
        }
    }

    fn deploy_nft_contract(&mut self, rover: &Addr) {
        let nft_minter = Addr::unchecked("original_nft_minter");

        if self.deploy_nft_contract {
            let nft_contract = deploy_nft_contract(&mut self.app, &nft_minter);
            if self.set_nft_contract_minter {
                propose_new_nft_minter(&mut self.app, nft_contract.clone(), &nft_minter, rover);
                self.update_config(
                    rover,
                    ConfigUpdates {
                        account_nft: Some(AccountNftUnchecked::new(nft_contract.to_string())),
                        ..Default::default()
                    },
                )
            }
        }
    }

    fn set_address(&mut self, address_type: MarsAddressType, address: Addr) {
        let address_provider_addr = self.get_address_provider();
        self.app
            .execute_contract(
                self.get_owner(),
                address_provider_addr,
                &address_provider::ExecuteMsg::SetAddress {
                    address_type,
                    address: address.into(),
                },
                &[],
            )
            .unwrap();
    }

    fn add_params_to_contract(&mut self) {
        let params_to_set = self.get_coin_params();
        let params_contract = self.get_params_contract();
        let red_bank_contract = self.get_red_bank();

        for coin_info in params_to_set {
            // initialize red bank market
            self.app
                .execute_contract(
                    Addr::unchecked("red_bank_contract_owner"),
                    Addr::unchecked(red_bank_contract.address()),
                    &red_bank::ExecuteMsg::InitAsset {
                        denom: coin_info.denom.clone(),
                        params: InitOrUpdateAssetParams {
                            reserve_factor: Some(Decimal::zero()),
                            interest_rate_model: Some(InterestRateModel::default()),
                        },
                    },
                    &[],
                )
                .unwrap();

            // save asset params to mars-params contract
            self.app
                .execute_contract(
                    self.get_owner(),
                    params_contract.address().clone(),
                    &UpdateAssetParams(AddOrUpdate {
                        params: coin_info.into(),
                    }),
                    &[],
                )
                .unwrap();
        }
    }

    pub fn set_emergency_owner(&mut self, rover: &Addr) {
        if let Some(eo) = self.emergency_owner.clone() {
            self.app
                .execute_contract(
                    self.get_owner(),
                    rover.clone(),
                    &ExecuteMsg::UpdateOwner(OwnerUpdate::SetEmergencyOwner {
                        emergency_owner: eo.to_string(),
                    }),
                    &[],
                )
                .unwrap();
        }
    }

    pub fn update_config(&mut self, rover: &Addr, updates: ConfigUpdates) {
        self.app
            .execute_contract(
                self.get_owner(),
                rover.clone(),
                &ExecuteMsg::UpdateConfig {
                    updates,
                },
                &[],
            )
            .unwrap();
    }

    //--------------------------------------------------------------------------------------------------
    // Get or defaults
    //--------------------------------------------------------------------------------------------------

    fn get_rover(&mut self) -> AnyResult<Addr> {
        let code_id = self.app.store_code(mock_rover_contract());
        let red_bank = self.get_red_bank();
        let incentives = self.get_incentives();
        let swapper = self.deploy_swapper().into();
        let max_unlocking_positions = self.get_max_unlocking_positions();
        let max_slippage = self.get_max_slippage();
        let perps_liquidation_bonus_ratio = self.get_perps_liquidation_ratio();

        let oracle = self.get_oracle().into();
        let zapper = self.deploy_zapper(&oracle)?.into();
        let health_contract = self.get_health_contract().into();
        let params = self.get_params_contract().into();
        let keeper_fee_config = self.get_keeper_fee_config();

        self.deploy_rewards_collector();
        self.deploy_astroport_incentives();

        let addr = self
            .app
            .instantiate_contract(
                code_id,
                self.get_owner(),
                &InstantiateMsg {
                    owner: self.get_owner().to_string(),
                    red_bank,
                    oracle,
                    max_unlocking_positions,
                    max_slippage,
                    swapper,
                    zapper,
                    health_contract,
                    params,
                    incentives,
                    keeper_fee_config,
                    perps_liquidation_bonus_ratio,
                },
                &[],
                "mock-rover-contract",
                None,
            )
            .unwrap();

        self.set_address(MarsAddressType::CreditManager, addr.clone());

        Ok(addr)
    }

    fn get_owner(&self) -> Addr {
        self.owner.clone().unwrap_or_else(|| Addr::unchecked("owner"))
    }

    fn get_address_provider(&mut self) -> Addr {
        if self.address_provider.is_none() {
            let addr = self.deploy_address_provider();
            self.address_provider = Some(addr);
        }
        self.address_provider.clone().unwrap()
    }

    fn deploy_address_provider(&mut self) -> Addr {
        let contract_code_id = self.app.store_code(mock_address_provider_contract());
        let owner = self.get_owner();

        self.app
            .instantiate_contract(
                contract_code_id,
                owner.clone(),
                &address_provider::InstantiateMsg {
                    owner: owner.into(),
                    prefix: "".into(),
                },
                &[],
                "mock-address-provider",
                None,
            )
            .unwrap()
    }

    fn get_oracle(&mut self) -> Oracle {
        if self.oracle.is_none() {
            let addr = self.deploy_oracle();
            self.oracle = Some(addr);
        }
        self.oracle.clone().unwrap()
    }

    fn deploy_oracle(&mut self) -> Oracle {
        let contract_code_id = self.app.store_code(mock_oracle_contract());
        let mut prices: Vec<CoinPrice> = self
            .get_coin_params()
            .iter()
            .map(|item| CoinPrice {
                pricing: ActionKind::Default,
                denom: item.denom.clone(),
                price: item.price,
            })
            .collect();

        // Don't override uusdc price if it's already set
        let usdc_price_set = prices.iter().any(|p| p.denom == "uusdc");
        if !usdc_price_set {
            prices.push(CoinPrice {
                pricing: ActionKind::Default,
                denom: "uusdc".to_string(),
                price: Decimal::from_atomics(12345u128, 4).unwrap(),
            });
        }

        // Ensures vault base token denoms are pricable in the oracle
        // even if they are not whitelisted in Rover
        let price_denoms = prices.iter().map(|c| c.denom.clone()).collect::<Vec<_>>();
        self.vault_configs.clone().unwrap_or_default().iter().for_each(|v| {
            if !price_denoms.contains(&v.base_token_denom) {
                prices.push(CoinPrice {
                    pricing: ActionKind::Default,
                    denom: v.base_token_denom.clone(),
                    price: Decimal::from_atomics(456u128, 5).unwrap(),
                });
            }
        });

        let addr = self
            .app
            .instantiate_contract(
                contract_code_id,
                Addr::unchecked("oracle_contract_owner"),
                &OracleInstantiateMsg {
                    prices,
                },
                &[],
                "mock-oracle",
                None,
            )
            .unwrap();

        self.set_address(MarsAddressType::Oracle, addr.clone());

        OracleBase::new(addr)
    }

    fn get_params_contract(&mut self) -> Params {
        if self.params.is_none() {
            let hc = self.deploy_params_contract();
            self.params = Some(hc);
        }
        self.params.clone().unwrap()
    }

    pub fn deploy_params_contract(&mut self) -> Params {
        let contract_code_id = self.app.store_code(mock_params_contract());
        let owner = self.get_owner();
        let address_provider = self.get_address_provider();

        let addr = self
            .app
            .instantiate_contract(
                contract_code_id,
                owner.clone(),
                &ParamsInstantiateMsg {
                    owner: owner.to_string(),
                    risk_manager: None,
                    address_provider: address_provider.into(),
                    max_perp_params: 40,
                },
                &[],
                "mock-params-contract",
                Some(owner.to_string()),
            )
            .unwrap();

        self.set_address(MarsAddressType::Params, addr.clone());

        Params::new(addr)
    }

    fn deploy_perps_contract(&mut self) -> Perps {
        let contract_code_id = self.app.store_code(mock_perps_contract());
        let owner = self.get_owner();
        let address_provider = self.get_address_provider();
        let target_vault_collateralization_ratio = self.get_target_vault_collateralization_ratio();
        let deleverage_enabled = self.get_delegerage_enabled();
        let vault_withdraw_enabled = self.get_withdraw_enabled();
        let perps_protocol_fee_ratio = self.get_perps_protocol_fee_ratio();
        let addr = self
            .app
            .instantiate_contract(
                contract_code_id,
                owner.clone(),
                &PerpsInstantiateMsg {
                    address_provider: address_provider.into(),
                    base_denom: "uusdc".to_string(),
                    cooldown_period: 360,
                    max_positions: 4,
                    protocol_fee_rate: perps_protocol_fee_ratio,
                    target_vault_collateralization_ratio,
                    deleverage_enabled,
                    vault_withdraw_enabled,
                    max_unlocks: 5,
                },
                &[],
                "mock-perps-contract",
                Some(owner.to_string()),
            )
            .unwrap();

        self.set_address(MarsAddressType::Perps, addr.clone());

        Perps::new(addr)
    }

    fn get_health_contract(&mut self) -> HealthContract {
        if self.health_contract.is_none() {
            let hc = self.deploy_health_contract();
            self.health_contract = Some(hc);
        }
        self.health_contract.clone().unwrap()
    }

    pub fn deploy_health_contract(&mut self) -> HealthContract {
        let contract_code_id = self.app.store_code(mock_health_contract());
        let owner = self.get_owner();

        let addr = self
            .app
            .instantiate_contract(
                contract_code_id,
                owner.clone(),
                &HealthInstantiateMsg {
                    owner: owner.to_string(),
                    credit_manager: None,
                },
                &[],
                "mock-health-contract",
                Some(owner.to_string()),
            )
            .unwrap();

        HealthContract::new(addr)
    }

    fn update_health_contract_config(&mut self, cm_addr: &Addr) {
        let health_contract = self.get_health_contract();

        self.app
            .execute_contract(
                self.get_owner(),
                health_contract.address().clone(),
                &UpdateConfig {
                    credit_manager: cm_addr.to_string(),
                },
                &[],
            )
            .unwrap();
    }

    fn get_red_bank(&mut self) -> RedBankUnchecked {
        if self.red_bank.is_none() {
            let rb = self.deploy_red_bank();
            self.red_bank = Some(rb);
        }
        self.red_bank.clone().unwrap()
    }

    pub fn deploy_red_bank(&mut self) -> RedBankUnchecked {
        let contract_code_id = self.app.store_code(mock_red_bank_contract());
        let addr = self
            .app
            .instantiate_contract(
                contract_code_id,
                Addr::unchecked("red_bank_contract_owner"),
                &Empty {},
                &[],
                "mock-red-bank",
                None,
            )
            .unwrap();

        // fund red bank with whitelisted coins
        if !self.get_coin_params().is_empty() {
            self.app
                .sudo(SudoMsg::Bank(BankSudo::Mint {
                    to_address: addr.to_string(),
                    amount: self
                        .get_coin_params()
                        .iter()
                        .map(|info| info.to_coin(DEFAULT_RED_BANK_COIN_BALANCE.u128()))
                        .collect(),
                }))
                .unwrap();
        }

        self.set_address(MarsAddressType::RedBank, addr.clone());

        RedBankUnchecked::new(addr.to_string())
    }

    fn get_incentives(&mut self) -> IncentivesUnchecked {
        if self.incentives.is_none() {
            let rb = self.deploy_incentives();
            self.incentives = Some(rb);
        }
        self.incentives.clone().unwrap()
    }

    pub fn deploy_incentives(&mut self) -> IncentivesUnchecked {
        let contract_code_id = self.app.store_code(mock_incentives_contract());
        let addr = self
            .app
            .instantiate_contract(
                contract_code_id,
                Addr::unchecked("incentives_contract_owner"),
                &Empty {},
                &[],
                "mock-incentives",
                None,
            )
            .unwrap();

        self.set_address(MarsAddressType::Incentives, addr.clone());

        IncentivesUnchecked::new(addr.to_string())
    }

    pub fn deploy_astroport_incentives(&mut self) -> Addr {
        let code_id = self.app.store_code(mock_astro_incentives_contract());
        let addr = self
            .app
            .instantiate_contract(
                code_id,
                Addr::unchecked("astroport_incentives_owner"),
                &Empty {},
                &[],
                "mock-astroport-incentives",
                None,
            )
            .unwrap();

        self.set_address(MarsAddressType::AstroportIncentives, addr.clone());

        addr
    }

    fn deploy_vault(&mut self, vault: &VaultTestInfo) -> Addr {
        let code_id = self.app.store_code(mock_vault_contract());
        let oracle = self.get_oracle().into();
        let vault_addr = self
            .app
            .instantiate_contract(
                code_id,
                Addr::unchecked("vault-instantiator"),
                &VaultInstantiateMsg {
                    vault_token_denom: vault.clone().vault_token_denom,
                    lockup: vault.lockup,
                    base_token_denom: vault.clone().base_token_denom,
                    oracle,
                    is_evil: self.evil_vault.clone(),
                },
                &[],
                "mock-vault",
                None,
            )
            .unwrap();
        self.fund_vault(&vault_addr, &vault.vault_token_denom);

        let params = self.get_params_contract();

        self.app
            .execute_contract(
                self.get_owner(),
                params.address().clone(),
                &UpdateVaultConfig(VaultConfigUpdate::AddOrUpdate {
                    config: VaultConfigUnchecked {
                        addr: vault_addr.to_string(),
                        deposit_cap: vault.deposit_cap.clone(),
                        max_loan_to_value: vault.max_ltv,
                        liquidation_threshold: vault.liquidation_threshold,
                        whitelisted: vault.whitelisted,
                        hls: vault.hls.clone(),
                    },
                }),
                &[],
            )
            .unwrap();

        vault_addr
    }

    fn deploy_rewards_collector(&mut self) -> Addr {
        let code_id = self.app.store_code(mock_rewards_collector_osmosis_contract());
        let owner = self.get_owner();
        let address_provider = self.get_address_provider();

        let addr = self
            .app
            .instantiate_contract(
                code_id,
                owner.clone(),
                &rewards_collector::InstantiateMsg {
                    owner: owner.clone().to_string(),
                    address_provider: address_provider.to_string(),
                    safety_tax_rate: Default::default(),
                    revenue_share_tax_rate: Default::default(),
                    safety_fund_config: RewardConfig {
                        target_denom: "uusdc".to_string(),
                        transfer_type: TransferType::Bank,
                    },
                    revenue_share_config: RewardConfig {
                        target_denom: "uusdc".to_string(),
                        transfer_type: TransferType::Bank,
                    },
                    fee_collector_config: RewardConfig {
                        target_denom: "umars".to_string(),
                        transfer_type: TransferType::Bank,
                    },
                    channel_id: "".to_string(),
                    timeout_seconds: 1,
                    slippage_tolerance: Default::default(),
                },
                &[],
                "mock-rewards-collector",
                None,
            )
            .unwrap();

        self.set_address(MarsAddressType::RewardsCollector, addr.clone());

        addr
    }

    fn deploy_swapper(&mut self) -> Swapper {
        let code_id = self.app.store_code(mock_swapper_contract());
        let addr = self
            .app
            .instantiate_contract(
                code_id,
                Addr::unchecked("swapper-instantiator"),
                &SwapperInstantiateMsg {
                    owner: self.get_owner().to_string(),
                },
                &[],
                "mock-vault",
                None,
            )
            .unwrap();
        // Fund with osmo to simulate swaps
        self.app
            .sudo(SudoMsg::Bank(BankSudo::Mint {
                to_address: addr.to_string(),
                amount: coins(1_000_000, "uosmo"),
            }))
            .unwrap();
        SwapperBase::new(addr)
    }

    fn deploy_zapper(&mut self, oracle: &OracleUnchecked) -> AnyResult<Zapper> {
        let code_id = self.app.store_code(mock_v2_zapper_contract());
        let lp_token = lp_token_info();
        let addr = self.app.instantiate_contract(
            code_id,
            Addr::unchecked("zapper-instantiator"),
            &ZapperInstantiateMsg {
                oracle: oracle.clone(),
                lp_configs: vec![
                    LpConfig {
                        lp_token_denom: lp_token.denom.to_string(),
                        lp_pair_denoms: ("uatom".to_string(), "uosmo".to_string()),
                    },
                    LpConfig {
                        lp_token_denom: ASTRO_LP_DENOM.to_string(),
                        lp_pair_denoms: ("ujake".to_string(), "uosmo".to_string()),
                    },
                ],
            },
            &[],
            "mock-vault",
            None,
        )?;
        // Fund with lp tokens to simulate mints
        self.app
            .sudo(SudoMsg::Bank(BankSudo::Mint {
                to_address: addr.to_string(),
                amount: vec![coin(10_000_000, lp_token.denom), coin(10_000_000, ASTRO_LP_DENOM)],
            }))
            .unwrap();
        Ok(ZapperBase::new(addr))
    }

    /// cw-multi-test does not yet have the ability to mint sdk coins. For this reason,
    /// this contract expects to be pre-funded with vault tokens and it will simulate the mint.
    fn fund_vault(&mut self, vault_addr: &Addr, denom: &str) {
        self.app
            .sudo(SudoMsg::Bank(BankSudo::Mint {
                to_address: vault_addr.to_string(),
                amount: vec![Coin {
                    denom: denom.into(),
                    amount: DEFAULT_VAULT_TOKEN_PREFUND,
                }],
            }))
            .unwrap();
    }

    fn deploy_vaults(&mut self) -> Vec<Addr> {
        self.vault_configs
            .clone()
            .unwrap_or_default()
            .iter()
            .map(|v| self.deploy_vault(v))
            .collect()
    }

    fn get_keeper_fee_config(&self) -> KeeperFeeConfig {
        self.keeper_fee_config.clone().unwrap_or(KeeperFeeConfig {
            min_fee: coin(1000000, "uusdc"),
        })
    }

    fn get_coin_params(&self) -> Vec<CoinInfo> {
        self.coin_params.clone().unwrap_or_default()
    }

    fn get_max_unlocking_positions(&self) -> Uint128 {
        self.max_unlocking_positions.unwrap_or_else(|| Uint128::new(100))
    }

    fn get_max_slippage(&self) -> Decimal {
        self.max_slippage.unwrap_or_else(|| Decimal::percent(99))
    }

    fn get_perps_liquidation_ratio(&self) -> Decimal {
        self.perps_liquidation_bonus_ratio.unwrap_or_else(|| Decimal::percent(60))
    }

    fn get_target_vault_collateralization_ratio(&self) -> Decimal {
        self.target_vault_collateralization_ratio.unwrap_or_else(|| Decimal::percent(125))
    }

    fn get_delegerage_enabled(&self) -> bool {
        self.deleverage_enabled.unwrap_or(true)
    }

    fn get_withdraw_enabled(&self) -> bool {
        self.withdraw_enabled.unwrap_or(true)
    }

    fn get_perps_protocol_fee_ratio(&self) -> Decimal {
        self.perps_protocol_fee_ratio.unwrap_or_else(|| Decimal::percent(0))
    }

    //--------------------------------------------------------------------------------------------------
    // Setter functions
    //--------------------------------------------------------------------------------------------------

    pub fn fund_account(mut self, account: AccountToFund) -> Self {
        self.accounts_to_fund.push(account);
        self
    }

    pub fn fund_accounts(mut self, addrs: Vec<Addr>, coins: Vec<Coin>) -> Self {
        for addr in addrs {
            self.accounts_to_fund.push(AccountToFund {
                addr,
                funds: coins.clone(),
            });
        }
        self
    }

    pub fn owner(mut self, owner: &str) -> Self {
        self.owner = Some(Addr::unchecked(owner));
        self
    }

    pub fn vault_configs(mut self, vault_configs: &[VaultTestInfo]) -> Self {
        self.vault_configs = Some(vault_configs.to_vec());
        self
    }

    pub fn set_params(mut self, coins: &[CoinInfo]) -> Self {
        self.coin_params = Some(coins.to_vec());
        self
    }

    pub fn params_contract(mut self, params: &str) -> Self {
        self.params = Some(Params::new(Addr::unchecked(params)));
        self
    }

    pub fn health_contract(mut self, health: &str) -> Self {
        self.health_contract = Some(HealthContract::new(Addr::unchecked(health)));
        self
    }

    pub fn red_bank(mut self, red_bank: &str) -> Self {
        self.red_bank = Some(RedBankUnchecked::new(red_bank.to_string()));
        self
    }

    pub fn oracle(mut self, addr: &str) -> Self {
        self.oracle = Some(OracleBase::new(Addr::unchecked(addr)));
        self
    }

    pub fn params(mut self, addr: &str) -> Self {
        self.params = Some(Params::new(Addr::unchecked(addr)));
        self
    }

    pub fn keeper_fee_config(mut self, config: KeeperFeeConfig) -> Self {
        self.keeper_fee_config = Some(config);
        self
    }

    pub fn no_nft_contract(mut self) -> Self {
        self.deploy_nft_contract = false;
        self
    }

    pub fn no_nft_contract_minter(mut self) -> Self {
        self.set_nft_contract_minter = false;
        self
    }

    pub fn max_unlocking_positions(mut self, max: u128) -> Self {
        self.max_unlocking_positions = Some(Uint128::new(max));
        self
    }

    pub fn max_slippage(mut self, max: Decimal) -> Self {
        self.max_slippage = Some(max);
        self
    }

    pub fn perps_liquidation_bonus_ratio(mut self, lb_ratio: Decimal) -> Self {
        self.perps_liquidation_bonus_ratio = Some(lb_ratio);
        self
    }

    pub fn evil_vault(mut self, credit_account: &str) -> Self {
        self.evil_vault = Some(credit_account.to_string());
        self
    }

    pub fn target_vault_collaterization_ratio(mut self, ratio: Decimal) -> Self {
        self.target_vault_collateralization_ratio = Some(ratio);
        self
    }

    pub fn deleverage_enabled(mut self, enabled: bool) -> Self {
        self.deleverage_enabled = Some(enabled);
        self
    }

    pub fn perps_protocol_fee_ratio(mut self, ratio: Decimal) -> Self {
        self.perps_protocol_fee_ratio = Some(ratio);
        self
    }
}

//--------------------------------------------------------------------------------------------------
// Shared utils between MockBuilder & MockEnv
//--------------------------------------------------------------------------------------------------

fn deploy_nft_contract(app: &mut CustomApp, minter: &Addr) -> Addr {
    let nft_contract_code_id = app.store_code(mock_account_nft_contract());
    app.instantiate_contract(
        nft_contract_code_id,
        minter.clone(),
        &NftInstantiateMsg {
            max_value_for_burn: Default::default(),
            health_contract: None,
            name: "Rover Credit Account".to_string(),
            symbol: "RCA".to_string(),
            minter: minter.to_string(),
            credit_manager_contract: None,
        },
        &[],
        "manager-mock-account-nft",
        None,
    )
    .unwrap()
}

fn propose_new_nft_minter(
    app: &mut CustomApp,
    nft_contract: Addr,
    old_minter: &Addr,
    new_minter: &Addr,
) {
    let proposal_msg: NftExecuteMsg = NftExecuteMsg::UpdateOwnership(TransferOwnership {
        new_owner: new_minter.into(),
        expiry: None,
    });
    app.execute_contract(old_minter.clone(), nft_contract, &proposal_msg, &[]).unwrap();
}

pub fn deploy_managed_vault(
    app: &mut CustomApp,
    sender: &Addr,
    credit_manager: &Addr,
    creation_fee: Option<Coin>,
) -> Addr {
    deploy_managed_vault_with_performance_fee(
        app,
        sender,
        credit_manager,
        60,
        PerformanceFeeConfig {
            fee_rate: Decimal::zero(),
            withdrawal_interval: 0,
        },
        "uusdc",
        creation_fee,
    )
}

pub fn deploy_managed_vault_with_performance_fee(
    app: &mut CustomApp,
    sender: &Addr,
    credit_manager: &Addr,
    cooldown_period: u64,
    pf_config: PerformanceFeeConfig,
    base_denom: &str,
    creation_fee: Option<Coin>,
) -> Addr {
    let mut funds = vec![];
    funds.push(coin(10_000_000, "untrn")); // Token Factory fee for minting new denom. Configured in the Token Factory module in `mars-testing` package.
    if let Some(creation_fee) = creation_fee {
        funds.push(creation_fee);
    }

    let contract_code_id = app.store_code(mock_managed_vault_contract());
    app.instantiate_contract(
        contract_code_id,
        sender.clone(),
        &ManagedVaultInstantiateMsg {
            base_token: base_denom.to_string(),
            vault_token_subdenom: "vault".to_string(),
            title: None,
            subtitle: None,
            description: None,
            credit_manager: credit_manager.to_string(),
            cooldown_period,
            performance_fee_config: pf_config,
        },
        &funds,
        "mock-managed-vault",
        None,
    )
    .unwrap()
}

pub fn deploy_vault_with_admin(
    app: &mut CustomApp,
    sender: &Addr,
    credit_manager: &Addr,
    creation_fee: Option<Coin>,
    base_denom: &str,
) -> Addr {
    let mut funds = vec![];
    funds.push(coin(10_000_000, "untrn")); // Token Factory fee for minting new denom. Configured in the Token Factory module in `mars-testing` package.
    if let Some(creation_fee) = creation_fee {
        funds.push(creation_fee);
    }

    let contract_code_id = app.store_code(mock_managed_vault_contract());
    app.instantiate_contract(
        contract_code_id,
        sender.clone(),
        &ManagedVaultInstantiateMsg {
            base_token: base_denom.to_string(),
            vault_token_subdenom: "vault".to_string(),
            title: None,
            subtitle: None,
            description: None,
            credit_manager: credit_manager.to_string(),
            cooldown_period: 86400,
            performance_fee_config: PerformanceFeeConfig {
                fee_rate: Decimal::zero(),
                withdrawal_interval: 86400,
            },
        },
        &funds,
        "mock-managed-vault",
        Some(sender.to_string()),
    )
    .unwrap()
}
