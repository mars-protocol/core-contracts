use cosmwasm_std::{Addr, Int128, SignedDecimal, Storage, Uint128};

use crate::{
    error::ContractError,
    helpers::i128_from_u128,
    state::{LAST_NET_WORTH, USER_ENTRY_PNL_INDEX, USER_TRACKED_PNL, VAULT_PNL, VAULT_PNL_INDEX},
};

// scaling factor for pnl calculations, to prevent precision loss
const SCALING_FACTOR: u64 = 1_000_000;

/// Updates the vault's profit and loss (PNL) index and total PNL based on the current net worth and total shares
///
/// # Arguments
///
/// * `storage` - Mutable reference to the contract's storage
/// * `net_worth_now` - Current net worth of the vault
/// * `vault_shares` - Total number of vault shares in circulation
///
/// # Returns
///
/// * `SignedDecimal` - The updated PNL index value
/// * `ContractError` - If calculation fails or if total shares is zero
///
/// # Example
///
/// ```
/// let pnl_index = update_vault_pnl_index(
///     &mut deps.storage,
///     Uint128::new(1_050_000_000),
///     Uint128::new(1_000_000_000),
/// )?;
/// // pnl_index now contains the updated PNL index value
/// ```
pub fn update_vault_pnl_index(
    storage: &mut dyn Storage,
    net_worth_now: Uint128,
    vault_shares: Uint128,
) -> Result<(SignedDecimal, Int128), ContractError> {
    let (vault_pnl_index, vault_pnl_delta) =
        query_current_vault_pnl_index(storage, net_worth_now, vault_shares)?;

    VAULT_PNL_INDEX.save(storage, &vault_pnl_index)?;

    let updated_vault_pnl =
        VAULT_PNL.may_load(storage)?.unwrap_or(Int128::zero()).checked_add(vault_pnl_delta)?;
    VAULT_PNL.save(storage, &updated_vault_pnl)?;

    Ok((vault_pnl_index, updated_vault_pnl))
}

/// Calculates the current vault PNL index and PNL delta based on the current net worth and total shares
///
/// # Arguments
///
/// * `storage` - Reference to the contract's storage
/// * `net_worth_now` - Current net worth of the vault
/// * `vault_shares` - Total number of vault shares in circulation
///
/// # Returns
///
/// * `(SignedDecimal, Int128)` - Tuple containing:
///   * The current PNL index value as a SignedDecimal
///   * The raw PNL delta as an Int128 (difference between current and last net worth)
/// * `ContractError` - If calculation fails
///
/// # Details
///
/// This function calculates two key values:
/// 1. The raw PNL delta, which is the difference between current and last net worth
/// 2. The indexed PNL value, which is the PNL delta scaled by SCALING_FACTOR and divided by shares
pub fn query_current_vault_pnl_index(
    storage: &dyn Storage,
    net_worth_now: Uint128,
    vault_shares: Uint128,
) -> Result<(SignedDecimal, Int128), ContractError> {
    let vault_pnl_delta_raw = query_vault_pnl_delta(storage, net_worth_now)?;
    if vault_shares.is_zero() {
        return Ok((SignedDecimal::zero(), Int128::zero()));
    }

    if vault_pnl_delta_raw.is_zero() {
        return Ok((VAULT_PNL_INDEX.may_load(storage)?.unwrap_or_default(), Int128::zero()));
    }
    let vault_pnl_delta_indexed = SignedDecimal::from_ratio(
        vault_pnl_delta_raw.checked_mul(SCALING_FACTOR.into())?,
        i128_from_u128(vault_shares)?,
    );
    // update vault index
    let new_vault_pnl_index = VAULT_PNL_INDEX
        .may_load(storage)?
        .unwrap_or_default()
        .checked_add(vault_pnl_delta_indexed)?;
    Ok((new_vault_pnl_index, vault_pnl_delta_raw))
}

/// Calculates the difference between the current net worth and the previously recorded net worth
///
/// # Arguments
///
/// * `storage` - Reference to the contract's storage
/// * `net_worth_now` - Current net worth of the vault
///
/// # Returns
///
/// * `Int128` - The difference between current and last net worth (can be positive or negative)
/// * `ContractError` - If calculation fails or if conversion between types fails
///
/// # Details
///
/// This function handles the base PNL delta calculation before any scaling or indexing:
/// - Returns zero for first-time calculations (when no previous net worth exists)
/// - Otherwise returns the signed difference between current and previous net worth
fn query_vault_pnl_delta(
    storage: &dyn Storage,
    net_worth_now: Uint128,
) -> Result<Int128, ContractError> {
    // get vault pnl delta
    let last_net_worth = LAST_NET_WORTH.may_load(storage)?;

    if last_net_worth.is_none() {
        // first time updating pnl
        return Ok(Int128::zero());
    }

    let last_net_worth: Uint128 = last_net_worth.unwrap();

    let vault_pnl_delta =
        i128_from_u128(net_worth_now)?.checked_sub(i128_from_u128(last_net_worth)?)?;

    Ok(vault_pnl_delta)
}

