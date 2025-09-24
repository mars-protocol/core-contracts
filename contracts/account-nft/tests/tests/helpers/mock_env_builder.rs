use std::mem::take;

use anyhow::Result as AnyResult;
use cosmwasm_std::{Addr, Decimal, Empty};
use cw_multi_test::{BasicApp, Executor};
use mars_mock_credit_manager::msg::InstantiateMsg as CmMockInstantiateMsg;
use mars_mock_oracle::msg::{CoinPrice, InstantiateMsg as OracleInstantiateMsg};
use mars_owner::OwnerResponse;
use mars_testing::multitest::helpers::mock_perps_contract;
use mars_types::{
    account_nft::InstantiateMsg,
    address_provider::{self, MarsAddressType},
    credit_manager::ConfigResponse,
    oracle::ActionKind,
};

use super::{
    mock_address_provider_contract, mock_credit_manager_contract, mock_health_contract,
    mock_incentives_contract, mock_nft_contract, mock_oracle_contract, mock_params_contract,
    MockEnv, MAX_VALUE_FOR_BURN,
};

pub struct MockEnvBuilder {
    pub app: BasicApp,
    pub deployer: Addr,
    pub minter: Option<Addr>,
    pub health_contract: Option<Addr>,
    pub nft_contract: Option<Addr>,
    pub cm_contract: Option<Addr>,
    pub ap_contract: Option<Addr>,
    pub oracle: Option<Addr>,
    pub params: Option<Addr>,
    pub incentives: Option<Addr>,
    pub perps: Option<Addr>,
}

impl MockEnvBuilder {
    pub fn build(&mut self) -> AnyResult<MockEnv> {
        Ok(MockEnv {
            minter: self.get_minter(),
            nft_contract: self.get_nft_contract(),
            cm_contract: self.get_cm_contract(),
            health_contract: self.get_health_contract(),
            ap_contract: self.get_ap_contract(),
            oracle: self.get_oracle(),
            params: self.get_params_contract(),
            incentives: self.get_incentives(),
            perps: self.get_perps_contract(),
            deployer: self.deployer.clone(),
            app: take(&mut self.app),
        })
    }

    pub fn set_minter(&mut self, minter: &str) -> &mut Self {
        self.minter = Some(Addr::unchecked(minter.to_string()));
        self
    }

    fn get_health_contract(&mut self) -> Addr {
        if self.health_contract.is_none() {
            return self.deploy_health_contract();
        }
        self.health_contract.clone().unwrap()
    }

    fn deploy_health_contract(&mut self) -> Addr {
        let contract = mock_health_contract();
        let code_id = self.app.store_code(contract);

        let health_contract = self
            .app
            .instantiate_contract(
                code_id,
                self.deployer.clone(),
                &Empty {},
                &[],
                "mock-health-contract",
                None,
            )
            .unwrap();
        self.set_address(MarsAddressType::Health, health_contract.clone());
        self.health_contract = Some(health_contract.clone());
        health_contract
    }

    fn get_minter(&mut self) -> Addr {
        self.minter.clone().unwrap_or_else(|| self.deployer.clone())
    }

    fn get_nft_contract(&mut self) -> Addr {
        if self.nft_contract.is_none() {
            self.deploy_nft_contract()
        }
        self.nft_contract.clone().unwrap()
    }

    fn deploy_nft_contract(&mut self) {
        let contract = mock_nft_contract();
        let code_id = self.app.store_code(contract);
        let minter = self.get_minter().into();
        let ap_contract = self.get_ap_contract();
        self.deploy_health_contract();
        self.deploy_health_contract();

        let addr = self
            .app
            .instantiate_contract(
                code_id,
                self.deployer.clone(),
                &InstantiateMsg {
                    max_value_for_burn: MAX_VALUE_FOR_BURN,
                    name: "mock_nft".to_string(),
                    symbol: "MOCK".to_string(),
                    minter,
                    address_provider_contract: ap_contract.to_string(),
                },
                &[],
                "mock-account-nft",
                None,
            )
            .unwrap();
        self.nft_contract = Some(addr);
    }

    fn get_cm_contract(&mut self) -> Addr {
        if self.cm_contract.is_none() {
            self.deploy_cm_contract()
        }
        self.cm_contract.clone().unwrap()
    }

