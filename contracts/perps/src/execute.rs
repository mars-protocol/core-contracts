use std::cmp::Ordering;

use cosmwasm_std::{
    coins, ensure_eq, Addr, BankMsg, Coin, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Response,
    StdError, Storage, Uint128,
};
use cw_utils::{may_pay, must_pay};
use mars_types::{
    math::SignedDecimal,
    oracle::ActionKind,
    perps::{
        CashFlow, Config, DenomState, Funding, PnL, PnlAmounts, Position, UnlockState, VaultState,
    },
};

use crate::{
    accounting::CashFlowExt,
    denom::DenomStateExt,
    error::{ContractError, ContractResult},
    position::{PositionExt, PositionModification},
    pricing::opening_execution_price,
    state::{
        decrease_deposit_shares, increase_deposit_shares, CONFIG, DENOM_STATES, OWNER, POSITIONS,
        REALIZED_PNL, TOTAL_CASH_FLOW, UNLOCKS, VAULT_STATE,
    },
    utils::{ensure_max_position, ensure_min_position, ensure_position_not_flipped},
    vault::{amount_to_shares, shares_to_amount},
};

pub fn initialize(store: &mut dyn Storage, cfg: Config<Addr>) -> ContractResult<Response> {
    CONFIG.save(store, &cfg)?;

    // initialize vault state to zero total liquidity and zero total shares
    VAULT_STATE.save(store, &VaultState::default())?;

    // initialize global cash flow to zero
    TOTAL_CASH_FLOW.save(store, &CashFlow::default())?;

    Ok(Response::new().add_attribute("method", "initialize"))
}

pub fn init_denom(
    store: &mut dyn Storage,
    env: Env,
    sender: &Addr,
    denom: &str,
    max_funding_velocity: Decimal,
    skew_scale: Decimal,
) -> ContractResult<Response> {
    OWNER.assert_owner(store, sender)?;

    if DENOM_STATES.has(store, denom) {
        return Err(ContractError::DenomAlreadyExists {
            denom: denom.into(),
        });
    }

    if skew_scale.is_zero() {
        return Err(ContractError::InvalidParam {
            reason: "skew_scale cannot be zero".to_string(),
        });
    }

    let denom_state = DenomState {
        enabled: true,
        funding: Funding {
            max_funding_velocity,
            skew_scale,
            last_funding_rate: SignedDecimal::zero(),
            last_funding_accrued_per_unit_in_base_denom: SignedDecimal::zero(),
        },
        last_updated: env.block.time.seconds(),
        ..Default::default()
    };
    DENOM_STATES.save(store, denom, &denom_state)?;

    Ok(Response::new()
        .add_attribute("method", "init_denom")
        .add_attribute("denom", denom)
        .add_attribute("max_funding_velocity", max_funding_velocity.to_string())
        .add_attribute("skew_scale", skew_scale.to_string()))
}

pub fn enable_denom(
    store: &mut dyn Storage,
    env: Env,
    sender: &Addr,
    denom: &str,
) -> ContractResult<Response> {
    OWNER.assert_owner(store, sender)?;

    DENOM_STATES.update(store, denom, |maybe_ds| {
        // if the denom does not already exist then we cannot enable it
        let Some(mut ds) = maybe_ds else {
            return Err(ContractError::DenomNotFound {
                denom: denom.into(),
            });
        };

        // if the denom already exists, if must have not already been enabled
        if ds.enabled {
            return Err(ContractError::DenomEnabled {
                denom: denom.into(),
            });
        }

        // now we know the denom exists and is not enabled
        // flip the enabled parameter to true and return
        ds.enabled = true;

        // When denom is disabled there is no trading activity so funding shouldn't be changed.
        // We just shift the last_updated time.
        ds.last_updated = env.block.time.seconds();

        Ok(ds)
    })?;

    Ok(Response::new().add_attribute("method", "enable_denom").add_attribute("denom", denom))
}

