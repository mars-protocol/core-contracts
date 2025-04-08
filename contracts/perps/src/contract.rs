use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Reply, Response,
};
use mars_owner::OwnerInit;
use mars_types::{
    oracle::ActionKind,
    perps::{ExecuteMsg, InstantiateMsg, QueryMsg},
};

use crate::{
    deleverage::{deleverage, handle_deleverage_request_reply, DELEVERAGE_REQUEST_REPLY_ID},
    error::{ContractError, ContractResult},
    initialize::initialize,
    market_management::update_market,
    migrations,
    position_management::{close_all_positions, execute_order},
    query::{
        query_config, query_market, query_market_accounting, query_market_state, query_markets,
        query_opening_fee, query_position, query_position_fees, query_positions,
        query_positions_by_account, query_realized_pnl_by_account_and_market,
        query_total_accounting, query_vault, query_vault_position,
    },
    state::OWNER,
    update_config::update_config,
    vault::{deposit, unlock, withdraw},
};

pub const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
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
    initialize(deps.storage, msg.check(deps.api)?)
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
        ExecuteMsg::Deposit {
            account_id,
            max_shares_receivable,
        } => deposit(deps, info, env.block.time.seconds(), account_id, max_shares_receivable),
        ExecuteMsg::Unlock {
            account_id,
            shares,
        } => unlock(deps, info, env.block.time.seconds(), account_id, shares),
        ExecuteMsg::Withdraw {
            account_id,
            min_receive,
        } => withdraw(deps, info, env.block.time.seconds(), account_id, min_receive),
        ExecuteMsg::CloseAllPositions {
            account_id,
            action,
        } => {
            close_all_positions(deps, env, info, account_id, action.unwrap_or(ActionKind::Default))
        }
        ExecuteMsg::ExecuteOrder {
            account_id,
            denom,
            size,
            reduce_only,
        } => execute_order(deps, env, info, account_id, denom, size, reduce_only),
        ExecuteMsg::Deleverage {
            account_id,
            denom,
        } => deleverage(deps, env, account_id, denom),
        ExecuteMsg::UpdateMarket {
            params,
        } => update_market(deps, env, info.sender, params),
        ExecuteMsg::UpdateConfig {
            updates,
        } => update_config(deps, info.sender, updates),
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
        QueryMsg::Config {} => to_json_binary(&query_config(deps.storage)?),
        QueryMsg::Vault {
            action,
        } => to_json_binary(&query_vault(
            deps,
            env.block.time.seconds(),
            action.unwrap_or(ActionKind::Default),
        )?),
        QueryMsg::Market {
            denom,
        } => to_json_binary(&query_market(deps, env.block.time.seconds(), denom)?),
        QueryMsg::Markets {
            start_after,
            limit,
        } => to_json_binary(&query_markets(deps, env.block.time.seconds(), start_after, limit)?),
        QueryMsg::VaultPosition {
            user_address,
            account_id,
        } => {
            let user_addr = deps.api.addr_validate(&user_address)?;
            to_json_binary(&query_vault_position(
                deps,
                user_addr,
                account_id,
                env.block.time.seconds(),
            )?)
        }
        QueryMsg::Position {
            account_id,
            denom,
            order_size,
            reduce_only,
        } => to_json_binary(&query_position(
            deps,
            env.block.time.seconds(),
            account_id,
            denom,
            order_size,
            reduce_only,
        )?),
        QueryMsg::Positions {
            start_after,
            limit,
        } => to_json_binary(&query_positions(deps, env.block.time.seconds(), start_after, limit)?),
        QueryMsg::PositionsByAccount {
            account_id,
            action,
        } => to_json_binary(&query_positions_by_account(
            deps,
            env.block.time.seconds(),
            account_id,
            action.unwrap_or(ActionKind::Default),
        )?),
        QueryMsg::RealizedPnlByAccountAndMarket {
            account_id,
            denom,
        } => to_json_binary(&query_realized_pnl_by_account_and_market(deps, account_id, denom)?),
        QueryMsg::MarketAccounting {
            denom,
        } => to_json_binary(&query_market_accounting(deps, &denom, env.block.time.seconds())?),
        QueryMsg::TotalAccounting {} => {
            to_json_binary(&query_total_accounting(deps, env.block.time.seconds())?)
        }
        QueryMsg::OpeningFee {
            denom,
            size,
        } => to_json_binary(&query_opening_fee(deps, &denom, size)?),
        QueryMsg::PositionFees {
            account_id,
            denom,
            new_size,
        } => to_json_binary(&query_position_fees(deps, &account_id, &denom, new_size)?),
        QueryMsg::MarketState {
            denom,
        } => to_json_binary(&query_market_state(deps.storage, denom)?),
    }
    .map_err(Into::into)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: Empty) -> Result<Response, ContractError> {
    migrations::v2_2_1::migrate(deps)
}
