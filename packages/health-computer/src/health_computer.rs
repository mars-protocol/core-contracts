use std::{
    cmp::{max, min},
    collections::HashMap,
    ops::Neg,
    str::FromStr,
};

use bigdecimal::{BigDecimal, One, RoundingMode, Zero};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, Decimal, Fraction, Int128, SignedDecimal, Uint128};
use mars_perps_common::pricing::closing_execution_price;
use mars_types::{
    credit_manager::Positions,
    health::{
        AccountKind, BorrowTarget, Health,
        HealthError::{
            DenomNotPresent, MissingAmount, MissingAssetParams, MissingHLSParams,
            MissingPerpParams, MissingPrice, MissingUSDCMarginParams, MissingVaultConfig,
            MissingVaultValues,
        },
        HealthResult, LiquidationPriceKind, SwapKind,
    },
    params::{AssetParams, CmSettings, HlsAssetType, VaultConfig},
    perps::{PerpPosition, PnL},
};
#[cfg(feature = "javascript")]
use tsify::Tsify;

use crate::{
    big_decimal::ToBigDecimal, utils::calculate_remaining_oi_amount, CollateralValue,
    PerpHealthFactorValues, PerpPnlValues, PerpsData, VaultsData,
};

/// `HealthComputer` is a shared struct with the frontend that gets compiled to wasm.
/// For this reason, it uses a dependency-injection-like pattern where all required data is needed up front.
#[cw_serde]
#[cfg_attr(feature = "javascript", derive(Tsify))]
#[cfg_attr(feature = "javascript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct HealthComputer {
    pub kind: AccountKind,
    pub positions: Positions,
    pub asset_params: HashMap<String, AssetParams>,
    pub vaults_data: VaultsData,
    pub perps_data: PerpsData,
    pub oracle_prices: HashMap<String, Decimal>,
}

#[cw_serde]
#[cfg_attr(feature = "javascript", derive(Tsify))]
#[cfg_attr(feature = "javascript", tsify(into_wasm_abi, from_wasm_abi))]
pub enum Direction {
    Long,
    Short,
}

impl Direction {
    pub fn sign(&self) -> Int128 {
        match self {
            Direction::Long => Int128::from_str("1").unwrap(),
            Direction::Short => Int128::from_str("-1").unwrap(),
        }
    }
}

impl HealthComputer {
    pub fn compute_health(&self) -> HealthResult<Health> {
        let CollateralValue {
            total_collateral_value,
            max_ltv_adjusted_collateral,
            liq_ltv_adjusted_collateral: liquidation_threshold_adjusted_collateral,
        } = self.total_collateral_value()?;

        let spot_debt_value = self.debt_value()?;
        let (perp_hf_values, perp_pnl_values) =
            self.perp_hf_values_and_pnl(&self.positions.perps)?;
        let ltv_numerator =
            max_ltv_adjusted_collateral.checked_add(perp_hf_values.max_ltv_numerator)?;
        let ltv_denominator = spot_debt_value.checked_add(perp_hf_values.max_ltv_denominator)?;

        let (max_ltv_health_factor, liquidation_health_factor) = if ltv_denominator.is_zero() {
            (None, None)
        } else {
            // NOTE : The HF calc in the latest doc (0.9) differs slightly from this implementation.
            // reason being that risk team is still deciding on the correctness of
            // that formula.
            // The difference is in how funding is applied.
            // Currently, we include uusdc collateral as part of RWA and apply f+ / f- to each perp position
            // The document uses C+, C- instead.
            // HF = (RWA + perp_numerator) / (spot_debt + perp_denominator)
            // where
            // RWA = risk weighted assets (i.e. ltv * collateral_value)
            // spot debt = total value of borrowed assets (does not include perp unrealized pnl)

            let max_ltv_hf = Decimal::checked_from_ratio(ltv_numerator, ltv_denominator)?;
            let liq_hf = self.calculate_liq_hf(
                &liquidation_threshold_adjusted_collateral,
                &spot_debt_value,
                &perp_hf_values,
            )?;

            (Some(max_ltv_hf), Some(liq_hf))
        };

        Ok(Health {
            total_debt_value: spot_debt_value,
            total_collateral_value,
            max_ltv_adjusted_collateral,
            liquidation_threshold_adjusted_collateral,
            max_ltv_health_factor,
            liquidation_health_factor,
            perps_pnl_profit: perp_pnl_values.profit,
            perps_pnl_loss: perp_pnl_values.loss,
            has_perps: !self.positions.perps.is_empty(),
        })
    }

    /// The max this account can withdraw of `withdraw_denom` and maintain max_ltv >= 1
    /// Note: This is an estimate. Guarantees to leave account healthy, but in edge cases,
    /// due to rounding, it may be slightly too conservative.
    pub fn max_withdraw_amount_estimate(&self, withdraw_denom: &str) -> HealthResult<Uint128> {
        // Both deposits and lends should be considered, as the funds can automatically be un-lent
        // and also used to withdraw.
        // Staked astro lps are also considered, given that the user will provide an unstake msg
        // before the actual withdraw msg
        let withdraw_coin = self.get_coin_from_positions(withdraw_denom)?;
        if withdraw_coin.amount.is_zero() {
            return Ok(Uint128::zero());
        };

        let params = self.asset_params.get(withdraw_denom);

        match params {
            None => Ok(withdraw_coin.amount),
            Some(params) => {
                // If no debt or coin is blacklisted (meaning does not contribute to max ltv hf),
                // the total amount deposited can be withdrawn
                if (self.positions.debts.is_empty() && self.positions.perps.is_empty())
                    || !params.credit_manager.whitelisted
                {
                    return Ok(withdraw_coin.amount);
                }

                // withdraw denom max ltv adjusted value = total max ltv adjusted value - debt value - perp_denominator + perp_numerator
                let total_max_ltv_adjusted_value =
                    self.total_collateral_value()?.max_ltv_adjusted_collateral;
                let debt_value = self.debt_value()?;

                let withdraw_denom_price = *self
                    .oracle_prices
                    .get(withdraw_denom)
                    .ok_or(MissingPrice(withdraw_denom.to_string()))?;

                let withdraw_denom_max_ltv = match self.kind {
                    AccountKind::Default => params.max_loan_to_value,
                    AccountKind::FundManager {
                        ..
                    } => params.max_loan_to_value,
                    AccountKind::UsdcMargin => params.max_loan_to_value,
                    AccountKind::HighLeveredStrategy => {
                        params
                            .credit_manager
                            .hls
                            .as_ref()
                            .ok_or(MissingHLSParams(withdraw_denom.to_string()))?
                            .max_loan_to_value
                    }
                };

                let PerpHealthFactorValues {
                    max_ltv_denominator: perp_denominator,
                    max_ltv_numerator: perp_numerator,
                    ..
                } = self.perp_hf_values_and_pnl(&self.positions.perps)?.0;

                let one = Uint128::one();
                let numerator = total_max_ltv_adjusted_value.checked_add(perp_numerator)?;
                let denominator = debt_value.checked_add(perp_denominator)?;

                if !numerator.is_zero() && !denominator.is_zero() {
                    let hf = Decimal::checked_from_ratio(numerator, denominator)?;

                    if hf.le(&Decimal::one()) {
                        return Ok(Uint128::zero());
                    }
                }

                // The max withdraw amount is calculated as:
                // withdraw denom max ltv adjusted value = total max ltv adjusted value  + perp_numerator - debt value - perp_denominator
                let max_withdraw_value = total_max_ltv_adjusted_value
                    .checked_add(perp_numerator)?
                    .checked_sub(debt_value)?
                    .checked_sub(perp_denominator)?
                    .checked_sub(one)?;

                // The above is the raw value, now we need to factor in price and LTV impact
                let max_withdraw_amount = max_withdraw_value
                    .checked_div_floor(withdraw_denom_price.checked_mul(withdraw_denom_max_ltv)?)?;

                Ok(min(max_withdraw_amount, withdraw_coin.amount))
            }
        }
    }