    fn deploy_cm_contract(&mut self) {
        let contract = mock_credit_manager_contract();
        let code_id = self.app.store_code(contract);

        let cm_addr = self
            .app
            .instantiate_contract(
                code_id,
                self.deployer.clone(),
                &CmMockInstantiateMsg {
                    config: ConfigResponse {
                        ownership: OwnerResponse {
                            owner: Some(self.deployer.to_string()),
                            proposed: None,
                            emergency_owner: None,
                            initialized: true,
                            abolished: false,
                        },
                        red_bank: "n/a".to_string(),
                        incentives: "n/a".to_string(),
                        oracle: "n/a".to_string(),
                        params: "n/a".to_string(),
                        account_nft: None,
                        max_unlocking_positions: Default::default(),
                        max_slippage: Decimal::percent(99),
                        swapper: "n/a".to_string(),
                        zapper: "n/a".to_string(),
                        health_contract: "n/a".to_string(),
                        rewards_collector: None,
                        perps: "n/a".to_string(),
                        keeper_fee_config: Default::default(),
                        perps_liquidation_bonus_ratio: Decimal::percent(60),
                        governance: "n/a".to_string(),
                    },
                },
                &[],
                "mock-credit-manager-contract",
                Some(self.deployer.clone().into()),
            )
            .unwrap();
        self.set_address(MarsAddressType::CreditManager, cm_addr.clone());
        self.cm_contract = Some(cm_addr);
    }

    fn get_ap_contract(&mut self) -> Addr {
        if self.ap_contract.is_none() {
            self.deploy_ap_contract();
        }
        self.ap_contract.clone().unwrap()
    }

    fn deploy_ap_contract(&mut self) {
        let contract = mock_address_provider_contract();
        let code_id = self.app.store_code(contract);

        let ap_addr = self
            .app
            .instantiate_contract(
                code_id,
                self.deployer.clone(),
                &mars_types::address_provider::InstantiateMsg {
                    owner: self.deployer.to_string(),
                    prefix: "".to_string(),
                },
                &[],
                "address-provider-contract",
                None,
            )
            .unwrap();
        self.ap_contract = Some(ap_addr);
    }

    fn set_address(&mut self, address_type: MarsAddressType, address: Addr) {
        let address_provider_addr = self.get_ap_contract();

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

    fn get_oracle(&mut self) -> Addr {
        if self.oracle.is_none() {
            let addr = self.deploy_oracle();
            self.oracle = Some(addr);
        }
        self.oracle.clone().unwrap()
    }

    fn deploy_oracle(&mut self) -> Addr {
        let contract_code_id = self.app.store_code(mock_oracle_contract());

        let prices = vec![CoinPrice {
            pricing: ActionKind::Default,
            denom: "uusdc".to_string(),
            price: Decimal::one(),
        }];

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

        addr
    }

    fn get_params_contract(&mut self) -> Addr {
        if self.params.is_none() {
            let p = self.deploy_params_contract();
            self.params = Some(p);
        }
        self.params.clone().unwrap()
    }

    pub fn deploy_params_contract(&mut self) -> Addr {
        let contract_code_id = self.app.store_code(mock_params_contract());
        let owner = self.deployer.clone();
        let address_provider = self.get_ap_contract();

        let addr = self
            .app
            .instantiate_contract(
                contract_code_id,
                owner.clone(),
                &mars_types::params::InstantiateMsg {
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

        addr
    }

    fn get_perps_contract(&mut self) -> Addr {
        if self.perps.is_none() {
            let p = self.deploy_perps_contract();
            self.perps = Some(p);
        }
        self.perps.clone().unwrap()
    }

    fn deploy_perps_contract(&mut self) -> Addr {
        let contract_code_id = self.app.store_code(mock_perps_contract());
        let owner = self.deployer.clone();
        let address_provider = self.get_ap_contract();

        let addr = self
            .app
            .instantiate_contract(
                contract_code_id,
                owner.clone(),
                &mars_types::perps::InstantiateMsg {
                    address_provider: address_provider.into(),
                    base_denom: "uusdc".to_string(),
                    cooldown_period: 360,
                    max_positions: 4,
                    protocol_fee_rate: Decimal::percent(0),
                    target_vault_collateralization_ratio: Decimal::from_ratio(12u128, 10u128),
                    deleverage_enabled: true,
                    vault_withdraw_enabled: true,
                    max_unlocks: 5,
                },
                &[],
                "mock-perps-contract",
                Some(owner.to_string()),
            )
            .unwrap();

        self.set_address(MarsAddressType::Perps, addr.clone());

        addr
    }

    fn get_incentives(&mut self) -> Addr {
        if self.incentives.is_none() {
            let rb = self.deploy_incentives();
            self.incentives = Some(rb);
        }
        self.incentives.clone().unwrap()
    }

    pub fn deploy_incentives(&mut self) -> Addr {
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

        addr
    }
}
