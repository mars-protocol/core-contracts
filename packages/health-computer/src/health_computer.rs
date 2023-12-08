use std::{cmp::min, str::FromStr};

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, Decimal, Uint128};
use mars_types::{
    credit_manager::Positions,
    health::{
        AccountKind, BorrowTarget, Health,
        HealthError::{
            MissingHLSParams, MissingParams, MissingPrice, MissingVaultConfig, MissingVaultValues,
        },
        HealthResult, SwapKind,
    },
    math::SignedDecimal,
    params::{AssetParams, CmSettings, VaultConfig},
    perps::{PerpPosition, PnL},
};
#[cfg(feature = "javascript")]
use tsify::Tsify;

use crate::{CollateralValue, DenomsData, PerpHealthFactorValues, PerpPnlValues, VaultsData};

/// `HealthComputer` is a shared struct with the frontend that gets compiled to wasm.
/// For this reason, it uses a dependency-injection-like pattern where all required data is needed up front.
#[cw_serde]
#[cfg_attr(feature = "javascript", derive(Tsify))]
#[cfg_attr(feature = "javascript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct HealthComputer {
    pub kind: AccountKind,
    pub positions: Positions,
    pub denoms_data: DenomsData,
    pub vaults_data: VaultsData,
}

impl HealthComputer {
    pub fn compute_health(&self) -> HealthResult<Health> {
        let CollateralValue {
            total_collateral_value,
            max_ltv_adjusted_collateral,
            liquidation_threshold_adjusted_collateral,
        } = self.total_collateral_value()?;

        let liquidation_threshold_adjusted_collateral_dec: SignedDecimal =
            liquidation_threshold_adjusted_collateral.into();
        let max_ltv_adjusted_collateral_dec: SignedDecimal = max_ltv_adjusted_collateral.into();
        let spot_debt_value: SignedDecimal = self.spot_debt_value()?.into();

        let perp_hf_values = self.perp_health_factor_values()?;

        let (max_ltv_health_factor, liquidation_health_factor) =
            if spot_debt_value.is_zero() && self.positions.perps.is_empty() {
                (None, None)
            } else {
                // HF = (RWA + perp_numerator) / (spot_debt + perp_denominator)
                // where
                // RWA = risk weighted assets (i.e ltv * collateral_value)
                // spot debt = total value of borrowed assets (does not include perp unrealised pnl)

                let max_ltv_hf = Decimal::checked_from_ratio(
                    max_ltv_adjusted_collateral_dec
                        .checked_add(perp_hf_values.max_ltv_numerator)?
                        .abs
                        .to_uint_floor(),
                    spot_debt_value
                        .checked_add(perp_hf_values.max_ltv_denominator)?
                        .abs
                        .to_uint_floor(),
                )?;
                let liq_hf = Decimal::checked_from_ratio(
                    liquidation_threshold_adjusted_collateral_dec
                        .checked_add(perp_hf_values.liq_ltv_numerator)?
                        .abs
                        .to_uint_floor(),
                    spot_debt_value
                        .checked_add(perp_hf_values.liq_ltv_denominator)?
                        .abs
                        .to_uint_floor(),
                )?;
                (Some(max_ltv_hf), Some(liq_hf))
            };

        Ok(Health {
            total_debt_value: spot_debt_value.abs.to_uint_floor(),
            total_collateral_value,
            max_ltv_adjusted_collateral,
            liquidation_threshold_adjusted_collateral,
            max_ltv_health_factor,
            liquidation_health_factor,
            perp_pnl_profit: perp_hf_values.pnl_values.profit,
            perp_pnl_losses: perp_hf_values.pnl_values.loss,
        })
    }

