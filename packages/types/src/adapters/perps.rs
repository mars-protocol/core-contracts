use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_binary, Addr, Api, Coin, CosmosMsg, QuerierWrapper, StdResult, WasmMsg};

use crate::{
    math::SignedDecimal,
    perps::{
        ExecuteMsg, PerpPosition, PositionResponse, PositionsByAccountResponse, QueryMsg,
        TradingFee,
    },
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
    /// Generate message for opening a new perp position
    pub fn open_msg(
        &self,
        account_id: impl Into<String>,
        denom: impl Into<String>,
        size: SignedDecimal,
        funds: Vec<Coin>,
    ) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.address().into(),
            msg: to_binary(&ExecuteMsg::OpenPosition {
                account_id: account_id.into(),
                denom: denom.into(),
                size,
            })?,
            funds,
        }))
    }

    /// Generate message for closing a perp position
    pub fn close_msg(
        &self,
        account_id: impl Into<String>,
        denom: impl Into<String>,
        funds: Vec<Coin>,
    ) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.address().into(),
            msg: to_binary(&ExecuteMsg::ClosePosition {
                account_id: account_id.into(),
                denom: denom.into(),
            })?,
            funds,
        }))
    }

    /// Generate message for modifying a perp position
    pub fn modify_msg(
        &self,
        account_id: impl Into<String>,
        denom: impl Into<String>,
        new_size: impl Into<SignedDecimal>,
        funds: Vec<Coin>,
    ) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.address().into(),
            msg: to_binary(&ExecuteMsg::ModifyPosition {
                account_id: account_id.into(),
                denom: denom.into(),
                new_size: new_size.into(),
            })?,
            funds,
        }))
    }

    pub fn query_position(
        &self,
        querier: &QuerierWrapper,
        account_id: impl Into<String>,
        denom: impl Into<String>,
    ) -> StdResult<PerpPosition> {
        let res: PositionResponse = querier.query_wasm_smart(
            self.address(),
            &QueryMsg::Position {
                account_id: account_id.into(),
                denom: denom.into(),
            },
        )?;
        Ok(res.position)
    }

    pub fn query_positions_by_account(
        &self,
        querier: &QuerierWrapper,
        account_id: impl Into<String>,
    ) -> StdResult<Vec<PerpPosition>> {
        let res: PositionsByAccountResponse = querier.query_wasm_smart(
            self.address(),
            &QueryMsg::PositionsByAccount {
                account_id: account_id.into(),
            },
        )?;
        Ok(res.positions)
    }

    pub fn query_opening_fee(
        &self,
        querier: &QuerierWrapper,
        denom: impl Into<String>,
        size: SignedDecimal,
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
}
