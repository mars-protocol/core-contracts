use std::collections::HashMap;

use cosmwasm_std::{
    coins, ensure_eq, Addr, Attribute, BankMsg, Coin, CosmosMsg, Decimal, DepsMut, Env, Int128,
    MessageInfo, Order, Response, StdError, Uint128,
};
use cw_utils::may_pay;
use mars_perps_common::pricing::opening_execution_price;
use mars_types::{
    address_provider::{self, helpers::query_contract_addrs, MarsAddressType},
    oracle::ActionKind,
    params::PerpParams,
    perps::{CashFlow, Config, MarketState, PnL, PnlAmounts, Position},
};

use crate::{
    accounting::CashFlowExt,
    error::{ContractError, ContractResult},
    market::MarketStateExt,
    position::{calculate_new_size, PositionExt, PositionModification},
    state::{
        ACCOUNT_OPENING_FEE_RATES, CONFIG, MARKET_STATES, POSITIONS, REALIZED_PNL, TOTAL_CASH_FLOW,
    },
    utils::{
        ensure_max_position, ensure_min_position, get_oracle_adapter, get_params_adapter,
        update_position_attributes,
    },
};

/// Helper function to compute discounted fee rates
pub fn compute_discounted_fee_rates(
    perp_params: &PerpParams,
    discount_pct: Option<Decimal>,
) -> (Decimal, Decimal) {
    let opening_fee_rate = if let Some(discount) = discount_pct {
        perp_params.opening_fee_rate * (Decimal::one() - discount)
    } else {
        perp_params.opening_fee_rate
    };

    let closing_fee_rate = if let Some(discount) = discount_pct {
        perp_params.closing_fee_rate * (Decimal::one() - discount)
    } else {
        perp_params.closing_fee_rate
    };

    (opening_fee_rate, closing_fee_rate)
}

