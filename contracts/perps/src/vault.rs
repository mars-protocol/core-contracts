use std::cmp::max;

use cosmwasm_std::{
    coins, ensure, to_json_binary, Addr, BankMsg, CosmosMsg, Deps, DepsMut, Int128, MessageInfo,
    Response, Uint128, WasmMsg,
};
use cw_utils::must_pay;
use mars_types::{
    adapters::{oracle::Oracle, params::Params},
    address_provider::{
        helpers::{query_contract_addr, query_contract_addrs},
        MarsAddressType,
    },
    incentives::{ExecuteMsg, IncentiveKind},
    keys::UserIdKey,
    oracle::ActionKind,
    perps::{UnlockState, VaultState},
};

use crate::{
    deleverage::query_vault_cr,
    error::{ContractError, ContractResult},
    market::compute_total_accounting_data,
    state::{
        decrease_deposit_shares, increase_deposit_shares, CONFIG, DEPOSIT_SHARES, UNLOCKS,
        VAULT_STATE,
    },
    utils::{create_user_id_key, get_oracle_adapter, get_params_adapter},
};

pub const DEFAULT_SHARES_PER_AMOUNT: u128 = 1_000_000;

/// Handles the logic for a user depositing funds into the vault.
/// The function verifies the sender's permission to deposit with an optional account id,
/// then calculates the number of shares to mint based on the deposit amount.
/// It updates the total vault balance, the user's deposit shares, and triggers an incentive message.
/// Returns a `Response` with details about the deposit, including the amount deposited and the number of shares minted.
pub fn deposit(
    deps: DepsMut,
    info: MessageInfo,
    current_time: u64,
    account_id: Option<String>,
    max_shares_receivable: Option<Uint128>,
) -> ContractResult<Response> {
    let cfg = CONFIG.load(deps.storage)?;

    let addresses = query_contract_addrs(
        deps.as_ref(),
        &cfg.address_provider,
        vec![
            MarsAddressType::CreditManager,
            MarsAddressType::Oracle,
            MarsAddressType::Params,
            MarsAddressType::Incentives,
        ],
    )?;

    // Don't allow users to create alternative account ids.
    // Only allow credit manager contract to create them.
    // Even if account_id contains empty string we won't allow it.
    if account_id.is_some() && info.sender != addresses[&MarsAddressType::CreditManager] {
        return Err(ContractError::SenderIsNotCreditManager);
    }

    let user_id_key = create_user_id_key(&info.sender, account_id.clone())?;

    let mut vs = VAULT_STATE.load(deps.storage)?;

    // Load the user's shares
    let user_vault_shares = UserVaultShares::load(deps.as_ref(), current_time, &user_id_key)?;
    let user_shares_before = user_vault_shares.total()?;

    let total_vault_shares_before = vs.total_shares;

    let msg = build_incentives_balance_changed_msg(
        &addresses[&MarsAddressType::Incentives],
        &info.sender,
        account_id,
        &cfg.base_denom,
        user_shares_before,
        total_vault_shares_before,
    )?;

    // Find the deposit amount
    let amount = must_pay(&info, &cfg.base_denom)?;

    let oracle = get_oracle_adapter(&addresses[&MarsAddressType::Oracle]);
    let params = get_params_adapter(&addresses[&MarsAddressType::Params]);

    // Compute the new shares to be minted to the depositor
    let shares = amount_to_shares(
        &deps.as_ref(),
        &vs,
        &oracle,
        &params,
        current_time,
        &cfg.base_denom,
        amount,
        ActionKind::Default,
    )?;

    if let Some(msr) = max_shares_receivable {
        if shares >= msr {
            return Err(ContractError::MaximumReceiveExceeded {
                max: msr,
                found: shares,
            });
        }
    }

    // Increment total liquidity and deposit shares
    vs.total_balance = vs.total_balance.checked_add(amount.try_into()?)?;
    vs.total_shares = vs.total_shares.checked_add(shares)?;
    VAULT_STATE.save(deps.storage, &vs)?;

    // Increment the user's deposit shares
    increase_deposit_shares(deps.storage, &user_id_key, shares)?;

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "deposit")
        .add_attribute("denom", cfg.base_denom)
        .add_attribute("amount", amount)
        .add_attribute("shares", shares)
        .add_attribute("user_shares_before", user_shares_before))
}

