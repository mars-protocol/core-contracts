use cosmwasm_std::{
    to_json_binary, Coin, CosmosMsg, Empty, Env, IbcMsg, IbcTimeout, Uint128, WasmMsg,
};
use mars_rewards_collector_base::{contract::Collector, ContractResult, SwapMsg, TransferMsg};
use mars_types::{
    address_provider::{AddressResponseItem, MarsAddressType},
    rewards_collector::{Config, TransferType},
    swapper::SwapperRoute,
};

pub mod migrations;

pub struct OsmosisMsgFactory {}

impl SwapMsg<Empty> for OsmosisMsgFactory {
    fn swap_msg(
        _env: &Env,
        swapper_addresses: &[AddressResponseItem],
        coin_in: Coin,
        denom_out: &str,
        min_receive: Uint128,
        route: Option<SwapperRoute>,
    ) -> ContractResult<CosmosMsg<Empty>> {
        let swapper = swapper_addresses
            .iter()
            .find(|addr| addr.address_type == MarsAddressType::Swapper)
            .ok_or(mars_rewards_collector_base::ContractError::NoSwapper {
                required: MarsAddressType::Swapper.to_string(),
            })?;

        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: swapper.address.clone(),
            msg: to_json_binary(&mars_types::swapper::ExecuteMsg::<Empty, Empty>::SwapExactIn {
                coin_in: coin_in.clone(),
                denom_out: denom_out.to_string(),
                min_receive,
                route,
            })?,
            funds: vec![coin_in],
        }))
    }
}

impl TransferMsg<Empty> for OsmosisMsgFactory {
    fn transfer_msg(
        env: &Env,
        to_address: &str,
        amount: Coin,
        cfg: &Config,
        transfer_type: &TransferType,
    ) -> ContractResult<CosmosMsg<Empty>> {
        match transfer_type {
            TransferType::Bank => Ok(CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
                to_address: to_address.to_string(),
                amount: vec![amount],
            })),
            TransferType::Ibc => Ok(CosmosMsg::Ibc(IbcMsg::Transfer {
                channel_id: cfg.channel_id.to_string(),
                to_address: to_address.to_string(),
                amount,
                timeout: IbcTimeout::with_timestamp(
                    env.block.time.plus_seconds(cfg.timeout_seconds),
                ),
            })),
        }
    }
}

pub type OsmosisCollector<'a> = Collector<'a, Empty, OsmosisMsgFactory>;

#[cfg(not(feature = "library"))]
pub mod entry {
    use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
    use cw2::set_contract_version;
    use mars_rewards_collector_base::{ContractError, ContractResult};
    use mars_types::rewards_collector::{ExecuteMsg, InstantiateMsg, OsmosisMigrateMsg, QueryMsg};

    use crate::{migrations, OsmosisCollector};

    pub const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
    pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

    #[entry_point]
    pub fn instantiate(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> ContractResult<Response> {
        set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;
        let collector = OsmosisCollector::default();
        collector.instantiate(deps, env, info, msg)
    }

    #[entry_point]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> ContractResult<Response> {
        let collector = OsmosisCollector::default();
        collector.execute(deps, env, info, msg)
    }

    #[entry_point]
    pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
        let collector = OsmosisCollector::default();
        collector.query(deps, env, msg)
    }

    #[entry_point]
    pub fn migrate(
        deps: DepsMut,
        _env: Env,
        msg: OsmosisMigrateMsg,
    ) -> Result<Response, ContractError> {
        match msg {
            OsmosisMigrateMsg::V1_0_0ToV2_0_0 {} => migrations::v2_0_0::migrate(deps),
            OsmosisMigrateMsg::V2_0_0ToV2_0_1 {} => migrations::v2_0_1::migrate(deps),
            OsmosisMigrateMsg::V2_1_0ToV2_1_1 {} => migrations::v2_1_1::migrate(deps),
        }
    }
}
