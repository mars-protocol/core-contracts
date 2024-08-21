use mars_types::health::{
    BorrowTarget, HealthValuesResponse, LiquidationPriceKind, Number, SwapKind, Uint,
};
use wasm_bindgen::prelude::*;

use crate::{Direction, HealthComputer};

// Note: Arguments and return values must use:
//          #[derive(Tsify)]
//          #[tsify(into_wasm_abi, from_wasm_abi)]
//      as attributes in order for Typescript type generation to work

#[wasm_bindgen]
pub fn compute_health_js(c: HealthComputer) -> HealthValuesResponse {
    c.compute_health().unwrap().into()
}

#[wasm_bindgen]
pub fn max_withdraw_estimate_js(c: HealthComputer, withdraw_denom: String) -> String {
    c.max_withdraw_amount_estimate(&withdraw_denom).unwrap().to_string()
}

#[wasm_bindgen]
pub fn max_borrow_estimate_js(
    c: HealthComputer,
    borrow_denom: String,
    target: BorrowTarget,
) -> String {
    c.max_borrow_amount_estimate(&borrow_denom, &target).unwrap().to_string()
}

#[wasm_bindgen]
pub fn max_swap_estimate_js(
    c: HealthComputer,
    from_denom: String,
    to_denom: String,
    kind: SwapKind,
    slippage: Number,
    is_repaying_debt: bool,
) -> String {
    c.max_swap_amount_estimate(
        &from_denom,
        &to_denom,
        &kind,
        slippage.as_decimal(),
        is_repaying_debt,
    )
    .unwrap()
    .to_string()
}

#[wasm_bindgen]
pub fn liquidation_price_js(
    c: HealthComputer,
    denom: String,
    kind: LiquidationPriceKind,
) -> String {
    c.liquidation_price(&denom, &kind).unwrap().to_string()
}

#[wasm_bindgen]
pub fn max_perp_size_estimate_js(
    c: HealthComputer,
    denom: String,
    base_denom: String,
    long_oi_amount: Uint,
    short_oi_amount: Uint,
    direction: Direction,
) -> String {
    c.max_perp_size_estimate(
        &denom,
        &base_denom,
        long_oi_amount.as_unit_128(),
        short_oi_amount.as_unit_128(),
        &direction,
    )
    .unwrap()
    .to_string()
}
