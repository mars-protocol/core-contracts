use std::str::FromStr;

use cosmwasm_std::{Coin, Decimal, Uint128};
use cosmwasm_std_2::Coin as Coin2;
use mars_types::swapper::{
    EstimateExactInSwapResponse, ExecuteMsg, InstantiateMsg, QueryMsg, SwapperRoute,
};
use neutron_test_tube::{
    neutron_std::types::{
        cosmos::bank::v1beta1::QueryBalanceRequest,
        cosmwasm::wasm::v1::MsgExecuteContractResponse,
        neutron::dex::{
            DepositOptions, MsgDeposit, MsgDepositResponse,
        },
    }, Account, Bank, Dex, ExecuteResponse, Module, NeutronTestApp, RunnerExecuteResult, SigningAccount, Wasm
};

// Constants
const ARTIFACTS_PATH: &str = "../../../artifacts";

/// DualitySwapperTester is a test helper that encapsulates all interactions with the Duality swapper.
/// It provides a clean, high-level interface for setting up and running tests with the Neutron DEX.
pub struct DualitySwapperTester<'a> {
    pub admin: SigningAccount,
    pub user: SigningAccount,
    pub contract_addr: String,
    wasm: Wasm<'a, NeutronTestApp>,
    dex: Dex<'a, NeutronTestApp>,
    bank: Bank<'a, NeutronTestApp>,
}

impl<'a> DualitySwapperTester<'a> {
    /// Creates a new test environment with the Duality swapper deployed
    pub fn new(app: &'a NeutronTestApp) -> Self {
        // Initialize admin and user accounts with funds
        let initial_balance = vec![
            Coin2::new(1_000_000_000_000u128, "untrn"),
            Coin2::new(1_000_000_000_000u128, "uusdc"),
            Coin2::new(1_000_000_000_000u128, "uatom"),
        ];

        let admin = app.init_account(initial_balance.as_slice()).unwrap();
        let user = app.init_account(initial_balance.as_slice()).unwrap();

        let wasm = Wasm::new(app);
        let dex = Dex::new(app);
        let bank = Bank::new(app);

        // Deploy the contract
        let wasm_byte_code = std::fs::read(format!("{}/mars_swapper_duality.wasm", ARTIFACTS_PATH))
            .unwrap_or_else(|_| {
                panic!("Failed to read WASM file at {}/mars_swapper_duality.wasm", ARTIFACTS_PATH)
            });

        let code_id = wasm.store_code(&wasm_byte_code, None, &admin).unwrap().data.code_id;

        let contract_addr = wasm
            .instantiate(
                code_id,
                &InstantiateMsg {
                    owner: admin.address(),
                },
                None,                         
                Some("Mars Duality Swapper"), 
                &[],                         
                &admin,                      
            )
            .unwrap()
            .data
            .address;

        Self {
            admin,
            user,
            contract_addr,
            wasm,
            dex,
            bank,
        }
    }

    /// Add liquidity to a pool with the specified tokens and amounts
    pub fn add_liquidity(
        &self,
        denom1: &str,
        denom2: &str,
        amount1: Uint128,
        amount2: Uint128,
    ) -> ExecuteResponse<MsgDepositResponse> {
        println!("Adding liquidity: {} {}, {} {}", amount1, denom1, amount2, denom2);

        // Calculate the price based on the ratio of amount2 to amount1
        let price_ratio = Decimal::from_ratio(amount2, amount1);

        println!("Price ratio: {}", price_ratio);

        self
            .dex
            .deposit(
                MsgDeposit {
                    creator: self.admin.address().clone(),
                    receiver: self.admin.address().clone(),
                    token_a: denom1.to_string(),
                    token_b: denom2.to_string(),
                    amounts_a: vec![amount1.to_string()],
                    amounts_b: vec![amount2.to_string()],
                    tick_indexes_a_to_b: vec![0],
                    fees: vec![0],
                    options: vec![DepositOptions {
                        disable_autoswap: false,
                        fail_tx_on_bel: true,
                    }],
                },
                &self.admin,
            )
            .unwrap()
    }

    /// Query for swap estimation from the swapper contract
    pub fn query_estimate_exact_in_swap(
        &self,
        coin_in: Coin,
        denom_out: impl Into<String>,
        route: Option<SwapperRoute>,
    ) -> EstimateExactInSwapResponse {
        let query_msg = QueryMsg::EstimateExactInSwap {
            coin_in,
            denom_out: denom_out.into(),
            route,
        };

        self.wasm.query(&self.contract_addr, &query_msg).unwrap()
    }

    /// Execute a swap operation
    pub fn execute_swap(
        &self,
        coin_in: Coin,
        denom_out: impl Into<String>,
        min_receive: Uint128,
        route: Option<SwapperRoute>,
        signer: &SigningAccount,
    ) -> RunnerExecuteResult<MsgExecuteContractResponse> {
        let denom_out = denom_out.into();

        // Create a coin using Coin from cw-std-2 for compatibility with the test-tube
        let coin_in2 = Coin2::new(coin_in.amount.u128(), coin_in.denom.clone());

        let execute_msg: ExecuteMsg<SwapperRoute, Coin> = ExecuteMsg::SwapExactIn {
            coin_in,
            denom_out,
            min_receive,
            route,
        };

        self.wasm.execute(&self.contract_addr, &execute_msg, &[coin_in2], signer)
    }

    /// Get the balance of [denom] for [address]
    pub fn get_balance(&self, address: &str, denom: &str) -> Uint128 {
        Uint128::from_str(
            &self
                .bank
                .query_balance(&QueryBalanceRequest {
                    address: address.to_string(),
                    denom: denom.to_string(),
                })
                .unwrap()
                .balance
                .unwrap()
                .amount,
        )
        .unwrap()
    }

    /// Create a SwapperRoute for a direct swap (two tokens)
    pub fn create_direct_route(&self, from: &str, to: &str) -> SwapperRoute {
        SwapperRoute::Duality(mars_types::swapper::DualitySwap {
            from: from.to_string(),
            to: to.to_string(),
            swap_denoms: vec![from.to_string(), to.to_string()],
        })
    }

    /// Create a SwapperRoute for a multi-hop swap (three or more tokens)
    pub fn create_multi_hop_route(&self, from: &str, intermediate: &str, to: &str) -> SwapperRoute {
        SwapperRoute::Duality(mars_types::swapper::DualitySwap {
            from: from.to_string(),
            to: to.to_string(),
            swap_denoms: vec![from.to_string(), intermediate.to_string(), to.to_string()],
        })
    }

    pub fn set_route(&self, route: SwapperRoute, denom_in: &str, denom_out: &str) -> RunnerExecuteResult<MsgExecuteContractResponse> {
        let execute_msg: ExecuteMsg<SwapperRoute, Coin> = ExecuteMsg::SetRoute { 
            denom_in: denom_in.to_string(), 
            denom_out: denom_out.to_string(), 
            route 
        };
        self.wasm.execute(&self.contract_addr, &execute_msg, &[], &self.admin)
    }
}
