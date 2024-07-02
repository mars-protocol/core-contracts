use cosmwasm_std::{Decimal, Uint128};
use mars_types::{
    math::SignedDecimal,
    perps::{Funding, PnlAmounts, Position},
    signed_uint::SignedUint,
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
        skew: SignedUint,
        denom_price: Decimal,
        base_denom_price: Decimal,
        opening_fee_rate: Decimal,
        closing_fee_rate: Decimal,
        modification: PositionModification,
    ) -> ContractResult<PnlAmounts>;
}

impl PositionExt for Position {
    fn compute_pnl(
        &self,
        funding: &Funding,
        skew: SignedUint,
        denom_price: Decimal,
        base_denom_price: Decimal,
        opening_fee_rate: Decimal,
        closing_fee_rate: Decimal,
        modification: PositionModification,
    ) -> ContractResult<PnlAmounts> {
        let exit_exec_price =
            closing_execution_price(skew, funding.skew_scale, self.size, denom_price)?;
        // size * (exit_exec_price - entry_exec_price)
        let price_diff =
            SignedDecimal::from(exit_exec_price).checked_sub(self.entry_exec_price.into())?;
        let price_pnl_in_base_denom =
            self.size.checked_mul_floor(price_diff.checked_div(base_denom_price.into())?)?;

        // size * (current_accrued_funding_per_unit - entry_accrued_funding_per_unit)
        let accrued_funding_diff = funding
            .last_funding_accrued_per_unit_in_base_denom
            .checked_sub(self.entry_accrued_funding_per_unit_in_base_denom)?;
        let accrued_funding_in_base_denom = self.size.checked_mul_floor(accrued_funding_diff)?;

        // Only charge:
        // - opening fee if we are increasing size
        // - closing fee if we are reducing size
        let (opening_fee, closing_fee) = modification.compute_fees(
            opening_fee_rate,
            closing_fee_rate,
            denom_price,
            base_denom_price,
            self.size,
            skew,
            funding.skew_scale,
        )?;

        let realized_pnl_in_base_denom = price_pnl_in_base_denom
            .checked_add(accrued_funding_in_base_denom)?
            .checked_add(opening_fee)?
            .checked_add(closing_fee)?;

        Ok(PnlAmounts {
            price_pnl: price_pnl_in_base_denom,
            accrued_funding: accrued_funding_in_base_denom,
            opening_fee,
            closing_fee,
            pnl: realized_pnl_in_base_denom,
        })
    }
}

/// PositionModification is used to specify the type of position modification in order to calculate the fees
pub enum PositionModification {
    Increase(SignedUint),
    Decrease(SignedUint),
    None,
}

impl PositionModification {
    // Compute the fees based on the modification type and parameters
    fn compute_fees(
        &self,
        opening_fee_rate: Decimal,
        closing_fee_rate: Decimal,
        denom_price: Decimal,
        base_denom_price: Decimal,
        size: SignedUint,
        skew: SignedUint,
        skew_scale: Uint128,
    ) -> ContractResult<(SignedUint, SignedUint)> {
        // Extract the relevant size based on the modification type
        match self {
            // Apply opening fee based on the position size change:
            // - if opening it is position size,
            // - if increasing it is q change
            PositionModification::Increase(size) => {
                let denom_exec_price =
                    opening_execution_price(skew, skew_scale, *size, denom_price)?;
                let opening_fee =
                    compute_fee(opening_fee_rate, *size, denom_exec_price, base_denom_price)?;
                Ok((opening_fee, SignedUint::zero()))
            }

            // Apply closing fee based on the position size change:
            // - if closing it is position size,
            // - if reducing it is q change
            PositionModification::Decrease(size) => {
                let denom_exec_price =
                    closing_execution_price(skew, skew_scale, *size, denom_price)?;
                let closing_fee =
                    compute_fee(closing_fee_rate, *size, denom_exec_price, base_denom_price)?;
                Ok((SignedUint::zero(), closing_fee))
            }

            // No modification needed, return the original size
            // This can be used when querying the current PnL without affecting the position
            PositionModification::None => {
                let denom_exec_price =
                    closing_execution_price(skew, skew_scale, size, denom_price)?;
                let closing_fee =
                    compute_fee(closing_fee_rate, size, denom_exec_price, base_denom_price)?;
                Ok((SignedUint::zero(), closing_fee))
            }
        }
    }
}

fn compute_fee(
    rate: Decimal,
    size: SignedUint,
    denom_price: Decimal,
    base_denom_price: Decimal,
) -> ContractResult<SignedUint> {
    // Calculate the fee amount in base denom. Use ceil in favor of the protocol
    let fee_amount =
        size.abs.checked_mul_ceil(denom_price.checked_mul(rate)?.checked_div(base_denom_price)?)?;

    // Make the fee negative to show that it's a cost for the user
    let fee_amount: SignedUint = SignedUint::zero().checked_sub(fee_amount.into())?;

    Ok(fee_amount)
}