    pub fn max_swap_amount_estimate(
        &self,
        from_denom: &str,
        to_denom: &str,
        kind: &SwapKind,
        slippage: Decimal,
        is_repaying_debt: bool,
    ) -> HealthResult<Uint128> {
        // Both deposits and lends should be considered, as the funds can automatically be un-lent and
        // and also used to swap.
        // Staked astro lps are also considered, given that the user will provide an unstake msg
        // before the actual withdraw msg
        let from_coin = self.get_coin_from_positions(from_denom)?;

        // If no debt the total amount deposited can be swapped (only for default swaps)
        // If repaying debt, the total amount deposited can be swapped
        if (kind == &SwapKind::Default
            && self.positions.debts.is_empty()
            && self.positions.perps.is_empty())
            || is_repaying_debt
        {
            return Ok(from_coin.amount);
        }

        let total_max_ltv_adjusted_value =
            self.total_collateral_value()?.max_ltv_adjusted_collateral;

        let debt_value = self.debt_value()?;

        if total_max_ltv_adjusted_value.is_zero() {
            return Ok(Uint128::zero());
        }

        let PerpHealthFactorValues {
            max_ltv_denominator: perp_denominator,
            max_ltv_numerator: perp_numerator,
            ..
        } = self.perp_hf_values_and_pnl(&self.positions.perps)?.0;

        let one = Uint128::one();
        let numerator = total_max_ltv_adjusted_value.checked_add(perp_numerator)?;
        let denominator = debt_value.checked_add(perp_denominator)?;

        // If we can check the health, we should check the health and return 0 if we cannot
        // swap.
        if !numerator.is_zero() && !denominator.is_zero() {
            let hf = Decimal::checked_from_ratio(numerator, denominator)?;

            if hf.le(&Decimal::one()) {
                return Ok(Uint128::zero());
            }
        }

        let from_ltv = self.get_coin_max_ltv(from_denom)?;
        let to_ltv = self.get_coin_max_ltv(to_denom)?;

        let zero = Decimal::zero();
        let from_price = self.oracle_prices.get(from_denom).unwrap_or(&zero);

        // An asset that has a price of 1 and max ltv of 0.5 has a collateral_value of 0.5.
        // Swapping that asset for an asset with the same price, but 0.8 max ltv results in a collateral_value of 0.8.
        // Therefore, when the asset that is swapped to has a higher or equal max ltv than the asset swapped from,
        // the collateral value will increase and we can allow the full balance to be swapped.
        // The ltv_out is adjusted for slippage, as the swap_out_value can drop by the slippage.
        let to_ltv_slippage_corrected = to_ltv.checked_mul(Decimal::one() - slippage)?;

        // The "trade any asset" feature allows for either or both of the assets to have an ltv of 0.
        // The following statement catches the cases where:
        // - If both assets ltv are 0, the full balance can be swapped
        // - If the from_ltv is 0 the ltv will increase, so the full balance can be swapped
        // - If the to_ltv is 0, the ltv will decrease, so we can rely on the extensive calculation below
        let swappable_amount = if to_ltv_slippage_corrected >= from_ltv {
            from_coin.amount
        } else {
            // In order to calculate the output of the swap, the formula looks like this:
            //     1 = (collateral_value + to_amount * to_price * to_ltv - from_amount * from_price * from_ltv) / debt_value
            // The unknown variables here are to_amount and from_amount. In order to only have 1 unknown variable, from_amount,
            // to_amount can be replaced by:
            //     to_amount = slippage * from_amount * from_price / to_price
            // This results in the following formula:
            //     1 = (collateral_value + slippage * from_amount * from_price / to_price * to_price * to_ltv - from_amount * from_price * from_ltv) / debt_value
            //     debt_value = collateral_value + slippage * from_amount * from_price * to_ltv - from_amount * from_price * from_ltv
            //     slippage * from_amount * from_price * to_ltv - from_amount * from_price * from_ltv = debt_value - collateral_value
            //     from_amount * (slippage * from_price * to_ltv - from_price * from_ltv) = debt_value - collateral_value
            // Rearranging this formula to isolate from_amount results in the following formula:
            //     from_amount = (debt_value - collateral_value) / (from_price * (slippage * to_ltv - from_ltv))
            // Rearranging to avoid negative numbers for the denominator (to_ltv_slippage_corrected < from_ltv):
            //     from_amount = (collateral_value - debt_value) / (from_price * (from_ltv - slippage * to_ltv)
            // Rearranging to include perp values:
            //    from_amount = (collateral_value + perpn - debt_value - perpd) / (from_price * (from_ltv - slippage * to_ltv)

            let amount = total_max_ltv_adjusted_value
                .checked_add(perp_numerator)?
                .checked_sub(debt_value)?
                .checked_sub(perp_denominator)?
                .checked_sub(one)?
                .checked_div_floor(from_price.checked_mul(from_ltv - to_ltv_slippage_corrected)?)?;

            // Cap the swappable amount at the current balance of the coin
            min(amount, from_coin.amount)
        };

        match kind {
            SwapKind::Default => Ok(swappable_amount),

            SwapKind::Margin => {
                // If the swappable amount is less than the available amount, no need to further calculate
                // the margin borrow amount.
                if swappable_amount < from_coin.amount {
                    return Ok(swappable_amount);
                }

                let from_coin_value = from_coin.amount.checked_mul_floor(*from_price)?;

                // This represents the max ltv adjusted value of the coin being swapped from
                let swap_from_ltv_value = from_coin_value.checked_mul_floor(from_ltv)?;

                // The from_denom is always taken on as debt, as the trade is in the bullish direction
                // of the to_denom (expecting it to outpace the borrow rate from the from_denom)
                let swap_to_ltv_value = from_coin_value.checked_mul_floor(to_ltv)?;

                let total_max_ltv_adjust_value_after_swap = total_max_ltv_adjusted_value
                    .checked_add(swap_to_ltv_value)?
                    .checked_sub(swap_from_ltv_value)?;

                // The total swappable amount for margin is represented by the available coin balance + the
                // the maximum amount that can be borrowed (and then swapped).
                // This is represented by the formula:
                //     1 = (collateral_after_swap + slippage * borrow_amount * borrow_price * to_ltv) / (debt + borrow_amount * borrow_price)
                //     debt + borrow_amount * borrow_price = collateral_after_swap + slippage * borrow_amount * borrow_price * to_ltv
                //     borrow_amount * borrow_price - slippage * borrow_amount * borrow_price * to_ltv = collateral_after_swap - debt
                //     borrow_amount * borrow_price * (1 - slippage * to_ltv) = collateral_after_swap - debt
                // Rearranging this results in:
                //     borrow_amount = (collateral_after_swap - debt) / (borrow_price * (1 - slippage * to_ltv))
                // Rearranging to include perp values:
                //    borrow_amount = (collateral_after_swap + perpn - debt - perpd) / (borrow_price * (1 - slippage * to_ltv))
                let borrow_amount = total_max_ltv_adjust_value_after_swap
                    .checked_add(perp_numerator)?
                    .checked_sub(debt_value)?
                    .checked_sub(perp_denominator)?
                    .checked_sub(one)?
                    .checked_div_floor(
                        Decimal::one()
                            .checked_sub(to_ltv_slippage_corrected)?
                            .checked_mul(*from_price)?,
                    )?;

                // The total amount that can be swapped is then the balance of the coin + the additional amount
                // that can be borrowed.
                Ok(borrow_amount.checked_add(from_coin.amount)?)
            }
        }
    }

