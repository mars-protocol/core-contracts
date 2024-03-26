use std::{
    cmp::{max, min},
    str::FromStr,
};

use cosmwasm_std::{Decimal, Deps, Order, Uint128};
use mars_types::{
    adapters::oracle::Oracle,
    math::SignedDecimal,
    oracle::ActionKind,
    params::PerpParams,
    perps::{Accounting, DenomState, Funding, PnlValues, Position},
    signed_uint::SignedUint,
};

use crate::{
    accounting::AccountingExt,
    error::{ContractError, ContractResult},
    pricing::opening_execution_price,
    state::{CONFIG, DENOM_STATES, TOTAL_CASH_FLOW},
};

pub const SECONDS_IN_DAY: u64 = 86400;

/// Total unrealized PnL of a denom is the sum of unrealized PnL of all open positions (without market impact).
///
/// PnL for a single position is computed as:
/// pnl = size * (exit_exec_price - entry_exec_price)
///
/// Accumulated funding for a position is computed as:
/// accumulated_funding = size * (current_accrued_funding_per_unit - entry_accrued_funding_per_unit) * usdc_price
/// where:
/// current_accrued_funding_per_unit - accrued_funding_per_unit in USDC at the time of position closing
/// entry_accrued_funding_per_unit - accrued_funding_per_unit in USDC at the time of position opening
pub trait DenomStateExt {
    /// Returns the time elapsed in days since last update
    fn time_elapsed_in_days(&self, current_time: u64) -> Decimal;

    /// Returns the skew
    fn skew(&self) -> ContractResult<SignedUint>;

    /// Total size of all outstanding positions
    fn total_size(&self) -> ContractResult<Uint128>;

    /// Returns current funding rate velocity.
    /// Should be used _before_ modifying the market skew.
    fn current_funding_rate_velocity(&self) -> ContractResult<SignedDecimal>;

    /// Returns current funding rate.
    /// Should be used _before_ modifying the market skew.
    fn current_funding_rate(&self, current_time: u64) -> ContractResult<SignedDecimal>;

    /// The USDC-denominated funding entrance u(t) calculated _before_ modifying the market skew.
    ///
    /// u(t) = denom_price(t) / usdc_price(t) * (r(t-1) + r(t)) / 2 * (t - t-1) / seconds_in_day
    fn current_funding_entrance_per_unit_in_base_denom(
        &self,
        current_time: u64,
        denom_price: Decimal,
        base_denom_price: Decimal,
    ) -> ContractResult<SignedUint>;

    /// The USDC-denominated cumulative funding calculated _before_ modifying the market skew.
    ///
    /// F(t) = F(t-1) - u(t)
    fn current_funding_accrued_per_unit_in_base_denom(
        &self,
        current_time: u64,
        denom_price: Decimal,
        base_denom_price: Decimal,
    ) -> ContractResult<SignedUint>;

    /// Compute the current funding
    fn current_funding(
        &self,
        current_time: u64,
        denom_price: Decimal,
        base_denom_price: Decimal,
    ) -> ContractResult<Funding>;

    /// Validate the position size against the open interest limits
    fn validate_open_interest(
        &self,
        size: SignedUint,
        denom_price: Decimal,
        param: &PerpParams,
    ) -> ContractResult<()>;

    /// Increase open interest accumulators (new position is opened)
    fn increase_open_interest(&mut self, size: SignedUint) -> ContractResult<()>;

    /// Decrease open interest accumulators (a position is closed)
    fn decrease_open_interest(&mut self, size: SignedUint) -> ContractResult<()>;

    /// Update the accumulators when a new position is opened
    fn open_position(
        &mut self,
        current_time: u64,
        size: SignedUint,
        denom_price: Decimal,
        base_denom_price: Decimal,
    ) -> ContractResult<()>;

    /// Update the accumulators when a position is closed
    fn close_position(
        &mut self,
        current_time: u64,
        denom_price: Decimal,
        base_denom_price: Decimal,
        position: &Position,
    ) -> ContractResult<()>;

    /// Update the accumulators when a position is modified
    fn modify_position(
        &mut self,
        current_time: u64,
        denom_price: Decimal,
        base_denom_price: Decimal,
        position: &Position,
        new_size: SignedUint,
    ) -> ContractResult<()>;

    /// Compute the price PnL of all open positions
    fn compute_price_pnl(&self, exit_price: Decimal) -> ContractResult<SignedUint>;

    /// Compute the closing fees of all open positions
    fn compute_closing_fee(
        &self,
        closing_fee_rate: Decimal,
        exit_price: Decimal,
    ) -> ContractResult<SignedUint>;

