use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response};
use mars_swapper_base::ContractResult;
use mars_types::swapper::{ExecuteMsg, InstantiateMsg, QueryMsg};
use neutron_sdk::bindings::msg::NeutronMsg;

use crate::{config::DualityConfig, route::DualityRoute};

pub type SwapBase<'a> =
    mars_swapper_base::SwapBase<'a, Empty, NeutronMsg, DualityRoute, DualityConfig>;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response<NeutronMsg>> {
    SwapBase::default().instantiate(deps, msg)
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg<DualityRoute, DualityConfig>,
) -> ContractResult<Response<NeutronMsg>> {
    SwapBase::default().execute(deps, env, info, msg)
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    SwapBase::default().query(deps, env, msg)
}