    /// The max this account can borrow of `borrow_denom` and maintain max_ltv >= 1
    /// Note: This is an estimate. Guarantees to leave account healthy, but in edge cases,
    /// due to rounding, it may be slightly too conservative.
    pub fn max_borrow_amount_estimate(
        &self,
        borrow_denom: &str,
        target: &BorrowTarget,
    ) -> HealthResult<Uint128> {
        let total_max_ltv_adjusted_value =
            self.total_collateral_value()?.max_ltv_adjusted_collateral;
        let debt_value = self.debt_value()?;

        // We often add one to calcs for a margin of error, so rather than create it multiple times we just create it once here.
        let one = Uint128::one();

        // Perp values
        let PerpHealthFactorValues {
            max_ltv_denominator: perp_denominator,
            max_ltv_numerator: perp_numerator,
            ..
        } = self.perp_hf_values_and_pnl(&self.positions.perps)?.0;

        let params = self
            .asset_params
            .get(borrow_denom)
            .ok_or(MissingAssetParams(borrow_denom.to_string()))?;

        // If asset not whitelisted we cannot borrow
        if !params.credit_manager.whitelisted || total_max_ltv_adjusted_value.is_zero() {
            return Ok(Uint128::zero());
        }

        let numerator = total_max_ltv_adjusted_value.checked_add(perp_numerator)?;
        let denominator = debt_value.checked_add(perp_denominator)?;

        if !numerator.is_zero() && !denominator.is_zero() {
            let hf = Decimal::checked_from_ratio(numerator, denominator)?;

            if hf.le(&Decimal::one()) {
                return Ok(Uint128::zero());
            }
        }

        let borrow_denom_max_ltv = match self.kind {
            AccountKind::Default => params.max_loan_to_value,
            AccountKind::FundManager {
                ..
            } => params.max_loan_to_value,
            AccountKind::UsdcMargin => params.max_loan_to_value,
            AccountKind::HighLeveredStrategy => {
                params
                    .credit_manager
                    .hls
                    .as_ref()
                    .ok_or(MissingHLSParams(borrow_denom.to_string()))?
                    .max_loan_to_value
            }
        };

        let borrow_denom_price = self
            .oracle_prices
            .get(borrow_denom)
            .cloned()
            .ok_or(MissingPrice(borrow_denom.to_string()))?;

        // The formulas look like this in practice:
        //      hf = rounddown(roundown(amount * price) * perp_numerator) / (spot_debt value + perp_denominator)
        // Which means re-arranging this to isolate borrow amount is an estimate,
        // quite close, but never precisely right. For this reason, the + 1 of the formulas
        // below are meant to err on the side of being more conservative vs aggressive.

        let max_borrow_amount = match target {
            // The max borrow for deposit can be calculated as:
            //      1 = (max ltv adjusted value + (borrow denom amount * borrow denom price * borrow denom max ltv) + perpn) / (debt value + (borrow denom amount * borrow denom price) + perpd)
            // Re-arranging this to isolate borrow denom amount renders:
            //      max_borrow_denom_amount = max ltv adjusted value  + perpn - debt value - perpd / (borrow_denom_price * (1 - borrow_denom_max_ltv)))
            BorrowTarget::Deposit => {
                let numerator = total_max_ltv_adjusted_value
                    .checked_add(perp_numerator)?
                    .checked_sub(debt_value)?
                    .checked_sub(perp_denominator)?
                    .checked_sub(one)?;

                let denominator: Decimal = borrow_denom_price
                    .checked_mul(Decimal::one().checked_sub(borrow_denom_max_ltv)?)?;

                numerator.checked_div_floor(denominator)?
            }

            // Borrowing assets to wallet does not count towards collateral. It only adds to debts.
            // Hence, the max borrow to wallet can be calculated as:
            //      1 = (max ltv adjusted value) + perpn / (debt value + (borrow denom amount * borrow denom price)) + perpd
            // Re-arranging this to isolate borrow denom amount renders:
            //      borrow denom amount = (max ltv adjusted value - debt_value - perpd + perpn) / denom_price
            BorrowTarget::Wallet => {
                let numerator = total_max_ltv_adjusted_value
                    .checked_add(perp_numerator)?
                    .checked_sub(debt_value)?
                    .checked_sub(perp_denominator)?
                    .checked_sub(one)?;

                numerator.checked_div_floor(borrow_denom_price)?
            }

            // When borrowing assets to add to a vault, the amount deposited into the vault counts towards collateral.
            // The health factor can be calculated as:
            //     1 = (max ltv adjusted value + (borrow amount * borrow price * vault max ltv)) / (debt value + (borrow amount * borrow price))
            // Re-arranging this to isolate borrow amount renders:
            //     borrow amount = (max ltv adjusted value - debt value + perpd - perpn) / (borrow price * (1 - vault max ltv)
            BorrowTarget::Vault {
                address,
            } => {
                let VaultConfig {
                    addr,
                    max_loan_to_value,
                    whitelisted,
                    hls,
                    ..
                } = self
                    .vaults_data
                    .vault_configs
                    .get(address)
                    .ok_or(MissingVaultConfig(address.to_string()))?;

                // If vault or base token has been de-listed, drop MaxLTV to zero
                let checked_vault_max_ltv = if *whitelisted {
                    match self.kind {
                        AccountKind::Default => *max_loan_to_value,
                        AccountKind::FundManager {
                            ..
                        } => *max_loan_to_value,
                        AccountKind::UsdcMargin => *max_loan_to_value,
                        AccountKind::HighLeveredStrategy => {
                            hls.as_ref()
                                .ok_or(MissingHLSParams(addr.to_string()))?
                                .max_loan_to_value
                        }
                    }
                } else {
                    Decimal::zero()
                };

                // The max borrow for deposit can be calculated as:
                //      1 = (total_max_ltv_adjusted_value + (max_borrow_denom_amount * borrow_denom_price * checked_vault_max_ltv) + perpn) / (debt_value + (max_borrow_denom_amount * borrow_denom_price)) + perpd
                // Re-arranging this to isolate borrow denom amount renders:
                //      max_borrow_denom_amount = (total_max_ltv_adjusted_value-debt_value + perpn - perpd) / (borrow_denom_price * (1 - checked_vault_max_ltv))
                // Which means re-arranging this to isolate borrow amount is an estimate,
                // quite close, but never precisely right. For this reason, the - 1 of the formulas
                // below are meant to err on the side of being more conservative vs aggressive.

                let numerator = total_max_ltv_adjusted_value
                    .checked_add(perp_numerator)?
                    .checked_sub(debt_value)?
                    .checked_sub(perp_denominator)?
                    .checked_sub(one)?;

                let denominator = borrow_denom_price
                    .checked_mul(Decimal::one().checked_sub(checked_vault_max_ltv)?)?;

                numerator.checked_div_floor(denominator)?
            }

            BorrowTarget::Swap {
                slippage,
                denom_out,
            } => {
                let denom_out_ltv = self.get_coin_max_ltv(denom_out).unwrap();

                // The max borrow for swap can be calculated as:
                //      1 = (total_max_ltv_adjusted_value + (denom_amount_out * denom_price_out * denom_out_ltv)) / (debt_value + (max_borrow_denom_amount * borrow_denom_price))
                // denom_amount_out can be replaced by:
                //      denom_amount_out = slippage * max_borrow_denom_amount * borrow_denom_price / denom_price_out
                // This results in the following formula:
                //      1 = (total_max_ltv_adjusted_value + (slippage * max_borrow_denom_amount * borrow_denom_price * denom_out_ltv)) / (debt_value + (max_borrow_denom_amount * borrow_denom_price))
                // Re-arranging this to isolate borrow denom amount renders:
                //      max_borrow_denom_amount = (total_max_ltv_adjusted_value - debt_value) / (borrow_denom_price * (1 - slippage * denom_out_ltv))
                // Re-arranging to include perp values:
                //      max_borrow_denom_amount = (total_max_ltv_adjusted_value - debt_value - perpd + perpn) / (borrow_denom_price * (1 - slippage * denom_out_ltv))
                let out_ltv_slippage_corrected =
                    denom_out_ltv.checked_mul(Decimal::one() - slippage)?;

                let numerator = total_max_ltv_adjusted_value
                    .checked_add(perp_numerator)?
                    .checked_sub(debt_value)?
                    .checked_sub(perp_denominator)?
                    .checked_sub(one)?;

                let denominator = borrow_denom_price
                    .checked_mul(Decimal::one().checked_sub(out_ltv_slippage_corrected)?)?;

                numerator.checked_div_floor(denominator)?
            }
        };

        Ok(max_borrow_amount)
    }

