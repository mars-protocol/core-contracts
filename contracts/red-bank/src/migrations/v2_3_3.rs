use cosmwasm_std::{Addr, Decimal, DepsMut, Response};
use cw2::{assert_contract_version, set_contract_version};
use mars_types::keys::{UserId, UserIdKey};

use crate::{
    contract::{CONTRACT_NAME, CONTRACT_VERSION},
    error::ContractError,
    state::{COLLATERALS, MARKETS},
};

const FROM_VERSION: &str = "2.3.2";

pub fn migrate(deps: DepsMut, haircut: Decimal, denom: &str) -> Result<Response, ContractError> {
    // Make sure we're migrating the correct contract and from the correct version
    assert_contract_version(deps.storage, &format!("crates.io:{CONTRACT_NAME}"), FROM_VERSION)?;
    // Load affected market
    let mut market = MARKETS.load(deps.storage, denom)?;
    // Apply haircut
    let new_index = market.liquidity_index.checked_mul(Decimal::one().checked_sub(haircut)?)?;
    market.liquidity_index = new_index;
    // Save new state
    MARKETS.save(deps.storage, denom, &market)?;

    // Remove MPF collateral
    let mpf_account_id = "4954";
    let acc_id = mpf_account_id.to_string();

    let user_id = UserId::credit_manager(
        Addr::unchecked(
            "neutron1qdzn3l4kn7gsjna2tfpg3g3mwd6kunx4p50lfya59k02846xas6qslgs3r".to_string(),
        ),
        acc_id,
    );
    let user_id_key: UserIdKey = user_id.try_into()?;

    COLLATERALS.remove(deps.storage, (&user_id_key, denom));

    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("from_version", FROM_VERSION)
        .add_attribute("to_version", CONTRACT_VERSION)
        .add_attribute("to_version", CONTRACT_VERSION)
        .add_attribute("haircut_percent", haircut.to_string())
        .add_attribute("haircut_market", denom))
}
