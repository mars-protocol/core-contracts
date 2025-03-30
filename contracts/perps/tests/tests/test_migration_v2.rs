use cosmwasm_std::{attr, testing::mock_env, Empty, Event, Order, StdResult};
use cw2::{ContractVersion, VersionError};
use mars_perps::{contract::migrate, error::ContractError, state::MARKET_STATES};
use mars_testing::mock_dependencies;
use mars_types::perps::MarketState;

#[test]
fn wrong_contract_name() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "contract_xyz", "2.2.0").unwrap();

    let err = migrate(deps.as_mut(), mock_env(), Empty {}).unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongContract {
            expected: "crates.io:mars-perps".to_string(),
            found: "contract_xyz".to_string()
        })
    );
}

#[test]
fn wrong_contract_version() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-perps", "4.1.0").unwrap();

    let err = migrate(deps.as_mut(), mock_env(), Empty {}).unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongVersion {
            expected: "2.2.0".to_string(),
            found: "4.1.0".to_string()
        })
    );
}

#[test]
fn successful_migration() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-perps", "2.2.0").unwrap();

    MARKET_STATES.save(deps.as_mut().storage, "perps/utia", &MarketState::default()).unwrap();
    MARKET_STATES.save(deps.as_mut().storage, "perps/unil", &MarketState::default()).unwrap();
    MARKET_STATES.save(deps.as_mut().storage, "perps/eigen", &MarketState::default()).unwrap();

    let market_states = MARKET_STATES
        .keys(deps.as_ref().storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();

    assert_eq!(market_states, vec!["perps/eigen", "perps/unil", "perps/utia"]);

    let res = migrate(deps.as_mut(), mock_env(), Empty {}).unwrap();

    let market_states = MARKET_STATES
        .keys(deps.as_ref().storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();

    assert_eq!(market_states, vec!["perps/eigen", "perps/utia"]);

    assert_eq!(res.messages, vec![]);
    assert_eq!(res.events, vec![] as Vec<Event>);
    assert!(res.data.is_none());
    assert_eq!(
        res.attributes,
        vec![attr("action", "migrate"), attr("from_version", "2.2.0"), attr("to_version", "2.2.1")]
    );

    let new_contract_version = ContractVersion {
        contract: "crates.io:mars-perps".to_string(),
        version: "2.2.1".to_string(),
    };
    assert_eq!(cw2::get_contract_version(deps.as_ref().storage).unwrap(), new_contract_version);
}
