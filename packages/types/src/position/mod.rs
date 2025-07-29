mod common;

use cosmwasm_std::{Decimal, Int128, Uint128};
use std::fmt;

/// Position type that holds all relevant data for a delta-neutral position
///
/// This struct tracks the state of an open delta-neutral position including size,
/// prices, value, direction, and accumulated funding and borrowing costs.
#[derive(Debug, Clone, PartialEq)]
pub struct Position {
    /// Amount of the spot asset held in the position
    pub spot_amount: Uint128,

    /// Amount of the perpetual futures asset held in the position
    pub perp_amount: Uint128,

    /// Volume-weighted average price of the spot asset entries
    pub avg_spot_price: Decimal,

    /// Volume-weighted average price of the perpetual futures entries
    pub avg_perp_price: Decimal,

    /// Total entry value of the position (spot_value - perp_value)
    pub entry_value: Int128,

    /// Direction of the position (long spot/short perp or short spot/long perp)
    pub direction: Side,

    /// Net accrued funding payments/receipts
    pub net_funding_accrued: Int128,

    /// Net accrued borrowing costs
    pub net_borrow_accrued: Int128,

    /// Timestamp of the last update to this position
    pub last_updated: u64,
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
            net_funding_accrued: Int128::zero(),
            net_borrow_accrued: Int128::zero(),
            last_updated: 0,
        }
    }
}

/// Result of a position decrease operation
///
/// Contains all information needed to calculate realized PnL
/// and update position state after a decrease.
#[derive(Debug, Clone, PartialEq)]
pub struct DecreaseResult {
    /// Exit price of the spot asset
    pub spot_exit_price: Decimal,

    /// Exit price of the perpetual futures asset
    pub perp_exit_price: Decimal,

    /// Size of the position that was closed
    pub size_closed: Uint128,

    /// Prorated slice of the entry value for the closed portion
    pub entry_value_slice: Int128,

    /// Realized funding payments/receipts for the closed portion
    pub realized_funding: Int128,

    /// Realized borrowing costs for the closed portion
    pub realized_borrow: Int128,
}

/// Trading side for a delta-neutral position
///
/// Represents the direction of the paired trades in spot and perpetual futures markets
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// Long the spot asset and short the perpetual futures
    LongSpotShortPerp,

    /// Short the spot asset and long the perpetual futures
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
