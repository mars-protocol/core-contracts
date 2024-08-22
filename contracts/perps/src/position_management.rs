use std::{cmp::Ordering, collections::HashMap};

use cosmwasm_std::{
    coins, ensure_eq, Addr, BankMsg, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Order, Response,
    StdError, Uint128,
};
use cw_utils::{may_pay, must_pay};
use mars_types::{
    address_provider,
    address_provider::MarsAddressType,
    oracle::ActionKind,
    perps::{CashFlow, Config, DenomState, PnL, PnlAmounts, Position},
    signed_uint::SignedUint,
};

use crate::{
    accounting::CashFlowExt,
    denom::DenomStateExt,
    error::{ContractError, ContractResult},
    position::{PositionExt, PositionModification},
    pricing::opening_execution_price,
    state::{CONFIG, DENOM_STATES, POSITIONS, REALIZED_PNL, TOTAL_CASH_FLOW},
    utils::{ensure_max_position, ensure_min_position},
};

pub fn execute_perp_order(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    account_id: String,
    denom: String,
    size: SignedUint,
    reduce_only: Option<bool>,
) -> ContractResult<Response> {
    let position = POSITIONS.may_load(deps.storage, (&account_id, &denom))?;
    let reduce_only_checked = reduce_only.unwrap_or(false);

    match position {
        None if reduce_only_checked => Err(ContractError::IllegalPositionModification {
            reason: "Cannot open position if reduce_only = true".to_string(),
        }),
        None => open_position(deps, env, info, account_id, denom, size),
        Some(position)
            if reduce_only_checked && size.is_positive() == position.size.is_positive() =>
        {
            Err(ContractError::IllegalPositionModification {
                reason: "Cannot increase position if reduce_only = true".to_string(),
            })
        }
        Some(position) => {
            let new_size = if reduce_only_checked && size.abs > position.size.abs {
                SignedUint::zero()
            } else {
                position.size.checked_add(size)?
            };

            update_position_state(deps, env, info, position, account_id, denom, new_size)
        }
    }
}

pub fn open_position(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    account_id: String,
    denom: String,
    size: SignedUint,
) -> ContractResult<Response> {
    let cfg = CONFIG.load(deps.storage)?;

    // only the credit manager contract can open positions
    ensure_eq!(info.sender, cfg.credit_manager, ContractError::SenderIsNotCreditManager);

    // the denom must exists and have been enabled
    let mut ds = DENOM_STATES.load(deps.storage, &denom)?;
    if !ds.enabled {
        return Err(ContractError::DenomNotEnabled {
            denom,
        });
    }

    // number of open positions per account is limited
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

    // each account can only have one position for a denom at the same time
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

    // Params for the given market
    let perp_params = cfg.params.query_perp_params(&deps.querier, &denom)?;

    // find the opening fee amount
    let opening_fee_amt = if !perp_params.opening_fee_rate.is_zero() {
        must_pay(&info, &cfg.base_denom)?
    } else {
        Uint128::zero()
    };

    // query the asset's price
    //
    // this will be the position's entry price, used to compute PnL when closing
    // the position
    let denom_price = cfg.oracle.query_price(&deps.querier, &denom, ActionKind::Default)?.price;
    let base_denom_price =
        cfg.oracle.query_price(&deps.querier, &cfg.base_denom, ActionKind::Default)?.price;

    // the position's initial value cannot be too small
    let position_value = size.abs.checked_mul_floor(denom_price)?;
    ensure_min_position(position_value, &perp_params)?;

    // the position's initial value cannot be too big
    ensure_max_position(position_value, &perp_params)?;

    // validate the position's size against OI limits
    ds.validate_open_interest(size, SignedUint::zero(), denom_price, &perp_params)?;

    // skew _before_ modification
    let initial_skew = ds.skew()?;

    // Update the denom's accumulators.
    // Funding rates and index is updated to the current block time (using old size).
    ds.open_position(env.block.time.seconds(), size, denom_price, base_denom_price)?;

    let mut res = Response::new();
    let mut msgs: Vec<CosmosMsg> = vec![];

    let mut realized_pnl = PnlAmounts::default();

    // update realized PnL with opening fee
    if !opening_fee_amt.is_zero() {
        let mut tcf = TOTAL_CASH_FLOW.may_load(deps.storage)?.unwrap_or_default();

        // Create unrealized pnl
        let unrealized_pnl = PnlAmounts::from_opening_fee(opening_fee_amt)?;

        res = apply_pnl_and_fees(
            &cfg,
            &rewards_collector_addr,
            &mut ds,
            &mut tcf,
            &mut realized_pnl,
            &unrealized_pnl,
            res,
            &mut msgs,
        )?;

        REALIZED_PNL.save(deps.storage, (&account_id, &denom), &realized_pnl)?;
        TOTAL_CASH_FLOW.save(deps.storage, &tcf)?;
    }

    let entry_exec_price =
        opening_execution_price(initial_skew, ds.funding.skew_scale, size, denom_price)?;

    DENOM_STATES.save(deps.storage, &denom, &ds)?;

    // save the user's new position with updated funding
    POSITIONS.save(
        deps.storage,
        (&account_id, &denom),
        &Position {
            size,
            entry_price: denom_price,
            entry_exec_price,
            entry_accrued_funding_per_unit_in_base_denom: ds
                .funding
                .last_funding_accrued_per_unit_in_base_denom,
            initial_skew,
            realized_pnl,
        },
    )?;

    Ok(res
        .add_messages(msgs)
        .add_attribute("action", "open_position")
        .add_attribute("account_id", account_id)
        .add_attribute("denom", denom)
        .add_attribute("size", size.to_string())
        .add_attribute("entry_price", denom_price.to_string()))
}

