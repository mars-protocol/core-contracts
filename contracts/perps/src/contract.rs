use cosmwasm_std::{entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response};
use mars_owner::OwnerInit;
use mars_types::perps::{ExecuteMsg, InstantiateMsg, QueryMsg};

use crate::{error::ContractResult, execute, query, state::OWNER};

pub const CONTRACT_NAME: &str = "mars-perps";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    // initialize contract version info
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // initialize contract ownership info
    OWNER.initialize(
        deps.storage,
        deps.api,
        OwnerInit::SetInitialOwner {
            owner: info.sender.into(),
        },
    )?;

    // initialize contract config and global state
    execute::initialize(deps.storage, msg.check(deps.api)?)
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    _: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::UpdateOwner(update) => OWNER.update(deps, info, update).map_err(Into::into),
        ExecuteMsg::EnableDenom {
            denom,
        } => execute::enable_denom(deps.storage, &info.sender, &denom),
        ExecuteMsg::DisableDenom {
            denom,
        } => execute::disable_denom(deps.storage, &info.sender, &denom),
        ExecuteMsg::Deposit {} => execute::deposit(deps.storage, info),
        ExecuteMsg::Withdraw {
            shares,
        } => execute::withdraw(deps.storage, &info.sender, shares),
        ExecuteMsg::OpenPosition {
            account_id,
            denom,
            size,
        } => execute::open_position(deps, info, account_id, denom, size),
        ExecuteMsg::ClosePosition {
            account_id,
            denom,
        } => execute::close_position(deps, info, account_id, denom),
    }
}

#[entry_point]
pub fn query(deps: Deps, _: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::Owner {} => to_binary(&OWNER.query(deps.storage)?),
        QueryMsg::Config {} => to_binary(&query::config(deps.storage)?),
        QueryMsg::VaultState {} => to_binary(&query::vault_state(deps.storage)?),
        QueryMsg::DenomState {
            denom,
        } => to_binary(&query::denom_state(deps.storage, denom)?),
        QueryMsg::DenomStates {
            start_after,
            limit,
        } => to_binary(&query::denom_states(deps.storage, start_after, limit)?),
        QueryMsg::Deposit {
            depositor,
        } => to_binary(&query::deposit(deps, depositor)?),
        QueryMsg::Deposits {
            start_after,
            limit,
        } => to_binary(&query::deposits(deps.storage, start_after, limit)?),
        QueryMsg::Position {
            account_id,
            denom,
        } => to_binary(&query::position(deps, account_id, denom)?),
        QueryMsg::Positions {
            start_after,
            limit,
        } => to_binary(&query::positions(deps, start_after, limit)?),
        QueryMsg::PositionsByAccount {
            account_id,
        } => to_binary(&query::positions_by_account(deps, account_id)?),
    }
    .map_err(Into::into)
}
