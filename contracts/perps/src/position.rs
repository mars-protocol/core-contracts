use cosmwasm_std::Decimal;
use mars_types::{
    math::SignedDecimal,
    perps::{Funding, PnlAmounts, PnlValues, Position},
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
        opening_fee_rate: Decimal,
        closing_fee_rate: Decimal,
        modification: PositionModification,
    ) -> ContractResult<(PnlValues, PnlAmounts)>;
}

impl PositionExt for Position {
    fn compute_pnl(
        &self,
        funding: &Funding,
        skew: SignedDecimal,
        denom_price: Decimal,
        base_denom_price: Decimal,
        opening_fee_rate: Decimal,
        closing_fee_rate: Decimal,
        modification: PositionModification,
    ) -> ContractResult<(PnlValues, PnlAmounts)> {
        let exit_exec_price =
            closing_execution_price(skew, funding.skew_scale, self.size, denom_price)?;

        // size * (exit_exec_price - entry_exec_price)
        let price_diff = exit_exec_price.checked_sub(self.entry_exec_price.into())?;
        let price_pnl_value = self.size.checked_mul(price_diff)?;
        let price_pnl_in_base_denom = price_pnl_value.checked_div(base_denom_price.into())?;

        // size * (current_accrued_funding_per_unit - entry_accrued_funding_per_unit) * usdc_price
        let accrued_funding_diff = funding
            .last_funding_accrued_per_unit_in_base_denom
            .checked_sub(self.entry_accrued_funding_per_unit_in_base_denom)?;
        let accrued_funding_in_base_denom = self.size.checked_mul(accrued_funding_diff)?;
        let accrued_funding_value =
            accrued_funding_in_base_denom.checked_mul(base_denom_price.into())?;

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

        let realized_pnl_value = price_pnl_value
            .checked_add(accrued_funding_value)?
            .checked_add(opening_fee.0)?
            .checked_add(closing_fee.0)?;

        let realized_pnl_in_base_denom = price_pnl_in_base_denom
            .checked_add(accrued_funding_in_base_denom)?
            .checked_add(opening_fee.1)?
            .checked_add(closing_fee.1)?;

        Ok((
            PnlValues {
                price_pnl: price_pnl_value,
                accrued_funding: accrued_funding_value,
                closing_fee: closing_fee.0,
                pnl: realized_pnl_value,
            },
            PnlAmounts {
                price_pnl: price_pnl_in_base_denom,
                accrued_funding: accrued_funding_in_base_denom,
                opening_fee: opening_fee.1,
                closing_fee: closing_fee.1,
                pnl: realized_pnl_in_base_denom,
            },
        ))
    }
}

/// PositionModification is used to specify the type of position modification in order to calculate the fees
pub enum PositionModification {
    Increase(SignedDecimal),
    Decrease(SignedDecimal),
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
        size: SignedDecimal,
        skew: SignedDecimal,
        skew_scale: Decimal,
    ) -> ContractResult<((SignedDecimal, SignedDecimal), (SignedDecimal, SignedDecimal))> {
        // Extract the relevant size based on the modification type
        match self {
            // Apply opening fee based on the position size change:
            // - if opening it is position size,
            // - if increasing it is q change
            PositionModification::Increase(size) => {
                let denom_exec_price =
                    opening_execution_price(skew, skew_scale, *size, denom_price)?.abs;
                let opening_fee =
                    compute_fee(opening_fee_rate, *size, denom_exec_price, base_denom_price)?;
                let closing_fee = (SignedDecimal::zero(), SignedDecimal::zero());
                Ok((opening_fee, closing_fee))
            }

            // Apply closing fee based on the position size change:
            // - if closing it is position size,
            // - if reducing it is q change
            PositionModification::Decrease(size) => {
                let denom_exec_price =
                    closing_execution_price(skew, skew_scale, *size, denom_price)?.abs;
                let opening_fee = (SignedDecimal::zero(), SignedDecimal::zero());
                let closing_fee =
                    compute_fee(closing_fee_rate, *size, denom_exec_price, base_denom_price)?;
                Ok((opening_fee, closing_fee))
            }

            // No modification needed, return the original size
            // This can be used when querying the current PnL without affecting the position
            PositionModification::None => {
                let denom_exec_price =
                    closing_execution_price(skew, skew_scale, size, denom_price)?.abs;
                let opening_fee = (SignedDecimal::zero(), SignedDecimal::zero());
                let closing_fee =
                    compute_fee(closing_fee_rate, size, denom_exec_price, base_denom_price)?;
                Ok((opening_fee, closing_fee))
            }
        }
    }
}

