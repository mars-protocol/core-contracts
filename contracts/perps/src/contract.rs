use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response,
};
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
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::UpdateOwner(update) => OWNER.update(deps, info, update).map_err(Into::into),
        ExecuteMsg::InitDenom {
            denom,
            max_funding_velocity,
            skew_scale,
        } => execute::init_denom(
            deps.storage,
            env,
            &info.sender,
            &denom,
            max_funding_velocity,
            skew_scale,
        ),
        ExecuteMsg::EnableDenom {
            denom,
        } => execute::enable_denom(deps.storage, env, &info.sender, &denom),
        ExecuteMsg::DisableDenom {
            denom,
        } => execute::disable_denom(deps, env, &info.sender, &denom),
        ExecuteMsg::Deposit {} => execute::deposit(deps, info, env.block.time.seconds()),
        ExecuteMsg::Unlock {
            shares,
        } => execute::unlock(deps, env.block.time.seconds(), &info.sender, shares),
        ExecuteMsg::Withdraw {} => {
            execute::withdraw(deps.storage, env.block.time.seconds(), &info.sender)
        }
        ExecuteMsg::OpenPosition {
            account_id,
            denom,
            size,
        } => execute::open_position(deps, env, info, account_id, denom, size),
        ExecuteMsg::ClosePosition {
            account_id,
            denom,
        } => execute::close_position(deps, env, info, account_id, denom),
        ExecuteMsg::ModifyPosition {
            account_id,
            denom,
            new_size,
        } => execute::modify_position(deps, env, info, account_id, denom, new_size),
    }
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::Owner {} => to_json_binary(&OWNER.query(deps.storage)?),
        QueryMsg::Config {} => to_json_binary(&query::config(deps.storage)?),
        QueryMsg::VaultState {} => to_json_binary(&query::vault_state(deps.storage)?),
        QueryMsg::DenomState {
            denom,
        } => to_json_binary(&query::denom_state(deps.storage, denom)?),
        QueryMsg::DenomStates {
            start_after,
            limit,
        } => to_json_binary(&query::denom_states(deps.storage, start_after, limit)?),
        QueryMsg::PerpDenomState {
            denom,
        } => to_json_binary(&query::perp_denom_state(deps, env.block.time.seconds(), denom)?),
        QueryMsg::Deposit {
            depositor,
        } => to_json_binary(&query::deposit(deps, depositor, env.block.time.seconds())?),
        QueryMsg::Deposits {
            start_after,
            limit,
        } => to_json_binary(&query::deposits(deps, start_after, limit, env.block.time.seconds())?),
        QueryMsg::Unlocks {
            depositor,
        } => to_json_binary(&query::unlocks(deps, depositor)?),
        QueryMsg::Position {
            account_id,
            denom,
            new_size,
        } => to_json_binary(&query::position(
            deps,
            env.block.time.seconds(),
            account_id,
            denom,
            new_size,
        )?),
        QueryMsg::Positions {
            start_after,
            limit,
        } => to_json_binary(&query::positions(deps, env.block.time.seconds(), start_after, limit)?),
        QueryMsg::PositionsByAccount {
            account_id,
        } => to_json_binary(&query::positions_by_account(
            deps,
            env.block.time.seconds(),
            account_id,
        )?),
        QueryMsg::TotalPnl {} => to_json_binary(&query::total_pnl(deps, env.block.time.seconds())?),
        QueryMsg::OpeningFee {
            denom,
            size,
        } => to_json_binary(&query::opening_fee(deps, &denom, size)?),
        QueryMsg::DenomAccounting {
            denom,
        } => to_json_binary(&query::denom_accounting(deps, &denom, env.block.time.seconds())?),
        QueryMsg::TotalAccounting {} => {
            to_json_binary(&query::total_accounting(deps, env.block.time.seconds())?)
        }
        QueryMsg::DenomRealizedPnlForAccount {
            account_id,
            denom,
        } => to_json_binary(&query::denom_realized_pnl_for_account(deps, account_id, denom)?),
        QueryMsg::PositionFees {
            account_id,
            denom,
            new_size,
        } => to_json_binary(&query::position_fees(deps, &account_id, &denom, new_size)?),
    }
    .map_err(Into::into)
}
