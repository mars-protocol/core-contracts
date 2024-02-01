/// Accounting module.
/// It is used to compute the accounting for a single denom or for all denoms.
/// Accounting represents the state of the base denom balance (vault) after applying the given cash flow, unrealized PnL and base denom price.
use std::cmp::max;

use cosmwasm_std::{Decimal, Uint128};
use mars_types::{
    math::SignedDecimal,
    perps::{Accounting, Balance, CashFlow, DenomPnlValues, PnlAmounts},
};

use crate::error::ContractResult;

pub trait CashFlowExt {
    /// Update the cash flow opening fees with the given amount
    fn update_opening_fees(&mut self, opening_fee: Uint128) -> ContractResult<()>;

    /// Update the cash flow with the given amounts
    fn update(&mut self, amounts: &PnlAmounts) -> ContractResult<()>;
}

pub trait BalanceExt {
    /// Compute the balance after applying the given cash flow, unrealized PnL and base denom price
    fn compute_balance(
        cash_flow: &CashFlow,
        unrealized_pnl: &DenomPnlValues,
        base_denom_price: Decimal,
    ) -> ContractResult<Balance>;

    /// Compute the withdrawal balance after applying the given cash flow, unrealized PnL and base denom price
    fn compute_withdrawal_balance(
        cash_flow: &CashFlow,
        unrealized_pnl: &DenomPnlValues,
        base_denom_price: Decimal,
    ) -> ContractResult<Balance>;
}

pub trait AccountingExt {
    /// Accounting after applying the given cash flow, unrealized PnL and base denom price.
    /// It can be used to compute the accounting for a single denom or for all denoms.
    fn compute(
        cash_flow: &CashFlow,
        unrealized_pnl: &DenomPnlValues,
        base_denom_price: Decimal,
    ) -> ContractResult<Accounting>;
}

impl CashFlowExt for CashFlow {
    fn update_opening_fees(&mut self, opening_fee: Uint128) -> ContractResult<()> {
        self.opening_fees = self.opening_fees.checked_add(opening_fee.into())?;
        Ok(())
    }

    fn update(&mut self, amounts: &PnlAmounts) -> ContractResult<()> {
        // Account profit is vault loss and vice versa.
        // If values are positive, vault is losing money.
        self.price_pnl = self.price_pnl.checked_sub(amounts.price_pnl)?;
        self.accrued_funding = self.accrued_funding.checked_sub(amounts.accrued_funding)?;
        self.closing_fees = self.closing_fees.checked_sub(amounts.closing_fee)?;
        Ok(())
    }
}

impl BalanceExt for Balance {
    fn compute_balance(
        cash_flow: &CashFlow,
        unrealized_pnl: &DenomPnlValues,
        base_denom_price: Decimal,
    ) -> ContractResult<Balance> {
        // denominate pnl values into base denom (e.g. USDC)
        let price_pnl_in_base_denom =
            unrealized_pnl.price_pnl.checked_div(base_denom_price.into())?;
        let accrued_funding_in_base_denom =
            unrealized_pnl.accrued_funding.checked_div(base_denom_price.into())?;
        let closing_fees_in_base_denom =
            unrealized_pnl.closing_fees.checked_div(base_denom_price.into())?;

        // Account profit is vault loss and vice versa.
        // If values are positive, vault is losing money.
        let price_pnl = cash_flow.price_pnl.checked_sub(price_pnl_in_base_denom)?;
        let accrued_funding =
            cash_flow.accrued_funding.checked_sub(accrued_funding_in_base_denom)?;
        let closing_fees = cash_flow.closing_fees.checked_sub(closing_fees_in_base_denom)?;

        let balance = Balance {
            price_pnl,
            opening_fees: cash_flow.opening_fees, // opening fees are already paid
            closing_fees,
            accrued_funding,
            total: price_pnl
                .checked_add(cash_flow.opening_fees)?
                .checked_add(closing_fees)?
                .checked_add(accrued_funding)?,
        };

        Ok(balance)
    }

    fn compute_withdrawal_balance(
        cash_flow: &CashFlow,
        unrealized_pnl: &DenomPnlValues,
        base_denom_price: Decimal,
    ) -> ContractResult<Balance> {
        // denominate pnl values into base denom (e.g. USDC)
        let price_pnl_in_base_denom =
            unrealized_pnl.price_pnl.checked_div(base_denom_price.into())?;
        let accrued_funding_in_base_denom =
            unrealized_pnl.accrued_funding.checked_div(base_denom_price.into())?;

        // If unrealized price pnl or accrued funding is positive it means that the vault is losing money.
        // We have to subtract amount which will be taken from the vault after realizing the pnl.
        // If unrealized price pnl or accrued funding is negative it means that the vault is making money.
        // We have to cap the amount to zero because we don't have that money in the vault (we will have once pnl is realized).
        let price_pnl =
            cash_flow.price_pnl.checked_sub(max(SignedDecimal::zero(), price_pnl_in_base_denom))?;
        let accrued_funding = cash_flow
            .accrued_funding
            .checked_sub(max(SignedDecimal::zero(), accrued_funding_in_base_denom))?;

        let balance = Balance {
            price_pnl,
            opening_fees: cash_flow.opening_fees, // opening fees are already paid
            closing_fees: cash_flow.closing_fees, // closing fees will be paid after realizing the pnl
            accrued_funding,
            total: price_pnl
                .checked_add(cash_flow.opening_fees)?
                .checked_add(cash_flow.closing_fees)?
                .checked_add(accrued_funding)?,
        };

        Ok(balance)
    }
}