// ----------------------------------- Tests -----------------------------------

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use cosmwasm_std::Uint128;
    use mars_types::perps::PnlAmounts;
    use test_case::test_case;

    use super::*;

    #[test_case(
        Position {
            size: SignedUint::from_str("100").unwrap(),
            entry_price: Decimal::from_str("4200").unwrap(), 
            entry_exec_price: Decimal::from_str("4200.966").unwrap(),
            entry_accrued_funding_per_unit_in_base_denom: SignedDecimal::from_str("-14").unwrap(),
            initial_skew: SignedUint::from_str("180").unwrap(),
            realized_pnl: PnlAmounts::default()
        },
        Decimal::from_str("4200").unwrap(),
        Decimal::zero(),
        PnlAmounts {
            opening_fee: SignedUint::zero(),
            price_pnl: SignedUint::zero(),
            accrued_funding: SignedUint::zero(),
            closing_fee: SignedUint::zero(),
            pnl: SignedUint::zero(),
        };
        "long position - break even"
    )]
    #[test_case(
        Position {
            size: SignedUint::from_str("100").unwrap(),
            entry_price: Decimal::from_str("4200").unwrap(), 
            entry_exec_price: Decimal::from_str("4201.134").unwrap(),
            entry_accrued_funding_per_unit_in_base_denom: SignedDecimal::from_str("-12.826").unwrap(),
            initial_skew: SignedUint::from_str("220").unwrap(),
            realized_pnl: PnlAmounts::default()
        },
        Decimal::from_str("4400").unwrap(),
        Decimal::from_str("0.02").unwrap(),
        PnlAmounts {
            opening_fee: SignedUint::zero(),
            price_pnl: SignedUint::from_str("24984").unwrap(),
            accrued_funding: SignedUint::from_str("-118").unwrap(),
            closing_fee: SignedUint::from_str("-11003").unwrap(),
            pnl: SignedUint::from_str("13863").unwrap(),
        };
        "long position - price up"
    )]
    #[test_case(
        Position {
            size: SignedUint::from_str("100").unwrap(),
            entry_price: Decimal::from_str("4200").unwrap(), 
            entry_exec_price: Decimal::from_str("4201.134").unwrap(),
            entry_accrued_funding_per_unit_in_base_denom: SignedDecimal::from_str("-12.826").unwrap(),
            initial_skew: SignedUint::from_str("220").unwrap(),
            realized_pnl: PnlAmounts::default()
        },
        Decimal::from_str("4000").unwrap(),
        Decimal::from_str("0.02").unwrap(),
        PnlAmounts {
            opening_fee: SignedUint::zero(),
            price_pnl: SignedUint::from_str("-25027").unwrap(),
            accrued_funding: SignedUint::from_str("-118").unwrap(),
            closing_fee: SignedUint::from_str("-10003").unwrap(),
            pnl: SignedUint::from_str("-35148").unwrap(),
        };
        "long position - price down"
    )]
    #[test_case(
        Position {
            size: SignedUint::from_str("-100").unwrap(),
            entry_price: Decimal::from_str("4200").unwrap(), 
            entry_exec_price: Decimal::from_str("4201.386").unwrap(),
            entry_accrued_funding_per_unit_in_base_denom: SignedDecimal::from_str("-14").unwrap(),
            initial_skew: SignedUint::from_str("380").unwrap(),
            realized_pnl: PnlAmounts::default()
        },
        Decimal::from_str("4200").unwrap(),
        Decimal::zero(),
        PnlAmounts {
            opening_fee: SignedUint::zero(),
            price_pnl: SignedUint::zero(),
            accrued_funding: SignedUint::zero(),
            closing_fee: SignedUint::zero(),
            pnl: SignedUint::zero(),
        };
        "short position - break even"
    )]
    #[test_case(
        Position {
            size: SignedUint::from_str("-100").unwrap(),
            entry_price: Decimal::from_str("4200").unwrap(), 
            entry_exec_price: Decimal::from_str("4200.714").unwrap(),
            entry_accrued_funding_per_unit_in_base_denom: SignedDecimal::from_str("-12.826").unwrap(),
            initial_skew: SignedUint::from_str("220").unwrap(),
            realized_pnl: PnlAmounts::default()
        },
        Decimal::from_str("4400").unwrap(),
        Decimal::from_str("0.02").unwrap(),
        PnlAmounts {
            opening_fee: SignedUint::zero(),
            price_pnl: SignedUint::from_str("-25093").unwrap(),
            accrued_funding: SignedUint::from_str("117").unwrap(),
            closing_fee: SignedUint::from_str("-11004").unwrap(),
            pnl: SignedUint::from_str("-35980").unwrap(),
        };
        "short position - price up"
    )]
    #[test_case(
        Position {
            size: SignedUint::from_str("-100").unwrap(),
            entry_price: Decimal::from_str("4200").unwrap(), 
            entry_exec_price: Decimal::from_str("4200.714").unwrap(),
            entry_accrued_funding_per_unit_in_base_denom: SignedDecimal::from_str("-12.826").unwrap(),
            initial_skew: SignedUint::from_str("220").unwrap(),
            realized_pnl: PnlAmounts::default()
        },
        Decimal::from_str("4000").unwrap(),
        Decimal::from_str("0.02").unwrap(),
        PnlAmounts {
            opening_fee: SignedUint::zero(),
            price_pnl: SignedUint::from_str("24924").unwrap(),
            accrued_funding: SignedUint::from_str("117").unwrap(),
            closing_fee: SignedUint::from_str("-10004").unwrap(),
            pnl: SignedUint::from_str("15037").unwrap(),
        };
        "short position - price down"
    )]
    fn computing_pnl(
        position: Position,
        current_price: Decimal,
        closing_fee: Decimal,
        expect_pnl: PnlAmounts,
    ) {
        let funding = Funding {
            skew_scale: Uint128::new(1000000u128),
            last_funding_accrued_per_unit_in_base_denom: SignedDecimal::from_str("-14").unwrap(),
            ..Default::default()
        };
        let pnl_amounts = position
            .compute_pnl(
                &funding,
                SignedUint::from_str("280").unwrap(),
                current_price,
                Decimal::from_str("0.8").unwrap(),
                Decimal::zero(),
                closing_fee,
                PositionModification::None,
            )
            .unwrap();
        assert_eq!(pnl_amounts, expect_pnl);
    }
}
