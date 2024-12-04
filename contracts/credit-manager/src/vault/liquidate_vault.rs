use std::cmp::min;

use cosmwasm_std::{Coin, Decimal, DepsMut, Env, Response, Uint128};
use cw_vault_standard::VaultInfoResponse;
use mars_types::{
    adapters::vault::{
        UnlockingChange, UnlockingPositions, UpdateType, Vault, VaultError, VaultPositionAmount,
        VaultPositionType, VaultPositionUpdate,
    },
    health::HealthValuesResponse,
};

use crate::{
    error::ContractResult,
    liquidate::{calculate_liquidation, LiquidationResult},
    liquidate_deposit::repay_debt,
    state::VAULT_POSITIONS,
    utils::update_balance_after_vault_liquidation_msg,
    vault::update_vault_position,
};

pub fn liquidate_vault(
    deps: DepsMut,
    env: Env,
    liquidator_account_id: &str,
    liquidatee_account_id: &str,
    debt_coin: Coin,
    request_vault: Vault,
    position_type: VaultPositionType,
    prev_health: HealthValuesResponse,
) -> ContractResult<Response> {
    let liquidatee_position = VAULT_POSITIONS
        .load(deps.storage, (liquidatee_account_id, request_vault.address.clone()))?;

    match liquidatee_position {
        VaultPositionAmount::Unlocked(a) => match position_type {
            VaultPositionType::UNLOCKED => liquidate_unlocked(
                deps,
                env,
                liquidator_account_id,
                liquidatee_account_id,
                debt_coin,
                request_vault,
                a.total(),
                prev_health,
            ),
            _ => Err(VaultError::MismatchedVaultType.into()),
        },
        VaultPositionAmount::Locking(ref a) => match position_type {
            VaultPositionType::LOCKED => liquidate_locked(
                deps,
                env,
                liquidator_account_id,
                liquidatee_account_id,
                debt_coin,
                request_vault,
                a.locked.total(),
                prev_health,
            ),
            VaultPositionType::UNLOCKING => liquidate_unlocking(
                deps,
                env,
                liquidator_account_id,
                liquidatee_account_id,
                debt_coin,
                request_vault,
                liquidatee_position.unlocking(),
                prev_health,
            ),
            _ => Err(VaultError::MismatchedVaultType.into()),
        },
    }
}

fn liquidate_unlocked(
    mut deps: DepsMut,
    env: Env,
    liquidator_account_id: &str,
    liquidatee_account_id: &str,
    debt_coin: Coin,
    request_vault: Vault,
    amount: Uint128,
    prev_health: HealthValuesResponse,
) -> ContractResult<Response> {
    let vault_info = request_vault.query_info(&deps.querier)?;

    let liquidation_res = calculate_vault_liquidation(
        &mut deps,
        env.clone(),
        liquidatee_account_id,
        &debt_coin,
        &request_vault,
        amount,
        &vault_info,
        prev_health,
    )?;

    let mut response = Response::new();

    if !liquidation_res.debt.amount.is_zero() {
        let repay_msg = repay_debt(
            deps.storage,
            &env,
            liquidator_account_id,
            liquidatee_account_id,
            &liquidation_res.debt,
        )?;
        response = response.add_message(repay_msg);
    }

    update_vault_position(
        deps.storage,
        liquidatee_account_id,
        &request_vault.address,
        VaultPositionUpdate::Unlocked(UpdateType::Decrement(
            liquidation_res.liquidatee_request.amount,
        )),
    )?;

    let vault_withdraw_msg =
        request_vault.withdraw_msg(&deps.querier, liquidation_res.liquidatee_request.amount)?;

    let protocol_fee = liquidation_res
        .liquidatee_request
        .amount
        .checked_sub(liquidation_res.liquidator_request.amount)?;
    let protocol_fee_percentage =
        Decimal::checked_from_ratio(protocol_fee, liquidation_res.liquidatee_request.amount)?;

    let update_coin_balance_msg = update_balance_after_vault_liquidation_msg(
        &deps.querier,
        &env.contract.address,
        liquidator_account_id,
        &vault_info.base_token,
        protocol_fee_percentage,
    )?;

    Ok(response
        .add_message(vault_withdraw_msg)
        .add_message(update_coin_balance_msg)
        .add_attribute("action", "liquidate_vault/unlocked")
        .add_attribute("account_id", liquidator_account_id)
        .add_attribute("liquidatee_account_id", liquidatee_account_id)
        .add_attribute("coin_debt_repaid", liquidation_res.debt.to_string())
        .add_attribute("coin_liquidated", liquidation_res.liquidatee_request.to_string())
        .add_attribute(
            "protocol_fee_coin",
            Coin::new(protocol_fee.u128(), liquidation_res.liquidatee_request.denom).to_string(),
        )
        .add_attribute("debt_price", liquidation_res.debt_price.to_string())
        .add_attribute("collateral_price", liquidation_res.collateral_price.to_string()))
}

