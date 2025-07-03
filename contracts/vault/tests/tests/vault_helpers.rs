use std::str::FromStr;

use anyhow::Result as AnyResult;
use cosmwasm_std::{coin, Addr, Coin, Decimal, Int128, Uint128};
use cw_multi_test::{AppResponse, Executor};
use cw_paginate::PaginationResponse;
use mars_testing::multitest::helpers::{
    deploy_managed_vault_with_performance_fee, AccountToFund, CoinInfo,
};
use mars_types::{
    credit_manager::{Action, Positions, QueryMsg as CreditManagerQueryMsg},
    params::ManagedVaultConfigUpdate,
};
use mars_vault::{
    msg::{
        ExecuteMsg, ExtensionExecuteMsg, ExtensionQueryMsg, QueryMsg, UserPnlResponse,
        VaultInfoResponseExt, VaultPnlResponse, VaultUnlock,
    },
    performance_fee::{PerformanceFeeConfig, PerformanceFeeState},
};

use super::helpers::MockEnv;

pub fn execute_bind_credit_manager_account(
    mock_env: &mut MockEnv,
    sender: &Addr,
    vault: &Addr,
    account_id: &str,
) -> AnyResult<AppResponse> {
    mock_env.app.execute_contract(
        sender.clone(),
        vault.clone(),
        &ExecuteMsg::VaultExtension(ExtensionExecuteMsg::BindCreditManagerAccount {
            account_id: account_id.to_string(),
        }),
        &[],
    )
}

pub fn execute_deposit(
    mock_env: &mut MockEnv,
    sender: &Addr,
    vault: &Addr,
    amount: Uint128,
    recipient: Option<String>,
    funds: &[Coin],
) -> AnyResult<AppResponse> {
    mock_env.app.execute_contract(
        sender.clone(),
        vault.clone(),
        &ExecuteMsg::Deposit {
            amount,
            recipient,
        },
        funds,
    )
}

pub fn execute_redeem(
    mock_env: &mut MockEnv,
    sender: &Addr,
    vault: &Addr,
    amount: Uint128,
    recipient: Option<String>,
    funds: &[Coin],
) -> AnyResult<AppResponse> {
    mock_env.app.execute_contract(
        sender.clone(),
        vault.clone(),
        &ExecuteMsg::Redeem {
            amount,
            recipient,
        },
        funds,
    )
}

pub fn execute_unlock(
    mock_env: &mut MockEnv,
    sender: &Addr,
    vault: &Addr,
    amount: Uint128,
    funds: &[Coin],
) -> AnyResult<AppResponse> {
    mock_env.app.execute_contract(
        sender.clone(),
        vault.clone(),
        &ExecuteMsg::VaultExtension(ExtensionExecuteMsg::Unlock {
            amount,
        }),
        funds,
    )
}

pub fn execute_withdraw_performance_fee(
    mock_env: &mut MockEnv,
    sender: &Addr,
    vault: &Addr,
    new_performance_fee_config: Option<PerformanceFeeConfig>,
) -> AnyResult<AppResponse> {
    mock_env.app.execute_contract(
        sender.clone(),
        vault.clone(),
        &ExecuteMsg::VaultExtension(ExtensionExecuteMsg::WithdrawPerformanceFee {
            new_performance_fee_config,
        }),
        &[],
    )
}

pub fn open_perp_position(
    mock: &mut MockEnv,
    fund_acc_id: &str,
    fund_manager: &Addr,
    perp_denom: &str,
    size: Int128,
) {
    mock.update_credit_account(
        fund_acc_id,
        fund_manager,
        vec![Action::ExecutePerpOrder {
            denom: perp_denom.to_string(),
            order_size: size,
            reduce_only: None,
            order_type: None,
        }],
        &[],
    )
    .unwrap();
}

pub fn query_vault_info(mock_env: &MockEnv, vault: &Addr) -> VaultInfoResponseExt {
    mock_env
        .app
        .wrap()
        .query_wasm_smart(
            vault.to_string(),
            &QueryMsg::VaultExtension(ExtensionQueryMsg::VaultInfo {}),
        )
        .unwrap()
}

pub fn query_total_assets(mock_env: &MockEnv, vault: &Addr) -> Uint128 {
    mock_env.app.wrap().query_wasm_smart(vault.to_string(), &QueryMsg::TotalAssets {}).unwrap()
}

pub fn query_total_vault_token_supply(mock_env: &MockEnv, vault: &Addr) -> Uint128 {
    mock_env
        .app
        .wrap()
        .query_wasm_smart(vault.to_string(), &QueryMsg::TotalVaultTokenSupply {})
        .unwrap()
}

pub fn query_user_unlocks(mock_env: &MockEnv, vault: &Addr, user_addr: &Addr) -> Vec<VaultUnlock> {
    mock_env
        .app
        .wrap()
        .query_wasm_smart(
            vault.to_string(),
            &QueryMsg::VaultExtension(ExtensionQueryMsg::UserUnlocks {
                user_address: user_addr.to_string(),
            }),
        )
        .unwrap()
}

