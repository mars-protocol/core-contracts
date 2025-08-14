// Extracts spot balance, debt, and funding delta from Mars positions
use cosmwasm_std::{Coin, Decimal, Int128, Uint128};
use mars_delta_neutral_position::types::Position;
use mars_types::{
    active_delta_neutral::query::MarketConfig,
    credit_manager::{DebtAmount, Positions},
    perps::{PerpPosition, PnlAmounts},
    swapper::SwapperRoute,
};
use mars_utils::helpers::uint128_to_int128;

use crate::error::{ContractError, ContractResult};

#[derive(Debug, Clone, PartialEq)]
pub struct PositionDeltas {
    pub funding_delta: Int128,
    pub borrow_delta: Uint128,
    pub spot_delta: Int128,
}

/// Calculates key position deltas (spot, debt, funding, borrow) from Mars on-chain positions and config.
///
/// # Arguments
/// * `mars_positions` - The current on-chain Mars positions for this strategy
/// * `config` - The strategy configuration (denoms, etc)
/// * `position_state` - The on-chain position state for this strategy
///
/// # Returns
/// * `PositionDeltas` struct containing:
///     - `current_balance`: spot balance for the configured denom
///     - `debt`: debt balance for the USDC denom
///     - `funding_delta`: accrued funding from the perp position
///     - `borrow_delta`: current borrow minus principle and accrued borrow
pub fn calculate_deltas(
    mars_positions: &Positions,
    market_config: &MarketConfig,
    position_state: &Position,
) -> ContractResult<PositionDeltas> {
    let current_balance = combined_balance(mars_positions, &market_config.spot_denom)?;

    let debt = mars_positions
        .debts
        .iter()
        .find(|debt| debt.denom == market_config.usdc_denom)
        .unwrap_or(&DebtAmount {
            denom: market_config.usdc_denom.clone(),
            amount: Uint128::zero(),
            shares: Uint128::zero(),
        })
        .amount;

    let funding_delta = mars_positions
        .perps
        .iter()
        .find(|perp| perp.denom == market_config.perp_denom)
        .unwrap_or(&PerpPosition {
            base_denom: market_config.perp_denom.clone(),
            denom: market_config.perp_denom.clone(),
            size: Int128::zero(),
            entry_price: Decimal::zero(),
            current_price: Decimal::zero(),
            entry_exec_price: Decimal::zero(),
            current_exec_price: Decimal::zero(),
            unrealized_pnl: PnlAmounts::default(),
            realized_pnl: PnlAmounts::default(),
        })
        .unrealized_pnl
        .accrued_funding;

    let borrow_delta = debt.checked_sub(
        position_state
            .debt_principle
            .checked_add(position_state.net_borrow_balance.unsigned_abs())?,
    )?;
    let spot_delta = uint128_to_int128(current_balance)?
        .checked_sub(uint128_to_int128(position_state.spot_amount)?)?;
    Ok(PositionDeltas {
        funding_delta,
        borrow_delta,
        spot_delta,
    })
}

// TODO remove this? validated already by swapper
pub fn validate_swapper_route(
    route: &SwapperRoute,
    denom_in: &str,
    denom_out: &str,
) -> ContractResult<()> {
    match route {
        SwapperRoute::Astro(astro_route) => {
            assert!(!astro_route.swaps.is_empty(), "Astro route must have at least one swap");
            assert!(
                astro_route.swaps[0].from == denom_in || astro_route.swaps[0].from == denom_out,
                "Invalid swap from asset"
            );
            assert!(
                astro_route.swaps[0].to == denom_in || astro_route.swaps[0].to == denom_out,
                "Invalid swap to asset"
            );
            Ok(())
        }
        SwapperRoute::Osmo(_) => {
            unimplemented!()
        }
        SwapperRoute::Duality(_duality_route) => Ok(()),
    }
}

/// Returns the total balance for a given denom by summing the deposit and lend positions.
///
/// # Arguments
/// * `positions` - Reference to the Positions struct containing all deposits and lends.
/// * `denom` - The denomination to sum balances for.
///
/// # Returns
/// * `ContractResult<Uint128>` - The sum of deposit and lend amounts for the given denom, or an error if either is missing.
pub fn combined_balance(positions: &Positions, denom: &str) -> ContractResult<Uint128> {
    let deposit = positions.deposits.iter().find(|deposit| deposit.denom == denom);
    let lend = positions.lends.iter().find(|lend| lend.denom == denom);

    if deposit.is_none() && lend.is_none() {
        return Err(ContractError::NoCollateralForDenom {
            denom: denom.to_string(),
        });
    }

    let deposit = deposit.map(|d| d.amount).unwrap_or_default();
    let lend = lend.map(|l| l.amount).unwrap_or_default();

    Ok(deposit.checked_add(lend)?)
}

pub fn assert_deposit_funds_valid(funds: &[Coin], denom: &str) -> ContractResult<()> {
    if funds.len() != 1 {
        return Err(ContractError::ExcessAssets {
            denom: denom.to_string(),
        });
    }

    let fund_denom = &funds[0].denom;

    if fund_denom != denom {
        return Err(ContractError::IncorrectDenom {
            denom: fund_denom.to_string(),
            base_denom: denom.to_string(),
        });
    }
    Ok(())
}

pub fn assert_no_funds(funds: &[Coin]) -> ContractResult<()> {
    if !funds.is_empty() {
        return Err(ContractError::IllegalFundsSent {});
    }
    Ok(())
}
