/// Accounting module.
/// It is used to compute the accounting for a single denom or for all denoms.
/// Accounting represents the state of the base denom balance (vault) after applying the given cash flow, unrealized PnL and base denom price.
use std::cmp::max;

use cosmwasm_std::Uint128;
use mars_types::{
    perps::{Accounting, Balance, CashFlow, PnlAmounts},
    signed_uint::SignedUint,
};

use crate::error::ContractResult;

pub trait CashFlowExt {
    /// Update the cash flow with the given amounts
    fn add(&mut self, amounts: &PnlAmounts, protocol_fee: Uint128) -> ContractResult<()>;
}

pub trait BalanceExt {
    /// Compute the balance after applying the given cash flow and unrealized PnL
    fn compute_balance(
        cash_flow: &CashFlow,
        unrealized_pnl: &PnlAmounts,
    ) -> ContractResult<Balance>;

    /// Compute the withdrawal balance after applying the given cash flow and unrealized PnL
    fn compute_withdrawal_balance(
        cash_flow: &CashFlow,
        unrealized_pnl: &PnlAmounts,
    ) -> ContractResult<Balance>;
}

pub trait AccountingExt {
    /// Accounting after applying the given cash flow and unrealized PnL.
    /// It can be used to compute the accounting for a single denom or for all denoms.
    fn compute(cash_flow: &CashFlow, unrealized_pnl: &PnlAmounts) -> ContractResult<Accounting>;
}

impl CashFlowExt for CashFlow {
    fn add(&mut self, amounts: &PnlAmounts, protocol_fee: Uint128) -> ContractResult<()> {
        // Account profit is vault loss and vice versa.
        // If values are positive, vault is losing money.
        // Protocol fee is always profit for the protocol, so positive.
        self.price_pnl = self.price_pnl.checked_sub(amounts.price_pnl)?;
        self.accrued_funding = self.accrued_funding.checked_sub(amounts.accrued_funding)?;
        self.opening_fee = self.opening_fee.checked_sub(amounts.opening_fee)?;
        self.closing_fee = self.closing_fee.checked_sub(amounts.closing_fee)?;
        self.protocol_fee = self.protocol_fee.checked_add(protocol_fee)?;
        Ok(())
    }
}

impl BalanceExt for Balance {
    fn compute_balance(
        cash_flow: &CashFlow,
        unrealized_pnl: &PnlAmounts,
    ) -> ContractResult<Balance> {
        // Account profit is vault loss and vice versa.
        // If values are positive, vault is losing money.
        let price_pnl = cash_flow.price_pnl.checked_sub(unrealized_pnl.price_pnl)?;
        let accrued_funding =
            cash_flow.accrued_funding.checked_sub(unrealized_pnl.accrued_funding)?;
        let closing_fee = cash_flow.closing_fee.checked_sub(unrealized_pnl.closing_fee)?;

        let balance = Balance {
            price_pnl,
            opening_fee: cash_flow.opening_fee, // opening fees are already paid
            closing_fee,
            accrued_funding,
            total: price_pnl
                .checked_add(cash_flow.opening_fee)?
                .checked_add(closing_fee)?
                .checked_add(accrued_funding)?,
        };

        Ok(balance)
    }

    fn compute_withdrawal_balance(
        cash_flow: &CashFlow,
        unrealized_pnl: &PnlAmounts,
    ) -> ContractResult<Balance> {
        // If unrealized price pnl or accrued funding is positive it means that the vault is losing money.
        // We have to subtract amount which will be taken from the vault after realizing the pnl.
        // If unrealized price pnl or accrued funding is negative it means that the vault is making money.
        // We have to cap the amount to zero because we don't have that money in the vault (we will have once pnl is realized).
        let price_pnl =
            cash_flow.price_pnl.checked_sub(max(SignedUint::zero(), unrealized_pnl.price_pnl))?;
        let accrued_funding = cash_flow
            .accrued_funding
            .checked_sub(max(SignedUint::zero(), unrealized_pnl.accrued_funding))?;

        let balance = Balance {
            price_pnl,
            opening_fee: cash_flow.opening_fee, // opening fees are already paid
            closing_fee: cash_flow.closing_fee, // closing fees will be paid after realizing the pnl
            accrued_funding,
            total: price_pnl
                .checked_add(cash_flow.opening_fee)?
                .checked_add(cash_flow.closing_fee)?
                .checked_add(accrued_funding)?,
        };

        Ok(balance)
    }
}