    /// Compute the accrued funding of all open positions based on current funding.
    /// Returns the accrued funding and the updated funding.
    fn compute_accrued_funding(
        &self,
        current_time: u64,
        denom_price: Decimal,
        base_denom_price: Decimal,
    ) -> ContractResult<(SignedUint, Funding)>;

    /// Compute the total PnL of all open positions based on current funding.
    /// Returns the total PnL and the updated funding.
    fn compute_pnl(
        &self,
        current_time: u64,
        denom_price: Decimal,
        base_denom_price: Decimal,
        closing_fee_rate: Decimal,
    ) -> ContractResult<(PnlValues, Funding)>;

    /// Compute the accounting data for a denom
    fn compute_accounting_data(
        &self,
        current_time: u64,
        denom_price: Decimal,
        base_denom_price: Decimal,
        closing_fee_rate: Decimal,
    ) -> ContractResult<Accounting>;
}

impl DenomStateExt for DenomState {
    fn time_elapsed_in_days(&self, current_time: u64) -> Decimal {
        let time_diff = current_time - self.last_updated;
        Decimal::from_ratio(time_diff, SECONDS_IN_DAY)
    }

    fn skew(&self) -> ContractResult<SignedUint> {
        let skew = SignedUint::from(self.long_oi).checked_sub(self.short_oi.into())?;
        Ok(skew)
    }

    fn total_size(&self) -> ContractResult<Uint128> {
        Ok(self.long_oi.checked_add(self.short_oi)?)
    }

    fn current_funding_rate_velocity(&self) -> ContractResult<SignedDecimal> {
        // avoid a panic due to div by zero
        if self.funding.skew_scale.is_zero() {
            return Ok(SignedDecimal::zero());
        }

        // ensures the proportional skew is between -1 and 1
        let p_skew =
            SignedDecimal::checked_from_ratio(self.skew()?, self.funding.skew_scale.into())?;
        let p_skew_bounded =
            min(max(SignedDecimal::from_str("-1").unwrap(), p_skew), SignedDecimal::one());

        let funding_rate_velocity =
            p_skew_bounded.checked_mul(self.funding.max_funding_velocity.into())?;
        Ok(funding_rate_velocity)
    }

    fn current_funding_rate(&self, current_time: u64) -> ContractResult<SignedDecimal> {
        let current_funding_rate = self.funding.last_funding_rate.checked_add(
            self.current_funding_rate_velocity()?
                .checked_mul(self.time_elapsed_in_days(current_time).into())?,
        )?;
        Ok(current_funding_rate)
    }

    fn current_funding_entrance_per_unit_in_base_denom(
        &self,
        current_time: u64,
        denom_price: Decimal,
        base_denom_price: Decimal,
    ) -> ContractResult<SignedUint> {
        let price = denom_price.checked_div(base_denom_price)?;
        let curr_funding_rate = self.current_funding_rate(current_time)?;
        let avg_funding_rate = self
            .funding
            .last_funding_rate
            .checked_add(curr_funding_rate)?
            .checked_div(Decimal::from_atomics(2u128, 0)?.into())?;
        let res = avg_funding_rate
            .checked_mul(self.time_elapsed_in_days(current_time).into())?
            .checked_mul(price.into())?;
        Ok(res.to_signed_uint_floor())
    }

    fn current_funding_accrued_per_unit_in_base_denom(
        &self,
        current_time: u64,
        denom_price: Decimal,
        base_denom_price: Decimal,
    ) -> ContractResult<SignedUint> {
        let curr_funding_entrance_per_unit = self.current_funding_entrance_per_unit_in_base_denom(
            current_time,
            denom_price,
            base_denom_price,
        )?;
        Ok(self
            .funding
            .last_funding_accrued_per_unit_in_base_denom
            .checked_sub(curr_funding_entrance_per_unit)?)
    }

    fn current_funding(
        &self,
        current_time: u64,
        denom_price: Decimal,
        base_denom_price: Decimal,
    ) -> ContractResult<Funding> {
        if self.last_updated == current_time {
            return Ok(self.funding.clone());
        };

        // update only rate and index here, the rest is copied from the previous funding
        Ok(Funding {
            last_funding_rate: self.current_funding_rate(current_time)?,
            last_funding_accrued_per_unit_in_base_denom: self
                .current_funding_accrued_per_unit_in_base_denom(
                    current_time,
                    denom_price,
                    base_denom_price,
                )?,
            ..self.funding
        })
    }