    /// Estimate the max long and short size that our user can take.
    /// The max position size can be calculated as: - (b+sqr(d)) / (2*a).
    ///
    /// This function utilizes the `bigdecimal` crate (https://crates.io/crates/bigdecimal)
    /// to handle high-precision calculations. The `cosmwasm-std` library only supports up to
    /// 18 decimal places, which may not be sufficient for our needs. Additionally, working with
    /// tokens like ETH, dYdX, and Injective, which use 18 decimal for token representation, can
    /// result in very large numbers. The `bigdecimal` crate allows us to efficiently manage both
    /// large numbers and high precision while maintaining code readability.
    ///
    /// NOTE: Intended for Frontend use only !!!
    pub fn max_perp_size_estimate(
        &self,
        denom: &str,
        base_denom: &str,
        long_oi_amount: Uint128,
        short_oi_amount: Uint128,
        direction: &Direction,
    ) -> HealthResult<Int128> {
        // Prices
        let perp_oracle_price = self.get_price(denom)?;
        let base_denom_price = self.get_price(base_denom)?;

        // Perp market params
        let perp_params =
            self.perps_data.params.get(denom).ok_or(MissingPerpParams(denom.to_string()))?;
        let closing_fee_rate = perp_params.closing_fee_rate;
        let opening_fee_rate = perp_params.opening_fee_rate;
        let skew_scale = perp_params.skew_scale;
        let ltv_base_denom = self.get_coin_max_ltv(base_denom)?;
        let ltv_p = match self.kind {
            AccountKind::UsdcMargin => perp_params
                .max_loan_to_value_usdc
                .ok_or(MissingUSDCMarginParams(self.kind.to_string()))?,
            _ => perp_params.max_loan_to_value,
        };

        // The max position change amount afforded by the open interest caps, in the given direction
        let max_oi_change_amount = calculate_remaining_oi_amount(
            long_oi_amount,
            short_oi_amount,
            perp_oracle_price,
            perp_params,
            direction,
        )?;

        // Current skew
        let k = Int128::try_from(long_oi_amount)?.checked_sub(short_oi_amount.try_into()?)?;

        let (
            // Current unrealized funding
            f_amount,
            // Current size,
            q_old,
            // Entry price
            p_ex_o,
        ) = self
            .positions
            .perps
            .iter()
            .find(|&x| x.denom == *denom)
            .map_or((Int128::zero(), Int128::zero(), Decimal::zero()), |f| {
                (f.unrealized_pnl.accrued_funding, f.size, f.entry_exec_price)
            });

        // Flag to indicate if we are reducing (and possibly reopening in other direction) or we are
        // increasing the position
        let position_increasing = match direction {
            Direction::Long => !q_old.is_negative(),
            Direction::Short => q_old.is_negative(),
        };

        if max_oi_change_amount.is_zero() {
            // if position is increasing, we have no more space to increase, return 0
            if position_increasing {
                return Ok(Int128::zero());
            } else {
                // If position is decreasing, we still need to close the position
                return Ok(Int128::neg(q_old));
            }
        }

        let p_ex = closing_execution_price(k, skew_scale, q_old, perp_oracle_price)?;
        let closing_fee_value =
            q_old.unsigned_abs().checked_mul_floor(p_ex.checked_mul(closing_fee_rate)?)?;

        // Indicator functions
        let (i, i_prim) = if (q_old.is_negative() && direction == &Direction::Long)
            || (!q_old.is_negative() && direction == &Direction::Short)
        {
            // Opposite direction
            (Uint128::zero(), Uint128::one())
        } else {
            // Same direction
            (Uint128::one(), Uint128::zero())
        };

        let u_pnl = match q_old.is_zero() {
            true => Int128::zero(),
            false => {
                let bd_num: Int128 = base_denom_price.numerator().try_into()?;
                let bd_den: Int128 = base_denom_price.denominator().try_into()?;
                let f_value = f_amount.checked_multiply_ratio(bd_num, bd_den)?;
                let price_diff = SignedDecimal::try_from(p_ex)?.checked_sub(p_ex_o.try_into()?)?;
                let closing_fee_value_prim = closing_fee_value.checked_mul(i_prim)?;
                q_old
                    .checked_multiply_ratio(price_diff.numerator(), price_diff.denominator())?
                    .checked_sub(closing_fee_value_prim.try_into()?)?
                    .checked_add(f_value)?
            }
        };

        let (base_denom_collateral_value, rwa_value, debt_value) =
            self.account_composition(base_denom, denom, base_denom_price)?;

        // z = LTVp - closing fee - opening fee - 1
        let z = ltv_p.bd() - closing_fee_rate.bd() - opening_fee_rate.bd() - BigDecimal::one();

        // a = - z * (price_oracle / (2 * skew_scale)) (SHORT)
        // a = z * (price_oracle / (2 * skew_scale)) (LONG)
        let two_times_skew_scale = BigDecimal::from(2u128) * skew_scale.bd();
        let a = direction.sign().bd() * z.clone() * perp_oracle_price.bd() / two_times_skew_scale;

        // b = z * price_oracle * (1 + (k - q_old) / skew_scale)
        let b = z
            * perp_oracle_price.bd()
            * (BigDecimal::one() + (k.bd() - q_old.bd()) / skew_scale.bd());

        // c = y + i * opening_fee_rate * |q_old| * price_oracle * (1 + (k - q_old / 2) / skew_scale)
        // y = RWA - debt + c_big_max * LTV_base_denom - c_big_min
        // c_big_max = max(0, c_big)
        // c_big_min = -min(0, c_big)
        // c_big = base_denom_collateral_value + u_pnl
        let c_big = base_denom_collateral_value.bd() + u_pnl.bd();
        let c_big_max = BigDecimal::max(BigDecimal::zero(), c_big.clone());
        let c_big_min = -BigDecimal::min(BigDecimal::zero(), c_big);
        let y = rwa_value.bd() - debt_value.bd() + c_big_max * ltv_base_denom.bd() - c_big_min;
        let c = y + i.bd()
            * opening_fee_rate.bd()
            * q_old.unsigned_abs().bd()
            * perp_oracle_price.bd()
            * (BigDecimal::one()
                + (k.bd() - q_old.bd() / BigDecimal::from(2u128)) / skew_scale.bd());

        // d = b^2 - 4ac
        let d = b.square() - BigDecimal::from(4u128) * a.clone() * c;

        // q_max = - (b + sqrt(d)) / (2 * a)
        let q_max_amount =
            -(b + d.sqrt().unwrap_or(BigDecimal::zero())) / (BigDecimal::from(2u128) * a);
        let q_max_amount = q_max_amount.with_scale_round(0, RoundingMode::Down);
        let mut q_max_amount = Int128::from_str(q_max_amount.to_string().as_str())?;

        // If we are increasing the position, we need to adjust the max oi to include the current position.
        // For example:
        // net oi = 20
        // max net oi = 30
        // current position = 10
        // q max = 25
        // available oi = 30 - 20 = 10
        // max oi is adjusted to be the existing position = 10 + 10 = 20
        // After that we reduce the q_max by the current position = 20 - 10 = 0
        let position_adjusted_max_oi_change_amount = if !q_old.is_zero() && position_increasing {
            max_oi_change_amount.checked_add(q_old.unsigned_abs())?
        } else {
            max_oi_change_amount
        };

        // Cap our size by remaining space in OI caps
        if q_max_amount.unsigned_abs() > position_adjusted_max_oi_change_amount {
            q_max_amount = position_adjusted_max_oi_change_amount.try_into()?;
        };
        if direction == &Direction::Short {
            q_max_amount = Int128::zero().checked_sub(q_max_amount)?;
        }

        // If the current size is already greater than the max allowed size, we should return 0
        if self.current_size_exceeds_max_for_direction(direction, q_old, q_max_amount) {
            return Ok(Int128::zero());
        }

        // Deduct current size from max amount
        q_max_amount = q_max_amount.checked_sub(q_old)?;

        Ok(q_max_amount)
    }