/// Executes a perpetual order for a specific account and denom.
///
/// Depending on whether a position exists and the reduce_only flag, this function either opens a new
/// position, modifies an existing one, or returns an error if the operation is illegal.
pub fn execute_order(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    account_id: String,
    denom: String,
    size: Int128,
    reduce_only: Option<bool>,
    discount_pct: Option<Decimal>,
) -> ContractResult<Response> {
    let position = POSITIONS.may_load(deps.storage, (&account_id, &denom))?;
    let reduce_only_checked = reduce_only.unwrap_or(false);

    match position {
        None if reduce_only_checked => Err(ContractError::IllegalPositionModification {
            reason: "Cannot open position if reduce_only = true".to_string(),
        }),
        None => open_position(deps, env, info, account_id, denom, size, discount_pct),
        Some(position) => {
            let new_size = calculate_new_size(position.size, size, reduce_only_checked)?;
            modify_position(deps, env, info, position, account_id, denom, new_size, discount_pct)
        }
    }
}
/// Opens a new position for a specific account and denom.
///
/// This function checks if the account can open a new position, validates the position parameters,
/// and then creates the new position, updating the necessary states and applying any opening fees.
fn open_position(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    account_id: String,
    denom: String,
    size: Int128,
    discount_pct: Option<Decimal>,
) -> ContractResult<Response> {
    let cfg = CONFIG.load(deps.storage)?;

    let addresses = query_contract_addrs(
        deps.as_ref(),
        &cfg.address_provider,
        vec![MarsAddressType::CreditManager, MarsAddressType::Oracle, MarsAddressType::Params],
    )?;

    // Only the credit manager contract can open positions
    ensure_eq!(
        info.sender,
        addresses[&MarsAddressType::CreditManager],
        ContractError::SenderIsNotCreditManager
    );

    // The denom must exist and have been enabled
    let mut ms = MARKET_STATES.load(deps.storage, &denom)?;
    if !ms.enabled {
        return Err(ContractError::DenomNotEnabled {
            denom,
        });
    }

    // Number of open positions per account is limited
    let positions = POSITIONS
        .prefix(&account_id)
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<Result<HashMap<_, _>, StdError>>()?;
    if positions.len() as u8 >= cfg.max_positions {
        return Err(ContractError::MaxPositionsReached {
            account_id,
            max_positions: cfg.max_positions,
        });
    }

    // Each account can only have one position for a denom at the same time
    if positions.contains_key(&denom) {
        return Err(ContractError::PositionExists {
            account_id,
            denom,
        });
    }

    let rewards_collector_addr = address_provider::helpers::query_contract_addr(
        deps.as_ref(),
        &cfg.address_provider,
        MarsAddressType::RewardsCollector,
    )?;

    let oracle = get_oracle_adapter(&addresses[&MarsAddressType::Oracle]);
    let params = get_params_adapter(&addresses[&MarsAddressType::Params]);

    // Params for the given market
    let perp_params = params.query_perp_params(&deps.querier, &denom)?;

    // Apply discount to fee rates if provided
    let (opening_fee_rate, closing_fee_rate) =
        compute_discounted_fee_rates(&perp_params, discount_pct);

    let opening_fee_amt = may_pay(&info, &cfg.base_denom)?;

    // Query the asset's price.
    //
    // This will be the position's entry price, used to compute PnL when closing
    // the position.
    let denom_price = oracle.query_price(&deps.querier, &denom, ActionKind::Default)?.price;
    let base_denom_price =
        oracle.query_price(&deps.querier, &cfg.base_denom, ActionKind::Default)?.price;

    // The position's initial value cannot be too small
    let position_value = size.unsigned_abs().checked_mul_floor(denom_price)?;
    ensure_min_position(position_value, &perp_params)?;

    // The position's initial value cannot be too big
    ensure_max_position(position_value, &perp_params)?;

    let fees = PositionModification::Increase(size).compute_fees(
        opening_fee_rate,
        closing_fee_rate,
        denom_price,
        base_denom_price,
        ms.skew()?,
        perp_params.skew_scale,
    )?;

    // Ensure the opening fee amount sent is correct
    ensure_eq!(
        opening_fee_amt,
        fees.opening_fee.unsigned_abs(),
        ContractError::InvalidPayment {
            denom,
            required: fees.opening_fee.unsigned_abs(),
            received: opening_fee_amt,
        }
    );

    // Validate the position's size against OI limits
    ms.validate_open_interest(size, Int128::zero(), denom_price, &perp_params)?;

    // Skew _before_ modification
    let initial_skew = ms.skew()?;

    // Update the denom's accumulators.
    // Funding rates and index is updated to the current block time (using old size).
    ms.open_position(env.block.time.seconds(), size, denom_price, base_denom_price)?;

    let mut attrs = vec![];
    let mut msgs: Vec<CosmosMsg> = vec![];

    let mut position_realized_pnl = PnlAmounts::default();

    // Update realized PnL with opening fee
    if !opening_fee_amt.is_zero() {
        let mut tcf = TOTAL_CASH_FLOW.may_load(deps.storage)?.unwrap_or_default();

        // Create unrealized pnl
        let unrealized_pnl = PnlAmounts::from_opening_fee(opening_fee_amt)?;

        apply_pnl_and_fees(
            &cfg,
            &rewards_collector_addr,
            &mut ms,
            &mut tcf,
            &mut position_realized_pnl,
            &unrealized_pnl,
            &mut attrs,
            &mut msgs,
        )?;

        let mut realized_pnl =
            REALIZED_PNL.may_load(deps.storage, (&account_id, &denom))?.unwrap_or_default();
        realized_pnl.add(&position_realized_pnl)?;
        REALIZED_PNL.save(deps.storage, (&account_id, &denom), &realized_pnl)?;
        TOTAL_CASH_FLOW.save(deps.storage, &tcf)?;
    }

    let entry_accrued_funding_per_unit_in_base_denom =
        ms.funding.last_funding_accrued_per_unit_in_base_denom;
    let entry_exec_price =
        opening_execution_price(initial_skew, ms.funding.skew_scale, size, denom_price)?;

    MARKET_STATES.save(deps.storage, &denom, &ms)?;

    // Save the user's new position with updated funding
    POSITIONS.save(
        deps.storage,
        (&account_id, &denom),
        &Position {
            size,
            entry_price: denom_price,
            entry_exec_price,
            entry_accrued_funding_per_unit_in_base_denom,
            initial_skew,
            realized_pnl: position_realized_pnl,
        },
    )?;

    // Save the actual opening fee rate that was applied to this position
    ACCOUNT_OPENING_FEE_RATES.save(deps.storage, (&account_id, &denom), &opening_fee_rate)?;

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "open_position")
        .add_attribute("account_id", account_id)
        .add_attribute("denom", denom)
        .add_attribute("new_size", size.to_string())
        .add_attribute("current_price", denom_price.to_string())
        .add_attribute("new_skew", initial_skew.to_string())
        .add_attribute(
            "new_accrued_funding_per_unit",
            entry_accrued_funding_per_unit_in_base_denom.to_string(),
        )
        .add_attributes(attrs))
}