pub fn disable_denom(
    deps: DepsMut,
    env: Env,
    sender: &Addr,
    denom: &str,
) -> ContractResult<Response> {
    OWNER.assert_owner(deps.storage, sender)?;

    let cfg = CONFIG.load(deps.storage)?;

    DENOM_STATES.update(deps.storage, denom, |maybe_ds| {
        let Some(mut ds) = maybe_ds else {
            return Err(ContractError::DenomNotFound {
                denom: denom.into(),
            });
        };

        let current_time = env.block.time.seconds();

        let denom_price = cfg.oracle.query_price(&deps.querier, denom, ActionKind::Default)?.price;
        let base_denom_price =
            cfg.oracle.query_price(&deps.querier, &cfg.base_denom, ActionKind::Default)?.price;

        // refresh funding rate and index before disabling trading
        let current_funding = ds.current_funding(current_time, denom_price, base_denom_price)?;
        ds.funding = current_funding;

        ds.enabled = false;
        ds.last_updated = current_time;

        Ok(ds)
    })?;

    Ok(Response::new().add_attribute("method", "disable_denom").add_attribute("denom", denom))
}

pub fn deposit(
    deps: DepsMut,
    info: MessageInfo,
    current_time: u64,
    account_id: &str,
) -> ContractResult<Response> {
    let cfg = CONFIG.load(deps.storage)?;

    // only the credit manager contract can open positions
    ensure_eq!(info.sender, cfg.credit_manager, ContractError::SenderIsNotCreditManager);

    let mut vs = VAULT_STATE.load(deps.storage)?;

    // find the deposit amount
    let amount = must_pay(&info, &cfg.base_denom)?;

    // compute the new shares to be minted to the depositor
    let shares =
        amount_to_shares(&deps.as_ref(), &vs, &cfg.oracle, current_time, &cfg.base_denom, amount)?;

    // increment total liquidity and deposit shares
    vs.total_liquidity = vs.total_liquidity.checked_add(amount)?;
    vs.total_shares = vs.total_shares.checked_add(shares)?;
    VAULT_STATE.save(deps.storage, &vs)?;

    // increment the user's deposit shares
    increase_deposit_shares(deps.storage, account_id, shares)?;

    Ok(Response::new()
        .add_attribute("method", "deposit")
        .add_attribute("amount", amount)
        .add_attribute("shares", shares))
}

pub fn unlock(
    deps: DepsMut,
    info: MessageInfo,
    current_time: u64,
    account_id: &str,
    shares: Uint128,
) -> ContractResult<Response> {
    let cfg = CONFIG.load(deps.storage)?;

    // only the credit manager contract can open positions
    ensure_eq!(info.sender, cfg.credit_manager, ContractError::SenderIsNotCreditManager);

    let mut vs = VAULT_STATE.load(deps.storage)?;

    // convert the shares to amount
    let amount =
        shares_to_amount(&deps.as_ref(), &vs, &cfg.oracle, current_time, &cfg.base_denom, shares)?;

    // cannot unlock when there is zero shares
    if amount.is_zero() {
        return Err(ContractError::ZeroShares);
    }

    // decrement total liquidity and deposit shares
    vs.total_liquidity = vs.total_liquidity.checked_sub(amount)?;
    vs.total_shares = vs.total_shares.checked_sub(shares)?;
    VAULT_STATE.save(deps.storage, &vs)?;

    // decrement the user's deposit shares
    decrease_deposit_shares(deps.storage, account_id, shares)?;

    // add new unlock position
    let cooldown_end = current_time + cfg.cooldown_period;
    UNLOCKS.update(deps.storage, account_id, |maybe_unlocks| {
        let mut unlocks = maybe_unlocks.unwrap_or_default();

        unlocks.push(UnlockState {
            created_at: current_time,
            cooldown_end,
            amount,
        });

        Ok::<Vec<UnlockState>, StdError>(unlocks)
    })?;

    Ok(Response::new()
        .add_attribute("method", "unlock")
        .add_attribute("amount", amount)
        .add_attribute("shares", shares)
        .add_attribute("created_at", current_time.to_string())
        .add_attribute("cooldown_end", cooldown_end.to_string()))
}

