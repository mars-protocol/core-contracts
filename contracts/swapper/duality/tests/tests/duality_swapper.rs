use std::str::FromStr;

use cosmwasm_std::{coin, from_json, to_json_binary, Coin, Decimal, Uint128};
use cosmwasm_std_2::Coin as Coin2;
use mars_types::swapper::{EstimateExactInSwapResponse, ExecuteMsg, InstantiateMsg, QueryMsg, SwapperRoute};
use neutron_test_tube::{
    neutron_std::types::{
        cosmwasm::wasm::v1::MsgExecuteContractResponse,
        neutron::dex::{DepositOptions, MsgDeposit, QueryAllUserDepositsRequest},
    },
    Account, Dex, ExecuteResponse, Module, NeutronTestApp, RunnerError, SigningAccount, Wasm,
};

// Constants
const ARTIFACTS_PATH: &str = "../../../artifacts";

/// DualitySwapperTester is a test helper that encapsulates all interactions with the Duality swapper.
/// It provides a clean, high-level interface for setting up and running tests with the Neutron DEX.
pub struct DualitySwapperTester<'a> {
    pub app: &'a NeutronTestApp,
    pub admin: SigningAccount,
    pub user: SigningAccount,
    pub contract_addr: String,
    wasm: Wasm<'a, NeutronTestApp>,
    dex: Dex<'a, NeutronTestApp>,
}

impl<'a> DualitySwapperTester<'a> {
    /// Creates a new test environment with the Duality swapper deployed
    pub fn new() -> Self {
        // Create the Neutron test app
        let app = NeutronTestApp::default();
        
        // Initialize admin and user accounts with funds
        let initial_balance = vec![
            Coin2::new(1_000_000_000_000u128, "untrn"),
            Coin2::new(1_000_000_000_000u128, "uusdc"),
            Coin2::new(1_000_000_000_000u128, "uatom"),
        ];
        
        let admin = app.init_account(initial_balance.as_slice()).unwrap();
        let user = app.init_account(initial_balance.as_slice()).unwrap();
        
        let wasm = Wasm::new(&app);
        let dex = Dex::new(&app);

        // Deploy the contract
        let wasm_byte_code = std::fs::read(format!("{}/mars_swapper_duality.wasm", ARTIFACTS_PATH))
            .unwrap_or_else(|_| panic!("Failed to read WASM file at {}/mars_swapper_duality.wasm", ARTIFACTS_PATH));
        
        let code_id = wasm
            .store_code(&wasm_byte_code, None, &admin)
            .unwrap()
            .data
            .code_id;

        let contract_addr = wasm
            .instantiate(
                code_id,
                &InstantiateMsg {
                    owner: admin.address(),
                },
                None, // contract admin used for migration
                Some("Mars Duality Swapper"), // contract label
                &[], // funds
                &admin, // signer
            )
            .unwrap()
            .data
            .address;
            
        Self {
            app: &app,
            admin,
            user,
            contract_addr,
            wasm,
            dex,
        }
    }
    
    /// Add liquidity to a pool with the specified tokens and amounts
    pub fn add_liquidity(
        &self,
        denom1: &str,
        denom2: &str,
        amount1: Uint128,
        amount2: Uint128,
    ) {
        println!("Adding liquidity: {} {}, {} {}", amount1, denom1, amount2, denom2);
        
        // Calculate the price based on the ratio of amount2 to amount1
        let price_ratio = Decimal::from_ratio(amount2, amount1)
            .checked_mul(Decimal::from_str("10000000").unwrap())
            .unwrap();
        
        println!("Price ratio: {}", price_ratio);
        
        let res = self.dex.deposit(
            MsgDeposit {
                creator: self.admin.address().clone(),
                receiver: self.admin.address().clone(),
                token_a: denom1.to_string(),
                token_b: denom2.to_string(),
                amounts_a: vec![amount1.to_string()],
                amounts_b: vec![amount2.to_string()],
                tick_indexes_a_to_b: vec![0],
                fees: vec![0],
                options: vec![
                    DepositOptions {
                        disable_autoswap: false,
                        fail_tx_on_bel: true,
                    }
                ],
            },
            &self.admin
        ).unwrap();

        println!("Deposit result: {:#?}", res);
    }
    
    /// Query all deposits for a user
    pub fn query_deposits(&self, user: &SigningAccount) {
        let query = QueryAllUserDepositsRequest {
            user: user.address().to_string(),
            pagination: None,
        };
        
        let deposits = self.dex.query_all_user_deposits(&query).unwrap();
        println!("User deposits: {:#?}", deposits);
    }

    /// Query for swap estimation from the swapper contract
    pub fn query_estimate_exact_in_swap(
        &self,
        coin_in: &Coin,
        denom_out: impl Into<String>,
        route: Option<SwapperRoute>,
    ) -> EstimateExactInSwapResponse {
        let query_msg = QueryMsg::EstimateExactInSwap {
            coin_in: coin_in.clone(),
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
    ) -> Result<ExecuteResponse<MsgExecuteContractResponse>, RunnerError> {
        println!("Executing swap");
        println!("Coin in: {:#?}", coin_in);
        let denom_out = denom_out.into();
        println!("Denom out: {}", denom_out);
        println!("Route: {:#?}", route);
        
        // Create a coin using Coin2 for compatibility with the test-tube
        let coin_in2 = Coin2::new(coin_in.amount.u128(), coin_in.denom.clone());

        let execute_msg = ExecuteMsg::SwapExactIn {
            coin_in,
            denom_out,
            min_receive,
            route,
        };

        self.wasm.execute(&self.contract_addr, &execute_msg, &[coin_in2], signer)
    }
    
    /// Create a SwapperRoute for a direct swap (two tokens)
    pub fn create_direct_route(&self, from: &str, to: &str) -> SwapperRoute {
        SwapperRoute::Duality(mars_types::swapper::NeutronRoute {
            from: from.to_string(),
            to: to.to_string(),
            swap_denoms: vec![from.to_string(), to.to_string()],
        })
    }
    
    /// Create a SwapperRoute for a multi-hop swap (three or more tokens)
    pub fn create_multi_hop_route(&self, from: &str, intermediate: &str, to: &str) -> SwapperRoute {
        SwapperRoute::Duality(mars_types::swapper::NeutronRoute {
            from: from.to_string(),
            to: to.to_string(),
            swap_denoms: vec![from.to_string(), intermediate.to_string(), to.to_string()],
        })
    }
    
    /// Get the balance of a token for a specific account
    pub fn get_balance(&self, address: &str, denom: &str) -> Uint128 {
        Uint128::new(self.app.get_balance(address, denom).unwrap())
    }
}
