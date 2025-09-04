use cosmwasm_std::{Addr, Coin, Uint128};
use cw_multi_test::{error::AnyResult, AppResponse, Executor};
use cw_paginate::PaginationResponse;
use mars_active_delta_neutral::error::ContractError;
use mars_testing::multitest::helpers::{active_delta_neutral_contract, MockEnv};
use mars_types::{
    active_delta_neutral::{
        execute::ExecuteMsg,
        instantiate::InstantiateMsg,
        query::{Config, MarketConfig, QueryMsg},
    },
    adapters::active_delta_neutral::ActiveDeltaNeutral,
    credit_manager::{Positions, QueryMsg as CreditManagerQueryMsg},
    swapper::SwapperRoute,
};

pub fn query_active_delta_neutral_market(
    mock_env: &MockEnv,
    delta_neutral: &ActiveDeltaNeutral,
    market_id: &str,
) -> MarketConfig {
    mock_env
        .app
        .wrap()
        .query_wasm_smart(
            delta_neutral.address(),
            &QueryMsg::MarketConfig {
                market_id: market_id.to_string(),
            },
        )
        .unwrap()
}

pub fn query_active_delta_neutral_config(
    mock_env: &MockEnv,
    delta_neutral: &ActiveDeltaNeutral,
) -> Config {
    mock_env.app.wrap().query_wasm_smart(delta_neutral.address(), &QueryMsg::Config {}).unwrap()
}

pub fn query_all_active_delta_neutral_markets(
    mock_env: &MockEnv,
    delta_neutral: &ActiveDeltaNeutral,
    start_after: Option<String>,
    limit: Option<u32>,
) -> PaginationResponse<MarketConfig> {
    mock_env
        .app
        .wrap()
        .query_wasm_smart(
            delta_neutral.address(),
            &QueryMsg::MarketConfigs {
                start_after,
                limit,
            },
        )
        .unwrap()
}

pub fn add_active_delta_neutral_market(
    sender: &Addr,
    market_config: MarketConfig,
    mock_env: &mut MockEnv,
    delta_neutral: &ActiveDeltaNeutral,
) -> AnyResult<AppResponse> {
    mock_env.app.execute_contract(
        sender.clone(),
        delta_neutral.address().clone(),
        &ExecuteMsg::AddMarket {
            config: market_config,
        },
        &[],
    )
}

pub fn deposit(
    sender: &Addr,
    funds: Vec<Coin>,
    mock_env: &mut MockEnv,
    delta_neutral: &ActiveDeltaNeutral,
) -> AnyResult<AppResponse> {
    mock_env.app.execute_contract(
        sender.clone(),
        delta_neutral.address().clone(),
        &ExecuteMsg::Deposit {},
        &funds,
    )
}

pub fn withdraw(
    sender: &Addr,
    amount: Uint128,
    recipient: Option<String>,
    mock_env: &mut MockEnv,
    delta_neutral: &ActiveDeltaNeutral,
) -> AnyResult<AppResponse> {
    mock_env.app.execute_contract(
        sender.clone(),
        delta_neutral.address().clone(),
        &ExecuteMsg::Withdraw {
            amount,
            recipient,
        },
        &[],
    )
}

pub fn buy_delta_neutral_market(
    sender: &Addr,
    market_id: &str,
    amount: Uint128,
    swapper_route: SwapperRoute,
    mock_env: &mut MockEnv,
    delta_neutral: &ActiveDeltaNeutral,
) -> AnyResult<AppResponse> {
    mock_env.app.execute_contract(
        sender.clone(),
        delta_neutral.address().clone(),
        &ExecuteMsg::Buy {
            amount,
            market_id: market_id.to_string(),
            swapper_route,
        },
        &[],
    )
}

pub fn sell_delta_neutral_market(
    sender: &Addr,
    market_id: &str,
    amount: Uint128,
    swapper_route: SwapperRoute,
    mock_env: &mut MockEnv,
    delta_neutral: &ActiveDeltaNeutral,
) -> AnyResult<AppResponse> {
    mock_env.app.execute_contract(
        sender.clone(),
        delta_neutral.address().clone(),
        &ExecuteMsg::Sell {
            market_id: market_id.to_string(),
            amount,
            swapper_route,
        },
        &[],
    )
}

pub fn assert_err(res: AnyResult<AppResponse>, err: ContractError) {
    match res {
        Ok(_) => panic!("Result was not an error"),
        Err(generic_err) => {
            let contract_err: ContractError = generic_err.downcast().unwrap();
            assert_eq!(contract_err, err);
        }
    }
}

pub fn deploy_active_delta_neutral_contract(
    mock_env: &mut MockEnv,
    base_denom: &str,
) -> ActiveDeltaNeutral {
    let contract_code_id = mock_env.app.store_code(active_delta_neutral_contract());
    let owner = Addr::unchecked("owner");

    let addr = mock_env
        .app
        .instantiate_contract(
            contract_code_id,
            owner.clone(),
            &InstantiateMsg {
                address_provider: mock_env.address_provider.clone().into(),
                base_denom: base_denom.to_string(),
            },
            &[],
            "mock-active-delta-neutral-contract",
            Some(owner.to_string()),
        )
        .unwrap();

    ActiveDeltaNeutral::new(addr)
}

pub fn query_contract_credit_manager_positions(
    mock_env: &MockEnv,
    delta_neutral: &ActiveDeltaNeutral,
) -> Positions {
    // query the contract config to get the credit manager address
    let config = query_active_delta_neutral_config(mock_env, delta_neutral);

    let credit_account_id = config.credit_account_id.as_ref().unwrap();

    let positions: Positions = mock_env
        .app
        .wrap()
        .query_wasm_smart(
            config.credit_manager_addr,
            &CreditManagerQueryMsg::Positions {
                account_id: credit_account_id.clone(),
                action: None,
            },
        )
        .unwrap();

    positions
}
