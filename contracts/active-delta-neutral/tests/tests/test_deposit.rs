use cosmwasm_std::{
    coin,
    testing::{mock_dependencies, mock_env, mock_info},
    Addr, Coin, Response, Uint128,
};
use mars_active_delta_neutral::{error::ContractError, execute, state::{CONFIG, OWNER}};
use mars_types::active_delta_neutral::query::Config;
use test_case::test_case;

// Helper to setup contract config
fn setup_config(
    deps: &mut cosmwasm_std::DepsMut,
    owner: &str,
    base_denom: &str,
    credit_manager_addr: &str,
    credit_account_id: Option<String>,
) {
    let config = Config {
        owner: Addr::unchecked(owner),
        base_denom: base_denom.to_string(),
        credit_manager_addr: Addr::unchecked(credit_manager_addr),
        credit_account_id,
        oracle_addr: Addr::unchecked("oracle"),
        perps_addr: Addr::unchecked("perps"),
        health_addr: Addr::unchecked("health"),
        red_bank_addr: Addr::unchecked("red_bank"),
    };
    OWNER.update(deps.storage, &config.owner).unwrap();
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
    Err(ContractError::Unauthorized {}),
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
