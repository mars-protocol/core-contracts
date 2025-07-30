use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use mars_types::active_delta_neutral::instantiate::InstantiateMsg;

use crate::error::ContractResult;

pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> ContractResult<Response> {
    Ok(Response::new())
}