/// Handles the unlocking of deposited shares, initiating a cooldown period before the user can withdraw.
/// The function verifies the sender's permission to unlock shares and ensures that the amount to unlock is non-zero.
/// It updates the user's deposit shares, adds the unlocked shares to the unlocks list with a cooldown period,
/// and returns a `Response` with details about the unlock.
pub fn unlock(
    deps: DepsMut,
    info: MessageInfo,
    current_time: u64,
    account_id: Option<String>,
    shares: Uint128,
) -> ContractResult<Response> {
    let cfg = CONFIG.load(deps.storage)?;

    let cm_address =
        query_contract_addr(deps.as_ref(), &cfg.address_provider, MarsAddressType::CreditManager)?;

    // Don't allow users to create alternative account ids.
    // Only allow credit manager contract to create them.
    // Even if account_id contains empty string we won't allow it.
    if account_id.is_some() && info.sender != cm_address {
        return Err(ContractError::SenderIsNotCreditManager);
    }

    let user_id_key = create_user_id_key(&info.sender, account_id)?;

    // Cannot unlock zero shares
    if shares.is_zero() {
        return Err(ContractError::ZeroShares);
    }

    // Decrement the user's deposit shares
    decrease_deposit_shares(deps.storage, &user_id_key, shares)?;

    // Add new unlock position
    let cooldown_end = current_time + cfg.cooldown_period;
    UNLOCKS.update(deps.storage, &user_id_key, |maybe_unlocks| {
        let mut unlocks = maybe_unlocks.unwrap_or_default();

        ensure!(
            unlocks.len() < cfg.max_unlocks as usize,
            ContractError::MaxUnlocksReached {
                max_unlocks: cfg.max_unlocks
            }
        );

        unlocks.push(UnlockState {
            created_at: current_time,
            cooldown_end,
            shares,
        });

        Ok::<Vec<UnlockState>, ContractError>(unlocks)
    })?;

    Ok(Response::new()
        .add_attribute("action", "unlock")
        .add_attribute("denom", cfg.base_denom)
        .add_attribute("shares", shares)
        .add_attribute("created_at", current_time.to_string())
        .add_attribute("cooldown_end", cooldown_end.to_string()))
}

