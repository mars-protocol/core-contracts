use cosmwasm_std::Env;
use mars_swapper_duality::DualityRoute;

/// helper function to create a simple direct route
pub fn create_direct_route(from: &str, to: &str) -> DualityRoute {
    DualityRoute {
        from: from.to_string(),
        to: to.to_string(),
        swap_denoms: vec![from.to_string(), to.to_string()],
    }
}

/// helper function to create a multi-hop route
pub fn create_multi_hop_route(from: &str, via: &[&str], to: &str) -> DualityRoute {
    let mut swap_denoms = vec![];
    swap_denoms.push(from.to_string());
    for denom in via {
        swap_denoms.push(denom.to_string());
    }
    swap_denoms.push(to.to_string());

    DualityRoute {
        from: from.to_string(),
        to: to.to_string(),
        swap_denoms,
    }
}

/// mock environment for testing
pub fn mock_env() -> Env {
    cosmwasm_std::testing::mock_env()
}

/// Helper functions for working with the Neutron DEX
pub mod neutron_dex_helpers {
    use std::str::FromStr;

    use cosmwasm_std::{from_json, to_json_binary, Coin, Decimal, Uint128};
    use cosmwasm_std_2::Coin as Coin2;
    use mars_types::swapper::{EstimateExactInSwapResponse, ExecuteMsg, InstantiateMsg, QueryMsg, SwapperRoute};
    use neutron_test_tube::{neutron_std::types::{cosmwasm::wasm::v1::MsgExecuteContractResponse, neutron::dex::{DepositOptions, MsgDeposit, MsgPlaceLimitOrder}}, Account, Dex, ExecuteResponse, Module, NeutronTestApp, RunnerError, SigningAccount, Wasm};

    const ARTIFACTS_PATH: &str = "../../../artifacts";

    pub fn init_dex(app: &NeutronTestApp) -> Dex<'_, NeutronTestApp> {
        Dex::new(app)       
    }

    /// Creates a new pool in the Neutron DEX
    // pub fn create_dex_pool(
    //     dex: &Dex<'_, NeutronTestApp>,
    //     creator: &SigningAccount,
    //     denom1: &str,
    //     denom2: &str,
    //     initial_liquidity1: Uint128,
    //     initial_liquidity2: Uint128,
    // ) -> u64 {
    //     // TODO: Implement pool creation using Neutron DEX module
    //     // This is a placeholder that will need to be replaced with actual implementation
    //     // once we have more information about the Neutron DEX API
        
    //     // For now, just return a mock pool ID
    //     1u64
    // }
    
    /// Adds liquidity to an existing pool
    pub fn add_liquidity(
        dex: &Dex<'_, NeutronTestApp>,
        signer: &SigningAccount,
        denom1: &str,
        denom2: &str,
        amount1: Uint128,
        amount2: Uint128,
    ) {
        // The price is determined by the ratio of amount2/amount1
        let scale_factor = 1_000_000_000_000_000_000u128;
        
        
        // Calculate the price based on the ratio of amount2 to amount1
        // We need to convert to u128 before calculating the ratio to avoid overflow
        let price_ratio = Decimal::from_ratio(amount2, amount1).checked_mul(Decimal::from_str("10000000").unwrap()).unwrap();
        println!("Price ratio: {}", price_ratio);
        
        let res = dex.deposit(MsgDeposit {
            creator: signer.address().clone(),
            receiver: signer.address().clone(),
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
        }, signer).unwrap();

        println!("Deposit result: {:#?}", res);

    }

     /// Queries for swap estimation from the swapper contract
     pub fn query_estimate_exact_in_swap(
        app: &NeutronTestApp,
        contract_addr: &str,
        coin_in: &Coin,
        denom_out: impl Into<String>,
        route: Option<SwapperRoute>,
    ) -> EstimateExactInSwapResponse {
        let wasm: Wasm<'_, NeutronTestApp> = Wasm::new(app);

        let query_msg = QueryMsg::EstimateExactInSwap {
            coin_in: coin_in.clone(),
            denom_out: denom_out.into(),
            route,
        };
        
        // Serialize the query message to binary
        // let binary_query = to_json_binary(&query_msg).unwrap();
        wasm.query(contract_addr, &query_msg).unwrap()
    }

    pub fn execute_swap(
        app: &NeutronTestApp,
        contract_addr: &str,
        coin_in: Coin,
        denom_out: impl Into<String>,
        min_receive: Uint128,
        route: Option<SwapperRoute>,
        admin: &SigningAccount,
    ) -> Result<ExecuteResponse<MsgExecuteContractResponse>, RunnerError> {
        let wasm = Wasm::new(app);
        println!("Executing swap");
        println!("Coin in: {:#?}", coin_in);
        let denom_out = denom_out.into();
        println!("Denom out: {}", denom_out);
        println!("Route: {:#?}", route);
        let coin_in2 = Coin2::new(coin_in.amount.u128(), coin_in.denom.clone());

        let execute_msg: ExecuteMsg<SwapperRoute, Coin> = ExecuteMsg::SwapExactIn {
            coin_in,
            denom_out,
            min_receive,
            route,
        };

        wasm.execute(contract_addr, &execute_msg, &[coin_in2], &admin)
    }
    
    /// Sets up a test environment with pools and initial balances
    pub fn setup_test_environment() -> (NeutronTestApp, SigningAccount, SigningAccount, String) {
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
        
        let wasm: Wasm<'_, NeutronTestApp> = Wasm::new(&app);

        let wasm_byte_code = std::fs::read(format!("{}/mars_swapper_duality.wasm", ARTIFACTS_PATH)).unwrap();
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
                None, // contract admin used for migration, not the same as cw1_whitelist admin
                Some("Test label"), // contract label
                &[], // funds
                &admin, // signer
            )
            .unwrap()
            .data
            .address;

        // Return the app and accounts
        (app, admin, user, contract_addr)
    }
}
