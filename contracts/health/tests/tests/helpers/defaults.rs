use std::str::FromStr;

use cosmwasm_std::Decimal;
use mars_types::{
    params::{
        AssetParamsUnchecked, CmSettings, HlsParamsUnchecked, LiquidationBonus, RedBankSettings,
    },
    red_bank::InterestRateModel,
};

pub fn default_asset_params(denom: &str) -> AssetParamsUnchecked {
    AssetParamsUnchecked {
        denom: denom.to_string(),
        credit_manager: CmSettings {
            whitelisted: true,
            withdraw_enabled: true,
            hls: Some(HlsParamsUnchecked {
                max_loan_to_value: Decimal::from_str("0.8").unwrap(),
                liquidation_threshold: Decimal::from_str("0.9").unwrap(),
                correlations: vec![],
            }),
        },
        red_bank: RedBankSettings {
            withdraw_enabled: true,
            deposit_enabled: false,
            borrow_enabled: false,
        },
        max_loan_to_value: Decimal::from_str("0.4523").unwrap(),
        liquidation_threshold: Decimal::from_str("0.5").unwrap(),
        liquidation_bonus: LiquidationBonus {
            starting_lb: Decimal::percent(1u64),
            slope: Decimal::from_atomics(2u128, 0).unwrap(),
            min_lb: Decimal::percent(2u64),
            max_lb: Decimal::percent(10u64),
        },
        protocol_liquidation_fee: Decimal::percent(2u64),
        deposit_cap: Default::default(),
        close_factor: Decimal::percent(80u64),
        reserve_factor: Decimal::percent(10u64),
        interest_rate_model: InterestRateModel {
            optimal_utilization_rate: Decimal::percent(80u64),
            base: Decimal::zero(),
            slope_1: Decimal::percent(7u64),
            slope_2: Decimal::percent(45u64),
        },
    }
}