/// Updates the state of a position for a specific account.
///
/// This function adjusts the position size based on the provided new size and performs necessary updates
/// to the position state, including PnL realization, funding rates, and accumulator updates. The function
/// ensures that the position modifications adhere to the market parameters and the denom's current state.
fn modify_position(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    position: Position,
    account_id: String,
    denom: String,
    new_size: Int128,
    discount_pct: Option<Decimal>,
) -> ContractResult<Response> {
    // Load the contract's configuration
    let cfg = CONFIG.load(deps.storage)?;

    let addresses = query_contract_addrs(
        deps.as_ref(),
        &cfg.address_provider,
        vec![
            MarsAddressType::CreditManager,
            MarsAddressType::Oracle,
            MarsAddressType::Params,
            MarsAddressType::RewardsCollector,
        ],
    )?;

    let cm_address = &addresses[&MarsAddressType::CreditManager];

    // Only the credit manager contract can adjust positions
    ensure_eq!(info.sender, cm_address, ContractError::SenderIsNotCreditManager);

    let oracle = get_oracle_adapter(&addresses[&MarsAddressType::Oracle]);
    let params = get_params_adapter(&addresses[&MarsAddressType::Params]);

    // Query the parameters for the given market (denom)
    let perp_params = params.query_perp_params(&deps.querier, &denom)?;

    // Apply discount to fee rates if provided
    let (opening_fee_rate, closing_fee_rate) =
        compute_discounted_fee_rates(&perp_params, discount_pct);

    // Load relevant state variables
    let mut realized_pnl =
        REALIZED_PNL.may_load(deps.storage, (&account_id, &denom))?.unwrap_or_default();
    let mut ms = MARKET_STATES.load(deps.storage, &denom)?;
    let mut tcf = TOTAL_CASH_FLOW.may_load(deps.storage)?.unwrap_or_default();

    let entry_size = position.size;

    // Query the current prices for the denom and the base denom
    let denom_price = oracle.query_price(&deps.querier, &denom, ActionKind::Default)?.price;
    let base_denom_price =
        oracle.query_price(&deps.querier, &cfg.base_denom, ActionKind::Default)?.price;

    // When modifying a position, we must realise all PnL. The credit manager
    // may send no coin (in case the position is winning or breaking even) or
    // one coin of the base denom (i.e usdc) in case the position is losing
    let paid_amount = may_pay(&info, &cfg.base_denom)?;

    // skew _before_ modification
    let initial_skew = ms.skew()?;

    // Determine the type of modification to the position based on the new size
    let modification = if new_size.is_zero() {
        // Close the position

        // Update the denoms accumulators.
        // Funding rates and index is updated to the current block time (using old size).
        ms.close_position(env.block.time.seconds(), denom_price, base_denom_price, &position)?;

        PositionModification::Decrease(entry_size)
    } else {
        // When a denom is disabled it should be close only
        if !ms.enabled {
            return Err(ContractError::PositionCannotBeModifiedIfDenomDisabled {
                denom,
            });
        }

        // Validate and adjust the position's size
        let modification =
            adjust_position_with_validation(new_size, entry_size, denom_price, &perp_params, &ms)?;

        // Update the denoms accumulators.
        // Funding rates and index is updated to the current block time (using old size).
        ms.modify_position(
            env.block.time.seconds(),
            denom_price,
            base_denom_price,
            &position,
            new_size,
        )?;

        modification
    };

    // Check if this is a position flip to save the opening fee rate later
    let is_position_flip = matches!(modification, PositionModification::Flip(_, _));

    // Compute the position's unrealized PnL
    let pnl_amounts = position.compute_pnl(
        &ms.funding,
        initial_skew,
        denom_price,
        base_denom_price,
        opening_fee_rate,
        closing_fee_rate,
        modification,
    )?;

    // Convert PnL amounts to coins
    let pnl = pnl_amounts.to_coins(&cfg.base_denom).pnl;

    let mut msgs = vec![];

    // Apply the payment to the credit manager if necessary
    apply_payment_to_cm_if_needed(&cfg, cm_address, &mut msgs, paid_amount, &pnl)?;

    // Reduce the initial skew by the old position size. It is new "initial skew".
    let initial_skew = initial_skew.checked_sub(position.size)?;

    // Prepare attributes for the response
    let mut attrs = vec![];
    let entry_accrued_funding_per_unit_in_base_denom =
        ms.funding.last_funding_accrued_per_unit_in_base_denom;
    update_position_attributes(
        &mut attrs,
        &denom,
        &position,
        new_size,
        denom_price,
        initial_skew,
        entry_accrued_funding_per_unit_in_base_denom,
        &pnl_amounts,
    );

    // Update the realized PnL, market state, and total cash flow based on the new amounts
    apply_pnl_and_fees(
        &cfg,
        &addresses[&MarsAddressType::RewardsCollector],
        &mut ms,
        &mut tcf,
        &mut realized_pnl,
        &pnl_amounts,
        &mut attrs,
        &mut msgs,
    )?;

    // Modify or delete the position state based on the new size
    let method = if new_size.is_zero() {
        // Delete the position if the new size is zero
        POSITIONS.remove(deps.storage, (&account_id, &denom));

        // Clean up the stored opening fee rate for this position
        ACCOUNT_OPENING_FEE_RATES.remove(deps.storage, (&account_id, &denom));

        "close_position"
    } else {
        // Save the updated position state
        let mut realized_pnl = position.realized_pnl;
        realized_pnl.add(&pnl_amounts)?;

        let entry_exec_price =
            opening_execution_price(initial_skew, ms.funding.skew_scale, new_size, denom_price)?;

        POSITIONS.save(
            deps.storage,
            (&account_id, &denom),
            &Position {
                size: new_size,
                entry_price: denom_price,
                entry_exec_price,
                entry_accrued_funding_per_unit_in_base_denom,
                initial_skew,
                realized_pnl,
            },
        )?;

        // Update the opening fee rate if this was a position flip (new opening fee charged)
        if is_position_flip {
            ACCOUNT_OPENING_FEE_RATES.save(
                deps.storage,
                (&account_id, &denom),
                &opening_fee_rate,
            )?;
        }

        "modify_position"
    };

    // Save the updated state variables
    REALIZED_PNL.save(deps.storage, (&account_id, &denom), &realized_pnl)?;
    MARKET_STATES.save(deps.storage, &denom, &ms)?;
    TOTAL_CASH_FLOW.save(deps.storage, &tcf)?;

    // Return the response with the appropriate attributes
    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", method)
        .add_attribute("account_id", account_id)
        .add_attributes(attrs))
}

