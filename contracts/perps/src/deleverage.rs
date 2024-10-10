use cosmwasm_std::{
    coins, to_json_binary, BalanceResponse, BankQuery, CosmosMsg, Decimal, Deps, DepsMut, Env,
    QueryRequest, Reply, Response, StdError, SubMsg, Uint128, WasmMsg,
};
use mars_types::{
    address_provider::{
        self,
        helpers::{query_contract_addr, query_contract_addrs},
        MarsAddressType,
    },
    credit_manager::ExecuteMsg,
    oracle::ActionKind,
    params::PerpParams,
    perps::{MarketState, Position},
};

use crate::{
    error::{ContractError, ContractResult},
    market::MarketStateExt,
    position::{PositionExt, PositionModification},
    position_management::apply_pnl_and_fees,
    query,
    state::{
        DeleverageRequestTempStorage, CONFIG, DELEVERAGE_REQUEST_TEMP_STORAGE, MARKET_STATES,
        POSITIONS, REALIZED_PNL, TOTAL_CASH_FLOW,
    },
    utils::{get_oracle_adapter, get_params_adapter},
};

pub const DELEVERAGE_REQUEST_REPLY_ID: u64 = 10_001;

/// Attempts to deleverage a specified position for a given account and denomination.
///
/// The deleverage process consists of the following steps:
/// 1. **Initial Checks:** Before closing a position, the function verifies that the
///    current Collateralization Ratio (CR) is below the target CR (TCR) or that the
///    Open Interest (OI) exceeds the maximum allowed OI for the position type
///    (long or short). If neither of these conditions are met, the deleverage
///    process is terminated early with an error.
///
/// 2. **Position Closure:** The position is then closed, and any associated
///    unrealized Profit and Loss (PnL) is computed. This PnL is applied to the
///    realized PnL of the account, and the position is removed from storage.
///
/// 3. **Final Checks:** After closing the position, the function checks if the
///    Collateralization Ratio (CR) has improved or has reached the target CR (TCR).
///    If the CR has not improved and is still below the target, the function
///    throws an error to indicate that the deleverage process was unsuccessful.
///
/// 4. **PnL Transfer:** If all checks pass, the realized PnL is converted to the
///    base denomination and transferred to the account via a CosmosMsg. The function
///    then returns a successful response with appropriate attributes.
///
/// The function ensures that the deleverage process is only performed when necessary,
/// and that the resulting position adjustments are valid according to the configured
/// risk parameters.
pub fn deleverage(
    deps: DepsMut,
    env: Env,
    account_id: String,
    denom: String,
) -> ContractResult<Response> {
    let cfg = CONFIG.load(deps.storage)?;

    if !cfg.deleverage_enabled {
        return Err(ContractError::DeleverageDisabled);
    }

    // Current block time
    let current_time = env.block.time.seconds();

    // Use Liquidation pricing because we want to be sure that the position will be closed
    let pricing = ActionKind::Liquidation;

    // Load states
    let position = POSITIONS.may_load(deps.storage, (&account_id, &denom))?.ok_or_else(|| {
        ContractError::PositionNotFound {
            account_id: account_id.clone(),
            denom: denom.clone(),
        }
    })?;
    let mut realized_pnl =
        REALIZED_PNL.may_load(deps.storage, (&account_id, &denom))?.unwrap_or_default();
    let mut ms = MARKET_STATES.load(deps.storage, &denom)?;
    let mut tcf = TOTAL_CASH_FLOW.may_load(deps.storage)?.unwrap_or_default();

    let addresses = query_contract_addrs(
        deps.as_ref(),
        &cfg.address_provider,
        vec![MarsAddressType::Oracle, MarsAddressType::Params],
    )?;

    let oracle = get_oracle_adapter(&addresses[&MarsAddressType::Oracle]);
    let params = get_params_adapter(&addresses[&MarsAddressType::Params]);

    // Query prices and parameters
    let base_denom_price =
        oracle.query_price(&deps.querier, &cfg.base_denom, pricing.clone())?.price;
    let denom_price = oracle.query_price(&deps.querier, &denom, pricing.clone())?.price;
    let perp_params = params.query_perp_params(&deps.querier, &denom)?;

    // Assert CR and OI before deleverage
    let cr_before = query_vault_cr(deps.as_ref(), current_time, pricing.clone())?;
    assert_cr_and_oi_before_deleverage(
        cr_before,
        cfg.target_vault_collateralization_ratio,
        denom_price,
        &ms,
        &perp_params,
        &position,
    )?;

    // Close the position
    let initial_skew = ms.skew()?;
    ms.close_position(current_time, denom_price, base_denom_price, &position)?;

    // Compute the position's unrealized PnL
    let pnl_amounts = position.compute_pnl(
        &ms.funding,
        initial_skew,
        denom_price,
        base_denom_price,
        perp_params.opening_fee_rate,
        perp_params.closing_fee_rate,
        PositionModification::Decrease(position.size),
    )?;

    // Query the rewards collector address
    let rewards_collector_addr = address_provider::helpers::query_contract_addr(
        deps.as_ref(),
        &cfg.address_provider,
        MarsAddressType::RewardsCollector,
    )?;

    // Apply the new PnL amounts to the accumators
    let mut res = Response::new();
    let mut msgs = vec![];
    res = apply_pnl_and_fees(
        &cfg,
        &rewards_collector_addr,
        &mut ms,
        &mut tcf,
        &mut realized_pnl,
        &pnl_amounts,
        res,
        &mut msgs,
    )?;

    // Save updated states
    POSITIONS.remove(deps.storage, (&account_id, &denom));
    REALIZED_PNL.save(deps.storage, (&account_id, &denom), &realized_pnl)?;
    MARKET_STATES.save(deps.storage, &denom, &ms)?;
    TOTAL_CASH_FLOW.save(deps.storage, &tcf)?;

    // Assert CR after deleverage.
    // OI always improves after closing a position.
    let cr_after = query_vault_cr(deps.as_ref(), current_time, pricing.clone())?;
    assert_cr_after_deleverage(cr_before, cr_after, cfg.target_vault_collateralization_ratio)?;

    // Convert PnL amounts to coins
    let pnl = pnl_amounts.to_coins(&cfg.base_denom).pnl;
    let signed_uint_pnl = pnl.to_signed_uint()?;
    let mut requested_amount_from_cm = Uint128::zero();
    let mut send_amount_from_perps = Uint128::zero();
    let funds = if !signed_uint_pnl.is_negative() {
        send_amount_from_perps = signed_uint_pnl.unsigned_abs();
        coins(signed_uint_pnl.unsigned_abs().u128(), cfg.base_denom.clone())
    } else {
        requested_amount_from_cm = signed_uint_pnl.unsigned_abs();
        vec![]
    };

    // Cache necessary data so that they can be accessed when handling reply
    let balance_res: BalanceResponse =
        deps.querier.query(&QueryRequest::Bank(BankQuery::Balance {
            address: env.contract.address.to_string(),
            denom: cfg.base_denom.clone(),
        }))?;
    let temp_storage = DeleverageRequestTempStorage {
        denom: cfg.base_denom.clone(),
        contract_balance: balance_res.amount.amount.checked_sub(send_amount_from_perps)?, // Subtract the amount send from the contract
        requested_amount: requested_amount_from_cm,
    };
    DELEVERAGE_REQUEST_TEMP_STORAGE.save(deps.storage, &temp_storage)?;

    let cm_address =
        query_contract_addr(deps.as_ref(), &cfg.address_provider, MarsAddressType::CreditManager)?;

    // Send a message to the credit manager to update the account's balance
    let msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cm_address.to_string(),
        msg: to_json_binary(&ExecuteMsg::UpdateBalanceAfterDeleverage {
            account_id: account_id.clone(),
            pnl: pnl.clone(),
        })?,
        funds,
    });
    let submsg = SubMsg::reply_on_success(msg, DELEVERAGE_REQUEST_REPLY_ID);

    Ok(res
        .add_messages(msgs)
        .add_submessage(submsg)
        .add_attribute("action", "deleverage")
        .add_attribute("account_id", account_id)
        .add_attribute("denom", denom)
        .add_attribute("cr_before", cr_before.to_string())
        .add_attribute("cr_after", cr_after.to_string())
        .add_attribute("realized_pnl", pnl_amounts.pnl.to_string()))
}

