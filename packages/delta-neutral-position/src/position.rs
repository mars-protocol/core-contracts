use std::convert::TryFrom;

use cosmwasm_std::{Decimal, Uint128};
use cosmwasm_std::{Int128, SignedDecimal};
use mars_utils::helpers::uint128_to_int128;

use crate::error::{ContractError, ContractResult};
use crate::helpers::{prorate_i128_by_amount, weighted_avg};
use crate::pnl::compute_realized_pnl;
use crate::types::Position;

impl Position {
    #[allow(clippy::too_many_arguments)]
    pub fn increase(
        &mut self,
        amount: Uint128,
        spot_price: Decimal,
        perp_price: Decimal,
        _perp_trading_fee_amount: Int128,
        now: u64,
        funding_delta: Int128,
        borrow_delta: Int128,
    ) -> ContractResult<Self> {
        if amount.is_zero() {
            return Err(ContractError::InvalidAmount {
                reason: "Amount must be greater than zero".to_string(),
            });
        }

        // Apply funding and borrow deltas
        self.net_funding_balance = self.net_funding_balance.checked_add(funding_delta)?;

        self.net_borrow_balance = self.net_borrow_balance.checked_add(borrow_delta)?;

        let new_size = self.spot_amount.checked_add(amount)?;

        // VWAP recalculations
        self.avg_spot_price =
            weighted_avg(self.avg_spot_price, self.spot_amount, spot_price, amount)?;

        self.avg_perp_price =
            weighted_avg(self.avg_perp_price, self.perp_amount, perp_price, amount)?;

        // Entry value update: (spot - perp) * amount
        let entry_delta = (SignedDecimal::try_from(spot_price)?
            .checked_sub(SignedDecimal::try_from(perp_price)?))?
        .checked_mul(SignedDecimal::from_atomics(uint128_to_int128(amount)?, 0)?)?
        .to_int_floor();

        self.entry_value = self.entry_value.checked_add(entry_delta)?;

        // Size updates
        self.spot_amount = new_size;
        self.perp_amount = new_size;

        self.last_updated = now;

        // TODO add fees

        Ok(self.clone())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn decrease(
        &mut self,
        amount: Uint128,
        spot_exit_price: Decimal,
        perp_exit_price: Decimal,
        perp_trading_fee_amount: Int128,
        now: u64,
        funding_delta: Int128,
        borrow_delta: Int128,
    ) -> ContractResult<Self> {
        if amount.is_zero() {
            return Err(ContractError::InvalidAmount {
                reason: "Amount must be greater than zero".to_string(),
            });
        }

        let total_size = self.spot_amount;

        if amount > total_size {
            return Err(ContractError::InvalidAmount {
                reason: "Cannot decrease more than current position size".to_string(),
            });
        }

        // Update accrued funding and borrow since our last update
        self.net_funding_balance = self.net_funding_balance.checked_add(funding_delta)?;
        self.net_borrow_balance = self.net_borrow_balance.checked_add(borrow_delta)?;

        // Calculate realized pnl
        // TODO check if we should be prorating funding and borrow both in this method
        // and here - need to confirm that is correct it looks wrong
        let realized_pnl = compute_realized_pnl(
            spot_exit_price,
            perp_exit_price,
            amount,
            self.entry_value,
            total_size,
            perp_trading_fee_amount,
            self.net_funding_balance,
            self.net_borrow_balance,
        )?;

        self.total_realized_pnl = self.total_realized_pnl.checked_add(realized_pnl)?;

        // Prorate entry value, funding, and borrow, to reduce
        let entry_value_slice = prorate_i128_by_amount(self.entry_value, amount, total_size)?;
        let realized_funding =
            prorate_i128_by_amount(self.net_funding_balance, amount, total_size)?;
        let realized_borrow = prorate_i128_by_amount(self.net_borrow_balance, amount, total_size)?;

        // Subtract from state
        self.entry_value = self.entry_value.checked_sub(entry_value_slice)?;

        self.net_funding_balance = self.net_funding_balance.checked_sub(realized_funding)?;
        self.net_borrow_balance = self.net_borrow_balance.checked_sub(realized_borrow)?;

        self.net_realized_funding = self.net_realized_funding.checked_add(realized_funding)?;
        self.net_realized_borrow = self.net_realized_borrow.checked_add(realized_borrow)?;

        self.spot_amount = self.spot_amount.checked_sub(amount)?;
        self.perp_amount = self.perp_amount.checked_sub(amount)?;

        // Reset fields if fully closed
        if self.spot_amount.is_zero() && self.perp_amount.is_zero() {
            self.avg_spot_price = Decimal::zero();
            self.avg_perp_price = Decimal::zero();
            self.entry_value = Int128::zero();
            self.net_funding_balance = Int128::zero();
            self.net_borrow_balance = Int128::zero();
            self.debt_principle = Uint128::zero();
        }

        self.last_updated = now;

        Ok(self.clone())
    }
}
