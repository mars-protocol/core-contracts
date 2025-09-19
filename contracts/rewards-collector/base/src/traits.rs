use cosmwasm_std::{
    to_json_binary, BankMsg, Coin, CosmosMsg, CustomMsg, Empty, Env, Uint128, WasmMsg,
};
use mars_types::{
    address_provider::AddressResponseItem,
    rewards_collector::{Config, TransferType},
    swapper::SwapperRoute,
};

use crate::{ContractError, ContractResult};

pub trait SwapMsg<M: CustomMsg> {
    fn swap_msg(
        env: &Env,
        default_swapper_addr: &AddressResponseItem,
        duality_swapper_addr: &Option<AddressResponseItem>,
        coin_in: Coin,
        denom_out: &str,
        min_receive: Uint128,
        route: Option<SwapperRoute>,
    ) -> ContractResult<CosmosMsg<M>>;
}

impl SwapMsg<Empty> for Empty {
    fn swap_msg(
        _env: &Env,
        default_swapper_addr: &AddressResponseItem,
        _duality_swapper_addr: &Option<AddressResponseItem>,
        coin_in: Coin,
        denom_out: &str,
        min_receive: Uint128,
        route: Option<SwapperRoute>,
    ) -> ContractResult<CosmosMsg<Empty>> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            // Default to first swapper
            contract_addr: default_swapper_addr.address.to_string(),
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

pub trait TransferMsg<M: CustomMsg> {
    fn transfer_msg(
        env: &Env,
        to_address: &str,
        amount: Coin,
        cfg: &Config,
        transfer_type: &TransferType,
    ) -> ContractResult<CosmosMsg<M>>;
}

impl TransferMsg<Empty> for Empty {
    fn transfer_msg(
        _: &Env,
        to_address: &str,
        amount: Coin,
        _: &Config,
        transfer_type: &TransferType,
    ) -> ContractResult<CosmosMsg<Empty>> {
        // By default, we only support bank transfers
        match transfer_type {
            TransferType::Bank => Ok(CosmosMsg::Bank(BankMsg::Send {
                to_address: to_address.to_string(),
                amount: vec![amount],
            })),
            TransferType::Ibc => Err(ContractError::UnsupportedTransferType {
                transfer_type: transfer_type.to_string(),
            }),
        }
    }
}