impl AccountingExt for Accounting {
    fn compute(cash_flow: &CashFlow, unrealized_pnl: &PnlAmounts) -> ContractResult<Accounting> {
        let balance = Balance::compute_balance(cash_flow, unrealized_pnl)?;
        let withdrawal_balance = Balance::compute_withdrawal_balance(cash_flow, unrealized_pnl)?;
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
    fn update_cash_flow() {
        let opening_fee = Uint128::new(120);
        let mut cf = CashFlow {
            opening_fee: SignedUint::from(opening_fee),
            ..Default::default()
        };

        let protocol_fee = Uint128::from_str("50").unwrap();

        // update with negative numbers
        let amounts = PnlAmounts {
            price_pnl: SignedUint::from_str("-100").unwrap(),
            accrued_funding: SignedUint::from_str("-300").unwrap(),
            opening_fee: SignedUint::zero(),
            closing_fee: SignedUint::from_str("-400").unwrap(),
            pnl: SignedUint::from_str("-800").unwrap(),
        };

        cf.add(&amounts, protocol_fee).unwrap();
        assert_eq!(
            cf,
            CashFlow {
                opening_fee: SignedUint::from(opening_fee),
                price_pnl: SignedUint::from_str("100").unwrap(),
                accrued_funding: SignedUint::from_str("300").unwrap(),
                closing_fee: SignedUint::from_str("400").unwrap(),
                protocol_fee: Uint128::from_str("50").unwrap(),
            }
        );

        let protocol_fee = Uint128::from_str("25").unwrap();

        // update with positive numbers
        let amounts = PnlAmounts {
            price_pnl: SignedUint::from_str("150").unwrap(),
            accrued_funding: SignedUint::from_str("320").unwrap(),
            opening_fee: SignedUint::zero(),
            closing_fee: SignedUint::from_str("430").unwrap(),
            pnl: SignedUint::from_str("900").unwrap(),
        };
        cf.add(&amounts, protocol_fee).unwrap();
        assert_eq!(
            cf,
            CashFlow {
                opening_fee: SignedUint::from(opening_fee),
                price_pnl: SignedUint::from_str("-50").unwrap(),
                accrued_funding: SignedUint::from_str("-20").unwrap(),
                closing_fee: SignedUint::from_str("-30").unwrap(),
                protocol_fee: Uint128::from_str("75").unwrap(),
            }
        );
    }

    #[test]
    fn compute_balance() {
        let cash_flow = CashFlow {
            opening_fee: SignedUint::from_str("100").unwrap(),
            price_pnl: SignedUint::from_str("300").unwrap(),
            accrued_funding: SignedUint::from_str("200").unwrap(),
            closing_fee: SignedUint::from_str("50").unwrap(),
            protocol_fee: Uint128::from_str("25").unwrap(),
        };

        // compute balance with positive numbers
        let unrealized_pnl = PnlAmounts {
            price_pnl: SignedUint::from_str("400").unwrap(),
            accrued_funding: SignedUint::from_str("240").unwrap(),
            closing_fee: SignedUint::from_str("60").unwrap(),
            pnl: SignedUint::from_str("700").unwrap(),
            opening_fee: SignedUint::zero(),
        };
        let expected_balance = Balance {
            price_pnl: SignedUint::from_str("-100").unwrap(),
            opening_fee: SignedUint::from_str("100").unwrap(),
            closing_fee: SignedUint::from_str("-10").unwrap(),
            accrued_funding: SignedUint::from_str("-40").unwrap(),
            total: SignedUint::from_str("-50").unwrap(),
        };
        let actual_balance = Balance::compute_balance(&cash_flow, &unrealized_pnl).unwrap();
        assert_eq!(actual_balance, expected_balance);

        // compute balance with negative numbers
        let unrealized_pnl = PnlAmounts {
            price_pnl: SignedUint::from_str("-400").unwrap(),
            accrued_funding: SignedUint::from_str("-240").unwrap(),
            closing_fee: SignedUint::from_str("-60").unwrap(),
            pnl: SignedUint::from_str("-700").unwrap(),
            opening_fee: SignedUint::zero(),
        };
        let expected_balance = Balance {
            price_pnl: SignedUint::from_str("700").unwrap(),
            opening_fee: SignedUint::from_str("100").unwrap(),
            closing_fee: SignedUint::from_str("110").unwrap(),
            accrued_funding: SignedUint::from_str("440").unwrap(),
            total: SignedUint::from_str("1350").unwrap(),
        };
        let actual_balance = Balance::compute_balance(&cash_flow, &unrealized_pnl).unwrap();
        assert_eq!(actual_balance, expected_balance);
    }

    #[test]
    fn compute_withdrawal_balance() {
        let cash_flow = CashFlow {
            opening_fee: SignedUint::from_str("100").unwrap(),
            price_pnl: SignedUint::from_str("300").unwrap(),
            accrued_funding: SignedUint::from_str("200").unwrap(),
            closing_fee: SignedUint::from_str("50").unwrap(),
            protocol_fee: Uint128::from_str("40").unwrap(),
        };

        // compute withdrawal balance with positive numbers
        let unrealized_pnl = PnlAmounts {
            price_pnl: SignedUint::from_str("400").unwrap(),
            accrued_funding: SignedUint::from_str("240").unwrap(),
            closing_fee: SignedUint::from_str("60").unwrap(),
            pnl: SignedUint::from_str("700").unwrap(),
            opening_fee: SignedUint::zero(),
        };
        let expected_balance = Balance {
            price_pnl: SignedUint::from_str("-100").unwrap(),
            opening_fee: cash_flow.opening_fee,
            closing_fee: cash_flow.closing_fee,
            accrued_funding: SignedUint::from_str("-40").unwrap(),
            total: SignedUint::from_str("10").unwrap(),
        };
        let actual_balance =
            Balance::compute_withdrawal_balance(&cash_flow, &unrealized_pnl).unwrap();
        assert_eq!(actual_balance, expected_balance);

        // compute withdrawal balance with negative numbers
        let unrealized_pnl = PnlAmounts {
            price_pnl: SignedUint::from_str("-400").unwrap(),
            accrued_funding: SignedUint::from_str("-240").unwrap(),
            closing_fee: SignedUint::from_str("-60").unwrap(),
            pnl: SignedUint::from_str("-700").unwrap(),
            opening_fee: SignedUint::zero(),
        };
        let expected_balance = Balance {
            price_pnl: cash_flow.price_pnl,
            opening_fee: cash_flow.opening_fee,
            closing_fee: cash_flow.closing_fee,
            accrued_funding: cash_flow.accrued_funding,
            total: SignedUint::from_str("650").unwrap(),
        };
        let actual_balance =
            Balance::compute_withdrawal_balance(&cash_flow, &unrealized_pnl).unwrap();
        assert_eq!(actual_balance, expected_balance);
    }

    #[test]
    fn compute_accounting() {
        let cash_flow = CashFlow {
            opening_fee: SignedUint::from_str("100").unwrap(),
            price_pnl: SignedUint::from_str("300").unwrap(),
            accrued_funding: SignedUint::from_str("200").unwrap(),
            closing_fee: SignedUint::from_str("50").unwrap(),
            protocol_fee: Uint128::from_str("40").unwrap(),
        };
        let unrealized_pnl = PnlAmounts {
            price_pnl: SignedUint::from_str("-400").unwrap(),
            accrued_funding: SignedUint::from_str("-240").unwrap(),
            closing_fee: SignedUint::from_str("-60").unwrap(),
            pnl: SignedUint::from_str("-700").unwrap(),
            opening_fee: SignedUint::zero(),
        };

        let balance = Balance::compute_balance(&cash_flow, &unrealized_pnl).unwrap();
        let withdrawal_balance =
            Balance::compute_withdrawal_balance(&cash_flow, &unrealized_pnl).unwrap();
        assert_ne!(balance, withdrawal_balance);

        let expected_accounting = Accounting {
            cash_flow: cash_flow.clone(),
            balance,
            withdrawal_balance,
        };
        let actual_accounting = Accounting::compute(&cash_flow, &unrealized_pnl).unwrap();
        assert_eq!(actual_accounting, expected_accounting);
    }
}