pub fn close_position(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    account_id: String,
    denom: String,
) -> ContractResult<Response> {
    let position = POSITIONS.load(deps.storage, (&account_id, &denom))?;
    update_position_state(deps, env, info, position, account_id, denom, SignedUint::zero())
}

pub fn modify_position(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    account_id: String,
    denom: String,
    new_size: SignedUint,
) -> ContractResult<Response> {
    let position = POSITIONS.load(deps.storage, (&account_id, &denom))?;
    update_position_state(deps, env, info, position, account_id, denom, new_size)
}

fn update_position_state(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    position: Position,
    account_id: String,
    denom: String,
    new_size: SignedUint,
) -> ContractResult<Response> {
    // States
    let cfg = CONFIG.load(deps.storage)?;

    // Only the credit manager contract can adjust positions
    ensure_eq!(info.sender, cfg.credit_manager, ContractError::SenderIsNotCreditManager);

    // Params for the given market
    let perp_params = cfg.params.query_perp_params(&deps.querier, &denom)?;

    let mut realized_pnl =
        REALIZED_PNL.may_load(deps.storage, (&account_id, &denom))?.unwrap_or_default();
    let mut ds = DENOM_STATES.load(deps.storage, &denom)?;
    let mut tcf = TOTAL_CASH_FLOW.may_load(deps.storage)?.unwrap_or_default();

    let rewards_collector_addr = address_provider::helpers::query_contract_addr(
        deps.as_ref(),
        &cfg.address_provider,
        MarsAddressType::RewardsCollector,
    )?;

    let entry_size = position.size;

    // Prices
    let entry_price = position.entry_price;
    let denom_price = cfg.oracle.query_price(&deps.querier, &denom, ActionKind::Default)?.price;
    let base_denom_price =
        cfg.oracle.query_price(&deps.querier, &cfg.base_denom, ActionKind::Default)?.price;
    let position_value = new_size.abs.checked_mul_floor(denom_price)?;

    // When modifying a position, we must realise all PnL. The credit manager
    // may send no coin (in case the position is winning or breaking even) or
    // one coin of the base denom (i.e usdc) in case the position is losing
    let paid_amount = may_pay(&info, &cfg.base_denom)?;

    // skew _before_ modification
    let initial_skew = ds.skew()?;

    let modification = if new_size.is_zero() {
        // Close the position

        // Update the denoms accumulators.
        // Funding rates and index is updated to the current block time (using old size).
        ds.close_position(env.block.time.seconds(), denom_price, base_denom_price, &position)?;

        PositionModification::Decrease(entry_size)
    } else {
        // When a denom is disabled it should be close only
        if !ds.enabled {
            return Err(ContractError::PositionCannotBeModifiedIfDenomDisabled {
                denom,
            });
        }

        let is_flipped = new_size.negative != entry_size.negative;

        match (is_flipped, new_size.abs.cmp(&entry_size.abs)) {
            // Position is not changed
            (false, Ordering::Equal) => {
                return Err(ContractError::IllegalPositionModification {
                    reason: "new_size is equal to old_size.".to_string(),
                });
            }

            // Position is decreasing
            (false, Ordering::Less) => {
                // Enforce min size when decreasing
                ensure_min_position(position_value, &perp_params)?;

                // Update the denoms accumulators.
                // Funding rates and index is updated to the current block time (using old size).
                ds.modify_position(
                    env.block.time.seconds(),
                    denom_price,
                    base_denom_price,
                    &position,
                    new_size,
                )?;

                let q_change = entry_size.checked_sub(new_size)?;
                PositionModification::Decrease(q_change)
            }

            // Position is increasing
            (false, Ordering::Greater) => {
                // Enforce position size cannot be too big when increasing
                ensure_max_position(position_value, &perp_params)?;

                // validate the position's size against OI limits
                ds.validate_open_interest(new_size, entry_size, denom_price, &perp_params)?;

                // Update the denoms accumulators.
                // Funding rates and index is updated to the current block time (using old size).
                ds.modify_position(
                    env.block.time.seconds(),
                    denom_price,
                    base_denom_price,
                    &position,
                    new_size,
                )?;

                let q_change = new_size.checked_sub(entry_size)?;
                PositionModification::Increase(q_change)
            }

            // Position is flipping
            (true, _) => {
                // Ensure min and max position size when flipping a position
                ensure_min_position(position_value, &perp_params)?;
                ensure_max_position(position_value, &perp_params)?;

                // Ensure the position's size against OI limits
                ds.validate_open_interest(new_size, entry_size, denom_price, &perp_params)?;

                // Update the denoms accumulators.
                // Funding rates and index is updated to the current block time (using old size).
                ds.modify_position(
                    env.block.time.seconds(),
                    denom_price,
                    base_denom_price,
                    &position,
                    new_size,
                )?;

                PositionModification::Flip(new_size, entry_size)
            }
        }
    };

    // REALISE PNL
    // ===========
    // compute the position's unrealized PnL
    let pnl_amounts = position.compute_pnl(
        &ds.funding,
        initial_skew,
        denom_price,
        base_denom_price,
        perp_params.opening_fee_rate,
        perp_params.closing_fee_rate,
        modification,
    )?;

    // Convert PnL amounts to coins
    let pnl = pnl_amounts.to_coins(&cfg.base_denom).pnl;

    let mut res = Response::new();
    let mut msgs = vec![];

    apply_payment_to_cm_if_needed(&cfg, &mut msgs, paid_amount, &pnl)?;

    res = apply_pnl_and_fees(
        &cfg,
        &rewards_collector_addr,
        &mut ds,
        &mut tcf,
        &mut realized_pnl,
        &pnl_amounts,
        res,
        &mut msgs,
    )?;

    // Modify or delete position states
    let method = if new_size.is_zero() {
        // Delete the position and related state when position size modified to zero.
        POSITIONS.remove(deps.storage, (&account_id, &denom));

        "close_position"
    } else {
        // Save updated position
        let mut realized_pnl = position.realized_pnl;
        realized_pnl.add(&pnl_amounts)?;

        let entry_exec_price =
            opening_execution_price(initial_skew, ds.funding.skew_scale, new_size, denom_price)?;

        POSITIONS.save(
            deps.storage,
            (&account_id, &denom),
            &Position {
                size: new_size,
                entry_price: denom_price,
                entry_exec_price,
                entry_accrued_funding_per_unit_in_base_denom: ds
                    .funding
                    .last_funding_accrued_per_unit_in_base_denom,
                initial_skew,
                realized_pnl,
            },
        )?;

        "modify_position"
    };

    // Save updated states
    REALIZED_PNL.save(deps.storage, (&account_id, &denom), &realized_pnl)?;
    DENOM_STATES.save(deps.storage, &denom, &ds)?;
    TOTAL_CASH_FLOW.save(deps.storage, &tcf)?;

    Ok(res
        .add_messages(msgs)
        .add_attribute("action", method)
        .add_attribute("account_id", account_id)
        .add_attribute("denom", denom)
        .add_attribute("entry_size", entry_size.to_string())
        .add_attribute("new_size", new_size.to_string())
        .add_attribute("entry_price", entry_price.to_string())
        .add_attribute("current_price", denom_price.to_string())
        .add_attribute("realised_pnl", pnl.to_string()))
}