fn compute_fee(
    rate: Decimal,
    size: SignedDecimal,
    denom_price: Decimal,
    base_denom_price: Decimal,
) -> ContractResult<(SignedDecimal, SignedDecimal)> {
    // Calculate the fee value
    let fee_value = size.abs.checked_mul(denom_price.checked_mul(rate)?)?;

    // Make the fee negative to show that it's a cost for the user
    let fee_value: SignedDecimal = SignedDecimal::zero().checked_sub(fee_value.into())?;

    // Calculate the fee in terms of the base denomination
    let fee_in_base_denom = fee_value.checked_div(base_denom_price.into())?;

    Ok((fee_value, fee_in_base_denom))
}

// ----------------------------------- Tests -----------------------------------

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use cosmwasm_std::{coin, Coin, Uint128};
    use mars_types::perps::{PnL, PnlAmounts, PnlCoins, PositionPnl};
    use test_case::test_case;

    use super::*;

    const MOCK_BASE_DENOM: &str = "uusdc";

    #[test_case(
        Position {
            size: SignedDecimal::from_str("100").unwrap(),
            entry_price: Decimal::from_str("4200").unwrap(), 
            entry_exec_price: Decimal::from_str("4200.966").unwrap(),
            entry_accrued_funding_per_unit_in_base_denom: SignedDecimal::from_str("-14").unwrap(),
            initial_skew: SignedDecimal::from_str("180").unwrap(),
            realized_pnl: PnlAmounts::default()
        },
        Decimal::from_str("4200").unwrap(),
        Decimal::zero(),
        PositionPnl {
            values: PnlValues {
                price_pnl: SignedDecimal::zero(),
                accrued_funding: SignedDecimal::zero(),
                closing_fee: SignedDecimal::zero(),
                pnl: SignedDecimal::zero(),
            },
            amounts: PnlAmounts::default(),
            coins: PnlCoins {
                closing_fee: coin(0, MOCK_BASE_DENOM),
                pnl: PnL::BreakEven,
            }
        };
        "long position - break even"
    )]
    #[test_case(
        Position {
            size: SignedDecimal::from_str("100").unwrap(),
            entry_price: Decimal::from_str("4200").unwrap(), 
            entry_exec_price: Decimal::from_str("4201.134").unwrap(),
            entry_accrued_funding_per_unit_in_base_denom: SignedDecimal::from_str("-12").unwrap(),
            initial_skew: SignedDecimal::from_str("220").unwrap(),
            realized_pnl: PnlAmounts::default()
        },
        Decimal::from_str("4400").unwrap(),
        Decimal::from_str("0.02").unwrap(),
        PositionPnl {
            values: PnlValues {
                price_pnl: SignedDecimal::from_str("19987.8").unwrap(),
                accrued_funding: SignedDecimal::from_str("-160").unwrap(),
                closing_fee: SignedDecimal::from_str("-8802.024").unwrap(),
                pnl: SignedDecimal::from_str("11025.776").unwrap(),
            },
            amounts: PnlAmounts::default(),
            coins: PnlCoins {
                closing_fee: coin(11002, MOCK_BASE_DENOM),
                pnl: PnL::Profit(Coin {
                    denom: MOCK_BASE_DENOM.into(),
                    amount: Uint128::new(13782),
                })
            }
        };
        "long position - price up"
    )]
    #[test_case(
        Position {
            size: SignedDecimal::from_str("100").unwrap(),
            entry_price: Decimal::from_str("4200").unwrap(), 
            entry_exec_price: Decimal::from_str("4201.134").unwrap(),
            entry_accrued_funding_per_unit_in_base_denom: SignedDecimal::from_str("-12").unwrap(),
            initial_skew: SignedDecimal::from_str("220").unwrap(),
            realized_pnl: PnlAmounts::default()
        },
        Decimal::from_str("4000").unwrap(),
        Decimal::from_str("0.02").unwrap(),
        PositionPnl {
            values: PnlValues {
                price_pnl: SignedDecimal::from_str("-20021.4").unwrap(),
                accrued_funding: SignedDecimal::from_str("-160").unwrap(),
                closing_fee: SignedDecimal::from_str("-8001.84").unwrap(),
                pnl: SignedDecimal::from_str("-28183.24").unwrap(),
            },
            amounts: PnlAmounts::default(),
            coins: PnlCoins {
                closing_fee: coin(10002, MOCK_BASE_DENOM),
                pnl: PnL::Loss(Coin {
                    denom: MOCK_BASE_DENOM.into(),
                    amount: Uint128::new(35229),
                })
            }
        };
        "long position - price down"
    )]
    #[test_case(
        Position {
            size: SignedDecimal::from_str("-100").unwrap(),
            entry_price: Decimal::from_str("4200").unwrap(), 
            entry_exec_price: Decimal::from_str("4201.386").unwrap(),
            entry_accrued_funding_per_unit_in_base_denom: SignedDecimal::from_str("-14").unwrap(),
            initial_skew: SignedDecimal::from_str("380").unwrap(),
            realized_pnl: PnlAmounts::default()
        },
        Decimal::from_str("4200").unwrap(),
        Decimal::zero(),
        PositionPnl {
            values: PnlValues {
                price_pnl: SignedDecimal::zero(),
                accrued_funding: SignedDecimal::zero(),
                closing_fee: SignedDecimal::zero(),
                pnl: SignedDecimal::zero(),
            },
            amounts: PnlAmounts::default(),
            coins: PnlCoins {
                closing_fee: coin(0, MOCK_BASE_DENOM),
                pnl:  PnL::BreakEven
            }
        };
        "short position - break even"
    )]
    #[test_case(
        Position {
            size: SignedDecimal::from_str("-100").unwrap(),
            entry_price: Decimal::from_str("4200").unwrap(), 
            entry_exec_price: Decimal::from_str("4200.714").unwrap(),
            entry_accrued_funding_per_unit_in_base_denom: SignedDecimal::from_str("-12").unwrap(),
            initial_skew: SignedDecimal::from_str("220").unwrap(),
            realized_pnl: PnlAmounts::default()
        },
        Decimal::from_str("4400").unwrap(),
        Decimal::from_str("0.02").unwrap(),
        PositionPnl {
            values: PnlValues {
                price_pnl: SignedDecimal::from_str("-20073.8").unwrap(),
                accrued_funding: SignedDecimal::from_str("160").unwrap(),
                closing_fee: SignedDecimal::from_str("-8802.904").unwrap(),
                pnl: SignedDecimal::from_str("-28716.704").unwrap(),
            },
            amounts: PnlAmounts::default(),
            coins: PnlCoins {
                closing_fee: coin(11003, MOCK_BASE_DENOM),
                pnl:  PnL::Loss(Coin {
                    denom: MOCK_BASE_DENOM.into(),
                    amount: Uint128::new(35895),
                })
            }
        };
        "short position - price up"
    )]
    #[test_case(
        Position {
            size: SignedDecimal::from_str("-100").unwrap(),
            entry_price: Decimal::from_str("4200").unwrap(), 
            entry_exec_price: Decimal::from_str("4200.714").unwrap(),
            entry_accrued_funding_per_unit_in_base_denom: SignedDecimal::from_str("-12").unwrap(),
            initial_skew: SignedDecimal::from_str("220").unwrap(),
            realized_pnl: PnlAmounts::default()
        },
        Decimal::from_str("4000").unwrap(),
        Decimal::from_str("0.02").unwrap(),
        PositionPnl {
            values: PnlValues {
                price_pnl: SignedDecimal::from_str("19939.4").unwrap(),
                accrued_funding: SignedDecimal::from_str("160").unwrap(),
                closing_fee: SignedDecimal::from_str("-8002.64").unwrap(),
                pnl: SignedDecimal::from_str("12096.76").unwrap(),
            },
            amounts: PnlAmounts::default(),
            coins: PnlCoins {
                closing_fee: coin(10003, MOCK_BASE_DENOM),
                pnl: PnL::Profit(Coin {
                    denom: MOCK_BASE_DENOM.into(),
                    amount: Uint128::new(15120),
                })
            }
        };
        "short position - price down"
    )]
    fn computing_pnl(
        position: Position,
        current_price: Decimal,
        closing_fee: Decimal,
        expect_pnl: PositionPnl,
    ) {
        let funding = Funding {
            skew_scale: Decimal::from_str("1000000").unwrap(),
            last_funding_accrued_per_unit_in_base_denom: SignedDecimal::from_str("-14").unwrap(),
            ..Default::default()
        };
        let (pnl_values, pnl_amounts) = position
            .compute_pnl(
                &funding,
                SignedDecimal::from_str("280").unwrap(),
                current_price,
                Decimal::from_str("0.8").unwrap(),
                Decimal::zero(),
                closing_fee,
                PositionModification::None,
            )
            .unwrap();
        let pnl_coins = pnl_amounts.to_coins(MOCK_BASE_DENOM);
        assert_eq!(pnl_values, expect_pnl.values);
        assert_eq!(pnl_coins, expect_pnl.coins);
    }
}
