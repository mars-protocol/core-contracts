use cosmwasm_std::{coin, Decimal};
use mars_types::{
    math::SignedDecimal,
    perps::{Funding, PnL, PnlAmounts, PnlCoins, PnlValues, Position, PositionPnl},
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
        base_denom: &str,
        closing_fee_rate: Decimal,
        reducing: bool,
        q_change: Option<SignedDecimal>,
    ) -> ContractResult<(PositionPnl, PnlAmounts)>;
}

impl PositionExt for Position {
    fn compute_pnl(
        &self,
        funding: &Funding,
        skew: SignedDecimal,
        denom_price: Decimal,
        base_denom_price: Decimal,
        base_denom: &str,
        closing_fee_rate: Decimal,
        reducing: bool,
        q_change: Option<SignedDecimal>,
    ) -> ContractResult<(PositionPnl, PnlAmounts)> {
        // TODO: exec price should be positive
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
        let price_pnl_value = self.size.checked_mul(price_diff)?;
        let price_pnl_in_base_denom = price_pnl_value.checked_div(base_denom_price.into())?;

        // size * (current_accrued_funding_per_unit - entry_accrued_funding_per_unit) * usdc_price
        let accrued_funding_diff = funding
            .last_funding_accrued_per_unit_in_base_denom
            .checked_sub(self.entry_accrued_funding_per_unit_in_base_denom)?;
        let accrued_funding_in_base_denom = self.size.checked_mul(accrued_funding_diff)?;
        let accrued_funding_value =
            accrued_funding_in_base_denom.checked_mul(base_denom_price.into())?;

        let denom_exec_price = exit_exec_price.abs;

        // Only charge closing fees if we are reducing size
        let (closing_fee_value, closing_fee_in_base_denom) = match reducing {
            true => {
                // fee_in_base_denom = closing_fee_rate * denom_exec_price * size
                let closing_fee_value = q_change
                    .unwrap_or(self.size)
                    .abs
                    .checked_mul(denom_exec_price.checked_mul(closing_fee_rate)?)?;
                // make closing fee negative to show that it's a cost for the user
                let closing_fee_value: SignedDecimal =
                    SignedDecimal::zero().checked_sub(closing_fee_value.into())?;
                let closing_fee_in_base_denom =
                    closing_fee_value.checked_div(base_denom_price.into())?;
                (closing_fee_value, closing_fee_in_base_denom)
            }
            false => {
                // we only apply closing fees to negative
                (SignedDecimal::zero(), SignedDecimal::zero())
            }
        };

        let realized_pnl_value =
            price_pnl_value.checked_add(accrued_funding_value)?.checked_add(closing_fee_value)?;

        let realized_pnl_in_base_denom = price_pnl_in_base_denom
            .checked_add(accrued_funding_in_base_denom)?
            .checked_add(closing_fee_in_base_denom)?;

        Ok((
            PositionPnl {
                values: PnlValues {
                    price_pnl: price_pnl_value,
                    accrued_funding: accrued_funding_value,
                    closing_fee: closing_fee_value,
                    pnl: realized_pnl_value,
                },
                coins: PnlCoins {
                    closing_fee: coin(
                        closing_fee_in_base_denom.abs.to_uint_floor().u128(),
                        base_denom,
                    ),
                    pnl: PnL::from_signed_decimal(base_denom, realized_pnl_in_base_denom),
                },
            },
            PnlAmounts {
                price_pnl: price_pnl_in_base_denom,
                accrued_funding: accrued_funding_in_base_denom,
                closing_fee: closing_fee_in_base_denom,
                pnl: realized_pnl_in_base_denom,
            },
        ))
    }
}

// ----------------------------------- Tests -----------------------------------

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use cosmwasm_std::{Coin, Uint128};
    use test_case::test_case;

    use super::*;

    const MOCK_BASE_DENOM: &str = "uusdc";

    #[test_case(
        Position {
            size: SignedDecimal::from_str("100").unwrap(),
            entry_price: Decimal::from_str("4200").unwrap(), 
            entry_accrued_funding_per_unit_in_base_denom: SignedDecimal::from_str("-14").unwrap(),
            initial_skew: SignedDecimal::from_str("180").unwrap(),
            opening_fee_in_base_denom: Uint128::zero()
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
            entry_accrued_funding_per_unit_in_base_denom: SignedDecimal::from_str("-12").unwrap(),
            initial_skew: SignedDecimal::from_str("220").unwrap(),
            opening_fee_in_base_denom: Uint128::zero()
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
            entry_accrued_funding_per_unit_in_base_denom: SignedDecimal::from_str("-12").unwrap(),
            initial_skew: SignedDecimal::from_str("220").unwrap(),
            opening_fee_in_base_denom: Uint128::zero()
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
            entry_accrued_funding_per_unit_in_base_denom: SignedDecimal::from_str("-14").unwrap(),
            initial_skew: SignedDecimal::from_str("380").unwrap(),
            opening_fee_in_base_denom: Uint128::zero()
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
            entry_accrued_funding_per_unit_in_base_denom: SignedDecimal::from_str("-12").unwrap(),
            initial_skew: SignedDecimal::from_str("220").unwrap(),
            opening_fee_in_base_denom: Uint128::zero()
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
            entry_accrued_funding_per_unit_in_base_denom: SignedDecimal::from_str("-12").unwrap(),
            initial_skew: SignedDecimal::from_str("220").unwrap(),
            opening_fee_in_base_denom: Uint128::zero()
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
        let (pnl, _) = position
            .compute_pnl(
                &funding,
                SignedDecimal::from_str("280").unwrap(),
                current_price,
                Decimal::from_str("0.8").unwrap(),
                MOCK_BASE_DENOM,
                closing_fee,
                true,
                None,
            )
            .unwrap();
        assert_eq!(pnl, expect_pnl);
    }
}