/// Handles the withdrawal of unlocked shares from the vault, converting them to the corresponding amount of the base denomination.
/// The function verifies permissions, checks that there are unlocked shares available for withdrawal, and ensures the vault
/// remains collateralized after the withdrawal. It then updates the vault's state and sends the withdrawn amount to the user.
/// Returns a `Response` with details about the withdrawal, including the shares and amount withdrawn.
pub fn withdraw(
    deps: DepsMut,
    info: MessageInfo,
    current_time: u64,
    account_id: Option<String>,
    min_recieve: Option<Uint128>,
) -> ContractResult<Response> {
    let cfg = CONFIG.load(deps.storage)?;

    if !cfg.vault_withdraw_enabled {
        return Err(ContractError::VaultWithdrawDisabled {});
    }

    let addresses = query_contract_addrs(
        deps.as_ref(),
        &cfg.address_provider,
        vec![
            MarsAddressType::CreditManager,
            MarsAddressType::Oracle,
            MarsAddressType::Params,
            MarsAddressType::Incentives,
        ],
    )?;

    // Don't allow users to create alternative account ids.
    // Only allow credit manager contract to create them.
    // Even if account_id contains empty string we won't allow it.
    if account_id.is_some() && info.sender != addresses[&MarsAddressType::CreditManager] {
        return Err(ContractError::SenderIsNotCreditManager);
    }

    let user_id_key = create_user_id_key(&info.sender, account_id.clone())?;

    // Load the user's shares
    let user_vault_shares = UserVaultShares::load(deps.as_ref(), current_time, &user_id_key)?;

    // Cannot withdraw when there is zero unlocked positions
    if user_vault_shares.unlocked.is_empty() {
        return Err(ContractError::UnlockedPositionsNotFound {});
    }

    // Clear state if no more unlocking positions
    if user_vault_shares.unlocking.is_empty() {
        UNLOCKS.remove(deps.storage, &user_id_key);
    } else {
        UNLOCKS.save(deps.storage, &user_id_key, &user_vault_shares.unlocking)?;
    }

    let mut vs = VAULT_STATE.load(deps.storage)?;

    let total_user_shares = user_vault_shares.total()?;
    let total_vault_shares_before = vs.total_shares;

    let mut msgs = vec![];

    msgs.push(build_incentives_balance_changed_msg(
        &addresses[&MarsAddressType::Incentives],
        &info.sender,
        account_id,
        &cfg.base_denom,
        total_user_shares,
        total_vault_shares_before,
    )?);

    let oracle = get_oracle_adapter(&addresses[&MarsAddressType::Oracle]);
    let params = get_params_adapter(&addresses[&MarsAddressType::Params]);

    // Convert the shares to amount
    let unlocked_user_amount = shares_to_amount(
        &deps.as_ref(),
        &vs,
        &oracle,
        &params,
        current_time,
        &cfg.base_denom,
        user_vault_shares.unlocked_amount,
        ActionKind::Default,
    )?;

    // Ensure slippage checks (if provided by user)
    if let Some(min) = min_recieve {
        if unlocked_user_amount < min {
            return Err(ContractError::MinimumReceiveExceeded {
                min,
                found: unlocked_user_amount,
                denom: cfg.base_denom,
            });
        }
    }

    // Decrement total liquidity and deposit shares
    vs.total_balance = vs.total_balance.checked_sub(unlocked_user_amount.try_into()?)?;
    vs.total_shares = vs.total_shares.checked_sub(user_vault_shares.unlocked_amount)?;
    VAULT_STATE.save(deps.storage, &vs)?;

    // Check if the vault is under-collateralized after the withdrawal
    let current_cr = query_vault_cr(deps.as_ref(), current_time, ActionKind::Default)?;
    if current_cr < cfg.target_vault_collateralization_ratio {
        return Err(ContractError::VaultUndercollateralized {
            current_cr,
            threshold_cr: cfg.target_vault_collateralization_ratio,
        });
    }

    msgs.push(CosmosMsg::from(BankMsg::Send {
        to_address: info.sender.into(),
        amount: coins(unlocked_user_amount.u128(), &cfg.base_denom),
    }));

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "withdraw")
        .add_attribute("denom", &cfg.base_denom)
        .add_attribute("unlocked_user_shares", user_vault_shares.unlocked_amount)
        .add_attribute("amount", unlocked_user_amount)
        .add_attribute("total_user_shares", total_user_shares))
}

/// Compute the counterparty vault's net asset value (NAV), denominated in the
/// base asset (i.e. USDC).
///
/// The NAV is defined as
///
/// ```
/// NAV := max(assets + totalWithdrawalBalance, 0)
/// ```
///
/// Here `totalWithdrawalBalance` is the amount of money available for withdrawal by LPs.
///
/// If a traders has an unrealized gain, it's a liability for the counterparty
/// vault, because if the user realizes the position it will be the vault to pay
/// for the profit.
///
/// Conversely, to realize a losing position the user must pay the vault, so
/// it's an asset for the vault.
pub fn compute_global_withdrawal_balance(
    deps: &Deps,
    vs: &VaultState,
    oracle: &Oracle,
    params: &Params,
    current_time: u64,
    base_denom: &str,
    action: ActionKind,
) -> ContractResult<Uint128> {
    let (global_acc_data, _) =
        compute_total_accounting_data(deps, oracle, params, current_time, base_denom, action)?;

    let global_withdrawal_balance =
        global_acc_data.withdrawal_balance.total.checked_add(vs.total_balance)?;
    let global_withdrawal_balance = max(global_withdrawal_balance, Int128::zero());

    Ok(global_withdrawal_balance.unsigned_abs())
}