    fn validate_open_interest(
        &self,
        size: SignedUint,
        denom_price: Decimal,
        param: &PerpParams,
    ) -> ContractResult<()> {
        let net_oi = if size.is_positive() {
            let long_oi = self.long_oi.checked_add(size.abs)?;
            let long_oi_value = long_oi.checked_mul_floor(denom_price)?;
            if long_oi_value > param.max_long_oi_value {
                return Err(ContractError::LongOpenInterestReached {
                    max: param.max_long_oi_value,
                    found: long_oi_value,
                });
            }

            long_oi.abs_diff(self.short_oi)
        } else {
            let short_oi = self.short_oi.checked_add(size.abs)?;
            let short_oi_value = short_oi.checked_mul_floor(denom_price)?;
            if short_oi_value > param.max_short_oi_value {
                return Err(ContractError::ShortOpenInterestReached {
                    max: param.max_short_oi_value,
                    found: short_oi_value,
                });
            }

            self.long_oi.abs_diff(short_oi)
        };

        let net_oi_value = net_oi.checked_mul_floor(denom_price)?;
        if net_oi_value > param.max_net_oi_value {
            return Err(ContractError::NetOpenInterestReached {
                max: param.max_net_oi_value,
                found: net_oi_value,
            });
        }

        Ok(())
    }

    fn increase_open_interest(&mut self, size: SignedUint) -> ContractResult<()> {
        if size.is_positive() {
            self.long_oi = self.long_oi.checked_add(size.abs)?;
        } else {
            self.short_oi = self.short_oi.checked_add(size.abs)?;
        }
        Ok(())
    }

    fn decrease_open_interest(&mut self, size: SignedUint) -> ContractResult<()> {
        if size.is_positive() {
            self.long_oi = self.long_oi.checked_sub(size.abs)?;
        } else {
            self.short_oi = self.short_oi.checked_sub(size.abs)?;
        }
        Ok(())
    }

    fn open_position(
        &mut self,
        current_time: u64,
        size: SignedUint,
        denom_price: Decimal,
        base_denom_price: Decimal,
    ) -> ContractResult<()> {
        // calculate the current funding with size up to the current time
        self.funding = self.current_funding(current_time, denom_price, base_denom_price)?;

        // increase the accumulators with new data
        increase_accumulators(self, size, denom_price)?;

        // update the open interest
        self.increase_open_interest(size)?;

        self.last_updated = current_time;

        Ok(())
    }

    fn close_position(
        &mut self,
        current_time: u64,
        denom_price: Decimal,
        base_denom_price: Decimal,
        position: &Position,
    ) -> ContractResult<()> {
        // calculate the current funding with size up to the current time
        self.funding = self.current_funding(current_time, denom_price, base_denom_price)?;

        // decrease the accumulators with old data
        decrease_accumulators(self, position)?;

        // update the open interest
        self.decrease_open_interest(position.size)?;

        self.last_updated = current_time;

        Ok(())
    }

    fn modify_position(
        &mut self,
        current_time: u64,
        denom_price: Decimal,
        base_denom_price: Decimal,
        position: &Position,
        new_size: SignedUint,
    ) -> ContractResult<()> {
        // calculate the current funding with size up to the current time
        self.funding = self.current_funding(current_time, denom_price, base_denom_price)?;

        // first we have to decrease the accumulators with old data
        decrease_accumulators(self, position)?;

        // then we increase the accumulators with new data
        increase_accumulators(self, new_size, denom_price)?;

        // update the open interest
        if new_size.abs > position.size.abs {
            let q_change = new_size.checked_sub(position.size)?;
            self.increase_open_interest(q_change)?;
        } else {
            let q_change = position.size.checked_sub(new_size)?;
            self.decrease_open_interest(q_change)?;
        }

        self.last_updated = current_time;

        Ok(())
    }

    fn compute_price_pnl(&self, exit_price: Decimal) -> ContractResult<SignedUint> {
        let skew = self.skew()?;

        // Original formula from the doc:
        // skew * exit_price - total_entry_cost + exit_price / skew_scale * (skew^2 - total_squared_positions / 2)
        //
        // If we use as it is we can accumulate rounding errors in:
        // - 'total_squared_positions / 2' will end up as a integer,
        // - 'exit_price / skew_scale * (skew^2 - total_squared_positions / 2)' will end up as a integer.
        //
        // Let's rewrite it to reduce number of rounding errors:
        // skew * exit_price - total_entry_cost + exit_price * ((2 * skew^2 - total_squared_positions) / (2 * skew_scale))
        // Introduce following variables:
        // val_1 = skew * exit_price - total_entry_cost
        // val_2 = 2 * skew^2 - total_squared_positions
        // val_3 = val_2 / (2 * skew_scale)
        // val_4 = exit_price * val_3
        // finally:
        // val_1 + val_4
        let val_1 =
            skew.checked_mul_floor(exit_price.into())?.checked_sub(self.total_entry_cost)?;
        let skew_squared = skew.checked_mul(skew)?;
        let val_2 = skew_squared
            .checked_mul(Uint128::new(2u128).into())?
            .checked_sub(self.total_squared_positions)?;
        let two_times_skew_scale = Uint128::new(2u128).checked_mul(self.funding.skew_scale)?;
        let val_3 = SignedDecimal::checked_from_ratio(val_2, two_times_skew_scale.into())?;
        // rounding errors here after rewriting the formula
        let val_4: SignedUint = val_3.checked_mul(exit_price.into())?.to_signed_uint_floor();
        let price_pnl = val_1.checked_add(val_4)?;

        Ok(price_pnl)
    }

