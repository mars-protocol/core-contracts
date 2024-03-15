use cosmwasm_std::{Addr, Response, Storage};
use mars_types::perps::{CashFlow, Config, VaultState};

use crate::{
    error::ContractResult,
    state::{CONFIG, TOTAL_CASH_FLOW, VAULT_STATE},
};

pub fn initialize(store: &mut dyn Storage, cfg: Config<Addr>) -> ContractResult<Response> {
    CONFIG.save(store, &cfg)?;

    // initialize vault state to zero total liquidity and zero total shares
    VAULT_STATE.save(store, &VaultState::default())?;

    // initialize global cash flow to zero
    TOTAL_CASH_FLOW.save(store, &CashFlow::default())?;

    Ok(Response::new().add_attribute("method", "initialize"))
}