    /// The max this account can withdraw of `withdraw_denom` and maintain max_ltv >= 1
    /// Note: This is an estimate. Guarantees to leave account healthy, but in edge cases,
    /// due to rounding, it may be slightly too conservative.
    pub fn max_withdraw_amount_estimate(&self, withdraw_denom: &str) -> HealthResult<Uint128> {
        // Both deposits and lends should be considered, as the funds can automatically be un-lent and
        // and also used to withdraw.
        let withdraw_coin = self.get_coin_from_deposits_and_lends(withdraw_denom)?;
        if withdraw_coin.amount.is_zero() {
            return Ok(Uint128::zero());
        };

        let params = self
            .denoms_data
            .params
            .get(withdraw_denom)
            .ok_or(MissingParams(withdraw_denom.to_string()))?;

        // If no debt or coin is blacklisted (meaning does not contribute to max ltv hf),
        // the total amount deposited can be withdrawn
        if (self.positions.debts.is_empty() && self.positions.perps.is_empty())
            || !params.credit_manager.whitelisted
        {
            return Ok(withdraw_coin.amount);
        }

        // withdraw denom max ltv adjusted value = total max ltv adjusted value - debt value - perp_denominator + perp_numerator
        let total_max_ltv_adjusted_value: SignedDecimal =
            self.total_collateral_value()?.max_ltv_adjusted_collateral.into();
        let debt_value: SignedDecimal = self.spot_debt_value()?.into();

        let withdraw_denom_price: SignedDecimal = (*self
            .denoms_data
            .prices
            .get(withdraw_denom)
            .ok_or(MissingPrice(withdraw_denom.to_string()))?)
        .into();

        let withdraw_denom_max_ltv: SignedDecimal = match self.kind {
            AccountKind::Default => params.max_loan_to_value,
            AccountKind::HighLeveredStrategy => {
                params
                    .credit_manager
                    .hls
                    .as_ref()
                    .ok_or(MissingHLSParams(withdraw_denom.to_string()))?
                    .max_loan_to_value
            }
        }
        .into();

        let PerpHealthFactorValues {
            max_ltv_denominator: perp_denominator,
            max_ltv_numerator: perp_numerator,
            ..
        } = self.perp_health_factor_values()?;

        // We often add one to calcs for a margin of error
        let one = SignedDecimal::one();

        // If we have any perps or debt, we need to check our health before continuing
        if !self.positions.perps.is_empty() || debt_value.abs > Decimal::zero() {
            let hf = total_max_ltv_adjusted_value
                .checked_add(perp_numerator)?
                .checked_div(debt_value.checked_add(perp_denominator)?)?;

            // Zero borrowable if unhealthy
            if hf.abs.le(&one.abs) {
                return Ok(Uint128::zero());
            }
        }

        // The max withdraw amount is calculated as:
        // withdraw denom max ltv adjusted value = total max ltv adjusted value - debt value - perp_denominator + perp_numerator
        let max_withdraw_value = total_max_ltv_adjusted_value
            .checked_sub(debt_value)?
            .checked_sub(perp_denominator)?
            .checked_add(perp_numerator)?
            .checked_sub(one)?;

        // The above is the raw value, now we need to factor in price and LTV impact
        let max_withdraw_amount = max_withdraw_value
            .checked_div(withdraw_denom_price.checked_mul(withdraw_denom_max_ltv)?)?;

        Ok(min(max_withdraw_amount.abs.to_uint_floor(), withdraw_coin.amount))
    }