/// Queries the current collateralization ratio (CR) from the vault
pub fn query_vault_cr(
    deps: Deps,
    current_time: u64,
    pricing: ActionKind,
) -> ContractResult<Decimal> {
    let vault_response = query::query_vault(deps, current_time, pricing)?;

    // If the vault response does not contain a CR, return the maximum Decimal value.
    // It means that the vault is over-collateralized. There is no debt.
    Ok(vault_response.collateralization_ratio.unwrap_or(Decimal::MAX))
}

/// Asserts that the Collateralization Ratio (CR) and Open Interest (OI) are in a state that requires deleveraging.
/// If CR >= TCR and OI <= max OI, an error is thrown to terminate the deleverage process.
fn assert_cr_and_oi_before_deleverage(
    cr_before: Decimal,
    target_cr: Decimal,
    denom_price: Decimal,
    ms: &MarketState,
    perp_params: &PerpParams,
    position: &Position,
) -> ContractResult<()> {
    let oi_before = if position.size.is_negative() {
        ms.short_oi.checked_mul_floor(denom_price)?
    } else {
        ms.long_oi.checked_mul_floor(denom_price)?
    };

    // If CR >= TCR and OI <= max OI, throw an error and terminate deleverage
    if cr_before >= target_cr
        && (if position.size.is_negative() {
            oi_before <= perp_params.max_short_oi_value
        } else {
            oi_before <= perp_params.max_long_oi_value
        })
    {
        return Err(ContractError::DeleverageInvalidPosition {
            reason: "CR >= TCR and OI <= max OI".to_string(),
        });
    }

    Ok(())
}

