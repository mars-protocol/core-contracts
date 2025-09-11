use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult,
};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub redemption_rate: Decimal,
    pub lst_asset_denom: String,
}

pub const STATE: Item<State> = Item::new("state");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let state = State {
        redemption_rate: msg.redemption_rate,
        lst_asset_denom: msg.lst_asset_denom,
    };
    STATE.save(deps.storage, &state)?;
    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::SetRedemptionRate {
            redemption_rate,
        } => {
            STATE.update(deps.storage, |mut state| -> StdResult<_> {
                state.redemption_rate = redemption_rate;
                Ok(state)
            })?;
            Ok(Response::default())
        }
        ExecuteMsg::SetLstAssetDenom {
            denom,
        } => {
            STATE.update(deps.storage, |mut state| -> StdResult<_> {
                state.lst_asset_denom = denom;
                Ok(state)
            })?;
            Ok(Response::default())
        }
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::RedemptionRate {} => {
            let state = STATE.load(deps.storage)?;
            to_json_binary(&state.redemption_rate)
        }
        QueryMsg::GetLstAssetDenom {} => {
            let state = STATE.load(deps.storage)?;
            to_json_binary(&state.lst_asset_denom)
        }
    }
}