    pub fn max_swap_amount_estimate(
        &self,
        from_denom: &str,
        to_denom: &str,
        kind: &SwapKind,
    ) -> HealthResult<Uint128> {
        // Both deposits and lends should be considered, as the funds can automatically be un-lent and
        // and also used to swap.
        let from_coin = self.get_coin_from_deposits_and_lends(from_denom)?;

        // If no debt the total amount deposited can be swapped (only for default swaps)
        if kind == &SwapKind::Default
            && self.positions.debts.is_empty()
            && self.positions.perps.is_empty()
        {
            return Ok(from_coin.amount);
        }

        let total_max_ltv_adjusted_value: SignedDecimal =
            self.total_collateral_value()?.max_ltv_adjusted_collateral.into();

        let debt_value: SignedDecimal = self.spot_debt_value()?.into();

        let PerpHealthFactorValues {
            max_ltv_denominator: perp_denominator,
            max_ltv_numerator: perp_numerator,
            ..
        } = self.perp_health_factor_values()?;

        let one = SignedDecimal::one();

        if !self.positions.perps.is_empty() || debt_value.abs > Decimal::zero() {
            let hf = total_max_ltv_adjusted_value
                .checked_add(perp_numerator)?
                .checked_div(debt_value.checked_add(perp_denominator)?)?;

            // Zero borrowable if unhealthy
            if hf.abs.le(&one.abs) {
                return Ok(Uint128::zero());
            }
        }

        let from_ltv = self.get_coin_max_ltv(from_denom)?;
        let to_ltv = self.get_coin_max_ltv(to_denom)?;

        // Don't allow swapping when one of the assets is not whitelisted
        if from_ltv == Decimal::zero() || to_ltv == Decimal::zero() {
            return Ok(Uint128::zero());
        }

        let from_price =
            self.denoms_data.prices.get(from_denom).ok_or(MissingPrice(from_denom.to_string()))?;

        // An asset that has a price of 1 and max ltv of 0.5 has a collateral_value of 0.5.
        // Swapping that asset for an asset with the same price, but 0.8 max ltv results in a collateral_value of 0.8.
        // Therefore, when the asset that is swapped to has a higher or equal max ltv than the asset swapped from,
        // the collateral value will increase and we can allow the full balance to be swapped.
        let swappable_amount = if to_ltv >= from_ltv {
            from_coin.amount
        } else {
            // In order to calculate the output of the swap, the formula looks like this:
            //    from_amount = (collateral_value - debt_value - perpd + perpn) / (from_price * ( from_ltv - to_ltv))
            let amount = total_max_ltv_adjusted_value
                .checked_sub(debt_value)?
                .checked_sub(perp_denominator)?
                .checked_add(perp_numerator)?
                .checked_sub(one)?
                .checked_div(from_price.checked_mul(from_ltv - to_ltv)?.into())?;

            // Cap the swappable amount at the current balance of the coin
            min(amount.abs.to_uint_floor(), from_coin.amount)
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

                // The from_denom is always taken on as debt, as the trade is the bullish direction
                // of the to_denom (expecting it to outpace the borrow rate from the from_denom)
                let swap_to_ltv_value = from_coin_value.checked_mul_floor(to_ltv)?;

                let total_max_ltv_adjust_value_after_swap = total_max_ltv_adjusted_value
                    .checked_add(SignedDecimal::from(swap_to_ltv_value))?
                    .checked_sub(SignedDecimal::from(swap_from_ltv_value))?;

                // The total swappable amount for margin is represented by the available coin balance + the
                // the maximum amount that can be borrowed (and then swapped).
                // This is represented by the formula:
                //     borrow_amount = (collateral_after_swap - debt - perpd + perpn) / ((1 - to_ltv) * borrow_price)
                let borrow_amount = total_max_ltv_adjust_value_after_swap
                    .checked_sub(debt_value)?
                    .checked_sub(perp_denominator)?
                    .checked_add(perp_numerator)?
                    .checked_sub(one)?
                    .checked_div(
                        Decimal::one().checked_sub(to_ltv)?.checked_mul(*from_price)?.into(),
                    )?;

                // The total amount that can be swapped is then the balance of the coin + the additional amount
                // that can be borrowed.
                Ok(borrow_amount.abs.to_uint_floor().checked_add(from_coin.amount)?)
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
        let total_max_ltv_adjusted_value: SignedDecimal =
            self.total_collateral_value()?.max_ltv_adjusted_collateral.into();
        let debt_value: SignedDecimal = self.spot_debt_value()?.into();

        // We often add one to calcs for a margin of error, so rather than create it multiple times we just create it once here.
        let one = SignedDecimal::one();

        // Perp values
        let PerpHealthFactorValues {
            max_ltv_denominator: perp_denominator,
            max_ltv_numerator: perp_numerator,
            ..
        } = self.perp_health_factor_values()?;

        let params = self
            .denoms_data
            .params
            .get(borrow_denom)
            .ok_or(MissingParams(borrow_denom.to_string()))?;

        // If asset not whitelisted we cannot borrow
        if !params.credit_manager.whitelisted || total_max_ltv_adjusted_value.is_zero() {
            return Ok(Uint128::zero());
        }

        // If we have perp positions or debt we need to check if the health factor is above 1
        if !self.positions.perps.is_empty() || debt_value.abs > Decimal::zero() {
            let hf = total_max_ltv_adjusted_value
                .checked_add(perp_numerator)?
                .checked_div(debt_value.checked_add(perp_denominator)?)?;

            // Zero borrowable if unhealthy
            if hf.abs.le(&one.abs) {
                return Ok(Uint128::zero());
            }
        }

        let borrow_denom_max_ltv = match self.kind {
            AccountKind::Default => params.max_loan_to_value,
            AccountKind::HighLeveredStrategy => {
                params
                    .credit_manager
                    .hls
                    .as_ref()
                    .ok_or(MissingHLSParams(borrow_denom.to_string()))?
                    .max_loan_to_value
            }
        }
        .into();

        let borrow_denom_price: SignedDecimal = self
            .denoms_data
            .prices
            .get(borrow_denom)
            .cloned()
            .ok_or(MissingPrice(borrow_denom.to_string()))?
            .into();

        // The formulas look like this in practice:
        //      hf = rounddown(roundown(amount * price) * perp_numerator) / (spot_debt value + perp_denominator)
        // Which means re-arranging this to isolate borrow amount is an estimate,
        // quite close, but never precisely right. For this reason, the + 1 of the formulas
        // below are meant to err on the side of being more conservative vs aggressive.

        let max_borrow_amount = match target {
            // The max borrow for deposit can be calculated as:
            //      1 = (max ltv adjusted value + (borrow denom amount * borrow denom price * borrow denom max ltv) + perpn) / (debt value + (borrow denom amount * borrow denom price) + perpd)
            // Re-arranging this to isolate borrow denom amount renders:
            //      max_borrow_denom_amount = max ltv adjusted value - debt value - perpd + perpn / (borrow_denom_price * (1 - borrow_denom_max_ltv)))
            BorrowTarget::Deposit => total_max_ltv_adjusted_value
                .checked_sub(debt_value)?
                .checked_sub(perp_denominator)?
                .checked_add(perp_numerator)?
                .checked_sub(one)?
                .checked_div(
                    borrow_denom_price.checked_mul(one.checked_sub(borrow_denom_max_ltv)?)?,
                )?
                .abs
                .to_uint_floor(),

            // Borrowing assets to wallet does not count towards collateral. It only adds to debts.
            // Hence, the max borrow to wallet can be calculated as:
            //      1 = (max ltv adjusted value) + perpn / (debt value + (borrow denom amount * borrow denom price)) + perpd
            // Re-arranging this to isolate borrow denom amount renders:
            //      borrow denom amount = (max ltv adjusted value - debt_value - perpd + perpn) / denom_price
            // TODO : tidy this variable creation
            BorrowTarget::Wallet => total_max_ltv_adjusted_value
                .checked_sub(debt_value)?
                .checked_sub(perp_denominator)?
                .checked_add(perp_numerator)?
                .checked_sub(one)?
                .checked_div(borrow_denom_price)?
                .abs
                .to_uint_floor(),

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
                        AccountKind::HighLeveredStrategy => {
                            hls.as_ref()
                                .ok_or(MissingHLSParams(addr.to_string()))?
                                .max_loan_to_value
                        }
                    }
                } else {
                    Decimal::zero()
                }
                .into();

                // The max borrow for deposit can be calculated as:
                //      1 = (total_max_ltv_adjusted_value + (max_borrow_denom_amount * borrow_denom_price * checked_vault_max_ltv) + perpn) / (debt_value + (max_borrow_denom_amount * borrow_denom_price)) + perpd
                // Re-arranging this to isolate borrow denom amount renders:
                //      max_borrow_denom_amount = (total_max_ltv_adjusted_value-debt_value + perpn - perpd) / (borrow_denom_price * (1 - checked_vault_max_ltv))
                // Which means re-arranging this to isolate borrow amount is an estimate,
                // quite close, but never precisely right. For this reason, the - 1 of the formulas
                // below are meant to err on the side of being more conservative vs aggressive.
                total_max_ltv_adjusted_value
                    .checked_sub(debt_value)?
                    .checked_sub(perp_denominator)?
                    .checked_add(perp_numerator)?
                    .checked_sub(one)?
                    .checked_div(
                        borrow_denom_price.checked_mul(one.checked_sub(checked_vault_max_ltv)?)?,
                    )?
                    .abs
                    .to_uint_floor()
            }
        };

        Ok(max_borrow_amount)
    }

    fn perp_health_factor_values(&self) -> HealthResult<PerpHealthFactorValues> {
        let mut max_ltv_numerator = SignedDecimal::zero();
        let mut max_ltv_denominator = SignedDecimal::zero();
        let mut liq_ltv_numerator = SignedDecimal::zero();
        let mut liq_ltv_denominator = SignedDecimal::zero();
        let mut profit = Uint128::zero();
        let mut loss = Uint128::zero();

        for position in self.positions.perps.iter() {
            // Update our pnl values
            match &position.pnl {
                PnL::Profit(pnl) => profit = profit.checked_add(pnl.amount)?,
                PnL::Loss(pnl) => loss = loss.checked_add(pnl.amount)?,
                _ => {}
            }

            let denom = &position.denom;
            let base_denom = &position.base_denom;
            let base_denom_price: SignedDecimal = (*self
                .denoms_data
                .prices
                .get(base_denom)
                .ok_or(MissingPrice(base_denom.to_string()))?)
            .into();

            let (funding_min, funding_max) = self.get_min_and_max_funding(position)?;

            let funding_min_value = funding_min.checked_mul(base_denom_price)?;
            let funding_max_value = funding_max.checked_mul(base_denom_price)?;

            let closing_rate = position.closing_fee_rate.into();

            // Perp(0)
            let position_value_open: SignedDecimal =
                position.size.abs.checked_mul(position.entry_price)?.into(); // todo: change to execution price
                                                                             // Perp(t)
            let position_value_current: SignedDecimal =
                position.size.checked_mul(position.current_price.into())?.abs.into();

            // Borrow and liquidation ltv maximums for the perp and the funding demom
            let checked_max_ltv: SignedDecimal = self.get_perp_max_ltv(denom)?.into();
            let checked_liq_ltv: SignedDecimal = self.get_perp_liq_ltv(denom)?.into();
            let checked_max_ltv_base_denom: SignedDecimal =
                self.get_coin_max_ltv(base_denom)?.into();
            let checked_liq_ltv_base_denom: SignedDecimal =
                self.get_liquidation_ltv(base_denom)?.into();

            // There are two different HF calculations, depending on if the perp
            // position is long or short.

            // For shorts, Health Factor = Perp(0) + (funding max accrued * base denom price * base denom ltv)  / (Perp (t) * (2 - MaxLTV + trading fee) + funding min * base denom price
            // For longs, Health Factor = (Perp (t) * (LTV-trading fee) + funding max * base denom price * base denom ltv  / Perp (t0) + funding min * base denom price
            // IF perp size is negative the position is short, positive long
            if position.size.is_negative() {
                // Numerator = position value(0) + (positive funding * base denom ltv * base denom price)
                let temp_ltv_numerator = position_value_open
                    .checked_add(funding_max_value.checked_mul(checked_max_ltv_base_denom)?)?;

                let temp_liq_numerator = position_value_open
                    .checked_add(funding_max_value.checked_mul(checked_liq_ltv_base_denom)?)?;

                // Denominator = position value(t) * (2 - max ltv + closing fee) + negative funding
                let temp_ltv_denominator = position_value_current
                    .checked_mul(
                        SignedDecimal::from_str("2.0")?
                            .checked_sub(checked_max_ltv)?
                            .checked_add(closing_rate)?,
                    )?
                    .checked_add(funding_min_value)?;

                let temp_liq_denominator = position_value_current
                    .checked_mul(
                        SignedDecimal::from_str("2.0")?
                            .checked_sub(checked_liq_ltv)?
                            .checked_add(closing_rate)?,
                    )?
                    .checked_add(funding_min_value)?;

                // Add values
                max_ltv_numerator = max_ltv_numerator.checked_add(temp_ltv_numerator)?;
                liq_ltv_numerator = liq_ltv_numerator.checked_add(temp_liq_numerator)?;
                max_ltv_denominator = max_ltv_denominator.checked_add(temp_ltv_denominator)?;
                liq_ltv_denominator = liq_ltv_denominator.checked_add(temp_liq_denominator)?;
            } else if position.size.is_positive() {
                // Numerator = position value(0) + (positive funding * base denom ltv)
                let temp_ltv_numerator = position_value_current
                    .checked_mul(checked_max_ltv.checked_sub(closing_rate)?)?
                    .checked_add(funding_max_value.checked_mul(checked_max_ltv_base_denom)?)?;

                let temp_liq_numerator = position_value_current
                    .checked_mul(checked_liq_ltv.checked_sub(closing_rate)?)?
                    .checked_add(funding_max_value.checked_mul(checked_liq_ltv_base_denom)?)?;

                // Denominator = position value(0) + negative funding
                let temp_denominator = position_value_open.checked_add(funding_min_value)?;

                // Add values
                max_ltv_numerator = max_ltv_numerator.checked_add(temp_ltv_numerator)?;
                liq_ltv_numerator = liq_ltv_numerator.checked_add(temp_liq_numerator)?;
                max_ltv_denominator = max_ltv_denominator.checked_add(temp_denominator)?;
                liq_ltv_denominator = liq_ltv_denominator.checked_add(temp_denominator)?;
            }

            // else perp size is zero - safe to do nothing? we should never get into this situation
            // but if we do we probably don't want to brick the HF calculation
        }

        Ok(PerpHealthFactorValues {
            max_ltv_numerator: max_ltv_numerator.floor(),
            max_ltv_denominator: max_ltv_denominator.floor(),
            liq_ltv_numerator: liq_ltv_numerator.floor(),
            liq_ltv_denominator: liq_ltv_denominator.floor(),
            pnl_values: PerpPnlValues {
                profit,
                loss,
            },
        })
    }

    fn total_collateral_value(&self) -> HealthResult<CollateralValue> {
        let deposits = self.coins_value(&self.positions.deposits)?;
        let lends = self.coins_value(&self.positions.lends)?;
        let vaults = self.vaults_value()?;

        Ok(CollateralValue {
            total_collateral_value: deposits
                .total_collateral_value
                .checked_add(vaults.total_collateral_value)?
                .checked_add(lends.total_collateral_value)?,
            max_ltv_adjusted_collateral: deposits
                .max_ltv_adjusted_collateral
                .checked_add(vaults.max_ltv_adjusted_collateral)?
                .checked_add(lends.max_ltv_adjusted_collateral)?,
            liquidation_threshold_adjusted_collateral: deposits
                .liquidation_threshold_adjusted_collateral
                .checked_add(vaults.liquidation_threshold_adjusted_collateral)?
                .checked_add(lends.liquidation_threshold_adjusted_collateral)?,
        })
    }

    fn coins_value(&self, coins: &[Coin]) -> HealthResult<CollateralValue> {
        let mut total_collateral_value = Uint128::zero();
        let mut max_ltv_adjusted_collateral = Uint128::zero();
        let mut liquidation_threshold_adjusted_collateral = Uint128::zero();

        for c in coins {
            let coin_price =
                self.denoms_data.prices.get(&c.denom).ok_or(MissingPrice(c.denom.clone()))?;
            let coin_value = c.amount.checked_mul_floor(*coin_price)?;
            total_collateral_value = total_collateral_value.checked_add(coin_value)?;

            let AssetParams {
                credit_manager:
                    CmSettings {
                        hls,
                        ..
                    },
                liquidation_threshold,
                ..
            } = self.denoms_data.params.get(&c.denom).ok_or(MissingParams(c.denom.clone()))?;

            let checked_max_ltv = self.get_coin_max_ltv(&c.denom)?;

            let max_ltv_adjusted = coin_value.checked_mul_floor(checked_max_ltv)?;
            max_ltv_adjusted_collateral =
                max_ltv_adjusted_collateral.checked_add(max_ltv_adjusted)?;

            let checked_liquidation_threshold = match self.kind {
                AccountKind::Default => *liquidation_threshold,
                AccountKind::HighLeveredStrategy => {
                    hls.as_ref().ok_or(MissingHLSParams(c.denom.clone()))?.liquidation_threshold
                }
            };
            let liq_adjusted = coin_value.checked_mul_floor(checked_liquidation_threshold)?;
            liquidation_threshold_adjusted_collateral =
                liquidation_threshold_adjusted_collateral.checked_add(liq_adjusted)?;
        }
        Ok(CollateralValue {
            total_collateral_value,
            max_ltv_adjusted_collateral,
            liquidation_threshold_adjusted_collateral,
        })
    }

    fn vaults_value(&self) -> HealthResult<CollateralValue> {
        let mut total_collateral_value = Uint128::zero();
        let mut max_ltv_adjusted_collateral = Uint128::zero();
        let mut liquidation_threshold_adjusted_collateral = Uint128::zero();

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
                .denoms_data
                .params
                .get(&values.base_coin.denom)
                .ok_or(MissingParams(values.base_coin.denom.clone()))?;

            // If vault or base token has been de-listed, drop MaxLTV to zero
            let checked_vault_max_ltv = if *whitelisted && base_params.credit_manager.whitelisted {
                match self.kind {
                    AccountKind::Default => *max_loan_to_value,
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
                AccountKind::HighLeveredStrategy => {
                    hls.as_ref().ok_or(MissingHLSParams(addr.to_string()))?.liquidation_threshold
                }
            };

            liquidation_threshold_adjusted_collateral = values
                .vault_coin
                .value
                .checked_mul_floor(checked_liquidation_threshold)?
                .checked_add(liquidation_threshold_adjusted_collateral)?;

            // Step 2: Calculate Base coin values
            let res = self.coins_value(&[Coin {
                denom: values.base_coin.denom.clone(),
                amount: v.amount.unlocking().total(),
            }])?;
            total_collateral_value =
                total_collateral_value.checked_add(res.total_collateral_value)?;
            max_ltv_adjusted_collateral =
                max_ltv_adjusted_collateral.checked_add(res.max_ltv_adjusted_collateral)?;
            liquidation_threshold_adjusted_collateral =
                liquidation_threshold_adjusted_collateral
                    .checked_add(res.liquidation_threshold_adjusted_collateral)?;
        }

        Ok(CollateralValue {
            total_collateral_value,
            max_ltv_adjusted_collateral,
            liquidation_threshold_adjusted_collateral,
        })
    }

    /// Total value of all spot debts.
    ///
    /// Denominated in the protocol's base asset (typically USDC).
    fn spot_debt_value(&self) -> HealthResult<Uint128> {
        let mut total = Uint128::zero();

        // spot debt borrowed from redbank
        for debt in &self.positions.debts {
            let coin_price =
                self.denoms_data.prices.get(&debt.denom).ok_or(MissingPrice(debt.denom.clone()))?;
            let debt_value = debt.amount.checked_mul_ceil(*coin_price)?;
            total = total.checked_add(debt_value)?;
        }

        Ok(total)
    }

    fn get_liquidation_ltv(&self, denom: &str) -> HealthResult<Decimal> {
        let AssetParams {
            liquidation_threshold,
            ..
        } = self.denoms_data.params.get(denom).ok_or(MissingParams(denom.to_string()))?;

        Ok(*liquidation_threshold)
    }

    fn get_perp_max_ltv(&self, denom: &str) -> HealthResult<Decimal> {
        let params = self.denoms_data.params.get(denom).ok_or(MissingParams(denom.to_string()))?;

        // If the coin has been de-listed, drop MaxLTV to zero
        if !params.credit_manager.whitelisted {
            return Ok(Decimal::zero());
        }

        Ok(params.max_loan_to_value)
    }

    fn get_perp_liq_ltv(&self, denom: &str) -> HealthResult<Decimal> {
        let params = self.denoms_data.params.get(denom).ok_or(MissingParams(denom.to_string()))?;

        // If the coin has been de-listed, drop MaxLTV to zero
        if !params.credit_manager.whitelisted {
            return Ok(Decimal::zero());
        }

        Ok(params.liquidation_threshold)
    }

    fn get_coin_max_ltv(&self, denom: &str) -> HealthResult<Decimal> {
        let params = self.denoms_data.params.get(denom).ok_or(MissingParams(denom.to_string()))?;

        // If the coin has been de-listed, drop MaxLTV to zero
        if !params.credit_manager.whitelisted {
            return Ok(Decimal::zero());
        }

        match self.kind {
            AccountKind::Default => Ok(params.max_loan_to_value),
            AccountKind::HighLeveredStrategy => Ok(params
                .credit_manager
                .hls
                .as_ref()
                .ok_or(MissingHLSParams(denom.to_string()))?
                .max_loan_to_value),
        }
    }

    fn get_coin_from_deposits_and_lends(&self, denom: &str) -> HealthResult<Coin> {
        let deposited_coin = self.positions.deposits.iter().find(|c| c.denom == denom);
        let deposited_amount = deposited_coin.unwrap_or(&Coin::default()).amount;

        let lent_coin = self.positions.lends.iter().find(|c| c.denom == denom);
        let lent_amount = lent_coin.unwrap_or(&Coin::default()).amount;

        Ok(Coin {
            denom: denom.to_string(),
            amount: deposited_amount.checked_add(lent_amount)?,
        })
    }

    // TODO - use comparison function
    fn get_min_and_max_funding(
        &self,
        position: &PerpPosition,
    ) -> HealthResult<(SignedDecimal, SignedDecimal)> {
        // funding_max = max(0, unrealised_funding_accrued)
        let funding_max = if position.unrealised_funding_accrued.is_positive() {
            position.unrealised_funding_accrued
        } else {
            SignedDecimal::zero()
        };

        // funding min = -min(0, unrealised_funding_accrued)
        let funding_min = if position.unrealised_funding_accrued.is_negative() {
            position.unrealised_funding_accrued.abs.into()
        } else {
            SignedDecimal::zero()
        };

        Ok((funding_min, funding_max))
    }
}
