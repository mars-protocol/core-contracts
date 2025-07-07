use std::str::FromStr;

use cosmwasm_std::{Decimal, Uint128};
use mars_types::{
    params::{AssetParams, CmSettings, LiquidationBonus, RedBankSettings},
    red_bank::InterestRateModel,
};

pub fn osmo_asset_params() -> AssetParams {
    default_asset_params_with("uosmo", Decimal::percent(70), Decimal::percent(78))
}

pub fn usdc_asset_params() -> AssetParams {
    default_asset_params_with("uusdc", Decimal::percent(90), Decimal::percent(96))
}

pub fn default_asset_params_with(
    denom: &str,
    max_loan_to_value: Decimal,
    liquidation_threshold: Decimal,
) -> AssetParams {
    AssetParams {
        denom: denom.to_string(),
        credit_manager: CmSettings {
            whitelisted: false,
            withdraw_enabled: true,
            hls: None,
        },
        red_bank: RedBankSettings {
            deposit_enabled: true,
            borrow_enabled: true,
            withdraw_enabled: true,
        },
        max_loan_to_value,
        liquidation_threshold,
        liquidation_bonus: LiquidationBonus {
            starting_lb: Decimal::percent(1),
            slope: Decimal::from_str("2.0").unwrap(),
            min_lb: Decimal::percent(2),
            max_lb: Decimal::percent(10),
        },
        protocol_liquidation_fee: Decimal::percent(2),
        deposit_cap: Uint128::MAX,
        close_factor: Decimal::percent(80),
        reserve_factor: Decimal::percent(20),
        interest_rate_model: InterestRateModel {
            optimal_utilization_rate: Decimal::percent(10),
            base: Decimal::percent(30),
            slope_1: Decimal::percent(25),
            slope_2: Decimal::percent(30),
        },
    }
}
