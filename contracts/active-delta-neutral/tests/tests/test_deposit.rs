use cosmwasm_std::{
    coin,
    testing::{mock_dependencies, mock_info},
    Addr, Coin,
};
use mars_active_delta_neutral::{
    error::ContractError,
    execute,
    state::{CONFIG, OWNER},
};
use mars_owner::OwnerInit;
use mars_testing::multitest::helpers::{AccountToFund, MockEnv};
use mars_types::active_delta_neutral::{order_validation::DynamicValidator, query::{Config, MarketConfig}};
use test_case::test_case;

use crate::tests::helpers::delta_neutral_helpers::{
    add_active_delta_neutral_market, assert_err, deploy_active_delta_neutral_contract, deposit,
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
    vec![coin(1000, "uusdc")], 
    Some("1"),
    Ok(()),
    "success"
    ; "success case"
)]
#[test_case(
    "not_owner", 
    vec![coin(1000, "uusdc")], 
    Some("1"),
    Err(ContractError::Owner(mars_owner::OwnerError::NotOwner {})),
    "unauthorized"
    ; "unauthorized sender"
)]
#[test_case(
    "owner", 
    vec![coin(1000, "uusdc"), coin(100, "uatom")], 
    Some("1"),
    Err(ContractError::ExcessAssets { denom: "uusdc".to_string() }),
    "excess assets"
    ; "excess assets"
)]
#[test_case(
    "owner", 
    vec![coin(1000, "uusdc")], 
    None,
    Err(ContractError::CreditAccountNotInitialized {}),
    "credit account not initialized"
    ; "credit account not initialized"
)]
fn test_deposit(
    sender: &str,
    funds: Vec<Coin>,
    credit_account_id: Option<&str>,
    expected: Result<(), ContractError>,
    _case: &str,
) {
    let mut deps = mock_dependencies();
    let owner = "owner";
    let base_denom = "uusdc";
    let credit_manager_addr = "credit_mgr";
    let credit_account_id = credit_account_id.map(|s| s.to_string());
    setup_config(&mut deps.as_mut(), owner, base_denom, credit_manager_addr, credit_account_id);

    let info = mock_info(sender, &funds);
    let res = execute::deposit(deps.as_mut(), info);

    match expected {
        Ok(()) => {
            let resp = res.expect("should succeed");
            let attrs = resp.attributes;
            assert!(attrs.iter().any(|a| a.key == "action" && a.value == "deposit"));
            assert!(attrs
                .iter()
                .any(|a| a.key == "amount" && a.value == funds[0].amount.to_string()));
            assert!(attrs.iter().any(|a| a.key == "denom" && a.value == funds[0].denom));
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
    vec![coin(1000, "uusdc")],
    true,
    None;
    "success multitest"
)]
#[test_case(
    vec![coin(1000, "uusdc"), coin(100, "uatom")],
    false,
    Some(ContractError::ExcessAssets { denom: "uusdc".to_string() })
    ;
    "excess assets multitest"
)]
fn test_deposit_multitest(
    funds: Vec<Coin>,
    should_succeed: bool,
    expected_err: Option<ContractError>,
) {
    let owner = Addr::unchecked("owner");
    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: owner.clone(),
            funds: funds.clone(),
        })
        .build()
        .unwrap();

    // Add a market
    let market_config = MarketConfig {
        market_id: "market_1".to_string(),
        usdc_denom: "uusdc"
            .to_string(),
        spot_denom: "uusdc"
            .to_string(),
        perp_denom: "perps/ubtc".to_string(),
        validation_model: DynamicValidator { k: 300 },
    };
    let active_delta_neutral = deploy_active_delta_neutral_contract(
        &mut mock,
        "uusdc",
    );
    let res = add_active_delta_neutral_market(
        &owner,
        market_config.clone(),
        &mut mock,
        &active_delta_neutral,
    );
    assert!(res.is_ok());

    let deposit_res = deposit(&owner, funds.clone(), &mut mock, &active_delta_neutral);

    if should_succeed {
        let resp = deposit_res.expect("should succeed");
        let attrs = &resp.events.iter().flat_map(|e| &e.attributes).collect::<Vec<_>>();
        assert!(attrs.iter().any(|a| a.key == "action" && a.value == "deposit"));
        assert!(attrs
            .iter()
            .any(|a| a.key == "amount" && a.value == funds[0].amount.clone().to_string()));
        assert!(attrs.iter().any(|a| a.key == "denom" && a.value == funds[0].denom.clone()));
    } else if let Some(expected) = expected_err {
        assert_err(deposit_res, expected);
    } else {
        panic!("Expected failure but no expected error provided");
    }
}