pub fn withdraw(
    store: &mut dyn Storage,
    info: MessageInfo,
    current_time: u64,
    account_id: &str,
) -> ContractResult<Response> {
    let cfg = CONFIG.load(store)?;

    // only the credit manager contract can open positions
    ensure_eq!(info.sender, cfg.credit_manager, ContractError::SenderIsNotCreditManager);

    let unlocks = UNLOCKS.load(store, account_id)?;

    // find all unlocked positions
    let (unlocked, unlocking): (Vec<_>, Vec<_>) =
        unlocks.into_iter().partition(|us| us.cooldown_end <= current_time);

    // cannot withdraw when there is zero unlocked positions
    if unlocked.is_empty() {
        return Err(ContractError::UnlockedPositionsNotFound {});
    }

    // clear state if no more unlocking positions
    if unlocking.is_empty() {
        UNLOCKS.remove(store, account_id);
    } else {
        UNLOCKS.save(store, account_id, &unlocking)?;
    }

    // compute the total amount to be withdrawn
    let unlocked_amt = unlocked.into_iter().map(|us| us.amount).sum::<Uint128>();

    Ok(Response::new()
        .add_attribute("method", "withdraw")
        .add_attribute("amount", unlocked_amt)
        .add_message(BankMsg::Send {
            to_address: info.sender.into(),
            amount: coins(unlocked_amt.u128(), cfg.base_denom),
        }))
}

pub fn open_position(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    account_id: String,
    denom: String,
    size: SignedDecimal,
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

    // each account can only have one position for a denom at the same time
    if POSITIONS.has(deps.storage, (&account_id, &denom)) {
        return Err(ContractError::PositionExists {
            account_id,
            denom,
        });
    }

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
    let position_value = size.abs.checked_mul(denom_price)?.to_uint_floor();
    ensure_min_position(position_value, &perp_params)?;

    // the position's initial value cannot be too big
    ensure_max_position(position_value, &perp_params)?;

    // validate the position's size against OI limits
    ds.validate_open_interest(size, denom_price, &perp_params)?;

    // skew _before_ modification
    let initial_skew = ds.skew()?;

    // Update the denom's accumulators.
    // Funding rates and index is updated to the current block time (using old size).
    ds.open_position(env.block.time.seconds(), size, denom_price, base_denom_price)?;

    // update realized PnL with opening fee
    if !opening_fee_amt.is_zero() {
        let mut realized_pnl =
            REALIZED_PNL.may_load(deps.storage, (&account_id, &denom))?.unwrap_or_default();
        let mut tcf = TOTAL_CASH_FLOW.may_load(deps.storage)?.unwrap_or_default();

        apply_opening_fee_to_realized_pnl(&mut realized_pnl, &mut ds, &mut tcf, opening_fee_amt)?;

        REALIZED_PNL.save(deps.storage, (&account_id, &denom), &realized_pnl)?;
        TOTAL_CASH_FLOW.save(deps.storage, &tcf)?;
    }

    let entry_exec_price =
        opening_execution_price(initial_skew, ds.funding.skew_scale, size, denom_price)?.abs;

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
            realized_pnl: PnlAmounts::from_opening_fee(opening_fee_amt)?,
        },
    )?;

    Ok(Response::new()
        .add_attribute("method", "open_position")
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
    update_position_state(deps, env, info, position, account_id, denom, SignedDecimal::zero())
}