    fn compute_closing_fee(
        &self,
        closing_fee_rate: Decimal,
        exit_price: Decimal,
    ) -> ContractResult<SignedUint> {
        let skew = self.skew()?;
        let total_size = self.total_size()?;

        // Original formula from the doc:
        // closing_fee_rate * exit_price * (1 / skew_scale * (- skew * total_size + total_abs_multiplied_positions / 2) - total_size)
        //
        // If we use as it is we can accumulate rounding errors in:
        // - 'total_abs_multiplied_positions / 2' will end up as a integer,
        // - '1 / skew_scale * (- skew * total_size + total_abs_multiplied_positions / 2)' will end up as a integer.
        //
        // Let's rewrite it to reduce number of rounding errors:
        // closing_fee_rate * exit_price * ((total_abs_multiplied_positions - 2 * skew * total_size) / (2 * skew_scale) - total_size)
        // Introduce following variables:
        // val_1 = closing_fee_rate * exit_price
        // val_2 = 2 * skew * total_size
        // val_3 = total_abs_multiplied_positions - val_2
        // val_4 = val_3 / (2 * skew_scale)
        // finally:
        // val_1 * (val_4 - total_size)
        let val_1 = closing_fee_rate.checked_mul(exit_price)?;
        let val_2 = skew.checked_mul(total_size.checked_mul(Uint128::new(2u128))?.into())?;
        let val_3 = self.total_abs_multiplied_positions.checked_sub(val_2)?;
        let two_times_skew_scale = Uint128::new(2u128).checked_mul(self.funding.skew_scale)?;
        // rounding errors here after rewriting the formula
        let val_4: SignedUint =
            SignedDecimal::checked_from_ratio(val_3, two_times_skew_scale.into())?
                .to_signed_uint_floor();
        let closing_fee = val_4.checked_sub(total_size.into())?.checked_mul_floor(val_1.into())?;

        Ok(closing_fee)
    }

    fn compute_accrued_funding(
        &self,
        current_time: u64,
        denom_price: Decimal,
        base_denom_price: Decimal,
    ) -> ContractResult<(SignedUint, Funding)> {
        let current_funding = self.current_funding(current_time, denom_price, base_denom_price)?;

        let accrued_funding = self
            .skew()?
            .checked_mul(current_funding.last_funding_accrued_per_unit_in_base_denom)?
            .checked_sub(self.total_entry_funding)?
            .checked_mul_floor(base_denom_price.into())?;

        Ok((accrued_funding, current_funding))
    }

    fn compute_pnl(
        &self,
        current_time: u64,
        denom_price: Decimal,
        base_denom_price: Decimal,
        closing_fee_rate: Decimal,
    ) -> ContractResult<(PnlValues, Funding)> {
        let price_pnl = self.compute_price_pnl(denom_price)?;
        let closing_fee = self.compute_closing_fee(closing_fee_rate, denom_price)?;
        let (accrued_funding, curr_funding) =
            self.compute_accrued_funding(current_time, denom_price, base_denom_price)?;
        let pnl_values = PnlValues {
            price_pnl,
            closing_fee,
            accrued_funding,
            pnl: price_pnl.checked_add(accrued_funding)?.checked_add(closing_fee)?,
        };
        Ok((pnl_values, curr_funding))
    }

    fn compute_accounting_data(
        &self,
        current_time: u64,
        denom_price: Decimal,
        base_denom_price: Decimal,
        closing_fee_rate: Decimal,
    ) -> ContractResult<Accounting> {
        let (unrealized_pnl, _) =
            self.compute_pnl(current_time, denom_price, base_denom_price, closing_fee_rate)?;
        let acc = Accounting::compute(&self.cash_flow, &unrealized_pnl, base_denom_price)?;
        Ok(acc)
    }
}