/// Asserts that the Collateralization Ratio (CR) has improved or is above the target after deleveraging.
/// If CR after deleveraging is not improved or remains below the target, an error is thrown.
fn assert_cr_after_deleverage(
    cr_before: Decimal,
    cr_after: Decimal,
    target_cr: Decimal,
) -> ContractResult<()> {
    // Check if CR improved after closing the position
    let cr_improved = cr_after >= cr_before;

    // CR after deleverage should be greater than or equal to target CR or improved if CR was less than target CR before deleverage
    let cr_after_ge_threshold = cr_after >= target_cr;
    if !cr_after_ge_threshold && !cr_improved {
        return Err(ContractError::DeleverageInvalidPosition {
            reason: "Position closure did not improve CR".to_string(),
        });
    }

    Ok(())
}

/// Handles the reply from the credit manager after updating the account's balance
pub fn handle_deleverage_request_reply(
    deps: DepsMut,
    env: Env,
    reply: Reply,
) -> ContractResult<Response> {
    // Process the reply from the credit manager
    reply.result.into_result().map_err(StdError::generic_err)?;

    // Compare contract balance after deleverage. If the difference is not equal to the requested amount, throw an error.
    // Requested amount is the amount that the credit manager should have sent to the contract.
    // If the requested amount is zero, it means that the contract should not have received any funds (closed position was in profit or break even).
    let temp_storage = DELEVERAGE_REQUEST_TEMP_STORAGE.load(deps.storage)?;
    if !temp_storage.requested_amount.is_zero() {
        let balance_res: BalanceResponse =
            deps.querier.query(&QueryRequest::Bank(BankQuery::Balance {
                address: env.contract.address.to_string(),
                denom: temp_storage.denom,
            }))?;
        let balance_diff = balance_res.amount.amount.checked_sub(temp_storage.contract_balance)?;
        if balance_diff != temp_storage.requested_amount {
            return Err(ContractError::InvalidFundsAfterDeleverage {
                expected: temp_storage.requested_amount,
                received: balance_diff,
            });
        }
    }
    DELEVERAGE_REQUEST_TEMP_STORAGE.remove(deps.storage);

    Ok(Response::new().add_attribute("action", "deleverage/handle_reply"))
}
