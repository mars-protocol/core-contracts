use std::str::FromStr;

use cosmwasm_std::{coin, Decimal, Uint128};
use mars_types::params::{
    AssetParamsUnchecked, CmSettings, LiquidationBonus, PerpParams, RedBankSettings,
    VaultConfigUnchecked,
};

pub fn default_asset_params(denom: &str) -> AssetParamsUnchecked {
    AssetParamsUnchecked {
        denom: denom.to_string(),
        credit_manager: CmSettings {
            whitelisted: false,
            withdraw_enabled: true,
            hls: None,
        },
        red_bank: RedBankSettings {
            deposit_enabled: true,
            borrow_enabled: false,
            withdraw_enabled: true,
        },
        max_loan_to_value: Decimal::from_str("0.6").unwrap(),
        liquidation_threshold: Decimal::from_str("0.7").unwrap(),
        liquidation_bonus: LiquidationBonus {
            starting_lb: Decimal::percent(4),
            slope: Decimal::from_str("2.0").unwrap(),
            min_lb: Decimal::percent(1),
            max_lb: Decimal::percent(8),
        },
        protocol_liquidation_fee: Decimal::percent(2),
        deposit_cap: Uint128::new(1_000_000_000),
        close_factor: Decimal::percent(80u64),
    }
}

pub fn default_vault_config(addr: &str) -> VaultConfigUnchecked {
    VaultConfigUnchecked {
        addr: addr.to_string(),
        deposit_cap: coin(100_000_000_000, "uusdc"),
        max_loan_to_value: Decimal::from_str("0.47").unwrap(),
        liquidation_threshold: Decimal::from_str("0.5").unwrap(),
        whitelisted: true,
        hls: None,
    }
}

pub fn default_perp_params(denom: &str) -> PerpParams {
    PerpParams {
        denom: denom.to_string(),
        enabled: true,
        max_net_oi_value: Uint128::new(1_000_000_000),
        max_long_oi_value: Uint128::new(1_000_000_000),
        max_short_oi_value: Uint128::new(1_000_000_000),
        closing_fee_rate: Decimal::from_str("0.006").unwrap(),
        opening_fee_rate: Decimal::from_str("0.004").unwrap(),
        liquidation_threshold: Decimal::from_str("0.85").unwrap(),
        max_loan_to_value: Decimal::from_str("0.8").unwrap(),
        max_loan_to_value_usdc: None,
        liquidation_threshold_usdc: None,
        max_position_value: None,
        min_position_value: Uint128::zero(),
        max_funding_velocity: Decimal::from_str("3").unwrap(),
        skew_scale: Uint128::new(1000000u128),
    }
}