/// Closes all positions for a given account.
pub fn close_all_positions(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    account_id: String,
    action: ActionKind,
    discount_pct: Option<Decimal>,
) -> ContractResult<Response> {
    let cfg = CONFIG.load(deps.storage)?;

    let addresses = query_contract_addrs(
        deps.as_ref(),
        &cfg.address_provider,
        vec![
            MarsAddressType::CreditManager,
            MarsAddressType::Oracle,
            MarsAddressType::Params,
            MarsAddressType::RewardsCollector,
        ],
    )?;

    // Only the credit manager contract can adjust positions
    ensure_eq!(
        info.sender,
        addresses[&MarsAddressType::CreditManager],
        ContractError::SenderIsNotCreditManager
    );

    let oracle = get_oracle_adapter(&addresses[&MarsAddressType::Oracle]);
    let params = get_params_adapter(&addresses[&MarsAddressType::Params]);

    // Read all positions for the account
    let account_positions: Vec<_> = {
        // Collect all positions for the account to avoid problems with mutable/immutable borrows in the same scope
        POSITIONS
            .prefix(&account_id)
            .range(deps.storage, None, None, Order::Ascending)
            .map(|item| {
                let (denom, position) = item?;
                Ok((denom, position))
            })
            .collect::<ContractResult<Vec<_>>>()?
    };

    // Read total cash flow
    let mut tcf = TOTAL_CASH_FLOW.may_load(deps.storage)?.unwrap_or_default();

    // When modifying a position, we must realise all PnL. The credit manager
    // may send no coin (in case the position is winning or breaking even) or
    // one coin of the base denom (i.e usdc) in case the position is losing
    let paid_amount = may_pay(&info, &cfg.base_denom)?;

    let base_denom_price =
        oracle.query_price(&deps.querier, &cfg.base_denom, action.clone())?.price;

    let mut attrs = vec![];
    let mut msgs = vec![];
    let mut pnl_amounts_accumulator = PnlAmounts::default();
    for (denom, position) in account_positions {
        let mut realized_pnl =
            REALIZED_PNL.may_load(deps.storage, (&account_id, &denom))?.unwrap_or_default();
        let mut ms = MARKET_STATES.load(deps.storage, &denom)?;

        // Params for the given market
        let perp_params = params.query_perp_params(&deps.querier, &denom)?;

        // Prices
        let denom_price = oracle.query_price(&deps.querier, &denom, action.clone())?.price;

        // skew _before_ modification
        let initial_skew = ms.skew()?;

        // Update the denom's accumulators.
        // Funding rates and index is updated to the current block time (using old size).
        ms.close_position(env.block.time.seconds(), denom_price, base_denom_price, &position)?;

        // Apply discount to fee rates if provided
        let (opening_fee_rate, closing_fee_rate) =
            compute_discounted_fee_rates(&perp_params, discount_pct);

        // Compute the position's unrealized PnL
        let pnl_amounts = position.compute_pnl(
            &ms.funding,
            initial_skew,
            denom_price,
            base_denom_price,
            opening_fee_rate,
            closing_fee_rate,
            PositionModification::Decrease(position.size),
        )?;

        // Prepare attributes for the response
        update_position_attributes(
            &mut attrs,
            &denom,
            &position,
            Int128::zero(),
            denom_price,
            initial_skew,
            ms.funding.last_funding_accrued_per_unit_in_base_denom,
            &pnl_amounts,
        );

        pnl_amounts_accumulator.add(&pnl_amounts)?;

        apply_pnl_and_fees(
            &cfg,
            &addresses[&MarsAddressType::RewardsCollector],
            &mut ms,
            &mut tcf,
            &mut realized_pnl,
            &pnl_amounts,
            &mut attrs,
            &mut msgs,
        )?;

        // Remove the position
        POSITIONS.remove(deps.storage, (&account_id, &denom));

        // Clean up the stored opening fee rate for this position
        ACCOUNT_OPENING_FEE_RATES.remove(deps.storage, (&account_id, &denom));

        // Save updated states
        REALIZED_PNL.save(deps.storage, (&account_id, &denom), &realized_pnl)?;
        MARKET_STATES.save(deps.storage, &denom, &ms)?;
    }

    // Convert PnL amounts to coins
    let pnl = pnl_amounts_accumulator.to_coins(&cfg.base_denom).pnl;

    apply_payment_to_cm_if_needed(
        &cfg,
        &addresses[&MarsAddressType::CreditManager],
        &mut msgs,
        paid_amount,
        &pnl,
    )?;

    TOTAL_CASH_FLOW.save(deps.storage, &tcf)?;

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "close_all_positions")
        .add_attribute("account_id", account_id)
        .add_attribute("total_realized_pnl_change", pnl_amounts_accumulator.pnl.to_string())
        .add_attributes(attrs))
}

