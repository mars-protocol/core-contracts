use std::collections::HashMap;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use mars_types::{
    adapters::vault::VaultPositionValue,
    params::{PerpParams, VaultConfig},
};

/// Used as storage when trying to compute Health
#[cw_serde]
pub struct CollateralValue {
    pub total_collateral_value: Uint128,
    pub max_ltv_adjusted_collateral: Uint128,
    pub liq_ltv_adjusted_collateral: Uint128,
}

#[cw_serde]
pub struct PerpHealthFactorValues {
    pub max_ltv_numerator: Uint128,
    pub max_ltv_denominator: Uint128,
    pub liq_ltv_numerator: Uint128,
    pub liq_ltv_denominator: Uint128,
}

#[cw_serde]
pub struct PerpPnlValues {
    pub profit: Uint128, // Values are in oracle denom (uusd)
    pub loss: Uint128,   // Values are in oracle denom (uusd)
}

#[cw_serde]
#[derive(Default)]
pub struct PerpsData {
    pub params: HashMap<String, PerpParams>,
}

#[cw_serde]
#[derive(Default)]
pub struct VaultsData {
    /// explain this, unlocked or locked value
    /// given the pricing method of vaults, cannot use individual coins
    pub vault_values: HashMap<Addr, VaultPositionValue>,
    pub vault_configs: HashMap<Addr, VaultConfig>,
}