/// Calculates the total cumulative PNL of the vault
///
/// # Arguments
///
/// * `storage` - Reference to the contract's storage
/// * `net_worth_now` - Current net worth of the vault
///
/// # Returns
///
/// * `Int128` - The total accumulated PNL of the vault
/// * `ContractError` - If calculation fails
///
/// # Details
///
/// This function:
/// 1. Gets the current PNL delta based on the provided net worth vs. stored net worth
/// 2. Adds this delta to the accumulated PNL value stored in the vault
/// 3. Returns the updated total PNL (without modifying storage)
pub fn query_vault_pnl(
    storage: &dyn Storage,
    net_worth_now: Uint128,
) -> Result<Int128, ContractError> {
    let vault_pnl_delta = query_vault_pnl_delta(storage, net_worth_now)?;
    let updated_vault_pnl =
        VAULT_PNL.may_load(storage)?.unwrap_or(Int128::zero()).checked_add(vault_pnl_delta)?;

    Ok(updated_vault_pnl)
}

/// Calculates a user's accumulated profit or loss based on their vault shares
///
/// # Arguments
///
/// * `storage` - Reference to the contract's storage
/// * `user` - Address of the user whose PNL is being calculated
/// * `user_shares` - Number of vault shares the user currently holds
/// * `vault_pnl_index` - The current vault PNL index value
///
/// # Returns
///
/// * `Int128` - The user's total accumulated PNL
/// * `ContractError` - If calculation fails
///
/// # Details
///
/// This function calculates a user's PNL by:
/// 1. Finding the difference between the current vault PNL index and the user's entry index
/// 2. Multiplying this difference by the user's shares and dividing by the scaling factor
/// 3. Adding this to the user's previously tracked PNL
///
/// The calculation follows the formula:
/// `user_pnl = previously_tracked_pnl + (user_shares * (current_index - entry_index) / scaling_factor)`
pub fn query_user_pnl(
    storage: &dyn Storage,
    user: &Addr,
    user_shares: Uint128,
    vault_pnl_index: SignedDecimal,
) -> Result<Int128, ContractError> {
    // a users pnl since their last update is calculated by the following formula:
    // pnl = shares * ((current_pnl_index - user_entry_pnl_index) / scaling_factor)
    let user_entry_pnl_index =
        USER_ENTRY_PNL_INDEX.may_load(storage, user)?.unwrap_or(vault_pnl_index);

    // current pnl index - user entry pnl index
    let user_pnl_index_diff: SignedDecimal = vault_pnl_index.checked_sub(user_entry_pnl_index)?;

    // first convert user_shares to int128 to handle potential conversion errors
    let user_shares_i128 = i128_from_u128(user_shares)?;

    // shares * ((current_pnl_index - user_entry_pnl_index) / scaling_factor)
    let untracked_user_pnl_delta: SignedDecimal = user_pnl_index_diff
        .checked_mul(SignedDecimal::checked_from_ratio(user_shares_i128, SCALING_FACTOR)?)?;

    // add our untracked pnl delta to the user's pnl
    let mut user_pnl = USER_TRACKED_PNL.may_load(storage, user)?.unwrap_or(Int128::zero());
    user_pnl = user_pnl.checked_add(untracked_user_pnl_delta.to_int_floor())?;

    Ok(user_pnl)
}

/// Updates a user's accumulated PNL and entry index in storage
///
/// # Arguments
///
/// * `storage` - Mutable reference to the contract's storage
/// * `user` - Address of the user whose PNL is being updated
/// * `user_shares` - Number of vault shares the user currently holds
/// * `vault_pnl_index` - The current vault PNL index value
///
/// # Returns
///
/// * `()` - Success indicator
/// * `ContractError` - If calculation or storage operations fail
///
/// # Details
///
/// This function:
/// 1. Calculates the user's updated PNL based on their current shares and the vault index
/// 2. Saves this updated PNL to storage
/// 3. Updates the user's entry PNL index to the current value for future calculations
///
/// This function should be called whenever a user interacts with the vault in ways that
/// might affect their PNL tracking (e.g., deposits, withdrawals, or at regular intervals).
pub fn update_user_pnl(
    storage: &mut dyn Storage,
    user: &Addr,
    user_shares: Uint128,
    vault_pnl_index: SignedDecimal,
) -> Result<Int128, ContractError> {
    let user_pnl = query_user_pnl(storage, user, user_shares, vault_pnl_index)?;

    USER_TRACKED_PNL.save(storage, user, &user_pnl)?;

    // set the users entry pnl index to the current pnl index
    USER_ENTRY_PNL_INDEX.save(storage, user, &vault_pnl_index)?;

    Ok(user_pnl)
}
