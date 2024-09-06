use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_json_binary, Addr, Api, Coin, CosmosMsg, QuerierWrapper, StdResult, Uint128, WasmMsg,
};

use crate::{
    oracle::ActionKind,
    perps::{
        Config, ExecuteMsg, MarketResponse, PerpPosition, PositionResponse,
        PositionsByAccountResponse, QueryMsg, TradingFee, VaultPositionResponse,
    },
    signed_uint::SignedUint,
};

#[cw_serde]
pub struct PerpsBase<T>(T);

impl<T> PerpsBase<T> {
    pub fn new(address: T) -> PerpsBase<T> {
        PerpsBase(address)
    }

    pub fn address(&self) -> &T {
        &self.0
    }
}

pub type PerpsUnchecked = PerpsBase<String>;
pub type Perps = PerpsBase<Addr>;

impl From<Perps> for PerpsUnchecked {
    fn from(perps: Perps) -> Self {
        Self(perps.address().to_string())
    }
}

impl PerpsUnchecked {
    pub fn check(&self, api: &dyn Api) -> StdResult<Perps> {
        Ok(PerpsBase::new(api.addr_validate(self.address())?))
    }
}

impl Perps {
    /// Generate message for deposit to perp vault
    pub fn deposit_msg(
        &self,
        account_id: impl Into<String>,
        coin: &Coin,
        max_shares_receivable: Option<Uint128>,
    ) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.address().into(),
            msg: to_json_binary(&ExecuteMsg::Deposit {
                account_id: Some(account_id.into()),
                max_shares_receivable,
            })?,
            funds: vec![coin.clone()],
        }))
    }

    /// Generate message for unlock from perp vault
    pub fn unlock_msg(
        &self,
        account_id: impl Into<String>,
        shares: Uint128,
    ) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.address().into(),
            msg: to_json_binary(&ExecuteMsg::Unlock {
                account_id: Some(account_id.into()),
                shares,
            })?,
            funds: vec![],
        }))
    }

    /// Generate message for withdraw from perp vault
    pub fn withdraw_msg(
        &self,
        account_id: impl Into<String>,
        min_receive: Option<Uint128>,
    ) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.address().into(),
            msg: to_json_binary(&ExecuteMsg::Withdraw {
                account_id: Some(account_id.into()),
                min_receive,
            })?,
            funds: vec![],
        }))
    }

    pub fn execute_perp_order(
        &self,
        account_id: impl Into<String>,
        denom: impl Into<String>,
        size: SignedUint,
        reduce_only: Option<bool>,
        funds: Vec<Coin>,
    ) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.address().into(),
            msg: to_json_binary(&ExecuteMsg::ExecuteOrder {
                account_id: account_id.into(),
                denom: denom.into(),
                size,
                reduce_only,
            })?,
            funds,
        }))
    }

    /// Generate message for closing all perp positions
    pub fn close_all_msg(
        &self,
        account_id: impl Into<String>,
        funds: Vec<Coin>,
        action: ActionKind,
    ) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.address().into(),
            msg: to_json_binary(&ExecuteMsg::CloseAllPositions {
                account_id: account_id.into(),
                action: Some(action),
            })?,
            funds,
        }))
    }

    pub fn query_position(
        &self,
        querier: &QuerierWrapper,
        account_id: impl Into<String>,
        denom: impl Into<String>,
        order_size: Option<SignedUint>,
    ) -> StdResult<Option<PerpPosition>> {
        let res: PositionResponse = querier.query_wasm_smart(
            self.address(),
            &QueryMsg::Position {
                account_id: account_id.into(),
                denom: denom.into(),
                order_size,
            },
        )?;
        Ok(res.position)
    }

    pub fn query_positions_by_account(
        &self,
        querier: &QuerierWrapper,
        account_id: impl Into<String>,
        action: ActionKind,
    ) -> StdResult<Vec<PerpPosition>> {
        let res: PositionsByAccountResponse = querier.query_wasm_smart(
            self.address(),
            &QueryMsg::PositionsByAccount {
                account_id: account_id.into(),
                action: Some(action),
            },
        )?;
        Ok(res.positions)
    }

    pub fn query_opening_fee(
        &self,
        querier: &QuerierWrapper,
        denom: impl Into<String>,
        size: SignedUint,
    ) -> StdResult<TradingFee> {
        let res: TradingFee = querier.query_wasm_smart(
            self.address(),
            &QueryMsg::OpeningFee {
                denom: denom.into(),
                size,
            },
        )?;
        Ok(res)
    }

    pub fn query_perp_market_state(
        &self,
        querier: &QuerierWrapper,
        denom: impl Into<String>,
    ) -> StdResult<MarketResponse> {
        let res: MarketResponse = querier.query_wasm_smart(
            self.address(),
            &QueryMsg::Market {
                denom: denom.into(),
            },
        )?;
        Ok(res)
    }

    pub fn query_config(&self, querier: &QuerierWrapper) -> StdResult<Config<String>> {
        let res: Config<String> = querier.query_wasm_smart(self.address(), &QueryMsg::Config {})?;
        Ok(res)
    }

    pub fn query_vault_position(
        &self,
        querier: &QuerierWrapper,
        credit_manager: impl Into<String>,
        account_id: impl Into<String>,
    ) -> StdResult<Option<VaultPositionResponse>> {
        let res: Option<VaultPositionResponse> = querier.query_wasm_smart(
            self.address(),
            &QueryMsg::VaultPosition {
                user_address: credit_manager.into(),
                account_id: Some(account_id.into()),
            },
        )?;
        Ok(res)
    }
}