pub fn modify_position(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    account_id: String,
    denom: String,
    new_size: SignedDecimal,
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
    new_size: SignedDecimal,
) -> ContractResult<Response> {
    let mut msgs = vec![];

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

    let entry_size = position.size;

    // Check if we have flipped sides (e.g long -> short or vice versa).
    // To reduce complexity and contract size we reject this.
    // Users should use independent close and open actions.
    ensure_position_not_flipped(entry_size, new_size)?;

    // Prices
    let entry_price = position.entry_price;
    let denom_price = cfg.oracle.query_price(&deps.querier, &denom, ActionKind::Default)?.price;
    let base_denom_price =
        cfg.oracle.query_price(&deps.querier, &cfg.base_denom, ActionKind::Default)?.price;
    let position_value = new_size.abs.checked_mul(denom_price)?.to_uint_floor();

    // When modifying a position, we must realise all PnL. The credit manager
    // may send no coin (in case the position is winning or breaking even) or
    // one coin of the base denom (i.e usdc) in case the position is losing
    let paid_amount = may_pay(&info, &cfg.base_denom)?;

    // skew _before_ modification
    let initial_skew = ds.skew()?;

    let modification = match new_size.abs.cmp(&entry_size.abs) {
        // Close the position
        Ordering::Less if new_size.is_zero() => {
            // Update the denom's accumulators.
            // Funding rates and index is updated to the current block time (using old size).
            ds.close_position(env.block.time.seconds(), denom_price, base_denom_price, &position)?;

            PositionModification::Decrease(entry_size)
        }

        // Decrease the position
        Ordering::Less => {
            // Enforce min size when decreasing
            ensure_min_position(position_value, &perp_params)?;

            // Update the denom's accumulators.
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

        // Increase position
        Ordering::Greater => {
            // When a denom is disabled it should be close only
            if !ds.enabled {
                return Err(ContractError::DenomNotEnabled {
                    denom,
                });
            }

            // Enforce position size cannot be too big when increasing
            ensure_max_position(position_value, &perp_params)?;

            let q_change = new_size.checked_sub(entry_size)?;

            // validate the position's size against OI limits
            let perp_params = cfg.params.query_perp_params(&deps.querier, &denom)?;
            ds.validate_open_interest(q_change, denom_price, &perp_params)?; // q change

            // Update the denom's accumulators.
            // Funding rates and index is updated to the current block time (using old size).
            ds.modify_position(
                env.block.time.seconds(),
                denom_price,
                base_denom_price,
                &position,
                new_size,
            )?;

            PositionModification::Increase(q_change)
        }

        // Means we have submitted a new size the same as the old size.
        Ordering::Equal => {
            return Err(ContractError::IllegalPositionModification {
                reason: "new_size is equal to old_size.".to_string(),
            })
        }
    };

    // REALISE PNL
    // ===========
    // compute the position's unrealized PnL
    let (_, pnl_amounts) = position.compute_pnl(
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

    let send_amount = execute_payment(&cfg.base_denom, paid_amount, &pnl)?;

    if !send_amount.is_zero() {
        // send coins to credit manager
        let send_msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: cfg.credit_manager.into(),
            amount: coins(send_amount.u128(), cfg.base_denom),
        });
        msgs.push(send_msg);
    }

    apply_new_amounts_to_realized_pnl(&mut realized_pnl, &mut ds, &mut tcf, &pnl_amounts)?;

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
            opening_execution_price(initial_skew, ds.funding.skew_scale, new_size, denom_price)?
                .abs;

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

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("method", method)
        .add_attribute("account_id", account_id)
        .add_attribute("denom", denom)
        .add_attribute("starting_size", entry_size.to_string())
        .add_attribute("new_size", new_size.to_string())
        .add_attribute("entry_price", entry_price.to_string())
        .add_attribute("current_price", denom_price.to_string())
        .add_attribute("realised_pnl", pnl.to_string()))
}

/// Update realized PnL accumulators with opening fee
fn apply_opening_fee_to_realized_pnl(
    realized_pnl: &mut PnlAmounts,
    ds: &mut DenomState,
    tcf: &mut CashFlow,
    opening_fee_amt: Uint128,
) -> ContractResult<()> {
    realized_pnl.add_opening_fee(opening_fee_amt)?;
    ds.cash_flow.add_opening_fee(opening_fee_amt)?;
    tcf.add_opening_fee(opening_fee_amt)?;
    Ok(())
}

/// Update realized PnL accumulators with new PnL amounts
fn apply_new_amounts_to_realized_pnl(
    realized_pnl: &mut PnlAmounts,
    ds: &mut DenomState,
    tcf: &mut CashFlow,
    pnl_amouts: &PnlAmounts,
) -> ContractResult<()> {
    realized_pnl.add(pnl_amouts)?;
    ds.cash_flow.add(pnl_amouts)?;
    tcf.add(pnl_amouts)?;
    Ok(())
}

/// Compute how many coins should be sent to the credit account.
/// Credit manager doesn't send more coins than required.
fn execute_payment(
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
