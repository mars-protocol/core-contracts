use cosmwasm_std::{testing::mock_env, Addr};
use mars_owner::OwnerError::NotOwner;
use mars_rewards_collector_base::{error::ContractError, ContractResult};
use mars_rewards_collector_osmosis::entry::{execute, query};
use mars_testing::mock_info;
use mars_types::rewards_collector::{
    ConfigResponse, ExecuteMsg, QueryMsg, UpdateConfig, WhitelistAction,
};

use super::helpers;

#[test]
fn whitelist_add_and_remove_distributor() {
    let mut deps = helpers::setup_test();

    // Initial state should have only the owner whitelisted
    let cfg: ConfigResponse = helpers::query(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(cfg.whitelisted_distributors.len(), 1);
    assert_eq!(cfg.whitelisted_distributors[0], "owner");

    // Non-owner cannot update the whitelist
    let info = mock_info("not_owner");
    let msg = ExecuteMsg::UpdateConfig {
        new_cfg: UpdateConfig {
            whitelist_actions: Some(vec![WhitelistAction::AddAddress {
                address: "alice".to_string(),
            }]),
            ..Default::default()
        },
    };
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(err, ContractError::Owner(NotOwner {}));

    // Add a new distributor by the owner
    let info = mock_info("owner");
    let msg = ExecuteMsg::UpdateConfig {
        new_cfg: UpdateConfig {
            whitelist_actions: Some(vec![WhitelistAction::AddAddress {
                address: "alice".to_string(),
            }]),
            ..Default::default()
        },
    };
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // Check if alice was added
    let cfg: ConfigResponse = helpers::query(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(cfg.whitelisted_distributors.len(), 2);
    assert!(cfg.whitelisted_distributors.contains(&"owner".to_string()));
    assert!(cfg.whitelisted_distributors.contains(&"alice".to_string()));

    // Add another distributor
    let msg = ExecuteMsg::UpdateConfig {
        new_cfg: UpdateConfig {
            whitelist_actions: Some(vec![WhitelistAction::AddAddress {
                address: "bob".to_string(),
            }]),
            ..Default::default()
        },
    };
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // Check if bob was added
    let cfg: ConfigResponse = helpers::query(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(cfg.whitelisted_distributors.len(), 3);
    assert!(cfg.whitelisted_distributors.contains(&"bob".to_string()));

    // Remove a distributor
    let msg = ExecuteMsg::UpdateConfig {
        new_cfg: UpdateConfig {
            whitelist_actions: Some(vec![WhitelistAction::RemoveAddress {
                address: "alice".to_string(),
            }]),
            ..Default::default()
        },
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Check if alice was removed
    let cfg: ConfigResponse = helpers::query(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(cfg.whitelisted_distributors.len(), 2);
    assert!(cfg.whitelisted_distributors.contains(&"owner".to_string()));
    assert!(cfg.whitelisted_distributors.contains(&"bob".to_string()));
    assert!(!cfg.whitelisted_distributors.contains(&"alice".to_string()));
}

#[test]
fn only_whitelisted_can_distribute_rewards() {
    let mut deps = helpers::setup_test();

    // Add a new distributor "alice" to the whitelist
    let info = mock_info("owner");
    let msg = ExecuteMsg::UpdateConfig {
        new_cfg: UpdateConfig {
            whitelist_actions: Some(vec![WhitelistAction::AddAddress {
                address: "alice".to_string(),
            }]),
            ..Default::default()
        },
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Set up a balance of tokens
    deps.querier.set_contract_balances(&[cosmwasm_std::coin(1000, "umars")]);

    // Owner can distribute rewards
    let info = mock_info("owner");
    let msg = ExecuteMsg::DistributeRewards {
        denom: "umars".to_string(),
    };
    let result: ContractResult<_> = execute(deps.as_mut(), mock_env(), info, msg);
    assert!(result.is_ok());

    // Alice can distribute rewards because she's whitelisted
    let info = mock_info("alice");
    let msg = ExecuteMsg::DistributeRewards {
        denom: "umars".to_string(),
    };
    let result: ContractResult<_> = execute(deps.as_mut(), mock_env(), info, msg);
    assert!(result.is_ok());

    // Bob cannot distribute rewards because he's not whitelisted
    let info = mock_info("bob");
    let msg = ExecuteMsg::DistributeRewards {
        denom: "umars".to_string(),
    };
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert!(matches!(
        err,
        ContractError::UnauthorizedDistributor { sender: _ }
    ));
    if let ContractError::UnauthorizedDistributor { sender } = err {
        assert_eq!(sender, "bob");
    }

    // Remove all distributors except owner
    let info = mock_info("owner");
    let msg = ExecuteMsg::UpdateConfig {
        new_cfg: UpdateConfig {
            whitelist_actions: Some(vec![WhitelistAction::RemoveAddress {
                address: "alice".to_string(),
            }]),
            ..Default::default()
        },
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Alice can no longer distribute rewards
    let info = mock_info("alice");
    let msg = ExecuteMsg::DistributeRewards {
        denom: "umars".to_string(),
    };
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert!(matches!(
        err,
        ContractError::UnauthorizedDistributor { sender: _ }
    ));
}