pub fn query_all_unlocks(
    mock_env: &MockEnv,
    vault: &Addr,
    start_after: Option<(String, u64)>,
    limit: Option<u32>,
) -> PaginationResponse<VaultUnlock> {
    mock_env
        .app
        .wrap()
        .query_wasm_smart(
            vault.to_string(),
            &QueryMsg::VaultExtension(ExtensionQueryMsg::AllUnlocks {
                start_after,
                limit,
            }),
        )
        .unwrap()
}

pub fn query_convert_to_assets(mock_env: &MockEnv, vault: &Addr, vault_tokens: Uint128) -> Uint128 {
    mock_env
        .app
        .wrap()
        .query_wasm_smart(
            vault.to_string(),
            &QueryMsg::ConvertToAssets {
                amount: vault_tokens,
            },
        )
        .unwrap()
}

pub fn query_convert_to_shares(mock_env: &MockEnv, vault: &Addr, base_tokens: Uint128) -> Uint128 {
    mock_env
        .app
        .wrap()
        .query_wasm_smart(
            vault.to_string(),
            &QueryMsg::ConvertToShares {
                amount: base_tokens,
            },
        )
        .unwrap()
}

pub fn query_performance_fee(mock_env: &MockEnv, vault: &Addr) -> PerformanceFeeState {
    mock_env
        .app
        .wrap()
        .query_wasm_smart(
            vault.to_string(),
            &QueryMsg::VaultExtension(ExtensionQueryMsg::PerformanceFeeState {}),
        )
        .unwrap()
}

pub fn query_user_pnl(mock_env: &MockEnv, vault: &Addr, user: &Addr) -> UserPnlResponse {
    mock_env
        .app
        .wrap()
        .query_wasm_smart(
            vault.to_string(),
            &QueryMsg::VaultExtension(ExtensionQueryMsg::UserPnl {
                user_address: user.to_string(),
            }),
        )
        .unwrap()
}

pub fn query_vault_pnl(mock_env: &MockEnv, vault: &Addr) -> VaultPnlResponse {
    mock_env
        .app
        .wrap()
        .query_wasm_smart(
            vault.to_string(),
            &QueryMsg::VaultExtension(ExtensionQueryMsg::VaultPnl {}),
        )
        .unwrap()
}

pub fn query_account_positions(
    mock_env: &MockEnv,
    credit_manager: &Addr,
    account_id: &str,
) -> Positions {
    mock_env
        .app
        .wrap()
        .query_wasm_smart(
            credit_manager.to_string(),
            &CreditManagerQueryMsg::Positions {
                account_id: account_id.to_string(),
                action: None,
            },
        )
        .unwrap()
}

pub fn assert_vault_err(res: AnyResult<AppResponse>, err: mars_vault::error::ContractError) {
    match res {
        Ok(_) => panic!("Result was not an error"),
        Err(generic_err) => {
            let contract_err: mars_vault::error::ContractError = generic_err.downcast().unwrap();
            assert_eq!(contract_err, err);
        }
    }
}

pub struct VaultSetup {
    pub mock: MockEnv,
    pub fund_manager: Addr,
    pub managed_vault_addr: Addr,
    pub fund_acc_id: String,
}

pub fn instantiate_vault(
    uusdc_info: &CoinInfo,
    uatom_info: &CoinInfo,
    base_denom: &str,
) -> VaultSetup {
    let fund_manager = Addr::unchecked("fund-manager");
    let user = Addr::unchecked("user");
    let user_funded_amt = Uint128::new(100_000_000_000);
    let user2 = Addr::unchecked("user2");
    let user2_funded_amt = Uint128::new(100_000_000_000);
    let mut mock = MockEnv::new()
        .set_params(&[uusdc_info.clone(), uatom_info.clone()])
        .fund_account(AccountToFund {
            addr: fund_manager.clone(),
            funds: vec![coin(1_000_000_000, "untrn")],
        })
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![coin(user_funded_amt.u128(), uusdc_info.denom.clone())],
        })
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![coin(user_funded_amt.u128(), uatom_info.denom.clone())],
        })
        .fund_account(AccountToFund {
            addr: user2.clone(),
            funds: vec![coin(user2_funded_amt.u128(), uusdc_info.denom.clone())],
        })
        .build()
        .unwrap();
    let credit_manager = mock.rover.clone();

    let managed_vault_addr = deploy_managed_vault_with_performance_fee(
        &mut mock.app,
        &fund_manager,
        &credit_manager,
        1,
        PerformanceFeeConfig {
            fee_rate: Decimal::from_str("0.0000208").unwrap(),
            withdrawal_interval: 60,
        },
        base_denom,
        None,
    );

    let code_id = mock.query_code_id(&managed_vault_addr);
    mock.update_managed_vault_config(ManagedVaultConfigUpdate::AddCodeId(code_id));

    let fund_acc_id = mock.create_fund_manager_account(&fund_manager, &managed_vault_addr);

    VaultSetup {
        mock,
        fund_manager,
        managed_vault_addr,
        fund_acc_id,
    }
}
