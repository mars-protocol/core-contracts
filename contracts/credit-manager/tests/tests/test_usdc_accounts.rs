use cosmwasm_std::{Addr, Decimal, Uint128};
use mars_credit_manager::error::ContractError;
use mars_testing::multitest::helpers::uusdc_info;
use mars_types::{
    adapters::vault::VaultUnchecked,
    credit_manager::{
        Action::{self},
        ActionAmount, ActionCoin, LiquidateRequest,
    },
    health::AccountKind,
    swapper::{OsmoRoute, OsmoSwap, SwapperRoute},
};
use test_case::test_case;

use super::helpers::{assert_err, MockEnv};

#[test]
fn queries_return_the_expected_kind() {
    let mut mock = MockEnv::new().build().unwrap();
    let user = Addr::unchecked("user");

    let account_id = mock.create_usdc_account(&user).unwrap();
    let kind = mock.query_account_kind(&account_id);
    assert_eq!(AccountKind::UsdcMargin, kind);
}

#[test_case("SwapExactIn", vec![Action::SwapExactIn {
    coin_in: ActionCoin {
        denom: "mars".to_string(),
        amount: ActionAmount::Exact(Uint128::new(12)),
    },
    denom_out: "osmo".to_string(),
    min_receive: Uint128::zero(),
    route: Some(SwapperRoute::Osmo(OsmoRoute {
        swaps: vec![OsmoSwap {
            pool_id: 101,
            to: "osmo".to_string(),
        }],
    })),
}]; "illegal SwapExactIn")]
#[test_case("Lend", vec![Action::Lend(uusdc_info().to_action_coin(50))]; "illegal Lend")]
#[test_case("Borrow", vec![Action::Borrow(uusdc_info().to_coin(50))]; "illegal Borrow")]
#[test_case("Reclaim", vec![Action::Reclaim(uusdc_info().to_action_coin(50))]; "illegal Reclaim")]
#[test_case("ClaimRewards", vec![Action::ClaimRewards {}]; "illegal ClaimRewards")]
#[test_case("ProvideLiquidity", vec![Action::ProvideLiquidity {
    coins_in: vec![uusdc_info().to_action_coin(100)],
    lp_token_out: "lp_token".to_string(),
    slippage: Decimal::percent(5),
}]; "illegal ProvideLiquidity")]
#[test_case("WithdrawLiquidity", vec![Action::WithdrawLiquidity {
    lp_token: uusdc_info().to_action_coin(100),
    slippage: Decimal::percent(5),
}]; "illegal WithdrawLiquidity")]
#[test_case("StakeAstroLp", vec![Action::StakeAstroLp {
    lp_token: uusdc_info().to_action_coin(100),
}]; "illegal StakeAstroLp")]
#[test_case("UnstakeAstroLp", vec![Action::UnstakeAstroLp {
    lp_token: uusdc_info().to_action_coin(100),
}]; "illegal UnstakeAstroLp")]
#[test_case("ClaimAstroLpRewards", vec![Action::ClaimAstroLpRewards {
    lp_denom: "astro_lp".to_string(),
}]; "illegal ClaimAstroLpRewards")]
#[test_case("Liquidate", vec![Action::Liquidate {
    liquidatee_account_id: "acc123".to_string(),
    debt_coin: uusdc_info().to_coin(100),
    request: LiquidateRequest::Deposit("uosmo".to_string()),
}]; "illegal Liquidate")]
#[test_case("Repay", vec![Action::Repay {
    recipient_account_id: None,
    coin: uusdc_info().to_action_coin(100),
}]; "illegal Repay")]
#[test_case("EnterVault", vec![Action::EnterVault {
    vault: VaultUnchecked::new("vault_1".to_string()),
    coin: uusdc_info().to_action_coin(100),
}]; "illegal EnterVault")]
#[test_case("ExitVault", vec![Action::ExitVault {
    vault: VaultUnchecked::new("vault_1".to_string()),
    amount: Uint128::new(50),
}]; "illegal ExitVault")]
#[test_case("RequestVaultUnlock", vec![Action::RequestVaultUnlock {
    vault: VaultUnchecked::new("vault_1".to_string()),
    amount: Uint128::new(50),
}]; "illegal RequestVaultUnlock")]
#[test_case("ExitVaultUnlocked", vec![Action::ExitVaultUnlocked {
    id: 1,
    vault: VaultUnchecked::new("vault_1".to_string()),
}]; "illegal ExitVaultUnlocked")]
fn cannot_perform_illegal_actions(action_name: &str, actions: Vec<Action>) {
    let coin_info = uusdc_info();
    let mut mock = MockEnv::new().set_params(&[coin_info.clone()]).build().unwrap();
    let user = Addr::unchecked("user");
    let account_id = mock.create_usdc_account(&user).unwrap();

    let res = mock.update_credit_account(&account_id, &user, actions, &[]);
    assert_err(
        res,
        ContractError::IllegalAction {
            user: account_id.to_string(),
            action: action_name.to_string(),
        },
    );
}
