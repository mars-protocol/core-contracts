use cosmwasm_std::{Coin, Decimal, Deps, Order, OverflowError};
use mars_types::{
    adapters::oracle::Oracle,
    math::SignedDecimal,
    oracle::ActionKind,
    perps::{DenomState, PnL, Position},
};

use crate::{error::ContractResult, state::DENOM_STATES};

/// Compute the unrealized PnL of a position, given the current price.
///
/// Note that if the position is winning, the profit is capped at the amount
/// of liquidity that was locked up during position opening.
pub fn compute_pnl(
    position: &Position,
    current_price: Decimal,
    base_denom: impl Into<String>,
) -> Result<PnL, OverflowError> {
    // cast the prices into SignedDecimal
    let entry_price: SignedDecimal = position.entry_price.into();
    let exit_price: SignedDecimal = current_price.into();

    // size * (exit_price - entry_price)
    let price_diff = exit_price.checked_sub(entry_price)?;
    let pnl = position.size.checked_mul(price_diff)?;

    if pnl.is_positive() {
        return Ok(PnL::Profit(Coin {
            denom: base_denom.into(),
            amount: pnl.abs.to_uint_floor(),
        }));
    }

    if pnl.is_negative() {
        return Ok(PnL::Loss(Coin {
            denom: base_denom.into(),
            amount: pnl.abs.to_uint_floor(),
        }));
    }

    Ok(PnL::BreakEven)
}

/// Total unrealized PnL of a denom is the sum of unrealized PnL of all open positions.
///
/// PnL for a single position is computed as:
/// pnl = size * (exit_price - entry_price)
///
/// PnL for all open positions is computed as:
/// total_pnl = size_1 * (exit_price - entry_price_1) + size_2 * (exit_price - entry_price_2) + ...
///           = size_1 * exit_price - size_1 * entry_price_1 + size_2 * exit_price - size_2 * entry_price_2 + ...
///           = exit_price * (size_1 + size_2 + ...) - (size_1 * entry_price_1 + size_2 * entry_price_2 + ...)
///           = exit_price * total_size - total_cost_base
///
/// To compute this, we keep two global "accumulators":
/// - total_size
/// - total_cost_base
/// When a user opens a new position of size, we do: total_size += size, total_cost_base += size * entry_price
/// When a user closes a position of size, we do: total_size -= size, total_cost_base -= size * entry_price
pub trait DenomStateExt {
    fn open_position(
        &mut self,
        size: SignedDecimal,
        entry_price: Decimal,
    ) -> Result<(), OverflowError>;

    fn close_position(
        &mut self,
        size: SignedDecimal,
        entry_price: Decimal,
    ) -> Result<(), OverflowError>;

    fn compute_unrealized_pnl(&self, exit_price: Decimal) -> Result<SignedDecimal, OverflowError>;
}

impl DenomStateExt for DenomState {
    fn open_position(
        &mut self,
        size: SignedDecimal,
        entry_price: Decimal,
    ) -> Result<(), OverflowError> {
        self.total_size = self.total_size.checked_add(size)?;
        let value = size.checked_mul(entry_price.into())?;
        self.total_cost_base = self.total_cost_base.checked_add(value)?;
        Ok(())
    }

    fn close_position(
        &mut self,
        size: SignedDecimal,
        entry_price: Decimal,
    ) -> Result<(), OverflowError> {
        self.total_size = self.total_size.checked_sub(size)?;
        let value = size.checked_mul(entry_price.into())?;
        self.total_cost_base = self.total_cost_base.checked_sub(value)?;
        Ok(())
    }

    fn compute_unrealized_pnl(&self, exit_price: Decimal) -> Result<SignedDecimal, OverflowError> {
        self.total_size.checked_mul(exit_price.into())?.checked_sub(self.total_cost_base)
    }
}