impl AccountingExt for Accounting {
    fn compute(
        cash_flow: &CashFlow,
        unrealized_pnl: &DenomPnlValues,
        base_denom_price: Decimal,
    ) -> ContractResult<Accounting> {
        let balance = Balance::compute_balance(cash_flow, unrealized_pnl, base_denom_price)?;
        let withdrawal_balance =
            Balance::compute_withdrawal_balance(cash_flow, unrealized_pnl, base_denom_price)?;
        Ok(Accounting {
            cash_flow: cash_flow.clone(),
            balance,
            withdrawal_balance,
        })
    }
}

// ----------------------------------- Tests -----------------------------------

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use cosmwasm_std::Uint128;
    use mars_types::perps::CashFlow;

    use super::*;

    #[test]
    fn update_cash_flow_with_opening_fee() {
        let mut cf = CashFlow::default();

        let opening_fee = Uint128::new(120);
        cf.update_opening_fees(opening_fee).unwrap();

        assert_eq!(
            cf,
            CashFlow {
                opening_fees: SignedDecimal::from(opening_fee),
                ..Default::default()
            }
        );
    }

    #[test]
    fn update_cash_flow() {
        let opening_fee = Uint128::new(120);
        let mut cf = CashFlow {
            opening_fees: SignedDecimal::from(opening_fee),
            ..Default::default()
        };

        // update with negative numbers
        let amounts = PnlAmounts {
            price_pnl: SignedDecimal::from_str("-100").unwrap(),
            accrued_funding: SignedDecimal::from_str("-300").unwrap(),
            closing_fee: SignedDecimal::from_str("-400").unwrap(),
            pnl: SignedDecimal::from_str("-800").unwrap(),
        };
        cf.update(&amounts).unwrap();
        assert_eq!(
            cf,
            CashFlow {
                opening_fees: SignedDecimal::from(opening_fee),
                price_pnl: SignedDecimal::from_str("100").unwrap(),
                accrued_funding: SignedDecimal::from_str("300").unwrap(),
                closing_fees: SignedDecimal::from_str("400").unwrap(),
            }
        );

        // update with positive numbers
        let amounts = PnlAmounts {
            price_pnl: SignedDecimal::from_str("150").unwrap(),
            accrued_funding: SignedDecimal::from_str("320").unwrap(),
            closing_fee: SignedDecimal::from_str("430").unwrap(),
            pnl: SignedDecimal::from_str("900").unwrap(),
        };
        cf.update(&amounts).unwrap();
        assert_eq!(
            cf,
            CashFlow {
                opening_fees: SignedDecimal::from(opening_fee),
                price_pnl: SignedDecimal::from_str("-50").unwrap(),
                accrued_funding: SignedDecimal::from_str("-20").unwrap(),
                closing_fees: SignedDecimal::from_str("-30").unwrap(),
            }
        );
    }

    #[test]
    fn compute_balance() {
        let cash_flow = CashFlow {
            opening_fees: SignedDecimal::from_str("100").unwrap(),
            price_pnl: SignedDecimal::from_str("300").unwrap(),
            accrued_funding: SignedDecimal::from_str("200").unwrap(),
            closing_fees: SignedDecimal::from_str("50").unwrap(),
        };
        let base_denom_price = Decimal::from_str("0.5").unwrap();

        // compute balance with positive numbers
        let unrealized_pnl = DenomPnlValues {
            price_pnl: SignedDecimal::from_str("200").unwrap(),
            accrued_funding: SignedDecimal::from_str("120").unwrap(),
            closing_fees: SignedDecimal::from_str("30").unwrap(),
            pnl: SignedDecimal::from_str("350").unwrap(),
        };
        let expected_balance = Balance {
            price_pnl: SignedDecimal::from_str("-100").unwrap(),
            opening_fees: SignedDecimal::from_str("100").unwrap(),
            closing_fees: SignedDecimal::from_str("-10").unwrap(),
            accrued_funding: SignedDecimal::from_str("-40").unwrap(),
            total: SignedDecimal::from_str("-50").unwrap(),
        };
        let actual_balance =
            Balance::compute_balance(&cash_flow, &unrealized_pnl, base_denom_price).unwrap();
        assert_eq!(actual_balance, expected_balance);

        // compute balance with negative numbers
        let unrealized_pnl = DenomPnlValues {
            price_pnl: SignedDecimal::from_str("-200").unwrap(),
            accrued_funding: SignedDecimal::from_str("-120").unwrap(),
            closing_fees: SignedDecimal::from_str("-30").unwrap(),
            pnl: SignedDecimal::from_str("-350").unwrap(),
        };
        let expected_balance = Balance {
            price_pnl: SignedDecimal::from_str("700").unwrap(),
            opening_fees: SignedDecimal::from_str("100").unwrap(),
            closing_fees: SignedDecimal::from_str("110").unwrap(),
            accrued_funding: SignedDecimal::from_str("440").unwrap(),
            total: SignedDecimal::from_str("1350").unwrap(),
        };
        let actual_balance =
            Balance::compute_balance(&cash_flow, &unrealized_pnl, base_denom_price).unwrap();
        assert_eq!(actual_balance, expected_balance);
    }

    #[test]
    fn compute_withdrawal_balance() {
        let cash_flow = CashFlow {
            opening_fees: SignedDecimal::from_str("100").unwrap(),
            price_pnl: SignedDecimal::from_str("300").unwrap(),
            accrued_funding: SignedDecimal::from_str("200").unwrap(),
            closing_fees: SignedDecimal::from_str("50").unwrap(),
        };
        let base_denom_price = Decimal::from_str("0.5").unwrap();

        // compute withdrawal balance with positive numbers
        let unrealized_pnl = DenomPnlValues {
            price_pnl: SignedDecimal::from_str("200").unwrap(),
            accrued_funding: SignedDecimal::from_str("120").unwrap(),
            closing_fees: SignedDecimal::from_str("30").unwrap(),
            pnl: SignedDecimal::from_str("350").unwrap(),
        };
        let expected_balance = Balance {
            price_pnl: SignedDecimal::from_str("-100").unwrap(),
            opening_fees: cash_flow.opening_fees,
            closing_fees: cash_flow.closing_fees,
            accrued_funding: SignedDecimal::from_str("-40").unwrap(),
            total: SignedDecimal::from_str("10").unwrap(),
        };
        let actual_balance =
            Balance::compute_withdrawal_balance(&cash_flow, &unrealized_pnl, base_denom_price)
                .unwrap();
        assert_eq!(actual_balance, expected_balance);

        // compute withdrawal balance with negative numbers
        let unrealized_pnl = DenomPnlValues {
            price_pnl: SignedDecimal::from_str("-200").unwrap(),
            accrued_funding: SignedDecimal::from_str("-120").unwrap(),
            closing_fees: SignedDecimal::from_str("-30").unwrap(),
            pnl: SignedDecimal::from_str("-350").unwrap(),
        };
        let expected_balance = Balance {
            price_pnl: cash_flow.price_pnl,
            opening_fees: cash_flow.opening_fees,
            closing_fees: cash_flow.closing_fees,
            accrued_funding: cash_flow.accrued_funding,
            total: SignedDecimal::from_str("650").unwrap(),
        };
        let actual_balance =
            Balance::compute_withdrawal_balance(&cash_flow, &unrealized_pnl, base_denom_price)
                .unwrap();
        assert_eq!(actual_balance, expected_balance);
    }

    #[test]
    fn compute_accounting() {
        let cash_flow = CashFlow {
            opening_fees: SignedDecimal::from_str("100").unwrap(),
            price_pnl: SignedDecimal::from_str("300").unwrap(),
            accrued_funding: SignedDecimal::from_str("200").unwrap(),
            closing_fees: SignedDecimal::from_str("50").unwrap(),
        };
        let base_denom_price = Decimal::from_str("0.5").unwrap();
        let unrealized_pnl = DenomPnlValues {
            price_pnl: SignedDecimal::from_str("-200").unwrap(),
            accrued_funding: SignedDecimal::from_str("-120").unwrap(),
            closing_fees: SignedDecimal::from_str("-30").unwrap(),
            pnl: SignedDecimal::from_str("-350").unwrap(),
        };

        let balance =
            Balance::compute_balance(&cash_flow, &unrealized_pnl, base_denom_price).unwrap();
        let withdrawal_balance =
            Balance::compute_withdrawal_balance(&cash_flow, &unrealized_pnl, base_denom_price)
                .unwrap();
        assert_ne!(balance, withdrawal_balance);

        let expected_accounting = Accounting {
            cash_flow: cash_flow.clone(),
            balance,
            withdrawal_balance,
        };
        let actual_accounting =
            Accounting::compute(&cash_flow, &unrealized_pnl, base_denom_price).unwrap();
        assert_eq!(actual_accounting, expected_accounting);
    }
}
