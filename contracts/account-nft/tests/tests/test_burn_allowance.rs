use std::ops::Add;

use cosmwasm_std::{coin, Addr, Decimal, Empty, StdResult, Uint128};
use cw721::NftInfoResponse;
use cw721_base::{ContractError::Ownership, OwnershipError::NotOwner};
use mars_account_nft::error::{
    ContractError,
    ContractError::{BaseError, BurnNotAllowed},
};
use mars_types::account_nft::QueryMsg::NftInfo;

use super::helpers::{below_max_for_burn, generate_health_response, MockEnv, MAX_VALUE_FOR_BURN};

#[test]
fn only_token_owner_can_burn() {
    let mut mock = MockEnv::new().build().unwrap();

    let user = Addr::unchecked("user");
    let token_id = mock.mint(&user).unwrap();
    mock.set_health_response(&user, &token_id, &below_max_for_burn());

    let bad_guy = Addr::unchecked("bad_guy");
    let res = mock.burn(&bad_guy, &token_id);
    let err: ContractError = res.unwrap_err().downcast().unwrap();
    assert_eq!(err, BaseError(Ownership(NotOwner)));

    mock.burn(&user, &token_id).unwrap();
}

#[test]
fn burn_not_allowed_if_debt_balance() {
    let mut mock = MockEnv::new().build().unwrap();

    let user = Addr::unchecked("user");
    let token_id = mock.mint(&user).unwrap();
    mock.set_health_response(&user, &token_id, &generate_health_response(10_000, 0, false));

    let res = mock.burn(&user, &token_id);
    let error: ContractError = res.unwrap_err().downcast().unwrap();
    assert_eq!(
        error,
        BurnNotAllowed {
            reason: "Account has a debt balance. Value: 10000.".to_string(),
        }
    )
}

#[test]
fn burn_not_allowed_if_too_much_collateral() {
    let mut mock = MockEnv::new().build().unwrap();

    let user = Addr::unchecked("user");
    let token_id = mock.mint(&user).unwrap();
    mock.set_health_response(
        &user,
        &token_id,
        &generate_health_response(0, MAX_VALUE_FOR_BURN.add(Uint128::one()).into(), false),
    );

    let res = mock.burn(&user, &token_id);
    let error: ContractError = res.unwrap_err().downcast().unwrap();
    assert_eq!(
        error,
        BurnNotAllowed {
            reason: "Account collateral value exceeds config set max (1000). Total collateral value: 1001.".to_string()
        }
    )
}

#[test]
fn burn_not_allowed_if_active_perp_positions() {
    let mut mock = MockEnv::new().build().unwrap();

    let user = Addr::unchecked("user");
    let token_id = mock.mint(&user).unwrap();
    mock.set_health_response(&user, &token_id, &generate_health_response(0, 0, true));

    let res = mock.burn(&user, &token_id);
    let error: ContractError = res.unwrap_err().downcast().unwrap();
    assert_eq!(
        error,
        BurnNotAllowed {
            reason: "Account has active perp positions".to_string()
        }
    )
}

#[test]
fn burn_not_allowed_if_active_perp_vault_deposits_or_unlocks() {
    let mut mock = MockEnv::new().build().unwrap();
    let credit_manager = mock.cm_contract.clone();

    let user = Addr::unchecked("user");
    let token_id = mock.mint(&user).unwrap();
    mock.set_health_response(&user, &token_id, &generate_health_response(0, 0, false));

    let usdc_coin = coin(300, "uusdc".to_string());
    mock.fund_user(&credit_manager, &[usdc_coin.clone()]);

    mock.deposit_to_perp_vault(&token_id, &[usdc_coin.clone()]);

    // Assert deposit was successful
    let vault_positon = mock.query_perp_vault_position(&token_id).unwrap();
    assert_eq!(vault_positon.deposit.amount.u128(), 300);
    assert!(!vault_positon.deposit.shares.is_zero());
    assert!(vault_positon.unlocks.is_empty());

    // Assert error on burn due to active perp vault deposits
    let res = mock.burn(&user, &token_id);
    let error: ContractError = res.unwrap_err().downcast().unwrap();
    assert_eq!(
        error,
        BurnNotAllowed {
            reason: "Account has active perp vault deposits / unlocks".to_string()
        }
    );

    // Unlock 50% of the deposit
    let shares = vault_positon.deposit.shares * Decimal::percent(50);
    mock.unlock_from_perp_vault(&token_id, shares);

    // Assert unlock was successful
    let vault_positon = mock.query_perp_vault_position(&token_id).unwrap();
    assert!(!vault_positon.deposit.amount.is_zero());
    assert!(!vault_positon.deposit.shares.is_zero());
    assert!(!vault_positon.unlocks.is_empty());

    // Assert error on burn due to active perp vault deposits / unlocks
    let res = mock.burn(&user, &token_id);
    let error: ContractError = res.unwrap_err().downcast().unwrap();
    assert_eq!(
        error,
        BurnNotAllowed {
            reason: "Account has active perp vault deposits / unlocks".to_string()
        }
    );

    // Unlock the rest of the deposit
    mock.unlock_from_perp_vault(&token_id, vault_positon.deposit.shares);

    // Assert unlock was successful
    let vault_positon = mock.query_perp_vault_position(&token_id).unwrap();
    assert!(vault_positon.deposit.amount.is_zero());
    assert!(vault_positon.deposit.shares.is_zero());
    assert!(!vault_positon.unlocks.is_empty());

    // Assert error on burn due to active perp vault unlocks
    let res = mock.burn(&user, &token_id);
    let error: ContractError = res.unwrap_err().downcast().unwrap();
    assert_eq!(
        error,
        BurnNotAllowed {
            reason: "Account has active perp vault deposits / unlocks".to_string()
        }
    );
}

#[test]
fn burn_allowance_at_exactly_max() {
    let mut mock = MockEnv::new().build().unwrap();

    let user = Addr::unchecked("user");
    let token_id = mock.mint(&user).unwrap();
    mock.set_health_response(
        &user,
        &token_id,
        &generate_health_response(0, MAX_VALUE_FOR_BURN.into(), false),
    );

    mock.burn(&user, &token_id).unwrap();
}

#[test]
fn burn_allowance_when_under_max() {
    let mut mock = MockEnv::new().build().unwrap();

    let user = Addr::unchecked("user");
    let token_id = mock.mint(&user).unwrap();
    mock.set_health_response(&user, &token_id, &generate_health_response(0, 500, false));

    // Assert no errors on calling for NftInfo
    let _: NftInfoResponse<Empty> = mock
        .app
        .wrap()
        .query_wasm_smart(
            mock.nft_contract.clone(),
            &NftInfo {
                token_id: token_id.clone(),
            },
        )
        .unwrap();

    mock.set_health_response(&user, &token_id, &below_max_for_burn());
    mock.burn(&user, &token_id).unwrap();

    let res: StdResult<NftInfoResponse<Empty>> = mock.app.wrap().query_wasm_smart(
        mock.nft_contract,
        &NftInfo {
            token_id,
        },
    );
    res.unwrap_err();
}