/// Converts vault coins to their underlying value. This allows for pricing and liquidation
/// values to be determined. Afterward, the final amount is converted back into vault coins.
fn calculate_vault_liquidation(
    deps: &mut DepsMut,
    env: Env,
    liquidatee_account_id: &str,
    debt_coin: &Coin,
    request_vault: &Vault,
    amount: Uint128,
    vault_info: &VaultInfoResponse,
    prev_health: HealthValuesResponse,
) -> ContractResult<LiquidationResult> {
    let total_underlying = request_vault.query_preview_redeem(&deps.querier, amount)?;
    let mut liquidation_res = calculate_liquidation(
        deps,
        env,
        liquidatee_account_id,
        debt_coin,
        &vault_info.base_token,
        total_underlying,
        prev_health,
    )?;
    liquidation_res.liquidatee_request.denom.clone_from(&vault_info.vault_token);
    liquidation_res.liquidatee_request.amount = amount
        .checked_multiply_ratio(liquidation_res.liquidatee_request.amount, total_underlying)?;
    liquidation_res.liquidator_request.denom.clone_from(&vault_info.vault_token);
    liquidation_res.liquidator_request.amount = amount
        .checked_multiply_ratio(liquidation_res.liquidator_request.amount, total_underlying)?;
    Ok(liquidation_res)
}

fn liquidate_unlocking(
    mut deps: DepsMut,
    env: Env,
    liquidator_account_id: &str,
    liquidatee_account_id: &str,
    debt_coin: Coin,
    request_vault: Vault,
    unlocking_positions: UnlockingPositions,
    prev_health: HealthValuesResponse,
) -> ContractResult<Response> {
    let vault_info = request_vault.query_info(&deps.querier)?;

    let liquidation_res = calculate_liquidation(
        &mut deps,
        env.clone(),
        liquidatee_account_id,
        &debt_coin,
        &vault_info.base_token,
        unlocking_positions.total(),
        prev_health,
    )?;

    let mut response = Response::new();

    if !liquidation_res.debt.amount.is_zero() {
        let repay_msg = repay_debt(
            deps.storage,
            &env,
            liquidator_account_id,
            liquidatee_account_id,
            &liquidation_res.debt,
        )?;
        response = response.add_message(repay_msg);
    }

    let mut total_to_liquidate = liquidation_res.liquidatee_request.amount;
    let mut vault_withdraw_msgs = vec![];

    for u in unlocking_positions.positions() {
        let amount = min(u.coin.amount, total_to_liquidate);

        if amount.is_zero() {
            break;
        }

        update_vault_position(
            deps.storage,
            liquidatee_account_id,
            &request_vault.address,
            VaultPositionUpdate::Unlocking(UnlockingChange::Decrement {
                id: u.id,
                amount,
            }),
        )?;

        let msg = request_vault.force_withdraw_unlocking_msg(u.id, Some(amount))?;
        vault_withdraw_msgs.push(msg);

        total_to_liquidate = total_to_liquidate.checked_sub(amount)?;
    }

    let protocol_fee = liquidation_res
        .liquidatee_request
        .amount
        .checked_sub(liquidation_res.liquidator_request.amount)?;
    let protocol_fee_percentage =
        Decimal::checked_from_ratio(protocol_fee, liquidation_res.liquidatee_request.amount)?;

    let update_coin_balance_msg = update_balance_after_vault_liquidation_msg(
        &deps.querier,
        &env.contract.address,
        liquidator_account_id,
        &vault_info.base_token,
        protocol_fee_percentage,
    )?;

    Ok(response
        .add_messages(vault_withdraw_msgs)
        .add_message(update_coin_balance_msg)
        .add_attribute("action", "liquidate_vault/unlocking")
        .add_attribute("account_id", liquidator_account_id)
        .add_attribute("liquidatee_account_id", liquidatee_account_id)
        .add_attribute("coin_debt_repaid", liquidation_res.debt.to_string())
        .add_attribute("coin_liquidated", liquidation_res.liquidatee_request.to_string())
        .add_attribute(
            "protocol_fee_coin",
            Coin::new(protocol_fee.u128(), liquidation_res.liquidatee_request.denom).to_string(),
        )
        .add_attribute("debt_price", liquidation_res.debt_price.to_string())
        .add_attribute("collateral_price", liquidation_res.collateral_price.to_string()))
}