    fn current_size_exceeds_max_for_direction(
        &self,
        direction: &Direction,
        q_old: Int128,
        q_max: Int128,
    ) -> bool {
        match direction {
            Direction::Long => q_old > q_max,
            Direction::Short => q_old < q_max,
        }
    }

    fn account_composition(
        &self,
        base_denom: &str,
        denom: &str,
        base_denom_price: Decimal,
    ) -> HealthResult<(Uint128, Uint128, Uint128)> {
        let (base_denom_deposits, other_deposits): (Vec<_>, Vec<_>) =
            self.positions.deposits.iter().partition(|deposit| deposit.denom == base_denom);

        // there is only one base denom deposit
        let account_base_denom_deposits =
            base_denom_deposits.first().map_or(Uint128::zero(), |d| d.amount);

        let (base_denom_lends, other_lends): (Vec<_>, Vec<_>) =
            self.positions.lends.iter().partition(|lend| lend.denom == base_denom);
        let account_base_denom_lends =
            base_denom_lends.first().map_or(Uint128::zero(), |l| l.amount);

        let filtered_perps: Vec<_> =
            self.positions.perps.iter().filter(|x| x.denom != denom).cloned().collect();

        // (named c_usdc in docs + sheet)
        // Refers to the value of collateral the user has in the base_denom (e.g usdc)
        let base_denom_collateral_value = account_base_denom_deposits
            .checked_add(account_base_denom_lends)?
            .checked_mul_floor(base_denom_price)?;

        let deref_deposits: Vec<Coin> = other_deposits.into_iter().cloned().collect();
        let deref_lends: Vec<Coin> = other_lends.into_iter().cloned().collect();

        let assets_ltv_adjusted_value = self
            .coins_value(deref_deposits.as_slice())?
            .max_ltv_adjusted_collateral
            .checked_add(self.coins_value(deref_lends.as_slice())?.max_ltv_adjusted_collateral)?
            .checked_add(self.vaults_value()?.max_ltv_adjusted_collateral)?;

        // Contains denominator / numerator for HF for all perps *excluding* a perp position for given denom
        let perp_hf_values = self.perp_hf_values_and_pnl(&filtered_perps)?.0;

        // Risk Weighted Assets (rwa) are assets other than base_denom and the perp position being considered, weighted using corresponding Maximum LTVs
        let other_collateral_value =
            assets_ltv_adjusted_value.checked_add(perp_hf_values.max_ltv_numerator)?;

        // raw_debt = all debt and everything from the denominator of perps besides
        // the position for given denom.
        let mut raw_debt_value = Uint128::zero();

        for d in &self.positions.debts {
            let price = self
                .oracle_prices
                .get(&d.denom)
                .ok_or_else(|| MissingPrice(d.denom.to_string()))?;

            let product = d.amount.checked_mul_ceil(*price)?;
            raw_debt_value += product;
        }

        // debt = raw_debt + max_ltv_denominator for perp positions *excluding* a perp position for given denom
        let debt_value = raw_debt_value.checked_add(perp_hf_values.max_ltv_denominator)?;

        Ok((base_denom_collateral_value, other_collateral_value, debt_value))
    }

    fn perp_hf_values_and_pnl(
        &self,
        perps: &[PerpPosition],
    ) -> HealthResult<(PerpHealthFactorValues, PerpPnlValues)> {
        let mut max_ltv_numerator = Uint128::zero();
        let mut max_ltv_denominator = Uint128::zero();
        let mut liq_ltv_numerator = Uint128::zero();
        let mut liq_ltv_denominator = Uint128::zero();
        let mut profit = Uint128::zero();
        let mut loss = Uint128::zero();

        for position in perps.iter() {
            let base_denom_price = self.get_price(&position.base_denom)?;

            match &position.unrealized_pnl.to_coins(&position.base_denom).pnl {
                // Round down the profits to be conservative
                PnL::Profit(pnl) => {
                    profit = profit.checked_add(pnl.amount.checked_mul_floor(base_denom_price)?)?
                }
                // Round up the losses to be conservative
                PnL::Loss(pnl) => {
                    loss = loss.checked_add(pnl.amount.checked_mul_ceil(base_denom_price)?)?
                }
                _ => {}
            }

            let perp_health_factor_values = self.perp_health_factor_values(position)?;
            max_ltv_numerator =
                max_ltv_numerator.checked_add(perp_health_factor_values.max_ltv_numerator)?;
            max_ltv_denominator =
                max_ltv_denominator.checked_add(perp_health_factor_values.max_ltv_denominator)?;
            liq_ltv_numerator =
                liq_ltv_numerator.checked_add(perp_health_factor_values.liq_ltv_numerator)?;
            liq_ltv_denominator =
                liq_ltv_denominator.checked_add(perp_health_factor_values.liq_ltv_denominator)?;
        }

        Ok((
            PerpHealthFactorValues {
                max_ltv_numerator,
                max_ltv_denominator,
                liq_ltv_numerator,
                liq_ltv_denominator,
            },
            PerpPnlValues {
                loss,
                profit,
            },
        ))
    }

