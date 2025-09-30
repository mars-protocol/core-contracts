use cosmwasm_std::{to_json_binary, Coin, CosmosMsg, Empty, Env, Uint128, WasmMsg};
use cw2::set_contract_version;
use mars_rewards_collector_base::{
    contract::Collector, ContractError, ContractResult, SwapMsg, TransferMsg,
};
use mars_types::{
    address_provider::{AddressResponseItem, MarsAddressType},
    rewards_collector::{
        Config, ExecuteMsg, InstantiateMsg, NeutronMigrateMsg, QueryMsg, TransferType,
    },
    swapper::SwapperRoute,
};

pub mod migrations;

pub struct NeutronMsgFactory {}

impl TransferMsg<Empty> for NeutronMsgFactory {
    fn transfer_msg(
        _env: &Env,
        to_address: &str,
        amount: Coin,
        _cfg: &Config,
        transfer_type: &TransferType,
    ) -> ContractResult<CosmosMsg<Empty>> {
        match transfer_type {
            TransferType::Bank => Ok(CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
                to_address: to_address.to_string(),
                amount: vec![amount],
            })),
            _ => Err(ContractError::UnsupportedTransferType {
                transfer_type: transfer_type.to_string(),
            }),
        }
    }
}

impl SwapMsg<Empty> for NeutronMsgFactory {
    fn swap_msg(
        _env: &Env,
        default_swapper_addr: &AddressResponseItem,
        duality_swapper_addr: &Option<AddressResponseItem>,
        coin_in: Coin,
        denom_out: &str,
        min_receive: Uint128,
        route: Option<SwapperRoute>,
    ) -> ContractResult<CosmosMsg<Empty>> {
        match route {
            Some(SwapperRoute::Duality(_)) => {
                // Use DualitySwapper for duality routes
                let duality_swapper =
                    duality_swapper_addr.clone().ok_or(ContractError::NoSwapper {
                        required: MarsAddressType::DualitySwapper.to_string(),
                    })?;

                // Call the duality swapper contract
                Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: duality_swapper.address.clone(),
                    msg: to_json_binary(
                        &mars_types::swapper::ExecuteMsg::<Empty, Empty>::SwapExactIn {
                            coin_in: coin_in.clone(),
                            denom_out: denom_out.to_string(),
                            min_receive,
                            route,
                        },
                    )?,
                    funds: vec![coin_in],
                }))
            }
            _ => {
                // Use default swapper for other routes or no route
                Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: default_swapper_addr.address.to_string(),
                    msg: to_json_binary(
                        &mars_types::swapper::ExecuteMsg::<Empty, Empty>::SwapExactIn {
                            coin_in: coin_in.clone(),
                            denom_out: denom_out.to_string(),
                            min_receive,
                            route,
                        },
                    )?,
                    funds: vec![coin_in],
                }))
            }
        }
    }
}

pub type NeutronCollector<'a> = Collector<'a, Empty, NeutronMsgFactory>;

pub const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(not(feature = "library"))]
pub mod entry {
    use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

    use super::*;

    #[entry_point]
    pub fn instantiate(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> ContractResult<Response> {
        set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;
        let collector = NeutronCollector::default();
        collector.instantiate(deps, env, info, msg)
    }

    #[entry_point]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> ContractResult<Response> {
        let collector = NeutronCollector::default();
        collector.execute(deps, env, info, msg)
    }

    #[entry_point]
    pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
        let collector = NeutronCollector::default();
        collector.query(deps, env, msg)
    }

    #[entry_point]
    pub fn migrate(deps: DepsMut, _env: Env, msg: NeutronMigrateMsg) -> ContractResult<Response> {
        match msg {
            NeutronMigrateMsg::V2_1_0ToV2_2_0 {} => migrations::v2_2_0::migrate(deps),
            NeutronMigrateMsg::V2_2_0ToV2_2_2 {} => migrations::v2_2_2::migrate(deps),
            NeutronMigrateMsg::V2_2_2ToV2_3_1 {} => migrations::v2_3_1::migrate(deps),
            NeutronMigrateMsg::V2_3_1ToV2_3_2 {} => migrations::v2_3_2::migrate(deps),
        }
    }
}