fn liquidate_locked(
    mut deps: DepsMut,
    env: Env,
    liquidator_account_id: &str,
    liquidatee_account_id: &str,
    debt_coin: Coin,
    request_vault: Vault,
    amount: Uint128,
    prev_health: HealthValuesResponse,
) -> ContractResult<Response> {
    let vault_info = request_vault.query_info(&deps.querier)?;

    let liquidation_res = calculate_vault_liquidation(
        &mut deps,
        env.clone(),
        liquidatee_account_id,
        &debt_coin,
        &request_vault,
        amount,
        &vault_info,
        prev_health,
    )?;

    let mut response = Response::new();

    if !liquidation_res.debt.amount.is_zero() {
        let repay_msg = repay_debt(
            deps.storage,
            &env,
            liquidator_account_id,
            liquidatee_account_id,
            &liquidation_res.debt,
        )?;
        response = response.add_message(repay_msg);
    }

    update_vault_position(
        deps.storage,
        liquidatee_account_id,
        &request_vault.address,
        VaultPositionUpdate::Locked(UpdateType::Decrement(
            liquidation_res.liquidatee_request.amount,
        )),
    )?;

    let vault_withdraw_msg = request_vault
        .force_withdraw_locked_msg(&deps.querier, liquidation_res.liquidatee_request.amount)?;

    let protocol_fee = liquidation_res
        .liquidatee_request
        .amount
        .checked_sub(liquidation_res.liquidator_request.amount)?;
    let protocol_fee_percentage =
        Decimal::checked_from_ratio(protocol_fee, liquidation_res.liquidatee_request.amount)?;

    let update_coin_balance_msg = update_balance_after_vault_liquidation_msg(
        &deps.querier,
        &env.contract.address,
        liquidator_account_id,
        &vault_info.base_token,
        protocol_fee_percentage,
    )?;

    Ok(response
        .add_message(vault_withdraw_msg)
        .add_message(update_coin_balance_msg)
        .add_attribute("action", "liquidate_vault/locked")
        .add_attribute("account_id", liquidator_account_id)
        .add_attribute("liquidatee_account_id", liquidatee_account_id)
        .add_attribute("coin_debt_repaid", liquidation_res.debt.to_string())
        .add_attribute("coin_liquidated", liquidation_res.liquidatee_request.to_string())
        .add_attribute(
            "protocol_fee_coin",
            Coin::new(protocol_fee.u128(), liquidation_res.liquidatee_request.denom).to_string(),
        )
        .add_attribute("debt_price", liquidation_res.debt_price.to_string())
        .add_attribute("collateral_price", liquidation_res.collateral_price.to_string()))
}