/// Convert a deposit amount to shares, given the current total amount and
/// shares.
///
/// If total shares is zero, in which case a conversion rate between amount and
/// shares is undefined, we use a default conversion rate.
pub fn amount_to_shares(
    deps: &Deps,
    vs: &VaultState,
    oracle: &Oracle,
    params: &Params,
    current_time: u64,
    base_denom: &str,
    amount: Uint128,
    action: ActionKind,
) -> ContractResult<Uint128> {
    let available_liquidity = compute_global_withdrawal_balance(
        deps,
        vs,
        oracle,
        params,
        current_time,
        base_denom,
        action,
    )?;

    if vs.total_shares.is_zero() || available_liquidity.is_zero() {
        return amount.checked_mul(Uint128::new(DEFAULT_SHARES_PER_AMOUNT)).map_err(Into::into);
    }

    vs.total_shares.checked_multiply_ratio(amount, available_liquidity).map_err(Into::into)
}

/// Convert a deposit shares to amount, given the current total amount and
/// shares.
///
/// If total shares is zero, in which case a conversion rate between amount and
/// shares if undefined, we throw an error.
pub fn shares_to_amount(
    deps: &Deps,
    vs: &VaultState,
    oracle: &Oracle,
    params: &Params,
    current_time: u64,
    base_denom: &str,
    shares: Uint128,
    action: ActionKind,
) -> ContractResult<Uint128> {
    // We technical don't need to check for this explicitly, because
    // checked_multiply_raio already checks for division-by-zero. However we
    // still do this to output a more descriptive error message. This consumes a
    // bit more gas but gas fee is not yet a problem on Cosmos chains anyways.
    if vs.total_shares.is_zero() {
        return Err(ContractError::ZeroTotalShares);
    }

    // We can't continue if there is zero available liquidity in the vault
    let available_liquidity = compute_global_withdrawal_balance(
        deps,
        vs,
        oracle,
        params,
        current_time,
        base_denom,
        action,
    )?;
    if available_liquidity.is_zero() {
        return Err(ContractError::ZeroWithdrawalBalance);
    }

    available_liquidity.checked_multiply_ratio(shares, vs.total_shares).map_err(Into::into)
}

/// For internal use by the struct only.
///
/// Create an execute message to inform the incentive contract to update the user's index upon a
/// change in the user's vault collateral amount.
fn build_incentives_balance_changed_msg(
    incentives_addr: &Addr,
    user_addr: &Addr,
    account_id: Option<String>,
    collateral_denom: &str,
    user_amount: Uint128,
    total_amount: Uint128,
) -> ContractResult<CosmosMsg> {
    Ok(WasmMsg::Execute {
        contract_addr: incentives_addr.into(),
        msg: to_json_binary(&ExecuteMsg::BalanceChange {
            user_addr: user_addr.clone(),
            account_id,
            kind: IncentiveKind::PerpVault,
            denom: collateral_denom.to_string(),
            user_amount,
            total_amount,
        })?,
        funds: vec![],
    }
    .into())
}

struct UserVaultShares {
    pub locked_amount: Uint128,
    pub unlocking_amount: Uint128,
    pub unlocking: Vec<UnlockState>,
    pub unlocked_amount: Uint128,
    pub unlocked: Vec<UnlockState>,
}

impl UserVaultShares {
    pub fn load(deps: Deps, current_time: u64, user_id_key: &UserIdKey) -> ContractResult<Self> {
        let locked_amount =
            DEPOSIT_SHARES.may_load(deps.storage, user_id_key)?.unwrap_or_else(Uint128::zero);
        let unlocks = UNLOCKS.may_load(deps.storage, user_id_key)?.unwrap_or(vec![]);
        let (unlocked, unlocking): (Vec<_>, Vec<_>) =
            unlocks.into_iter().partition(|us| us.cooldown_end <= current_time);
        Ok(Self {
            locked_amount,
            unlocking_amount: unlocking.iter().map(|us| us.shares).sum::<Uint128>(),
            unlocking,
            unlocked_amount: unlocked.iter().map(|us| us.shares).sum::<Uint128>(),
            unlocked,
        })
    }

    pub fn total(&self) -> ContractResult<Uint128> {
        Ok(self
            .locked_amount
            .checked_add(self.unlocking_amount)?
            .checked_add(self.unlocked_amount)?)
    }
}