/// Adjusts the position size with validation and determines the type of position modification.
///
/// This function takes the new position size and validates it against the current position size,
/// market parameters, and open interest limits. Depending on whether the new size is an increase,
/// decrease, or flip of the position, it returns the appropriate `PositionModification`.
fn adjust_position_with_validation(
    new_size: Int128,
    entry_size: Int128,
    denom_price: Decimal,
    perp_params: &PerpParams,
    ms: &MarketState,
) -> Result<PositionModification, ContractError> {
    let position_value = new_size.unsigned_abs().checked_mul_floor(denom_price)?;
    let modification = PositionModification::from_new_size(entry_size, new_size)?;
    match modification {
        PositionModification::Increase(..) => {
            // Enforce position size cannot be too big when increasing
            ensure_max_position(position_value, perp_params)?;

            // Validate the position's size against OI limits
            ms.validate_open_interest(new_size, entry_size, denom_price, perp_params)?;
        }
        PositionModification::Decrease(..) => {
            // Enforce min size when decreasing
            ensure_min_position(position_value, perp_params)?;
        }
        PositionModification::Flip(..) => {
            // Ensure min and max position size when flipping a position
            ensure_min_position(position_value, perp_params)?;
            ensure_max_position(position_value, perp_params)?;

            // Ensure the position's size against OI limits
            ms.validate_open_interest(new_size, entry_size, denom_price, perp_params)?;
        }
    };
    Ok(modification)
}

