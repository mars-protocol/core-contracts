use cosmwasm_std::{
    coin,
    testing::{mock_dependencies, mock_info},
    Addr, Uint128,
};
use mars_active_delta_neutral::{
    error::ContractError,
    execute,
    state::{CONFIG, OWNER},
};
use mars_owner::OwnerInit;
use mars_testing::multitest::helpers::{AccountToFund, MockEnv};
use mars_types::active_delta_neutral::query::{Config, MarketConfig};
use test_case::test_case;

use crate::tests::helpers::delta_neutral_helpers::{
    add_active_delta_neutral_market, assert_err, deploy_active_delta_neutral_contract, deposit,
    withdraw,
};

// Helper to setup contract config
fn setup_config(
    deps: &mut cosmwasm_std::DepsMut,
    owner: &str,
    base_denom: &str,
    credit_manager_addr: &str,
    credit_account_id: Option<String>,
) {
    let config = Config {
        base_denom: base_denom.to_string(),
        credit_manager_addr: Addr::unchecked(credit_manager_addr),
        credit_account_id,
        oracle_addr: Addr::unchecked("oracle"),
        perps_addr: Addr::unchecked("perps"),
        health_addr: Addr::unchecked("health"),
        red_bank_addr: Addr::unchecked("red_bank"),
        params_addr: Addr::unchecked("params"),
    };
    OWNER
        .initialize(
            deps.storage,
            deps.api,
            OwnerInit::SetInitialOwner {
                owner: owner.to_string(),
            },
        )
        .unwrap();
    CONFIG.save(deps.storage, &config).unwrap();
}

#[test_case(
    "owner",
    Uint128::new(1000),
    None,
    Ok(()),
    true
    ; "success case"
)]
#[test_case(
    "not_owner",
    Uint128::new(1000),
    None,
    Err(ContractError::Owner(mars_owner::OwnerError::NotOwner {})),
    true
    ; "unauthorized sender"
)]
#[test_case(
    "owner",
    Uint128::new(1000),
    None,
    Err(ContractError::CreditAccountNotInitialized {}),
    false
    ; "credit account not initialized"
)]
fn test_withdraw(
    sender: &str,
    amount: Uint128,
    recipient: Option<&str>,
    expected: Result<(), ContractError>,
    credit_account_initialized: bool,
) {
    let mut deps = mock_dependencies();
    let owner = "owner";
    let base_denom = "uusdc";
    let credit_manager_addr = "credit_mgr";
    let credit_account_id = if credit_account_initialized {
        Some("1".to_string())
    } else {
        None
    };
    setup_config(&mut deps.as_mut(), owner, base_denom, credit_manager_addr, credit_account_id);

    let info = mock_info(sender, &[]);
    let res = execute::withdraw(deps.as_mut(), info, amount, recipient.map(|r| r.to_string()));

    match expected {
        Ok(()) => {
            let resp = res.expect("should succeed");
            let attrs = resp.attributes;
            assert!(attrs.iter().any(|a| a.key == "action" && a.value == "withdraw"));
            assert!(attrs.iter().any(|a| a.key == "amount" && a.value == amount.to_string()));
        }
        Err(ref err) => {
            let err_str = format!("{err:?}");
            let got_err = res.unwrap_err();
            let got_str = format!("{got_err:?}");
            assert_eq!(got_str, err_str, "expected {err_str}, got {got_str}");
        }
    }
}

#[test_case(
    Uint128::new(1000),
    true,
    None;
    "success multitest"
)]
fn test_withdraw_multitest(
    amount: Uint128,
    should_succeed: bool,
    expected_err: Option<ContractError>,
) {
    let owner = Addr::unchecked("owner");
    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: owner.clone(),
            funds: vec![coin(1000, "uusdc")],
        })
        .build()
        .unwrap();

    // Add a market
    let market_config = MarketConfig {
        market_id: "market_1".to_string(),
        usdc_denom: "ibc/B559A80D62249C8AA07A380E2A2BEA6E5CA9A6F079C912C3A9E9B494105E4F81"
            .to_string(),
        spot_denom: "ibc/0000000000000000000000000000000000000000000000000000000000000000"
            .to_string(),
        perp_denom: "perps/ubtc".to_string(),
        k: 300u64,
    };
    let active_delta_neutral = deploy_active_delta_neutral_contract(&mut mock);
    let res = add_active_delta_neutral_market(
        &owner,
        market_config.clone(),
        &mut mock,
        &active_delta_neutral,
    );
    assert!(res.is_ok());

    let deposit_res = deposit(&owner, vec![coin(1000, "uusdc")], &mut mock, &active_delta_neutral);
    assert!(deposit_res.is_ok());

    let withdraw_res = withdraw(&owner, amount, None, &mut mock, &active_delta_neutral);

    if should_succeed {
        let resp = withdraw_res.expect("should succeed");
        let attrs = &resp.events.iter().flat_map(|e| &e.attributes).collect::<Vec<_>>();
        assert!(attrs.iter().any(|a| a.key == "action" && a.value == "withdraw"));
        assert!(attrs.iter().any(|a| a.key == "amount" && a.value == amount.to_string()));
    } else if let Some(expected) = expected_err {
        assert_err(withdraw_res, expected);
    } else {
        panic!("Expected failure but no expected error provided");
    }
}
