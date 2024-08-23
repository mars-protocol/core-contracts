use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response,
};
use mars_owner::OwnerInit;
use mars_types::{
    oracle::ActionKind,
    perps::{ExecuteMsg, InstantiateMsg, QueryMsg},
};

use crate::{
    deleverage::{self, handle_deleverage_request_reply, DELEVERAGE_REQUEST_REPLY_ID},
    denom_management,
    error::{ContractError, ContractResult},
    initialize, position_management, query,
    state::OWNER,
    vault,
};

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
    initialize::initialize(deps.storage, msg.check(deps.api)?)
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
        } => denom_management::init_denom(
            deps.storage,
            env,
            &info.sender,
            &denom,
            max_funding_velocity,
            skew_scale,
        ),
        ExecuteMsg::EnableDenom {
            denom,
        } => denom_management::enable_denom(deps.storage, env, &info.sender, &denom),
        ExecuteMsg::DisableDenom {
            denom,
        } => denom_management::disable_denom(deps, env, &info.sender, &denom),
        ExecuteMsg::Deposit {
            account_id,
        } => vault::deposit(deps, info, env.block.time.seconds(), account_id),
        ExecuteMsg::Unlock {
            account_id,
            shares,
        } => vault::unlock(deps, info, env.block.time.seconds(), account_id, shares),
        ExecuteMsg::Withdraw {
            account_id,
        } => vault::withdraw(deps, info, env.block.time.seconds(), account_id),
        ExecuteMsg::CloseAllPositions {
            account_id,
            action,
        } => position_management::close_all_positions(
            deps,
            env,
            info,
            account_id,
            action.unwrap_or(ActionKind::Default),
        ),
        ExecuteMsg::ExecutePerpOrder {
            account_id,
            denom,
            size,
            reduce_only,
        } => position_management::execute_perp_order(
            deps,
            env,
            info,
            account_id,
            denom,
            size,
            reduce_only,
        ),
        ExecuteMsg::Deleverage {
            account_id,
            denom,
        } => deleverage::deleverage(deps, env, account_id, denom),
    }
}

#[entry_point]
pub fn reply(deps: DepsMut, env: Env, reply: Reply) -> ContractResult<Response> {
    match reply.id {
        DELEVERAGE_REQUEST_REPLY_ID => handle_deleverage_request_reply(deps, env, reply),
        id => Err(ContractError::ReplyIdError(id)),
    }
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::Owner {} => to_json_binary(&OWNER.query(deps.storage)?),
        QueryMsg::Config {} => to_json_binary(&query::config(deps.storage)?),
        QueryMsg::Vault {
            action,
        } => to_json_binary(&query::vault(
            deps,
            env.block.time.seconds(),
            action.unwrap_or(ActionKind::Default),
        )?),
        QueryMsg::DenomState {
            denom,
        } => to_json_binary(&query::denom_state(deps.storage, denom)?),
        QueryMsg::DenomStates {
            start_after,
            limit,
        } => to_json_binary(&query::denom_states(deps.storage, start_after, limit)?),
        QueryMsg::PerpDenomState {
            denom,
            action,
        } => {
            to_json_binary(&query::perp_denom_state(deps, env.block.time.seconds(), action, denom)?)
        }
        QueryMsg::PerpDenomStates {
            action,
            start_after,
            limit,
        } => to_json_binary(&query::perp_denom_states(
            deps,
            env.block.time.seconds(),
            action,
            start_after,
            limit,
        )?),
        QueryMsg::PerpVaultPosition {
            user_address,
            account_id,
            action,
        } => {
            let user_addr = deps.api.addr_validate(&user_address)?;
            to_json_binary(&query::perp_vault_position(
                deps,
                user_addr,
                account_id,
                env.block.time.seconds(),
                action.unwrap_or(ActionKind::Default),
            )?)
        }
        QueryMsg::Deposit {
            user_address,
            account_id,
        } => {
            let user_addr = deps.api.addr_validate(&user_address)?;
            to_json_binary(&query::deposit(deps, user_addr, account_id, env.block.time.seconds())?)
        }
        QueryMsg::Unlocks {
            user_address,
            account_id,
        } => {
            let user_addr = deps.api.addr_validate(&user_address)?;
            to_json_binary(&query::unlocks(deps, user_addr, account_id, env.block.time.seconds())?)
        }
        QueryMsg::Position {
            account_id,
            denom,
            order_size,
        } => to_json_binary(&query::position(
            deps,
            env.block.time.seconds(),
            account_id,
            denom,
            order_size,
        )?),
        QueryMsg::Positions {
            start_after,
            limit,
        } => to_json_binary(&query::positions(deps, env.block.time.seconds(), start_after, limit)?),
        QueryMsg::PositionsByAccount {
            account_id,
            action,
        } => to_json_binary(&query::positions_by_account(
            deps,
            env.block.time.seconds(),
            account_id,
            action.unwrap_or(ActionKind::Default),
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