fn decrease_accumulators(denom_state: &mut DenomState, position: &Position) -> ContractResult<()> {
    // decrease the total_entry_cost accumulator
    let value = position.size.checked_mul_floor(position.entry_exec_price.into())?;
    denom_state.total_entry_cost = denom_state.total_entry_cost.checked_sub(value)?;

    // decrease the total_entry_funding accumulator accordingly
    denom_state.total_entry_funding = denom_state.total_entry_funding.checked_sub(
        position.size.checked_mul(position.entry_accrued_funding_per_unit_in_base_denom)?,
    )?;

    // decrease the total_squared_positions accumulator
    denom_state.total_squared_positions = denom_state
        .total_squared_positions
        .checked_sub(position.size.abs.checked_pow(2)?.into())?;

    // decrease the total_abs_multiplied_positions accumulator
    denom_state.total_abs_multiplied_positions = denom_state
        .total_abs_multiplied_positions
        .checked_sub(position.size.checked_mul(position.size.abs.into())?)?;

    Ok(())
}

fn increase_accumulators(
    denom_state: &mut DenomState,
    size: SignedUint,
    denom_price: Decimal,
) -> ContractResult<()> {
    // increase the total_entry_cost accumulator
    let entry_exec_price = opening_execution_price(
        denom_state.skew()?,
        denom_state.funding.skew_scale,
        size,
        denom_price,
    )?;
    let value = size.checked_mul_floor(entry_exec_price.into())?;
    denom_state.total_entry_cost = denom_state.total_entry_cost.checked_add(value)?;

    // increase the total_entry_funding accumulator with recalculated funding
    denom_state.total_entry_funding = denom_state.total_entry_funding.checked_add(
        size.checked_mul(denom_state.funding.last_funding_accrued_per_unit_in_base_denom)?,
    )?;

    // increase the total_squared_positions accumulator
    denom_state.total_squared_positions =
        denom_state.total_squared_positions.checked_add(size.abs.checked_pow(2)?.into())?;

    // increase the total_abs_multiplied_positions accumulator
    denom_state.total_abs_multiplied_positions = denom_state
        .total_abs_multiplied_positions
        .checked_add(size.checked_mul(size.abs.into())?)?;

    Ok(())
}

/// Loop through denoms and compute the total PnL.
/// This PnL is denominated in uusd (1 USD = 1e6 uusd -> configured in Oracle).
pub fn compute_total_pnl(
    deps: &Deps,
    oracle: &Oracle,
    current_time: u64,
    action: ActionKind,
) -> ContractResult<PnlValues> {
    let config = CONFIG.load(deps.storage)?;

    let base_denom_price =
        oracle.query_price(&deps.querier, &config.base_denom, action.clone())?.price;
    let total_pnl = DENOM_STATES.range(deps.storage, None, None, Order::Ascending).try_fold(
        PnlValues::default(),
        |acc, item| -> ContractResult<_> {
            let (denom, ds) = item?;
            let perp_params = config.params.query_perp_params(&deps.querier, &denom)?;

            let denom_price = oracle.query_price(&deps.querier, &denom, action.clone())?.price;
            let (pnl_values, _) = ds.compute_pnl(
                current_time,
                denom_price,
                base_denom_price,
                perp_params.closing_fee_rate,
            )?;

            Ok(PnlValues {
                price_pnl: acc.price_pnl.checked_add(pnl_values.price_pnl)?,
                closing_fee: acc.closing_fee.checked_add(pnl_values.closing_fee)?,
                accrued_funding: acc.accrued_funding.checked_add(pnl_values.accrued_funding)?,
                pnl: acc.pnl.checked_add(pnl_values.pnl)?,
            })
        },
    )?;

    Ok(total_pnl)
}

/// Compute the total accounting data based on the total unrealized PnL and cash flow accumulator.
pub fn compute_total_accounting_data(
    deps: &Deps,
    oracle: &Oracle,
    current_time: u64,
    base_denom_price: Decimal,
    action: ActionKind,
) -> ContractResult<Accounting> {
    let gcf = TOTAL_CASH_FLOW.load(deps.storage)?;
    let unrealized_pnl = compute_total_pnl(deps, oracle, current_time, action)?;
    let acc = Accounting::compute(&gcf, &unrealized_pnl, base_denom_price)?;
    Ok(acc)
}

