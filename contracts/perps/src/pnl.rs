use cosmwasm_std::{Coin, Decimal, Deps, Order};
use mars_types::{
    adapters::oracle::Oracle,
    math::SignedDecimal,
    oracle::ActionKind,
    perps::{DenomState, Funding, PnL, PnlValues, Position},
};

use crate::{error::ContractResult, state::DENOM_STATES};

pub const SECONDS_IN_DAY: u64 = 86400;

/// Compute the unrealized PnL of a position, given the current price
pub fn compute_pnl(
    funding: &Funding,
    position: &Position,
    current_price: Decimal,
    base_denom: impl Into<String>,
) -> ContractResult<PnL> {
    // cast the prices into SignedDecimal
    let entry_price: SignedDecimal = position.entry_price.into();
    let exit_price: SignedDecimal = current_price.into();

    // size * (exit_price - entry_price)
    let price_diff = exit_price.checked_sub(entry_price)?;
    let pnl = position.size.checked_mul(price_diff)?;

    // size * exit_price * (current_funding_index / pos_open_funding_index - 1)
    let idx = funding
        .index
        .checked_div(position.entry_funding_index)?
        .checked_sub(SignedDecimal::one())?;
    let accrued_funding = position.size.checked_mul(exit_price)?.checked_mul(idx)?;

    let realized_pnl = pnl.checked_sub(accrued_funding)?;

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

/// Total unrealized PnL of a denom is the sum of unrealized PnL of all open positions.
///
/// PnL for a single position is computed as:
/// pnl = size * (exit_price - entry_price)
///
/// PnL for all open positions is computed as:
/// size_1 * (exit_price - entry_price_1) + size_2 * (exit_price - entry_price_2) + ...
/// = size_1 * exit_price - size_1 * entry_price_1 + size_2 * exit_price - size_2 * entry_price_2 + ...
/// = exit_price * (size_1 + size_2 + ...) - (size_1 * entry_price_1 + size_2 * entry_price_2 + ...)
/// = exit_price * total_size - total_cost_base
///
/// To compute this, we keep two global "accumulators":
/// - total_size
/// - total_cost_base
/// When a user opens a new position of size, we do:
/// total_size += size, total_cost_base += size * entry_price
/// When a user closes a position of size, we do:
/// total_size -= size, total_cost_base -= size * entry_price
///
/// Realized PnL is computed when a position is closed with deducted funding from unrealized PnL.
///
/// Accumulated funding for a position is computed as:
/// accumulated_funding = size * exit_price * (close_fun_idx / popen_fun_idx - 1)
/// where:
/// close_fun_idx - funding index at the time of position closing
/// open_fun_idx - funding index at the time of position opening
///
/// Accumulated funding for all open positions is computed as:
/// size_1 * exit_price * (close_fun_idx / open_fun_idx_1 - 1) + size_2 * exit_price * (close_fun_idx / open_fun_idx_2 - 1) + ...
/// = size_1 * exit_price * close_fun_idx / open_fun_idx_1 - size_1 * exit_price + size_2 * exit_price * close_fun_idx / open_fun_idx_2 - size_2 * exit_price + ...
/// = exit_price * close_fun_idx * (size_1 / open_fun_idx_1 + size_2 / open_fun_idx_2 + ...) -  exit_price * (size_1 + size_2 + ...)
/// = exit_price * close_fun_idx * accumulated_size_weighted_by_index - exit_price * total_size
///
/// To compute this, we keep additional "accumulator": accumulated_size_weighted_by_index
/// When a user opens a new position of size, we do:
/// accumulated_size_weighted_by_index += size / open_fun_idx
/// When a user closes a position of size, we do:
/// accumulated_size_weighted_by_index -= size / open_fun_idx
pub trait DenomStateExt {
    /// Update the accumulators when a new position is opened
    fn open_position(
        &mut self,
        current_time: u64,
        size: SignedDecimal,
        entry_price: Decimal,
    ) -> ContractResult<()>;

    /// Update the accumulators when a position is closed
    fn close_position(&mut self, current_time: u64, position: &Position) -> ContractResult<()>;

    /// Compute the unrealized PnL of all open positions
    fn compute_unrealized_pnl(&self, exit_price: Decimal) -> ContractResult<SignedDecimal>;

    /// Compute the funding rate velocity
    fn compute_funding_rate_velocity(&self) -> ContractResult<SignedDecimal>;

    /// Compute the current funding: rate, index and accumulator
    fn compute_current_funding(&self, current_time: u64) -> ContractResult<Funding>;

    /// Compute the accrued funding of all open positions based on current funding.
    /// Returns the accrued funding and the updated funding.
    fn compute_accrued_funding(
        &self,
        current_time: u64,
        exit_price: Decimal,
    ) -> ContractResult<(SignedDecimal, Funding)>;

    /// Compute the total PnL of all open positions based on current funding.
    /// Returns the total PnL and the updated funding.
    fn compute_pnl(
        &self,
        current_time: u64,
        exit_price: Decimal,
    ) -> ContractResult<(PnlValues, Funding)>;
}

impl DenomStateExt for DenomState {
    fn open_position(
        &mut self,
        current_time: u64,
        size: SignedDecimal,
        entry_price: Decimal,
    ) -> ContractResult<()> {
        // calculate the current funding rate and index with size up to the current time
        self.funding = self.compute_current_funding(current_time)?;

        // increase the accumulator by size weighted by current funding index
        self.funding.accumulated_size_weighted_by_index = self
            .funding
            .accumulated_size_weighted_by_index
            .checked_add(size.checked_div(self.funding.index)?)?;

        // update the total size and cost base
        self.total_size = self.total_size.checked_add(size)?;
        let value = size.checked_mul(entry_price.into())?;
        self.total_cost_base = self.total_cost_base.checked_add(value)?;

        self.last_updated = current_time;

        Ok(())
    }

    fn close_position(&mut self, current_time: u64, position: &Position) -> ContractResult<()> {
        // calculate the current funding rate and index with size up to the current time
        self.funding = self.compute_current_funding(current_time)?;

        // decrease the accumulator by size weighted by opening funding index
        self.funding.accumulated_size_weighted_by_index = self
            .funding
            .accumulated_size_weighted_by_index
            .checked_sub(position.size.checked_div(position.entry_funding_index)?)?;

        // update the total size and cost base
        self.total_size = self.total_size.checked_sub(position.size)?;
        let value = position.size.checked_mul(position.entry_price.into())?;
        self.total_cost_base = self.total_cost_base.checked_sub(value)?;

        self.last_updated = current_time;

        Ok(())
    }

    fn compute_unrealized_pnl(&self, exit_price: Decimal) -> ContractResult<SignedDecimal> {
        Ok(self.total_size.checked_mul(exit_price.into())?.checked_sub(self.total_cost_base)?)
    }

    fn compute_funding_rate_velocity(&self) -> ContractResult<SignedDecimal> {
        // skew is just total size (sum of all open positions)
        let skew = self.total_size;

        let funding_rate_velocity = self.funding.constant_factor.checked_mul(skew)?;
        Ok(funding_rate_velocity)
    }

    fn compute_current_funding(&self, current_time: u64) -> ContractResult<Funding> {
        if self.last_updated == current_time {
            return Ok(self.funding.clone());
        };

        // how much seconds has passed from last update
        let time_diff = current_time - self.last_updated;

        let ratio: SignedDecimal = Decimal::from_ratio(time_diff, SECONDS_IN_DAY).into();

        let current_funding_rate = self
            .funding
            .rate
            .checked_add(self.compute_funding_rate_velocity()?.checked_mul(ratio)?)?;

        let current_funding_index = self
            .funding
            .index
            .checked_mul(SignedDecimal::one().checked_add(current_funding_rate)?)?;

        // update only rate and index here, the rest is copied from the previous funding
        Ok(Funding {
            rate: current_funding_rate,
            index: current_funding_index,
            ..self.funding
        })
    }

    fn compute_accrued_funding(
        &self,
        current_time: u64,
        exit_price: Decimal,
    ) -> ContractResult<(SignedDecimal, Funding)> {
        let current_funding = self.compute_current_funding(current_time)?;

        let a = current_funding
            .index
            .checked_mul(exit_price.into())?
            .checked_mul(current_funding.accumulated_size_weighted_by_index)?;
        let b = self.total_size.checked_mul(exit_price.into())?;

        let accrued_funding = a.checked_sub(b)?;

        Ok((accrued_funding, current_funding))
    }

    fn compute_pnl(
        &self,
        current_time: u64,
        exit_price: Decimal,
    ) -> ContractResult<(PnlValues, Funding)> {
        let unrealized_pnl = self.compute_unrealized_pnl(exit_price)?;
        let (accrued_funding, curr_funding) =
            self.compute_accrued_funding(current_time, exit_price)?;
        let pnl_values = PnlValues {
            unrealized_pnl,
            accrued_funding,
            pnl: unrealized_pnl.checked_sub(accrued_funding)?,
        };
        Ok((pnl_values, curr_funding))
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
            entry_funding_index: SignedDecimal::one()
        },
        Decimal::from_str("234.56").unwrap(),
        PnL::BreakEven;
        "long position - price no change"
    )]
    #[test_case(
        Position {
            size: SignedDecimal::from_str("123.45").unwrap(),
            entry_price: Decimal::from_str("234.56").unwrap(),
            entry_funding_index: SignedDecimal::one()
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
            entry_funding_index: SignedDecimal::one()
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
            entry_funding_index: SignedDecimal::one()
        },
        Decimal::from_str("234.56").unwrap(),
        PnL::BreakEven;
        "short position - price no change"
    )]
    #[test_case(
        Position {
            size: SignedDecimal::from_str("-123.45").unwrap(),
            entry_price: Decimal::from_str("234.56").unwrap(),
            entry_funding_index: SignedDecimal::one()
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
            entry_funding_index: SignedDecimal::one()
        },
        Decimal::from_str("200").unwrap(),
        PnL::Profit(Coin {
            denom: MOCK_BASE_DENOM.into(),
            amount: Uint128::new(4266), // floor(-123.45 * (200 - 234.56))
        });
        "short position - price down"
    )]
    fn computing_pnl(position: Position, current_price: Decimal, expect_pnl: PnL) {
        let funding = Funding::default();
        let pnl = compute_pnl(&funding, &position, current_price, MOCK_BASE_DENOM).unwrap();
        assert_eq!(pnl, expect_pnl);
    }

    #[test_case(
        vec![Position {
            size: SignedDecimal::from_str("123.45").unwrap(),
            entry_price: Decimal::from_str("234.56").unwrap(),
            entry_funding_index: SignedDecimal::zero() // doesn't matter here
        },
        Position {
            size: SignedDecimal::from_str("-12.50").unwrap(),
            entry_price: Decimal::from_str("260").unwrap(),
            entry_funding_index: SignedDecimal::zero() // doesn't matter here
        }],
        DenomState {
            enabled: false, // doesn't matter here
            total_size: SignedDecimal::from_str("110.95").unwrap(), // 123.45 + (-12.50)
            total_cost_base: SignedDecimal::from_str("25706.432").unwrap(), // 123.45 * 234.56 + (-12.50 * 260)
            funding: Funding {
                max_funding_velocity: Decimal::from_str("3").unwrap(),
                skew_scale: Decimal::from_str("1000000").unwrap(),
                constant_factor: Funding::constant_factor(Decimal::from_str("3").unwrap(), Decimal::from_str("1000000").unwrap())
                    .unwrap(),
                rate: SignedDecimal::from_str("0.04537035").unwrap(),
                index: SignedDecimal::from_str("1.638618023625").unwrap(),
                accumulated_size_weighted_by_index: SignedDecimal::from_str("71.127601446530195461").unwrap(),
            },
            last_updated: 2 * SECONDS_IN_DAY,
        };
        "accumulators for open positions"
    )]
    fn computing_accumulators_for_open_positions(
        open_positions: Vec<Position>,
        expect_ds: DenomState,
    ) {
        let mut ds = DenomState {
            funding: Funding {
                max_funding_velocity: expect_ds.funding.max_funding_velocity,
                skew_scale: expect_ds.funding.skew_scale,
                constant_factor: expect_ds.funding.constant_factor,
                rate: SignedDecimal::from_str("0.045").unwrap(),
                index: SignedDecimal::from_str("1.5").unwrap(),
                accumulated_size_weighted_by_index: SignedDecimal::zero(),
            },
            last_updated: 0,
            ..Default::default()
        };
        let mut current_time = 0;
        for open_position in open_positions {
            current_time += SECONDS_IN_DAY;
            ds.open_position(current_time, open_position.size, open_position.entry_price).unwrap();
        }
        assert_eq!(ds, expect_ds);
    }

    #[test_case(
        vec![Position {
            size: SignedDecimal::from_str("123.45").unwrap(),
            entry_price: Decimal::from_str("234.56").unwrap(),
            entry_funding_index: SignedDecimal::from_str("1.2").unwrap()
        },
        Position {
            size: SignedDecimal::from_str("-12.50").unwrap(),
            entry_price: Decimal::from_str("260").unwrap(),
            entry_funding_index: SignedDecimal::from_str("2.8").unwrap()
        }],
        DenomState {
            enabled: false, // doesn't matter here
            total_size: SignedDecimal::from_str("-110.95").unwrap(), // -123.45 - (-12.50)
            total_cost_base: SignedDecimal::from_str("-25706.432").unwrap(), // -(123.45 * 234.56) - (-12.50 * 260)
            funding: Funding {
                max_funding_velocity: Decimal::from_str("3").unwrap(),
                skew_scale: Decimal::from_str("1000000").unwrap(),
                constant_factor: Funding::constant_factor(Decimal::from_str("3").unwrap(), Decimal::from_str("1000000").unwrap())
                    .unwrap(),
                    rate: SignedDecimal::from_str("0.04462965").unwrap(),
                    index: SignedDecimal::from_str("1.637456976375").unwrap(),
                accumulated_size_weighted_by_index: SignedDecimal::from_str("-98.410714285714285715").unwrap(), // when closing it uses position funding index: 0 - (123.45 / 1.2) - (-12.50 / 2.8)
            },
            last_updated: 2 * SECONDS_IN_DAY,
        };
        "accumulators for close positions"
    )]
    fn computing_accumulators_for_close_positions(
        close_positions: Vec<Position>,
        expect_ds: DenomState,
    ) {
        let mut ds = DenomState {
            funding: Funding {
                max_funding_velocity: expect_ds.funding.max_funding_velocity,
                skew_scale: expect_ds.funding.skew_scale,
                constant_factor: expect_ds.funding.constant_factor,
                rate: SignedDecimal::from_str("0.045").unwrap(),
                index: SignedDecimal::from_str("1.5").unwrap(),
                accumulated_size_weighted_by_index: SignedDecimal::zero(),
            },
            last_updated: 0,
            ..Default::default()
        };
        let mut current_time = 0;
        for close_position in close_positions {
            current_time += SECONDS_IN_DAY;
            ds.close_position(current_time, &close_position).unwrap();
        }
        assert_eq!(ds, expect_ds);
    }

    #[test_case(
        vec![Position {
            size: SignedDecimal::from_str("123.45").unwrap(),
            entry_price: Decimal::from_str("234.56").unwrap(),
            entry_funding_index: SignedDecimal::one()
        },
        Position {
            size: SignedDecimal::from_str("123.45").unwrap(),
            entry_price: Decimal::from_str("260").unwrap(),
            entry_funding_index: SignedDecimal::one()
        },
        Position {
            size: SignedDecimal::from_str("-12.50").unwrap(),
            entry_price: Decimal::from_str("240.12").unwrap(),
            entry_funding_index: SignedDecimal::one()
        },
        Position {
            size: SignedDecimal::from_str("-12.50").unwrap(),
            entry_price: Decimal::from_str("280.50").unwrap(),
            entry_funding_index: SignedDecimal::one()
        }], // 123.45 * (250 - 234.56) + 123.45 * (250 - 260) + (-12.50) * (250 - 240.12) + (-12.50) * (250 - 280.50) = 929.318
        vec![Position {
            size: SignedDecimal::from_str("12.45").unwrap(),
            entry_price: Decimal::from_str("234.56").unwrap(),
            entry_funding_index: SignedDecimal::one()
        },
        Position {
            size: SignedDecimal::from_str("12.45").unwrap(),
            entry_price: Decimal::from_str("260").unwrap(),
            entry_funding_index: SignedDecimal::one()
        },
        Position {
            size: SignedDecimal::from_str("-10.50").unwrap(),
            entry_price: Decimal::from_str("240.12").unwrap(),
            entry_funding_index: SignedDecimal::one()
        },
        Position {
            size: SignedDecimal::from_str("-10.50").unwrap(),
            entry_price: Decimal::from_str("280.50").unwrap(),
            entry_funding_index: SignedDecimal::one()
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
            ds.open_position(0, open_position.size, open_position.entry_price).unwrap();
        }

        for close_position in close_positions {
            ds.close_position(0, &close_position).unwrap();
        }

        let total_pnl = ds.compute_unrealized_pnl(exit_price).unwrap();
        assert_eq!(total_pnl, expect_total_pnl);
    }

    #[test]
    fn compute_funding_rate_velocity_correctly() {
        let ds = DenomState {
            total_size: SignedDecimal::from_str("123.86").unwrap(),
            funding: Funding {
                constant_factor: SignedDecimal::from_str("0.045").unwrap(),
                ..Default::default()
            },
            ..Default::default()
        };

        let frv = ds.compute_funding_rate_velocity().unwrap();
        assert_eq!(frv, SignedDecimal::from_str("5.5737").unwrap());
    }

    #[test]
    fn compute_current_funding_correctly() {
        let ds = denom_state();

        let curr_funding =
            ds.compute_current_funding(ds.last_updated + SECONDS_IN_DAY / 2).unwrap();
        let expect_funding = Funding {
            rate: SignedDecimal::from_str("0.025375").unwrap(),
            index: SignedDecimal::from_str("1.025375").unwrap(),
            ..ds.funding // only rate and index should be changed, the rest stays the same
        };
        assert_eq!(curr_funding, expect_funding);
    }

    #[test]
    fn compute_accrued_funding_correctly() {
        let ds = denom_state();

        let current_time = ds.last_updated + SECONDS_IN_DAY / 2;
        let (accrued_funding, curr_funding) =
            ds.compute_accrued_funding(current_time, Decimal::from_str("20.5").unwrap()).unwrap();
        assert_eq!(accrued_funding, SignedDecimal::from_str("-5098.724765625").unwrap());
        let expect_funding = Funding {
            rate: SignedDecimal::from_str("0.025375").unwrap(),
            index: SignedDecimal::from_str("1.025375").unwrap(),
            ..ds.funding // only rate and index should be changed, the rest stays the same
        };
        assert_eq!(curr_funding, expect_funding);
    }

    #[test]
    fn compute_pnl_correctly() {
        let ds = denom_state();

        let current_time = ds.last_updated + SECONDS_IN_DAY / 2;
        let (pnl_values, curr_funding) =
            ds.compute_pnl(current_time, Decimal::from_str("20.5").unwrap()).unwrap();
        assert_eq!(
            pnl_values,
            PnlValues {
                unrealized_pnl: SignedDecimal::from_str("2625").unwrap(),
                accrued_funding: SignedDecimal::from_str("-5098.724765625").unwrap(),
                pnl: SignedDecimal::from_str("7723.724765625").unwrap()
            }
        );
        let expect_funding = Funding {
            rate: SignedDecimal::from_str("0.025375").unwrap(),
            index: SignedDecimal::from_str("1.025375").unwrap(),
            ..ds.funding // only rate and index should be changed, the rest stays the same
        };
        assert_eq!(curr_funding, expect_funding);
    }

    fn denom_state() -> DenomState {
        let max_funding_velocity = Decimal::from_str("3").unwrap();
        let skew_scale = Decimal::from_str("1000000").unwrap();
        DenomState {
            enabled: true,
            total_size: SignedDecimal::from_str("250").unwrap(),
            total_cost_base: SignedDecimal::from_str("2500").unwrap(),
            funding: Funding {
                max_funding_velocity,
                skew_scale,
                constant_factor: Funding::constant_factor(max_funding_velocity, skew_scale)
                    .unwrap(),
                rate: SignedDecimal::from_str("0.025").unwrap(),
                index: SignedDecimal::one(),
                accumulated_size_weighted_by_index: SignedDecimal::from_str("1.25").unwrap(),
            },
            last_updated: 10,
        }
    }
}