/// Applies profit and loss (PnL) and associated fees to the market state and cash flow.
///
/// This function performs the following tasks:
/// 1. **Update Realized PnL**: Adds unrealized PnL to realized PnL.
/// 2. **Calculate and Send Protocol Fees**: Computes protocol fees as a percentage of unrealized opening and closing fees,
///    then creates and adds a message to send these fees to the rewards collector if applicable.
/// 3. **Adjust PnL for Protocol Fees**: Calculates the PnL after accounting for protocol fees and updates the market and total cash flows accordingly.
/// 4. **Update Response**: Adds attributes to the response indicating the protocol opening and closing fees.
pub fn apply_pnl_and_fees(
    cfg: &Config<Addr>,
    rewards_collector: &Addr,
    ms: &mut MarketState,
    tcf: &mut CashFlow,
    realized_pnl: &mut PnlAmounts,
    unrealized_pnl: &PnlAmounts,
    attrs: &mut Vec<Attribute>,
    msgs: &mut Vec<CosmosMsg>,
) -> ContractResult<Uint128> {
    // Update realized pnl with total fees
    realized_pnl.add(unrealized_pnl)?;

    // Protocol fee is calculated on the opening and closing fee charged to the user
    // The calculation is rounded up in favour of the protocol
    // Absolute values are used, as unrealized opening/closing fee are a cost (negative) to
    // the user, but a revenue (positive) to the protocol
    let protocol_opening_fee =
        unrealized_pnl.opening_fee.unsigned_abs().checked_mul_ceil(cfg.protocol_fee_rate)?;
    let protocol_closing_fee =
        unrealized_pnl.closing_fee.unsigned_abs().checked_mul_ceil(cfg.protocol_fee_rate)?;

    let total_protocol_fee = protocol_opening_fee + protocol_closing_fee;

    if !total_protocol_fee.is_zero() {
        // Create message to send protocol fee to rewards collector
        let msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: rewards_collector.into(),
            amount: coins(total_protocol_fee.u128(), &cfg.base_denom),
        });

        msgs.push(msg);
    }

    // Example calculation for pnl without protocol fee:
    // opening_fee = -2
    // closing_fee = -4
    // price_pnl = 10
    // funding = -1
    // pnl = opening_fee + closing_fee + funding + price_pnl = -2 + (-4) + (-1) + 10 = 3
    //
    // protocol_fee_rate = 50%
    //
    // opening_fee = -1
    // closing_fee = -2
    // price_pnl = 10
    // funding = -1
    // protocol_opening_fee = 1
    // protocol_closing_fee = 2
    // total_protocol_fee = 3
    // pnl_without_protocol = -1 + (-2) + (-1) + 10 = 6
    //
    // pnl + protocol_opening_fee + protocol_closing_fee = 3 + 1 + 2 = 6 which is equal to pnl_without_protocol

    let pnl_without_protocol_fee = PnlAmounts {
        opening_fee: unrealized_pnl.opening_fee.checked_add(protocol_opening_fee.try_into()?)?,
        closing_fee: unrealized_pnl.closing_fee.checked_add(protocol_closing_fee.try_into()?)?,
        pnl: unrealized_pnl
            .pnl
            .checked_add(protocol_opening_fee.try_into()?)?
            .checked_add(protocol_closing_fee.try_into()?)?,
        ..unrealized_pnl.clone()
    };

    // Apply pnl to denom cash flow (without protocol fee)
    ms.cash_flow.add(&pnl_without_protocol_fee, total_protocol_fee)?;

    // Apply pnl to total cash flow (without protocol fee)
    tcf.add(&pnl_without_protocol_fee, total_protocol_fee)?;

    // Add attributes for protocol fees
    attrs.push(Attribute::new("protocol_opening_fee", protocol_opening_fee.to_string()));
    attrs.push(Attribute::new("protocol_closing_fee", protocol_closing_fee.to_string()));

    Ok(total_protocol_fee)
}

