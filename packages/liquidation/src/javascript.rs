use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, Uint128};
use mars_types::params::AssetParams;
use tsify::Tsify;
use wasm_bindgen::prelude::*;

use crate::{calculate_liquidation_amounts, HealthData, LiquidationAmounts};

#[cw_serde]
#[cfg_attr(feature = "javascript", derive(Tsify))]
#[cfg_attr(feature = "javascript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct LiquidationAmountInputs {
    pub collateral_amount: Uint128,
    pub collateral_price: Decimal,
    pub collateral_params: AssetParams,
    pub debt_amount: Uint128,
    pub debt_requested_to_repay: Uint128,
    pub debt_price: Decimal,
    pub debt_params: AssetParams,
    pub health: HealthData,
    pub perps_lb_ratio: Decimal,
}

#[wasm_bindgen]
pub fn calculate_liquidation_amounts_js(inputs: LiquidationAmountInputs) -> LiquidationAmounts {
    calculate_liquidation_amounts(
        inputs.collateral_amount,
        inputs.collateral_price,
        &inputs.collateral_params,
        inputs.debt_amount,
        inputs.debt_requested_to_repay,
        inputs.debt_price,
        &inputs.debt_params,
        &inputs.health,
        inputs.perps_lb_ratio,
    )
    .unwrap()
}