// ----------------------------------- Tests -----------------------------------

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use mars_types::perps::{CashFlow, PnlAmounts};
    use test_case::test_case;

    use super::*;

    #[test]
    fn time_elapsed_in_days() {
        let ds = DenomState {
            last_updated: 120,
            ..Default::default()
        };

        let res = ds.time_elapsed_in_days((2 * SECONDS_IN_DAY) + ds.last_updated);
        assert_eq!(res, Decimal::from_str("2").unwrap());
    }

    #[test]
    fn skew() {
        let ds = DenomState {
            long_oi: Uint128::new(100u128),
            short_oi: Uint128::new(20u128),
            ..Default::default()
        };
        assert_eq!(ds.skew().unwrap(), SignedUint::from_str("80").unwrap());

        let ds = DenomState {
            long_oi: Uint128::new(100u128),
            short_oi: Uint128::new(256u128),
            ..Default::default()
        };
        assert_eq!(ds.skew().unwrap(), SignedUint::from_str("-156").unwrap());
    }

    #[test]
    fn total_size() {
        let ds = DenomState {
            long_oi: Uint128::new(100u128),
            short_oi: Uint128::new(20u128),
            ..Default::default()
        };
        assert_eq!(ds.total_size().unwrap(), Uint128::new(120u128));
    }

    #[test]
    fn current_funding_rate_velocity() {
        let ds = DenomState {
            long_oi: Uint128::new(300u128),
            short_oi: Uint128::new(150u128),
            funding: Funding {
                max_funding_velocity: Decimal::from_str("3").unwrap(),
                skew_scale: Uint128::new(1000000u128),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(
            ds.current_funding_rate_velocity().unwrap(),
            SignedDecimal::from_str("0.00045").unwrap()
        );

        // upper bound
        let ds = DenomState {
            long_oi: Uint128::new(3000000u128),
            short_oi: Uint128::new(150u128),
            funding: Funding {
                max_funding_velocity: Decimal::from_str("3").unwrap(),
                skew_scale: Uint128::new(1000000u128),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(
            ds.current_funding_rate_velocity().unwrap(),
            SignedDecimal::from_str("3").unwrap()
        );

        // lower bound
        let ds = DenomState {
            long_oi: Uint128::new(300u128),
            short_oi: Uint128::new(1500000u128),
            funding: Funding {
                max_funding_velocity: Decimal::from_str("3").unwrap(),
                skew_scale: Uint128::new(1000000u128),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(
            ds.current_funding_rate_velocity().unwrap(),
            SignedDecimal::from_str("-3").unwrap()
        );
    }

    #[test]
    fn current_funding_rate() {
        let ds = denom_state();
        assert_eq!(
            ds.current_funding_rate(43400).unwrap(),
            SignedDecimal::from_str("-0.043").unwrap()
        );
    }

    #[test]
    fn current_funding_entrance_per_unit_in_base_denom() {
        let ds = denom_state();
        assert_eq!(
            ds.current_funding_entrance_per_unit_in_base_denom(
                43400,
                Decimal::from_str("3600").unwrap(),
                Decimal::from_str("0.9").unwrap()
            )
            .unwrap(),
            SignedUint::from_str("-68").unwrap()
        );
    }

    #[test]
    fn current_funding_accrued_per_unit_in_base_denom() {
        let ds = denom_state();
        assert_eq!(
            ds.current_funding_accrued_per_unit_in_base_denom(
                43400,
                Decimal::from_str("3600").unwrap(),
                Decimal::from_str("0.9").unwrap()
            )
            .unwrap(),
            SignedUint::from_str("56").unwrap()
        );
    }

    #[test]
    fn current_funding() {
        let ds = denom_state();
        assert_eq!(
            ds.current_funding(
                ds.last_updated,
                Decimal::from_str("4200").unwrap(),
                Decimal::from_str("0.8").unwrap()
            )
            .unwrap(),
            ds.funding
        );

        assert_eq!(
            ds.current_funding(
                43400,
                Decimal::from_str("4200").unwrap(),
                Decimal::from_str("0.8").unwrap()
            )
            .unwrap(),
            Funding {
                last_funding_rate: SignedDecimal::from_str("-0.043").unwrap(),
                last_funding_accrued_per_unit_in_base_denom: SignedUint::from_str("78").unwrap(),
                ..ds.funding
            }
        );
    }

    #[test]
    fn increase_open_interest() {
        let mut ds = DenomState {
            long_oi: Uint128::new(100u128),
            short_oi: Uint128::new(20u128),
            ..Default::default()
        };

        ds.increase_open_interest(SignedUint::from_str("70").unwrap()).unwrap();
        assert_eq!(ds.long_oi, Uint128::new(170u128));
        assert_eq!(ds.short_oi, Uint128::new(20u128));

        ds.increase_open_interest(SignedUint::from_str("-70").unwrap()).unwrap();
        assert_eq!(ds.long_oi, Uint128::new(170u128));
        assert_eq!(ds.short_oi, Uint128::new(90u128));
    }

    #[test]
    fn decrease_open_interest() {
        let mut ds = DenomState {
            long_oi: Uint128::new(100u128),
            short_oi: Uint128::new(200u128),
            ..Default::default()
        };

        ds.decrease_open_interest(SignedUint::from_str("70").unwrap()).unwrap();
        assert_eq!(ds.long_oi, Uint128::new(30u128));
        assert_eq!(ds.short_oi, Uint128::new(200u128));

        ds.decrease_open_interest(SignedUint::from_str("-70").unwrap()).unwrap();
        assert_eq!(ds.long_oi, Uint128::new(30u128));
        assert_eq!(ds.short_oi, Uint128::new(130u128));
    }

    #[test]
    fn open_position() {
        let mut ds = denom_state();
        let ds_before_modification = ds.clone();

        ds.open_position(
            43400,
            SignedUint::from_str("-100").unwrap(),
            Decimal::from_str("4200").unwrap(),
            Decimal::from_str("0.8").unwrap(),
        )
        .unwrap();

        assert_eq!(
            ds,
            DenomState {
                funding: Funding {
                    last_funding_rate: SignedDecimal::from_str("-0.043").unwrap(),
                    last_funding_accrued_per_unit_in_base_denom: SignedUint::from_str("78")
                        .unwrap(),
                    ..ds_before_modification.funding
                },
                total_entry_cost: SignedUint::from_str("-415064").unwrap(),
                total_entry_funding: SignedUint::from_str("-7532").unwrap(),
                total_squared_positions: SignedUint::from_str("24400").unwrap(),
                total_abs_multiplied_positions: SignedUint::from_str("-10225").unwrap(),
                short_oi: ds_before_modification.short_oi + Uint128::new(100u128),
                last_updated: 43400,
                ..ds_before_modification
            }
        );
    }

    #[test]
    fn close_position() {
        let mut ds = denom_state();
        let ds_before_modification = ds.clone();

        ds.close_position(
            43400,
            Decimal::from_str("4200").unwrap(),
            Decimal::from_str("0.8").unwrap(),
            &Position {
                size: SignedUint::from_str("-100").unwrap(),
                entry_price: Decimal::from_str("4200").unwrap(),
                entry_exec_price: Decimal::from_str("4149.39").unwrap(),
                entry_accrued_funding_per_unit_in_base_denom: SignedUint::from_str("78").unwrap(),
                initial_skew: SignedUint::from_str("-12000").unwrap(),
                realized_pnl: PnlAmounts::default(),
            },
        )
        .unwrap();

        assert_eq!(
            ds,
            DenomState {
                funding: Funding {
                    last_funding_rate: SignedDecimal::from_str("-0.043").unwrap(),
                    last_funding_accrued_per_unit_in_base_denom: SignedUint::from_str("78")
                        .unwrap(),
                    ..ds_before_modification.funding
                },
                total_entry_cost: SignedUint::from_str("414814").unwrap(),
                total_entry_funding: SignedUint::from_str("8068").unwrap(),
                total_squared_positions: SignedUint::from_str("4400").unwrap(),
                total_abs_multiplied_positions: SignedUint::from_str("9775").unwrap(),
                short_oi: ds_before_modification.short_oi - Uint128::new(100u128),
                last_updated: 43400,
                ..ds_before_modification
            }
        );
    }

    #[test_case(
        SignedUint::from_str("400").unwrap(),
        SignedUint::from_str("650").unwrap();
        "long position - increase"
    )]
    #[test_case(
        SignedUint::from_str("400").unwrap(),
        SignedUint::from_str("180").unwrap();
        "long position - decrease"
    )]
    #[test_case(
        SignedUint::from_str("400").unwrap(),
        SignedUint::from_str("400").unwrap();
        "long position - decrease to zero"
    )]
    #[test_case(
        SignedUint::from_str("-400").unwrap(),
        SignedUint::from_str("-650").unwrap();
        "short position - increase"
    )]
    #[test_case(
        SignedUint::from_str("-400").unwrap(),
        SignedUint::from_str("-180").unwrap();
        "short position - decrease"
    )]
    #[test_case(
        SignedUint::from_str("-400").unwrap(),
        SignedUint::from_str("-400").unwrap();
        "short position - decrease to zero"
    )]
    fn modify_position(size: SignedUint, new_size: SignedUint) {
        let ds_before_modification = denom_state();

        let mut ds_1 = ds_before_modification.clone();

        let skew = ds_1.skew().unwrap();
        let entry_price = Decimal::from_str("4200").unwrap();
        let entry_exec_price =
            opening_execution_price(skew, ds_1.funding.skew_scale, size, entry_price).unwrap();
        let mut pos = Position {
            size,
            entry_price,
            entry_exec_price,
            initial_skew: skew,
            ..Default::default()
        };
        ds_1.open_position(43400, pos.size, pos.entry_price, Decimal::from_str("0.8").unwrap())
            .unwrap();
        pos.entry_accrued_funding_per_unit_in_base_denom =
            ds_1.funding.last_funding_accrued_per_unit_in_base_denom;

        // modify with new denom price, base denom price and new decreased size
        let new_denom_price = Decimal::from_str("4400").unwrap();
        let new_base_denom_price = Decimal::from_str("0.9").unwrap();
        let new_skew = ds_1.skew().unwrap();
        ds_1.modify_position(43600, new_denom_price, new_base_denom_price, &pos, new_size).unwrap();

        // update the position with new data
        pos.size = new_size;
        pos.entry_price = new_denom_price;
        pos.entry_exec_price =
            opening_execution_price(new_skew, ds_1.funding.skew_scale, new_size, new_denom_price)
                .unwrap();
        pos.entry_accrued_funding_per_unit_in_base_denom =
            ds_1.funding.last_funding_accrued_per_unit_in_base_denom;
        pos.initial_skew = new_skew;

        // after closing the position, the accumulators should be the same as before the modification
        let mut ds_1_closed = ds_1.clone();
        ds_1_closed
            .close_position(
                43800,
                Decimal::from_str("3000").unwrap(),
                Decimal::from_str("0.6").unwrap(),
                &pos,
            )
            .unwrap();
        assert_eq!(ds_1_closed.skew().unwrap(), ds_before_modification.skew().unwrap());
        assert_eq!(ds_1_closed.total_entry_cost, ds_before_modification.total_entry_cost);
        assert_eq!(ds_1_closed.total_entry_funding, ds_before_modification.total_entry_funding);
        assert_eq!(
            ds_1_closed.total_squared_positions,
            ds_before_modification.total_squared_positions
        );
        assert_eq!(
            ds_1_closed.total_abs_multiplied_positions,
            ds_before_modification.total_abs_multiplied_positions
        );
    }

    #[test]
    fn compute_price_pnl() {
        let ds = denom_state();
        assert_eq!(
            ds.compute_price_pnl(Decimal::from_str("4200").unwrap()).unwrap(),
            SignedUint::from_str("-49795106").unwrap()
        );
    }

    #[test]
    fn compute_closing_fee() {
        let ds = denom_state();
        assert_eq!(
            ds.compute_closing_fee(Decimal::percent(2), Decimal::from_str("4200").unwrap())
                .unwrap(),
            SignedUint::from_str("-1493940").unwrap()
        );
    }

    #[test]
    fn compute_accrued_funding() {
        let ds = denom_state();

        let (accrued_funding, funding) = ds
            .compute_accrued_funding(
                43400,
                Decimal::from_str("4200").unwrap(),
                Decimal::from_str("0.8").unwrap(),
            )
            .unwrap();

        assert_eq!(accrued_funding, SignedUint::from_str("-749015").unwrap());
        assert_eq!(
            funding,
            ds.current_funding(
                43400,
                Decimal::from_str("4200").unwrap(),
                Decimal::from_str("0.8").unwrap(),
            )
            .unwrap()
        )
    }

    #[test]
    fn compute_pnl() {
        let ds = denom_state();

        let (pnl_values, funding) = ds
            .compute_pnl(
                43400,
                Decimal::from_str("4200").unwrap(),
                Decimal::from_str("0.8").unwrap(),
                Decimal::percent(2),
            )
            .unwrap();

        assert_eq!(
            pnl_values,
            PnlValues {
                price_pnl: SignedUint::from_str("-49795106").unwrap(),
                closing_fee: SignedUint::from_str("-1493940").unwrap(),
                accrued_funding: SignedUint::from_str("-749015").unwrap(),
                pnl: SignedUint::from_str("-52038061").unwrap()
            }
        );
        assert_eq!(
            funding,
            ds.current_funding(
                43400,
                Decimal::from_str("4200").unwrap(),
                Decimal::from_str("0.8").unwrap(),
            )
            .unwrap()
        )
    }

    fn denom_state() -> DenomState {
        DenomState {
            enabled: true,
            long_oi: Uint128::new(3000u128),
            short_oi: Uint128::new(15000u128),
            funding: Funding {
                max_funding_velocity: Decimal::from_str("3").unwrap(),
                skew_scale: Uint128::new(1000000u128),
                last_funding_rate: SignedDecimal::from_str("-0.025").unwrap(),
                last_funding_accrued_per_unit_in_base_denom: SignedUint::from_str("-12").unwrap(),
            },
            last_updated: 200,
            total_entry_cost: SignedUint::from_str("-125").unwrap(),
            total_entry_funding: SignedUint::from_str("268").unwrap(),
            total_squared_positions: SignedUint::from_str("14400").unwrap(),
            total_abs_multiplied_positions: SignedUint::from_str("-225").unwrap(),
            cash_flow: CashFlow::default(),
        }
    }
}
