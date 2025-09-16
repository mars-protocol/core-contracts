use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    entry_point, to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdResult, Uint128,
};
use cw_storage_plus::Map;
use mars_types::adapters::governance::{
    GovernanceQueryMsg, VotingPowerAtHeightQuery, VotingPowerAtHeightResponse,
};

const VOTING_POWER: Map<&Addr, Uint128> = Map::new("voting_power");

#[cw_serde]
pub enum ExecMsg {
    SetVotingPower {
        address: String,
        power: Uint128,
    },
}

#[entry_point]
pub fn instantiate(_deps: DepsMut, _env: Env, _info: MessageInfo, _msg: ()) -> StdResult<Response> {
    Ok(Response::new())
}

#[entry_point]
pub fn execute(deps: DepsMut, _env: Env, _info: MessageInfo, msg: ExecMsg) -> StdResult<Response> {
    match msg {
        ExecMsg::SetVotingPower {
            address,
            power,
        } => {
            let addr = deps.api.addr_validate(&address)?;
            VOTING_POWER.save(deps.storage, &addr, &power)?;
            Ok(Response::new())
        }
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: GovernanceQueryMsg) -> StdResult<Binary> {
    match msg {
        GovernanceQueryMsg::VotingPowerAtHeight(VotingPowerAtHeightQuery {
            address,
        }) => {
            let addr = deps.api.addr_validate(&address)?;
            let power = VOTING_POWER.may_load(deps.storage, &addr)?.unwrap_or(Uint128::zero());
            to_json_binary(&VotingPowerAtHeightResponse {
                power,
                height: 0,
            })
        }
    }
}

#[entry_point]
pub fn reply(_deps: DepsMut, _env: Env, _reply: Reply) -> StdResult<Response> {
    Ok(Response::new())
}
