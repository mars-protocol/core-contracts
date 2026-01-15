use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Uint128};
use mars_interest_rate::get_scaled_debt_amount;
use mars_types::{
    address_provider::{self, MarsAddressType},
    error::MarsError,
};

use crate::{
    error::ContractError,
    interest_rates::{apply_accumulated_interests, update_interest_rates},
    state::{CONFIG, DEBTS, MARKETS},
    user::User,
};

pub fn write_off_bad_debt(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    denom: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    cw_utils::nonpayable(&info)?;

    let config = CONFIG.load(deps.storage)?;
    let addresses = address_provider::helpers::query_contract_addrs(
        deps.as_ref(),
        &config.address_provider,
        vec![
            MarsAddressType::CreditManager,
            MarsAddressType::RewardsCollector,
            MarsAddressType::Incentives,
        ],
    )?;
    let credit_manager_addr = &addresses[&MarsAddressType::CreditManager];

    if info.sender != *credit_manager_addr {
        return Err(ContractError::Mars(MarsError::Unauthorized {}));
    }

    let rewards_collector_addr = &addresses[&MarsAddressType::RewardsCollector];
    let incentives_addr = &addresses[&MarsAddressType::Incentives];

    let mut market = MARKETS.load(deps.storage, &denom)?;
    let mut response = Response::new();

    response = apply_accumulated_interests(
        deps.storage,
        &env,
        &mut market,
        rewards_collector_addr,
        incentives_addr,
        response,
    )?;

    let mut amount_scaled = get_scaled_debt_amount(amount, &market, env.block.time.seconds())?;

    let user_debt = DEBTS.load(deps.storage, (credit_manager_addr, &denom))?;
    if amount_scaled > user_debt.amount_scaled {
        amount_scaled = user_debt.amount_scaled;
    }
    if amount_scaled > market.debt_total_scaled {
        amount_scaled = market.debt_total_scaled;
    }

    User(credit_manager_addr).decrease_debt(deps.storage, &denom, amount_scaled)?;
    market.decrease_debt(amount_scaled)?;

    response = update_interest_rates(&env, &mut market, response)?;
    MARKETS.save(deps.storage, &denom, &market)?;

    Ok(response
        .add_attribute("action", "write_off_bad_debt")
        .add_attribute("denom", denom)
        .add_attribute("amount", amount)
        .add_attribute("amount_scaled", amount_scaled)
        .add_attribute("credit_manager", credit_manager_addr))
}