fn apply_pnl_and_fees(
    cfg: &Config<Addr>,
    rewards_collector: &Addr,
    ds: &mut DenomState,
    tcf: &mut CashFlow,
    realized_pnl: &mut PnlAmounts,
    unrealized_pnl: &PnlAmounts,
    response: Response,
    msgs: &mut Vec<CosmosMsg>,
) -> ContractResult<Response> {
    // Update realized pnl with total fees
    realized_pnl.add(unrealized_pnl)?;

    // Protocol fee is calculated on the opening and closing fee charged to the user
    // The calculation is rounded up in favour of the protocol
    // Absolute values are used, as unrealized opening/closing fee are a cost (negative) to
    // the user, but a revenue (positive) to the protocol
    let protocol_opening_fee =
        unrealized_pnl.opening_fee.abs.checked_mul_ceil(cfg.protocol_fee_rate)?;
    let protocol_closing_fee =
        unrealized_pnl.closing_fee.abs.checked_mul_ceil(cfg.protocol_fee_rate)?;

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
    // pnl = opening_fee + closing_fee + funding + price_pnl = -3
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
    // pnl_without_protocol = -6
    //
    // pnl - protocol_opening_fee - protocol_closing_fee = -3 - 1 - 2 = -6 which is equal to pnl_without_protocol

    let pnl_without_protocol_fee = PnlAmounts {
        opening_fee: unrealized_pnl.opening_fee.checked_add(protocol_opening_fee.into())?,
        closing_fee: unrealized_pnl.closing_fee.checked_add(protocol_closing_fee.into())?,
        pnl: unrealized_pnl
            .pnl
            .checked_sub(protocol_opening_fee.into())?
            .checked_sub(protocol_closing_fee.into())?,
        ..unrealized_pnl.clone()
    };

    // Apply pnl to denom cash flow (without protocol fee)
    ds.cash_flow.add(&pnl_without_protocol_fee)?;

    // Apply pnl to total cash flow (without protocol fee)
    tcf.add(&pnl_without_protocol_fee)?;

    Ok(response
        .add_attribute("protocol_opening_fee", protocol_opening_fee.to_string())
        .add_attribute("protocol_closing_fee", protocol_closing_fee.to_string()))
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

pub fn close_all_positions(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    account_id: String,
    action: ActionKind,
) -> ContractResult<Response> {
    let cfg = CONFIG.load(deps.storage)?;

    // Only the credit manager contract can adjust positions
    ensure_eq!(info.sender, cfg.credit_manager, ContractError::SenderIsNotCreditManager);

    let rewards_collector_addr = address_provider::helpers::query_contract_addr(
        deps.as_ref(),
        &cfg.address_provider,
        MarsAddressType::RewardsCollector,
    )?;

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
        cfg.oracle.query_price(&deps.querier, &cfg.base_denom, action.clone())?.price;

    let mut msgs = vec![];
    let mut res = Response::new();
    let mut pnl_amounts_accumulator = PnlAmounts::default();
    for (denom, position) in account_positions {
        let mut realized_pnl =
            REALIZED_PNL.may_load(deps.storage, (&account_id, &denom))?.unwrap_or_default();
        let mut ds = DENOM_STATES.load(deps.storage, &denom)?;

        // Params for the given market
        let perp_params = cfg.params.query_perp_params(&deps.querier, &denom)?;

        // Prices
        let denom_price = cfg.oracle.query_price(&deps.querier, &denom, action.clone())?.price;

        // skew _before_ modification
        let initial_skew = ds.skew()?;

        // Update the denom's accumulators.
        // Funding rates and index is updated to the current block time (using old size).
        ds.close_position(env.block.time.seconds(), denom_price, base_denom_price, &position)?;

        // Compute the position's unrealized PnL
        let pnl_amounts = position.compute_pnl(
            &ds.funding,
            initial_skew,
            denom_price,
            base_denom_price,
            perp_params.opening_fee_rate,
            perp_params.closing_fee_rate,
            PositionModification::Decrease(position.size),
        )?;

        pnl_amounts_accumulator.add(&pnl_amounts)?;

        res = apply_pnl_and_fees(
            &cfg,
            &rewards_collector_addr,
            &mut ds,
            &mut tcf,
            &mut realized_pnl,
            &pnl_amounts,
            res,
            &mut msgs,
        )?;

        // Remove the position
        POSITIONS.remove(deps.storage, (&account_id, &denom));

        // Save updated states
        REALIZED_PNL.save(deps.storage, (&account_id, &denom), &realized_pnl)?;
        DENOM_STATES.save(deps.storage, &denom, &ds)?;
    }

    // Convert PnL amounts to coins
    let pnl = pnl_amounts_accumulator.to_coins(&cfg.base_denom).pnl;

    apply_payment_to_cm_if_needed(&cfg, &mut msgs, paid_amount, &pnl)?;

    TOTAL_CASH_FLOW.save(deps.storage, &tcf)?;

    Ok(res
        .add_messages(msgs)
        .add_attribute("action", "close_all_positions")
        .add_attribute("account_id", account_id)
        .add_attribute("realised_pnl", pnl.to_string()))
}

fn apply_payment_to_cm_if_needed(
    cfg: &Config<Addr>,
    msgs: &mut Vec<CosmosMsg>,
    paid_amount: Uint128,
    pnl: &PnL,
) -> ContractResult<()> {
    let payment_amount = get_payment_amount_to_cm(&cfg.base_denom, paid_amount, pnl)?;
    if !payment_amount.is_zero() {
        // send coins to credit manager
        let send_msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: cfg.credit_manager.clone().into(),
            amount: coins(payment_amount.u128(), &cfg.base_denom),
        });
        msgs.push(send_msg);
    }

    Ok(())
}
