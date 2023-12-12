use cosmwasm_std::{Coin, Decimal};
use mars_types::{
    math::SignedDecimal,
    perps::{Funding, PnL, Position},
};

use crate::{
    error::ContractResult,
    pricing::{closing_execution_price, opening_execution_price},
};

pub trait PositionExt {
    /// Compute the unrealized PnL of a position, given the current price
    fn compute_pnl(
        &self,
        funding: &Funding,
        skew: SignedDecimal,
        denom_price: Decimal,
        base_denom_price: Decimal,
        base_denom: impl Into<String>,
    ) -> ContractResult<PnL>;

    fn compute_accrued_funding(
        &self,
        funding: &Funding,
        base_denom_price: Decimal,
    ) -> ContractResult<SignedDecimal>;
}

impl PositionExt for Position {
    fn compute_pnl(
        &self,
        funding: &Funding,
        skew: SignedDecimal,
        denom_price: Decimal,
        base_denom_price: Decimal,
        base_denom: impl Into<String>,
    ) -> ContractResult<PnL> {
        let entry_exec_price = opening_execution_price(
            self.initial_skew,
            funding.skew_scale,
            self.size,
            self.entry_price,
        )?;
        let exit_exec_price =
            closing_execution_price(skew, funding.skew_scale, self.size, denom_price)?;

        // size * (exit_exec_price - entry_exec_price)
        let price_diff = exit_exec_price.checked_sub(entry_exec_price)?;
        let price_pnl = self.size.checked_mul(price_diff)?;

        // size * (current_accrued_funding_per_unit - entry_accrued_funding_per_unit) * usdc_price
        let accrued_funding_diff = funding
            .last_funding_accrued_per_unit_in_base_denom
            .checked_sub(self.entry_accrued_funding_per_unit_in_base_denom)?;
        let accrued_funding =
            self.size.checked_mul(accrued_funding_diff)?.checked_mul(base_denom_price.into())?;

        let realized_pnl = price_pnl.checked_add(accrued_funding)?;

        if realized_pnl.is_positive() {
            return Ok(PnL::Profit(Coin {
                denom: base_denom.into(),
                amount: realized_pnl.abs.to_uint_floor(),
            }));
        }

        if realized_pnl.is_negative() {
            return Ok(PnL::Loss(Coin {
                denom: base_denom.into(),
                amount: realized_pnl.abs.to_uint_floor(),
            }));
        }

        Ok(PnL::BreakEven)
    }

    fn compute_accrued_funding(
        &self,
        funding: &Funding,
        base_denom_price: Decimal,
    ) -> ContractResult<SignedDecimal> {
        let accrued_funding_diff = funding
            .last_funding_accrued_per_unit_in_base_denom
            .checked_sub(self.entry_accrued_funding_per_unit_in_base_denom)?;
        let accrued_funding =
            self.size.checked_mul(accrued_funding_diff)?.checked_mul(base_denom_price.into())?;
        Ok(accrued_funding)
    }
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
            size: SignedDecimal::from_str("100").unwrap(),
            entry_price: Decimal::from_str("4200").unwrap(), 
            entry_accrued_funding_per_unit_in_base_denom: SignedDecimal::from_str("-14").unwrap(),
            initial_skew: SignedDecimal::from_str("180").unwrap()
        },
        Decimal::from_str("4200").unwrap(),
        PnL::BreakEven;
        "long position - break even"
    )]
    #[test_case(
        Position {
            size: SignedDecimal::from_str("100").unwrap(),
            entry_price: Decimal::from_str("4200").unwrap(), 
            entry_accrued_funding_per_unit_in_base_denom: SignedDecimal::from_str("-12").unwrap(),
            initial_skew: SignedDecimal::from_str("220").unwrap()
        },
        Decimal::from_str("4400").unwrap(),
        PnL::Profit(Coin {
            denom: MOCK_BASE_DENOM.into(),
            amount: Uint128::new(19827),
        });
        "long position - price up"
    )]
    #[test_case(
        Position {
            size: SignedDecimal::from_str("100").unwrap(),
            entry_price: Decimal::from_str("4200").unwrap(), 
            entry_accrued_funding_per_unit_in_base_denom: SignedDecimal::from_str("-12").unwrap(),
            initial_skew: SignedDecimal::from_str("220").unwrap()
        },
        Decimal::from_str("4000").unwrap(),
        PnL::Loss(Coin {
            denom: MOCK_BASE_DENOM.into(),
            amount: Uint128::new(20181),
        });
        "long position - price down"
    )]
    #[test_case(
        Position {
            size: SignedDecimal::from_str("-100").unwrap(),
            entry_price: Decimal::from_str("4200").unwrap(), 
            entry_accrued_funding_per_unit_in_base_denom: SignedDecimal::from_str("-14").unwrap(),
            initial_skew: SignedDecimal::from_str("380").unwrap()
        },
        Decimal::from_str("4200").unwrap(),
        PnL::BreakEven;
        "short position - break even"
    )]
    #[test_case(
        Position {
            size: SignedDecimal::from_str("-100").unwrap(),
            entry_price: Decimal::from_str("4200").unwrap(), 
            entry_accrued_funding_per_unit_in_base_denom: SignedDecimal::from_str("-12").unwrap(),
            initial_skew: SignedDecimal::from_str("220").unwrap()
        },
        Decimal::from_str("4400").unwrap(),
        PnL::Loss(Coin {
            denom: MOCK_BASE_DENOM.into(),
            amount: Uint128::new(19913),
        });
        "short position - price up"
    )]
    #[test_case(
        Position {
            size: SignedDecimal::from_str("-100").unwrap(),
            entry_price: Decimal::from_str("4200").unwrap(), 
            entry_accrued_funding_per_unit_in_base_denom: SignedDecimal::from_str("-12").unwrap(),
            initial_skew: SignedDecimal::from_str("220").unwrap()
        },
        Decimal::from_str("4000").unwrap(),
        PnL::Profit(Coin {
            denom: MOCK_BASE_DENOM.into(),
            amount: Uint128::new(20099),
        });
        "short position - price down"
    )]
    fn computing_pnl(position: Position, current_price: Decimal, expect_pnl: PnL) {
        let funding = Funding {
            skew_scale: Decimal::from_str("1000000").unwrap(),
            last_funding_accrued_per_unit_in_base_denom: SignedDecimal::from_str("-14").unwrap(),
            ..Default::default()
        };
        let pnl = position
            .compute_pnl(
                &funding,
                SignedDecimal::from_str("280").unwrap(),
                current_price,
                Decimal::from_str("0.8").unwrap(),
                MOCK_BASE_DENOM,
            )
            .unwrap();
        assert_eq!(pnl, expect_pnl);
    }
}