/// Applies payments to the credit manager if necessary based on the PnL and paid amount.
fn apply_payment_to_cm_if_needed(
    cfg: &Config<Addr>,
    cm_address: &Addr,
    msgs: &mut Vec<CosmosMsg>,
    paid_amount: Uint128,
    pnl: &PnL,
) -> ContractResult<()> {
    let payment_amount = get_payment_amount_to_cm(&cfg.base_denom, paid_amount, pnl)?;
    if !payment_amount.is_zero() {
        // send coins to credit manager
        let send_msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: cm_address.clone().into(),
            amount: coins(payment_amount.u128(), &cfg.base_denom),
        });
        msgs.push(send_msg);
    }

    Ok(())
}

/// Compute how many coins should be sent to the credit account.
/// Credit manager doesn't send more coins than required.
fn get_payment_amount_to_cm(
    base_denom: &str,
    paid_amount: Uint128,
    pnl: &PnL,
) -> Result<Uint128, ContractError> {
    match pnl {
        PnL::Profit(Coin {
            amount,
            ..
        }) => {
            if !paid_amount.is_zero() {
                // if the position is profitable, the credit manager should not send any coins
                return Err(ContractError::InvalidPayment {
                    denom: base_denom.to_string(),
                    required: Uint128::zero(),
                    received: paid_amount,
                });
            }

            Ok(*amount)
        }
        PnL::Loss(Coin {
            amount,
            ..
        }) => {
            if paid_amount != *amount {
                // if the position is losing, the credit manager should send exactly one coin
                // of the base denom
                return Err(ContractError::InvalidPayment {
                    denom: base_denom.to_string(),
                    required: *amount,
                    received: paid_amount,
                });
            }

            Ok(Uint128::zero())
        }
        PnL::BreakEven => {
            if !paid_amount.is_zero() {
                // if the position is breaking even, the credit manager should not send any coins
                return Err(ContractError::InvalidPayment {
                    denom: base_denom.to_string(),
                    required: Uint128::zero(),
                    received: paid_amount,
                });
            }

            Ok(Uint128::zero())
        }
    }
}
