use std::fmt;

use cosmwasm_std::{Decimal, Int128, Uint128};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub spot_amount: Uint128,
    pub perp_amount: Uint128,
    pub avg_spot_price: Decimal,
    pub avg_perp_price: Decimal,
    pub entry_value: Int128,
    pub direction: Side,
    pub net_funding_balance: Int128,
    pub net_borrow_balance: Int128,
    pub net_realized_funding: Int128,
    pub net_realized_borrow: Int128,
    pub debt_principle: Uint128,
    pub last_updated: u64,
    pub total_realized_pnl: Int128,
}

impl Default for Position {
    fn default() -> Self {
        Self {
            spot_amount: Uint128::zero(),
            perp_amount: Uint128::zero(),
            avg_spot_price: Decimal::zero(),
            avg_perp_price: Decimal::zero(),
            entry_value: Int128::zero(),
            direction: Side::LongSpotShortPerp,
            net_funding_balance: Int128::zero(),
            net_borrow_balance: Int128::zero(),
            net_realized_funding: Int128::zero(),
            net_realized_borrow: Int128::zero(),
            debt_principle: Uint128::zero(),
            last_updated: 0,
            total_realized_pnl: Int128::zero(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecreaseResult {
    pub spot_exit_price: Decimal,
    pub perp_exit_price: Decimal,
    pub size_closed: Uint128,
    pub entry_value_slice: Int128,
    pub realized_funding: Int128,
    pub realized_borrow: Int128,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    LongSpotShortPerp,
    ShortSpotLongPerp,
}

impl Side {
    pub fn display(&self) -> String {
        match self {
            Self::LongSpotShortPerp => "LongSpotShortPerp".to_string(),
            Self::ShortSpotLongPerp => "ShortSpotLongPerp".to_string(),
        }
    }
}

impl fmt::Display for Side {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display())
    }
}
