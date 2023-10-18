use cosmwasm_std::{Coin, Decimal, OverflowError};
use mars_types::{
    math::SignedDecimal,
    perps::{PnL, Position},
};

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
}