/// Loop through denoms and compute the total unrealized PnL.
/// This PnL is denominated in uusd (1 USD = 1e6 uusd -> configured in Oracle).
pub fn compute_total_unrealized_pnl(deps: Deps, oracle: &Oracle) -> ContractResult<SignedDecimal> {
    let total_unrealized_pnl = DENOM_STATES
        .range(deps.storage, None, None, Order::Ascending)
        .try_fold(SignedDecimal::zero(), |acc, item| -> ContractResult<_> {
            let (denom, ds) = item?;

            let price = oracle.query_price(&deps.querier, &denom, ActionKind::Default)?.price;
            let pnl = ds.compute_unrealized_pnl(price)?;

            acc.checked_add(pnl).map_err(Into::into)
        })?;

    Ok(total_unrealized_pnl)
}

// ----------------------------------- Tests -----------------------------------

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use cosmwasm_std::Uint128;
    use test_case::test_case;

    use super::*;

    const MOCK_BASE_DENOM: &str = "uusdc";

    #[test_case(
        Position {
            size: SignedDecimal::from_str("123.45").unwrap(),
            entry_price: Decimal::from_str("234.56").unwrap(),
        },
        Decimal::from_str("234.56").unwrap(),
        PnL::BreakEven;
        "long position - price no change"
    )]
    #[test_case(
        Position {
            size: SignedDecimal::from_str("123.45").unwrap(),
            entry_price: Decimal::from_str("234.56").unwrap(),
        },
        Decimal::from_str("250").unwrap(),
        PnL::Profit(Coin {
            denom: MOCK_BASE_DENOM.into(),
            amount: Uint128::new(1906), // floor(123.45 * (250 - 234.56))
        });
        "long position - price up"
    )]
    #[test_case(
        Position {
            size: SignedDecimal::from_str("123.45").unwrap(),
            entry_price: Decimal::from_str("234.56").unwrap(),
        },
        Decimal::from_str("200").unwrap(),
        PnL::Loss(Coin {
            denom: MOCK_BASE_DENOM.into(),
            amount: Uint128::new(4266), // floor(123.45 * (200 - 234.56))
        });
        "long position - price down"
    )]
    #[test_case(
        Position {
            size: SignedDecimal::from_str("-123.45").unwrap(),
            entry_price: Decimal::from_str("234.56").unwrap(),
        },
        Decimal::from_str("234.56").unwrap(),
        PnL::BreakEven;
        "short position - price no change"
    )]
    #[test_case(
        Position {
            size: SignedDecimal::from_str("-123.45").unwrap(),
            entry_price: Decimal::from_str("234.56").unwrap(),
        },
        Decimal::from_str("250").unwrap(),
        PnL::Loss(Coin {
            denom: MOCK_BASE_DENOM.into(),
            amount: Uint128::new(1906), // floor(-123.45 * (250 - 234.56))
        });
        "short position - price up"
    )]
    #[test_case(
        Position {
            size: SignedDecimal::from_str("-123.45").unwrap(),
            entry_price: Decimal::from_str("234.56").unwrap(),
        },
        Decimal::from_str("200").unwrap(),
        PnL::Profit(Coin {
            denom: MOCK_BASE_DENOM.into(),
            amount: Uint128::new(4266), // floor(-123.45 * (200 - 234.56))
        });
        "short position - price down"
    )]
    fn computing_pnl(position: Position, current_price: Decimal, expect_pnl: PnL) {
        let pnl = compute_pnl(&position, current_price, MOCK_BASE_DENOM).unwrap();
        assert_eq!(pnl, expect_pnl);
    }

    #[test_case(
        vec![Position {
            size: SignedDecimal::from_str("123.45").unwrap(),
            entry_price: Decimal::from_str("234.56").unwrap(),
        },
        Position {
            size: SignedDecimal::from_str("-12.50").unwrap(),
            entry_price: Decimal::from_str("260").unwrap(),
        }],
        DenomState {
            enabled: false, // doesn't matter here
            total_size: SignedDecimal::from_str("110.95").unwrap(), // 123.45 + (-12.50)
            total_cost_base: SignedDecimal::from_str("25706.432").unwrap(), // 123.45 * 234.56 + (-12.50 * 260)
        };
        "accumulators for open positions"
    )]
    fn computing_accumulators_for_open_positions(
        open_positions: Vec<Position>,
        expect_ds: DenomState,
    ) {
        let mut ds = DenomState::default();
        for open_position in open_positions {
            ds.open_position(open_position.size, open_position.entry_price).unwrap();
        }
        assert_eq!(ds, expect_ds);
    }

    #[test_case(
        vec![Position {
            size: SignedDecimal::from_str("123.45").unwrap(),
            entry_price: Decimal::from_str("234.56").unwrap(),
        },
        Position {
            size: SignedDecimal::from_str("-12.50").unwrap(),
            entry_price: Decimal::from_str("260").unwrap(),
        }],
        DenomState {
            enabled: false, // doesn't matter here
            total_size: SignedDecimal::from_str("-110.95").unwrap(), // -123.45 - (-12.50)
            total_cost_base: SignedDecimal::from_str("-25706.432").unwrap(), // -(123.45 * 234.56) - (-12.50 * 260)
        };
        "accumulators for close positions"
    )]
    fn computing_accumulators_for_close_positions(
        close_positions: Vec<Position>,
        expect_ds: DenomState,
    ) {
        let mut ds = DenomState::default();
        for close_position in close_positions {
            ds.close_position(close_position.size, close_position.entry_price).unwrap();
        }
        assert_eq!(ds, expect_ds);
    }

    #[test_case(
        vec![Position {
            size: SignedDecimal::from_str("123.45").unwrap(),
            entry_price: Decimal::from_str("234.56").unwrap(),
        },
        Position {
            size: SignedDecimal::from_str("123.45").unwrap(),
            entry_price: Decimal::from_str("260").unwrap(),
        },
        Position {
            size: SignedDecimal::from_str("-12.50").unwrap(),
            entry_price: Decimal::from_str("240.12").unwrap(),
        },
        Position {
            size: SignedDecimal::from_str("-12.50").unwrap(),
            entry_price: Decimal::from_str("280.50").unwrap(),
        }], // 123.45 * (250 - 234.56) + 123.45 * (250 - 260) + (-12.50) * (250 - 240.12) + (-12.50) * (250 - 280.50) = 929.318
        vec![Position {
            size: SignedDecimal::from_str("12.45").unwrap(),
            entry_price: Decimal::from_str("234.56").unwrap(),
        },
        Position {
            size: SignedDecimal::from_str("12.45").unwrap(),
            entry_price: Decimal::from_str("260").unwrap(),
        },
        Position {
            size: SignedDecimal::from_str("-10.50").unwrap(),
            entry_price: Decimal::from_str("240.12").unwrap(),
        },
        Position {
            size: SignedDecimal::from_str("-10.50").unwrap(),
            entry_price: Decimal::from_str("280.50").unwrap(),
        }], // 12.45 * (250 - 234.56) + 12.45 * (250 - 260) + (-10.50) * (250 - 240.12) + (-10.50) * (250 - 280.50) = 284.238
        Decimal::from_str("250").unwrap(),
        SignedDecimal::from_str("645.08").unwrap(); // 929.318 - 284.238
        "compute total pnl based on accumulators"
    )]
    fn computing_unrealized_pnl(
        open_positions: Vec<Position>,
        close_positions: Vec<Position>,
        exit_price: Decimal,
        expect_total_pnl: SignedDecimal,
    ) {
        let mut ds = DenomState::default();
        for open_position in open_positions {
            ds.open_position(open_position.size, open_position.entry_price).unwrap();
        }

        for close_position in close_positions {
            ds.close_position(close_position.size, close_position.entry_price).unwrap();
        }

        let total_pnl = ds.compute_unrealized_pnl(exit_price).unwrap();
        assert_eq!(total_pnl, expect_total_pnl);
    }
}