    fn perp_health_factor_values(
        &self,
        position: &PerpPosition,
    ) -> HealthResult<PerpHealthFactorValues> {
        let denom = &position.denom;
        let base_denom = &position.base_denom;
        let base_denom_price = self.get_price(base_denom)?;

        let perp_params =
            self.perps_data.params.get(denom).ok_or(MissingPerpParams(denom.to_string()))?;
        let closing_rate = perp_params.closing_fee_rate;

        // Perp(0)
        let position_value_entry =
            position.size.unsigned_abs().checked_mul_floor(position.entry_exec_price)?;

        // Perp(t)
        let position_value_current =
            position.size.unsigned_abs().checked_mul_floor(position.current_exec_price)?;

        // Borrow and liquidation ltv maximums for the perp and the funding denom
        // It was agreed to change LTV in the formula from usdc to perp ltv, as it should be more
        // conservative (when we make usdc LTV greater than or equal to any perp LTV)
        let checked_max_ltv = self.get_perp_max_ltv(denom)?;
        let checked_liq_ltv = self.get_perp_liq_ltv(denom)?;
        let (funding_min, funding_max) = self.get_min_and_max_funding_amounts(position)?;
        let funding_min_value = funding_min.checked_mul_floor(base_denom_price)?;
        let funding_max_value_ltv =
            funding_max.checked_mul_floor(base_denom_price.checked_mul(checked_max_ltv)?)?;
        let funding_max_value_liq =
            funding_max.checked_mul_floor(base_denom_price.checked_mul(checked_liq_ltv)?)?;

        // There are two different HF calculations, depending on if the perp
        // position is long or short.
        // For shorts, Health Factor = Perp(0) + (funding max accrued * base denom price * perp ltv)  / (Perp (t) * (2 - MaxLTV + trading fee) + funding min * base denom price
        // For longs, Health Factor = (Perp (t) * (LTV-trading fee) + funding max * base denom price * perp ltv  / Perp (t0) + funding min * base denom price
        // If perp size is negative the position is short, positive long
        if position.size.is_negative() {
            // Numerator = position value(0) + (positive funding * perp ltv * base denom price)
            let max_ltv_numerator = position_value_entry.checked_add(funding_max_value_ltv)?;
            let liq_ltv_numerator = position_value_entry.checked_add(funding_max_value_liq)?;

            // Denominator = position value(t) * (2 - max ltv + closing fee) + negative funding
            // Safe math because max ltv is always less than 2 (it is < 1 actually)
            let max_ltv_denominator = position_value_current
                .checked_mul_floor(
                    Decimal::from_str("2.0")?
                        .checked_sub(checked_max_ltv)?
                        .checked_add(closing_rate)?,
                )?
                .checked_add(funding_min_value)?;

            let liq_ltv_denominator = position_value_current
                .checked_mul_floor(
                    Decimal::from_str("2.0")?
                        .checked_sub(checked_liq_ltv)?
                        .checked_add(closing_rate)?,
                )?
                .checked_add(funding_min_value)?;

            Ok(PerpHealthFactorValues {
                liq_ltv_numerator,
                liq_ltv_denominator,
                max_ltv_numerator,
                max_ltv_denominator,
            })
        } else {
            // If our ltvs are less than the closing rate we will get overflow, so we
            // need to protect against this
            let checked_max_ltv_multiplier = checked_max_ltv.saturating_sub(closing_rate);
            let checked_liq_ltv_multiplier = checked_liq_ltv.saturating_sub(closing_rate);

            // Numerator = position value(0) + (positive funding * denom ltv)
            let max_ltv_numerator = position_value_current
                .checked_mul_floor(checked_max_ltv_multiplier)?
                .checked_add(funding_max_value_ltv)?;

            let liq_ltv_numerator = position_value_current
                .checked_mul_floor(checked_liq_ltv_multiplier)?
                .checked_add(funding_max_value_liq)?;

            // Denominator = position value(0) + negative funding
            let denominator = position_value_entry.checked_add(funding_min_value)?;

            Ok(PerpHealthFactorValues {
                liq_ltv_numerator,
                liq_ltv_denominator: denominator,
                max_ltv_numerator,
                max_ltv_denominator: denominator,
            })
        }
        // else perp size is zero - safe to do nothing? we should never get into this situation
        // but if we do we probably don't want to brick the HF calculation
    }

    fn total_collateral_value(&self) -> HealthResult<CollateralValue> {
        let deposits = self.coins_value(&self.positions.deposits)?;
        let lends = self.coins_value(&self.positions.lends)?;
        let vaults = self.vaults_value()?;
        let staked_lp = self.coins_value(&self.positions.staked_astro_lps)?;

        Ok(CollateralValue {
            total_collateral_value: deposits
                .total_collateral_value
                .checked_add(vaults.total_collateral_value)?
                .checked_add(lends.total_collateral_value)?
                .checked_add(staked_lp.total_collateral_value)?,
            max_ltv_adjusted_collateral: deposits
                .max_ltv_adjusted_collateral
                .checked_add(vaults.max_ltv_adjusted_collateral)?
                .checked_add(lends.max_ltv_adjusted_collateral)?
                .checked_add(staked_lp.max_ltv_adjusted_collateral)?,
            liq_ltv_adjusted_collateral: deposits
                .liq_ltv_adjusted_collateral
                .checked_add(vaults.liq_ltv_adjusted_collateral)?
                .checked_add(lends.liq_ltv_adjusted_collateral)?
                .checked_add(staked_lp.liq_ltv_adjusted_collateral)?,
        })
    }

    fn coins_value(&self, coins: &[Coin]) -> HealthResult<CollateralValue> {
        let mut total_collateral_value = Uint128::zero();
        let mut max_ltv_adjusted_collateral = Uint128::zero();
        let mut liq_ltv_adjusted_collateral = Uint128::zero();

        for c in coins {
            let Some(AssetParams {
                credit_manager:
                    CmSettings {
                        hls,
                        ..
                    },
                liquidation_threshold,
                ..
            }) = self.coin_contribution_to_collateral(c)?
            else {
                continue;
            };

            let coin_price = self.get_price(&c.denom)?;
            let coin_value = c.amount.checked_mul_floor(coin_price)?;
            total_collateral_value = total_collateral_value.checked_add(coin_value)?;

            let checked_max_ltv = self.get_coin_max_ltv(&c.denom)?;

            let max_ltv_adjusted = coin_value.checked_mul_floor(checked_max_ltv)?;
            max_ltv_adjusted_collateral =
                max_ltv_adjusted_collateral.checked_add(max_ltv_adjusted)?;

            let checked_liquidation_threshold = match self.kind {
                AccountKind::HighLeveredStrategy => {
                    hls.as_ref().ok_or(MissingHLSParams(c.denom.clone()))?.liquidation_threshold
                }
                _ => *liquidation_threshold,
            };
            let liq_adjusted = coin_value.checked_mul_floor(checked_liquidation_threshold)?;
            liq_ltv_adjusted_collateral = liq_ltv_adjusted_collateral.checked_add(liq_adjusted)?;
        }
        Ok(CollateralValue {
            total_collateral_value,
            max_ltv_adjusted_collateral,
            liq_ltv_adjusted_collateral,
        })
    }

    fn coin_contribution_to_collateral(&self, coin: &Coin) -> HealthResult<Option<&AssetParams>> {
        let Some(asset_params) = self.asset_params.get(&coin.denom) else {
            // If the coin is not found (whitelisted), it is not considered for collateral
            return Ok(None);
        };

        match self.kind {
            AccountKind::HighLeveredStrategy => {
                // HLS should have 0 or 1 debt denom in the account. If there are more than 1 we can safely calculate the collateral value
                // because the rule will be checked in the Credit Manager contract and won't allow more than 1 debt denom in the account.
                if !self.positions.debts.is_empty() {
                    let mut correlations = vec![];
                    for debt in self.positions.debts.iter() {
                        let debt_params = self
                            .asset_params
                            .get(&debt.denom)
                            .ok_or(MissingAssetParams(debt.denom.clone()))?;
                        let debt_hls = debt_params
                            .credit_manager
                            .hls
                            .as_ref()
                            .ok_or(MissingHLSParams(debt.denom.clone()))?;

                        // collect all the correlations of the debts
                        correlations.extend(&debt_hls.correlations);
                    }

                    // If the collateral is not correlated with any of the debts, skip it.
                    // It doesn't contribute to the collateral value.
                    if !correlations.contains(&&HlsAssetType::Coin {
                        denom: coin.denom.clone(),
                    }) {
                        return Ok(None);
                    }
                } else if asset_params.credit_manager.hls.is_none() {
                    // Only collateral with hls params can be used in an HLS account and can contribute to the collateral value
                    return Ok(None);
                }
            }
            AccountKind::UsdcMargin => {}
            AccountKind::Default => {}
            AccountKind::FundManager {
                ..
            } => {}
        }

        Ok(Some(asset_params))
    }

    fn vaults_value(&self) -> HealthResult<CollateralValue> {
        let mut total_collateral_value = Uint128::zero();
        let mut max_ltv_adjusted_collateral = Uint128::zero();
        let mut liq_ltv_adjusted_collateral = Uint128::zero();

        for v in &self.positions.vaults {
            // Step 1: Calculate Vault coin values
            let values = self
                .vaults_data
                .vault_values
                .get(&v.vault.address)
                .ok_or(MissingVaultValues(v.vault.address.to_string()))?;

            total_collateral_value = total_collateral_value.checked_add(values.vault_coin.value)?;

            let VaultConfig {
                addr,
                max_loan_to_value,
                liquidation_threshold,
                whitelisted,
                hls,
                ..
            } = self
                .vaults_data
                .vault_configs
                .get(&v.vault.address)
                .ok_or(MissingVaultConfig(v.vault.address.to_string()))?;

            let base_params = self
                .asset_params
                .get(&values.base_coin.denom)
                .ok_or(MissingAssetParams(values.base_coin.denom.clone()))?;

            // If vault or base token has been de-listed, drop MaxLTV to zero
            let checked_vault_max_ltv = if *whitelisted && base_params.credit_manager.whitelisted {
                match self.kind {
                    AccountKind::Default => *max_loan_to_value,
                    AccountKind::UsdcMargin => *max_loan_to_value,
                    AccountKind::FundManager {
                        ..
                    } => *max_loan_to_value,
                    AccountKind::HighLeveredStrategy => {
                        hls.as_ref().ok_or(MissingHLSParams(addr.to_string()))?.max_loan_to_value
                    }
                }
            } else {
                Decimal::zero()
            };

            max_ltv_adjusted_collateral = values
                .vault_coin
                .value
                .checked_mul_floor(checked_vault_max_ltv)?
                .checked_add(max_ltv_adjusted_collateral)?;

            let checked_liquidation_threshold = match self.kind {
                AccountKind::Default => *liquidation_threshold,
                AccountKind::UsdcMargin => *liquidation_threshold,
                AccountKind::FundManager {
                    ..
                } => *liquidation_threshold,
                AccountKind::HighLeveredStrategy => {
                    hls.as_ref().ok_or(MissingHLSParams(addr.to_string()))?.liquidation_threshold
                }
            };

            liq_ltv_adjusted_collateral = values
                .vault_coin
                .value
                .checked_mul_floor(checked_liquidation_threshold)?
                .checked_add(liq_ltv_adjusted_collateral)?;

            // Step 2: Calculate Base coin values
            let res = self.coins_value(&[Coin {
                denom: values.base_coin.denom.clone(),
                amount: v.amount.unlocking().total(),
            }])?;
            total_collateral_value =
                total_collateral_value.checked_add(res.total_collateral_value)?;
            max_ltv_adjusted_collateral =
                max_ltv_adjusted_collateral.checked_add(res.max_ltv_adjusted_collateral)?;
            liq_ltv_adjusted_collateral =
                liq_ltv_adjusted_collateral.checked_add(res.liq_ltv_adjusted_collateral)?;
        }

        Ok(CollateralValue {
            total_collateral_value,
            max_ltv_adjusted_collateral,
            liq_ltv_adjusted_collateral,
        })
    }

    /// Total value of all spot debts.
    ///
    /// Denominated in the protocol's base asset (typically USDC).
    fn debt_value(&self) -> HealthResult<Uint128> {
        let mut total = Uint128::zero();

        // spot debt borrowed from redbank
        for debt in &self.positions.debts {
            let coin_price = self.get_price(&debt.denom)?;
            let debt_value = debt.amount.checked_mul_ceil(coin_price)?;
            total = total.checked_add(debt_value)?;
        }

        Ok(total)
    }

    fn calculate_liq_hf(
        &self,
        liq_adjusted_collateral: &Uint128,
        spot_debt_value: &Uint128,
        perp_hf_values: &PerpHealthFactorValues,
    ) -> HealthResult<Decimal> {
        Ok(Decimal::checked_from_ratio(
            liq_adjusted_collateral.checked_add(perp_hf_values.liq_ltv_numerator)?,
            spot_debt_value.checked_add(perp_hf_values.liq_ltv_denominator)?,
        )?)
    }

    fn get_perp_max_ltv(&self, denom: &str) -> HealthResult<Decimal> {
        let params =
            self.perps_data.params.get(denom).ok_or(MissingPerpParams(denom.to_string()))?;

        if !params.enabled {
            return Ok(Decimal::zero());
        }

        match self.kind {
            AccountKind::Default => Ok(params.max_loan_to_value),
            AccountKind::UsdcMargin => Ok(params
                .max_loan_to_value_usdc
                .ok_or(MissingUSDCMarginParams(self.kind.to_string()))?),
            AccountKind::FundManager {
                ..
            } => Ok(params.max_loan_to_value),
            _ => Ok(params.max_loan_to_value),
        }
    }

    fn get_perp_liq_ltv(&self, denom: &str) -> HealthResult<Decimal> {
        let params =
            self.perps_data.params.get(denom).ok_or(MissingPerpParams(denom.to_string()))?;

        if !params.enabled {
            return Ok(Decimal::zero());
        }

        match self.kind {
            AccountKind::Default => Ok(params.liquidation_threshold),
            AccountKind::UsdcMargin => Ok(params
                .liquidation_threshold_usdc
                .ok_or(MissingUSDCMarginParams(self.kind.to_string()))?),
            AccountKind::FundManager {
                ..
            } => Ok(params.liquidation_threshold),
            _ => Ok(params.liquidation_threshold),
        }
    }

    fn get_coin_max_ltv(&self, denom: &str) -> HealthResult<Decimal> {
        let params = self.asset_params.get(denom);

        match params {
            Some(params) => {
                // If the coin has been de-listed, drop MaxLTV to zero
                if !params.credit_manager.whitelisted {
                    return Ok(Decimal::zero());
                }

                match self.kind {
                    AccountKind::Default => Ok(params.max_loan_to_value),
                    AccountKind::UsdcMargin => Ok(params.max_loan_to_value),
                    AccountKind::FundManager {
                        ..
                    } => Ok(params.max_loan_to_value),
                    AccountKind::HighLeveredStrategy => Ok(params
                        .credit_manager
                        .hls
                        .as_ref()
                        .ok_or(MissingHLSParams(denom.to_string()))?
                        .max_loan_to_value),
                }
            }
            None => {
                // If the asset is not listed, set MaxLtv to zero
                Ok(Decimal::zero())
            }
        }
    }

    fn get_coin_liq_ltv(&self, denom: &str) -> HealthResult<Decimal> {
        let params = self.asset_params.get(denom);

        match params {
            Some(params) => {
                // If the coin has been de-listed, drop LiqLTV to zero
                if !params.credit_manager.whitelisted {
                    return Ok(Decimal::zero());
                }

                match self.kind {
                    AccountKind::Default => Ok(params.liquidation_threshold),
                    AccountKind::UsdcMargin => Ok(params.liquidation_threshold),
                    AccountKind::FundManager {
                        ..
                    } => Ok(params.liquidation_threshold),
                    AccountKind::HighLeveredStrategy => Ok(params
                        .credit_manager
                        .hls
                        .as_ref()
                        .ok_or(MissingHLSParams(denom.to_string()))?
                        .liquidation_threshold),
                }
            }
            None => {
                // If the asset is not listed, set LiqLtv to zero
                Ok(Decimal::zero())
            }
        }
    }

    fn get_coin_from_positions(&self, denom: &str) -> HealthResult<Coin> {
        let deposited_coin = self.positions.deposits.iter().find(|c| c.denom == denom);
        let deposited_amount = deposited_coin.unwrap_or(&Coin::default()).amount;

        let lent_coin = self.positions.lends.iter().find(|c| c.denom == denom);
        let lent_amount = lent_coin.unwrap_or(&Coin::default()).amount;

        let staked_coin = self.positions.staked_astro_lps.iter().find(|c| c.denom == denom);
        let staked_amount = staked_coin.unwrap_or(&Coin::default()).amount;

        Ok(Coin {
            denom: denom.to_string(),
            amount: deposited_amount.checked_add(lent_amount)?.checked_add(staked_amount)?,
        })
    }

    fn get_min_and_max_funding_amounts(
        &self,
        position: &PerpPosition,
    ) -> HealthResult<(Uint128, Uint128)> {
        let accrued_funding_amount = position.unrealized_pnl.accrued_funding;

        // funding_max = max(0, unrealized_funding_accrued)
        let funding_max = max(Int128::zero(), accrued_funding_amount);
        // safe to use Uint128 because of the max function above
        let funding_max = funding_max.unsigned_abs();

        // funding min = -min(0, unrealized_funding_accrued)
        let funding_min = if accrued_funding_amount.is_negative() {
            accrued_funding_amount.unsigned_abs()
        } else {
            Uint128::zero()
        };

        Ok((funding_min, funding_max))
    }

    fn get_price(&self, denom: &str) -> HealthResult<Decimal> {
        let price = self.oracle_prices.get(denom).ok_or(MissingPrice(denom.to_string()))?;
        Ok(*price)
    }

    pub fn liquidation_price(
        &self,
        denom: &str,
        kind: &LiquidationPriceKind,
    ) -> HealthResult<Decimal> {
        let debt_value = self.debt_value()?;
        let current_price = self.get_price(denom)?;
        let collateral_ltv_value = self.total_collateral_value()?.liq_ltv_adjusted_collateral;
        let (perps_hf_values, _) = self.perp_hf_values_and_pnl(&self.positions.perps)?;

        // When debt and liq_ltv_denominator are zero, there is no debt, so also no
        // liquidation price
        if debt_value.checked_add(perps_hf_values.liq_ltv_denominator)?.is_zero() {
            return Ok(Decimal::zero());
        }

        let liq_hf = self.calculate_liq_hf(&collateral_ltv_value, &debt_value, &perps_hf_values)?;

        // If the account is unhealthy, the liquidation price is the current price
        if liq_hf < Decimal::one() {
            return Ok(current_price);
        }

        match kind {
            LiquidationPriceKind::Asset => {
                // liq_price = lhs / rhs
                // lhs = debt + asset_ltv_value + perps_den - col_ltv_value - perps_num
                // rhs = size * liq_ltv
                let asset_amount = self.get_coin_from_positions(denom)?.amount;
                if asset_amount.is_zero() {
                    return Err(MissingAmount(denom.to_string()));
                }

                let asset_ltv = self.get_coin_liq_ltv(denom)?;

                let asset_ltv_value =
                    asset_amount.checked_mul_floor(current_price.checked_mul(asset_ltv)?)?;

                let lhs_positives = debt_value
                    .checked_add(asset_ltv_value)?
                    .checked_add(perps_hf_values.liq_ltv_denominator)?;
                let lhs_negatives =
                    collateral_ltv_value.checked_add(perps_hf_values.liq_ltv_numerator)?;

                if lhs_negatives >= lhs_positives {
                    return Ok(Decimal::zero());
                };

                let lhs = lhs_positives - lhs_negatives;
                let rhs = asset_amount.checked_mul_floor(asset_ltv)?;

                Ok(Decimal::from_ratio(lhs, rhs))
            }

            LiquidationPriceKind::Debt => {
                // liq_price = lhs / rhs
                // lhs = col_ltv_value + debt_value_asset + perps_num - perps_denom - debt
                // rhs = size
                let debt_amount = self
                    .positions
                    .debts
                    .iter()
                    .find(|c| c.denom == denom)
                    .ok_or(MissingAmount(denom.to_string()))?
                    .amount;

                if debt_amount.is_zero() {
                    return Err(MissingAmount(denom.to_string()));
                }

                let asset_debt_value = debt_amount.checked_mul_ceil(current_price)?;

                let lhs_positives = collateral_ltv_value
                    .checked_add(asset_debt_value)?
                    .checked_add(perps_hf_values.liq_ltv_numerator)?;
                let lhs_negatives = perps_hf_values.liq_ltv_denominator.checked_add(debt_value)?;

                if lhs_negatives >= lhs_positives {
                    return Ok(Decimal::zero());
                };

                let lhs = lhs_positives - lhs_negatives;

                Ok(Decimal::from_ratio(lhs, debt_amount))
            }

            LiquidationPriceKind::Perp => {
                let perp_position = self
                    .positions
                    .perps
                    .iter()
                    .find(|x| x.denom == *denom)
                    .ok_or(DenomNotPresent(denom.to_string()))?;

                if perp_position.size.is_zero() {
                    return Err(MissingAmount(denom.to_string()));
                }

                let closing_rate = self
                    .perps_data
                    .params
                    .get(denom)
                    .ok_or(MissingPerpParams(denom.to_string()))?
                    .closing_fee_rate;

                let perp_ltv = self.get_perp_liq_ltv(denom)?;
                let current_perp_price = perp_position.current_exec_price;

                match perp_position.size.is_negative() {
                    // LONG position
                    // ----------------
                    // liq_price = lhs / rhs
                    // lhs = debt + perps_den + market_val_num - col - perps_num
                    // rhs = abs(size) * (perps_liq_ltv - closing_rate)
                    false => {
                        let market_value_num =
                            self.perp_health_factor_values(perp_position)?.liq_ltv_numerator;

                        let lhs_positives = debt_value
                            .checked_add(perps_hf_values.liq_ltv_denominator)?
                            .checked_add(market_value_num)?;

                        let lhs_negatives =
                            collateral_ltv_value.checked_add(perps_hf_values.liq_ltv_numerator)?;

                        if lhs_negatives >= lhs_positives {
                            return Ok(Decimal::zero());
                        };

                        let lhs = Decimal::from_atomics(lhs_positives - lhs_negatives, 0)?;
                        let rhs = Decimal::from_atomics(perp_position.size.unsigned_abs(), 0)?
                            .checked_mul(perp_ltv.checked_sub(closing_rate)?)?;

                        Ok(lhs.checked_div(rhs)?)
                    }
                    // SHORT position
                    // ----------------
                    // liq_price = lhs / rhs
                    // lhs = col + perps_num + curr_exposure * ltv_adjusted - debt - perps_den
                    // rhs = abs(size) * ltv_adjusted
                    // ----------------
                    // ltv_adjusted = 2 - perps_liq_ltv + closing_rate
                    // curr_exposure = abs(size) * perps_current_price
                    true => {
                        let ltv_adjusted = Decimal::from_str("2")?
                            .checked_sub(perp_ltv)?
                            .checked_add(closing_rate)?;

                        let curr_exposure_ltv_adjusted = perp_position
                            .size
                            .unsigned_abs()
                            .checked_mul_ceil(current_perp_price.checked_mul(ltv_adjusted)?)?;

                        let lhs_positives = collateral_ltv_value
                            .checked_add(perps_hf_values.liq_ltv_numerator)?
                            .checked_add(curr_exposure_ltv_adjusted)?;

                        let lhs_negatives =
                            debt_value.checked_add(perps_hf_values.liq_ltv_denominator)?;

                        if lhs_negatives >= lhs_positives {
                            return Ok(Decimal::zero());
                        };

                        let lhs = lhs_positives - lhs_negatives;
                        let rhs =
                            perp_position.size.unsigned_abs().checked_mul_ceil(ltv_adjusted)?;

                        Ok(Decimal::from_ratio(lhs, rhs))
                    }
                }
            }
        }
    }
}
