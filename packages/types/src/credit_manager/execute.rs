use std::collections::BTreeMap;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_json_binary, Addr, Coin, CosmosMsg, Decimal, Int128, StdResult, Uint128, WasmMsg,
};
use mars_owner::OwnerUpdate;

use super::ConfigUpdates;
use crate::{
    account_nft::NftConfigUpdates,
    adapters::vault::{Vault, VaultPositionType, VaultUnchecked},
    health::{AccountKind, HealthState, HealthValuesResponse},
    perps::PnL,
    swapper::SwapperRoute,
};

#[allow(clippy::large_enum_variant)]
#[cw_serde]
pub enum ExecuteMsg {
    //--------------------------------------------------------------------------------------------------
    // Public messages
    //--------------------------------------------------------------------------------------------------
    /// Mints NFT representing a credit account for user. User can have many.
    CreateCreditAccount(AccountKind),
    /// Update user's position on their credit account
    UpdateCreditAccount {
        account_id: Option<String>,
        account_kind: Option<AccountKind>,
        actions: Vec<Action>,
    },
    /// Repay debt on behalf of an account, funded from wallet. Must send exactly one coin in message funds.
    /// Allows repaying debts of assets that have been de-listed from credit manager.
    RepayFromWallet {
        account_id: String,
    },

    ExecuteTriggerOrder {
        account_id: String,
        trigger_order_id: String,
    },

    //--------------------------------------------------------------------------------------------------
    // Privileged messages
    //--------------------------------------------------------------------------------------------------
    /// Update contract config constants
    UpdateConfig {
        updates: ConfigUpdates,
    },
    /// Manages owner role state
    UpdateOwner(OwnerUpdate),
    /// Update nft contract config
    UpdateNftConfig {
        config: Option<NftConfigUpdates>,
        ownership: Option<cw721_base::Action>,
    },
    /// Internal actions only callable by the contract itself
    Callback(CallbackMsg),

    /// This is part of the deleveraging process initiated by the perps contract.
    ///
    /// Updates the account balance based on the specified PnL for the given account.
    /// Depending on the PnL type:
    /// - Profit: increases the account balance.
    /// - Loss: decreases the account balance and borrows from the red-bank if necessary.
    /// - Break-even: no action is taken.
    /// If the total PnL results in a loss, the corresponding amount of coins must be sent to the perps contract.
    UpdateBalanceAfterDeleverage {
        account_id: String,
        pnl: PnL,
    },
}

#[cw_serde]
pub enum ActionAmount {
    Exact(Uint128),
    AccountBalance,
}

impl ActionAmount {
    pub fn value(&self) -> Option<Uint128> {
        match self {
            ActionAmount::Exact(amt) => Some(*amt),
            ActionAmount::AccountBalance => None,
        }
    }
}

#[cw_serde]
pub struct ActionCoin {
    pub denom: String,
    pub amount: ActionAmount,
}

impl From<&Coin> for ActionCoin {
    fn from(value: &Coin) -> Self {
        Self {
            denom: value.denom.to_string(),
            amount: ActionAmount::Exact(value.amount),
        }
    }
}

#[cw_serde]
pub enum ChangeExpected {
    Increase,
    Decrease,
}

#[cw_serde]
pub enum LiquidateRequest<T> {
    /// Pay back debt of a liquidatable rover account for a bonus. Requires specifying 1) the debt
    /// denom/amount of what the liquidator wants to payoff and 2) the request coin denom which the
    /// liquidatee should have a balance of. The amount returned to liquidator will be the request coin
    /// of the amount that precisely matches the value of the debt + a liquidation bonus.
    /// The debt amount will be adjusted down if:
    /// - Exceeds liquidatee's total debt for denom
    /// - Not enough liquidatee request coin balance to match
    /// - The value of the debt repaid exceeds the maximum close factor %
    ///
    /// Liquidation should prioritize first the not lent coin and if more needs to be serviced to the liquidator
    /// it should reclaim (withdrawn from Red Bank).
    Deposit(String),
    /// Pay back debt of a liquidatable rover account for a via liquidating a Lent position.
    /// Lent shares are transfered from the liquidatable to the liquidator.
    Lend(String),
    /// Pay back debt of a liquidatable rover account for a via liquidating a vault position.
    /// Similar to `Deposit` msg and will make similar adjustments to the request.
    /// The vault position will be withdrawn (and force withdrawn if a locked vault position) and
    /// the underlying assets will transferred to the liquidator.
    /// The `VaultPositionType` will determine which bucket to liquidate from.
    Vault {
        request_vault: T,
        position_type: VaultPositionType,
    },
    /// Pay back debt of a liquidatable credit manager account for a via liquidating an Astro LP position.
    /// LP shares are transfered from the liquidatable to the liquidator.
    StakedAstroLp(String),
}

#[cw_serde]
pub enum Comparison {
    GreaterThan,
    LessThan,
}

impl Comparison {
    pub fn is_met(&self, lhs: Decimal, rhs: Decimal) -> bool {
        match self {
            Comparison::GreaterThan => lhs > rhs,
            Comparison::LessThan => lhs < rhs,
        }
    }
}

#[cw_serde]
pub enum Condition {
    /// If the oracle price is above or below the specified threshold, depending
    /// on the comparison, the condition is met.
    OraclePrice {
        denom: String,
        price: Decimal,
        comparison: Comparison,
    },
    /// Trigger based on a relative price or price ratio of two prices.
    /// The given price is compared to the Base / Quote ratio.
    RelativePrice {
        base_price_denom: String,
        quote_price_denom: String,
        price: Decimal,
        comparison: Comparison,
    },
    /// If the health factor of the account is above or below the specified
    /// threshold, depending on the comparison, the condition is met.
    HealthFactor {
        threshold: Decimal,
        comparison: Comparison,
    },
    /// If the other trigger_order is successfully executed, the condition is met.
    TriggerOrderExecuted {
        // When empty string is provided, a base order should be provided in the same tx.
        // The `trigger_order_id` will then be set to that base order.
        trigger_order_id: String,
    },
}

/// The list of actions that users can perform on their positions
#[cw_serde]
pub enum Action {
    /// Deposit coin of specified denom and amount. Verifies if the correct amount is sent with transaction.
    Deposit(Coin),
    /// Withdraw coin of specified denom and amount
    Withdraw(ActionCoin),
    /// Withdraw coin of specified denom and amount to a wallet address
    WithdrawToWallet {
        coin: ActionCoin,
        recipient: String,
    },
    /// Borrow coin of specified amount from Red Bank
    Borrow(Coin),
    /// Lend coin to the Red Bank
    Lend(ActionCoin),
    /// Reclaim the coins that were lent to the Red Bank.
    Reclaim(ActionCoin),
    /// For assets lent to the Red Bank, some can accumulate incentive rewards.
    /// This message claims all of them adds them to account balance.
    ClaimRewards {},
    /// Repay coin of specified amount back to Red Bank. If `amount: AccountBalance` is passed,
    /// the repaid amount will be the minimum between account balance for denom and total owed.
    /// The sender will repay on behalf of the recipient account. If 'recipient_account_id: None',
    /// the sender repays to its own account.
    Repay {
        recipient_account_id: Option<String>,
        coin: ActionCoin,
    },
    /// Provide liquidity of the base token to the perp vault
    DepositToPerpVault {
        coin: ActionCoin,
        max_receivable_shares: Option<Uint128>,
    },

    /// Unlock liquidity from the perp vault. The unlocked tokens will have to wait
    /// a cooldown period before they can be withdrawn.
    UnlockFromPerpVault {
        shares: Uint128,
    },
    /// Withdraw liquidity from the perp vault
    WithdrawFromPerpVault {
        min_receive: Option<Uint128>,
    },
    /// Execute a state update against the specified perp market for the given account.
    /// If no position exists in the given market, a position is created. Existing postions are modified.
    /// Note that size is signed
    ///     - to increase short or reduce long, use a negative value
    ///     - to reduce short or increase long, use a positive value
    ExecutePerpOrder {
        denom: String,
        order_size: Int128,
        reduce_only: Option<bool>,
        order_type: Option<ExecutePerpOrderType>,
    },

    /// Executes a perp order against the given market for the current position size to close the
    /// position.
    ClosePerpPosition {
        denom: String,
    },

    /// Dispatch orders to be triggered under specified conditions.
    CreateTriggerOrder {
        actions: Vec<Action>,
        conditions: Vec<Condition>,
        keeper_fee: Coin,
        order_type: Option<CreateTriggerOrderType>,
    },

    DeleteTriggerOrder {
        trigger_order_id: String,
    },

    /// Deposit coins into vault strategy.
    /// If `coin.amount: AccountBalance`, Rover attempts to deposit the account's entire balance into the vault.
    EnterVault {
        vault: VaultUnchecked,
        coin: ActionCoin,
    },
    /// Withdraw underlying coins from vault
    ExitVault {
        vault: VaultUnchecked,
        amount: Uint128,
    },
    /// Requests unlocking of shares for a vault with a required lock period
    RequestVaultUnlock {
        vault: VaultUnchecked,
        amount: Uint128,
    },
    /// Withdraws the assets for unlocking position id from vault. Required time must have elapsed.
    ExitVaultUnlocked {
        id: u64,
        vault: VaultUnchecked,
    },
    /// Pay back debt of a liquidatable rover account for a via liquidating a specific type of the position.
    Liquidate {
        /// The credit account id of the one with a liquidation threshold health factor 1 or below
        liquidatee_account_id: String,
        /// The coin they wish to acquire from the liquidatee (amount returned will include the bonus)
        debt_coin: Coin,
        /// Position details to be liquidated
        request: LiquidateRequest<VaultUnchecked>,
    },
    /// Perform a swapper with an exact-in amount. Requires slippage allowance %.
    /// If `coin_in.amount: AccountBalance`, the accounts entire balance of `coin_in.denom` will be used.
    SwapExactIn {
        coin_in: ActionCoin,
        denom_out: String,
        min_receive: Uint128,
        route: Option<SwapperRoute>,
    },
    /// Add Vec<Coin> to liquidity pool in exchange for LP tokens.
    /// Slippage allowance (%) is used to calculate the minimum amount of LP tokens to receive.
    ProvideLiquidity {
        coins_in: Vec<ActionCoin>,
        lp_token_out: String,
        slippage: Decimal,
    },
    /// Send LP token and withdraw corresponding reserve assets from pool.
    /// If `lp_token.amount: AccountBalance`, the account balance of `lp_token.denom` will be used.
    /// /// Slippage allowance (%) is used to calculate the minimum amount of reserve assets to receive.
    WithdrawLiquidity {
        lp_token: ActionCoin,
        slippage: Decimal,
    },
    /// Stake lp token in astroport incentives contract via mars incentives
    StakeAstroLp {
        lp_token: ActionCoin,
    },
    /// Unstake lp token from astroport incentives contract via mars incentives
    UnstakeAstroLp {
        lp_token: ActionCoin,
    },
    /// Claim accrued LP incentive rewards from astroport incentives contract via mars incentives
    ClaimAstroLpRewards {
        lp_denom: String,
    },
    /// Refunds all coin balances back to user wallet
    RefundAllCoinBalances {},
}

impl Action {
    pub fn is_allowed_for_usdc_margin(&self) -> bool {
        match self {
            // Allowed actions
            Action::Deposit(..) => true,
            Action::Withdraw(..) => true,
            Action::DepositToPerpVault {
                ..
            } => true,
            Action::WithdrawToWallet {
                ..
            } => true,
            Action::UnlockFromPerpVault {
                ..
            } => true,
            Action::WithdrawFromPerpVault {
                ..
            } => true,
            Action::CreateTriggerOrder {
                ..
            } => true,
            Action::DeleteTriggerOrder {
                ..
            } => true,
            Action::ExecutePerpOrder {
                ..
            } => true,
            Action::ClosePerpPosition {
                ..
            } => true,
            Action::RefundAllCoinBalances {} => true,

            // Forbidden actions
            Action::Borrow(..) => false,
            Action::Lend(..) => false,
            Action::Reclaim(..) => false,
            Action::ClaimRewards {} => false,
            Action::EnterVault {
                ..
            } => false,
            Action::ExitVault {
                ..
            } => false,
            Action::RequestVaultUnlock {
                ..
            } => false,
            Action::ExitVaultUnlocked {
                ..
            } => false,
            Action::ProvideLiquidity {
                ..
            } => false,
            Action::WithdrawLiquidity {
                ..
            } => false,
            Action::StakeAstroLp {
                ..
            } => false,
            Action::UnstakeAstroLp {
                ..
            } => false,
            Action::ClaimAstroLpRewards {
                ..
            } => false,
            Action::Liquidate {
                ..
            } => false,
            Action::SwapExactIn {
                ..
            } => false,
            Action::Repay {
                ..
            } => false,
        }
    }
}

/// Internal actions made by the contract with pre-validated inputs
#[cw_serde]
pub enum CallbackMsg {
    /// Withdraw specified amount of coin from credit account;
    /// Decrement the token's asset amount;
    Withdraw {
        account_id: String,
        coin: ActionCoin,
        recipient: Addr,
    },
    /// Borrow specified amount of coin from Red Bank;
    /// Increase the token's coin amount and debt shares;
    Borrow {
        account_id: String,
        coin: Coin,
    },
    /// Repay coin of specified amount back to Red Bank;
    /// Decrement the token's coin amount and debt shares;
    /// If `coin.amount: AccountBalance` is passed, the repaid amount will be the minimum
    /// between account balance for denom and total owed;
    Repay {
        account_id: String,
        coin: ActionCoin,
    },
    /// Benefactor account repays debt on behalf of recipient
    RepayForRecipient {
        benefactor_account_id: String,
        recipient_account_id: String,
        coin: ActionCoin,
    },
    /// Lend coin to the Red Bank
    Lend {
        account_id: String,
        coin: ActionCoin,
    },
    /// Reclaim lent coin from the Red Bank;
    /// Decrement the token's lent shares and increment the coin amount;
    Reclaim {
        account_id: String,
        coin: ActionCoin,
    },
    /// Calls incentive contract to claim all rewards and increment account balance
    ClaimRewards {
        account_id: String,
    },
    /// Assert MaxLTV is either:
    /// - Healthy, if prior to actions MaxLTV health factor >= 1 or None
    /// - Not further weakened, if prior to actions MaxLTV health factor < 1
    /// Emits a `position_changed` event.
    #[serde(rename = "assert_max_ltv")]
    AssertMaxLTV {
        account_id: String,
        prev_health_state: HealthState,
    },
    /// Assert that the total deposit amounts of the given denoms across Red
    /// Bank and Rover do not exceed their respective deposit caps.
    AssertDepositCaps {
        denoms: BTreeMap<String, Option<Uint128>>,
    },
    /// Corresponding to the DepositToPerpVault action
    DepositToPerpVault {
        account_id: String,
        coin: ActionCoin,
        max_receivable_shares: Option<Uint128>,
    },
    /// Corresponding to the UnlockFromPerpVault action
    UnlockFromPerpVault {
        account_id: String,
        shares: Uint128,
    },
    /// Corresponding to the WithdrawFromPerpVault action
    WithdrawFromPerpVault {
        account_id: String,
        min_receive: Option<Uint128>,
    },
    // Creates a trigger order for an account
    CreateTriggerOrder {
        account_id: String,
        actions: Vec<Action>,
        conditions: Vec<Condition>,
        keeper_fee: Coin,
    },
    // Deletes an accounts trigger order
    DeleteTriggerOrder {
        account_id: String,
        trigger_order_id: String,
    },
    /// Adds coin to a vault strategy
    EnterVault {
        account_id: String,
        vault: Vault,
        coin: ActionCoin,
    },
    /// Exchanges vault LP shares for assets
    ExitVault {
        account_id: String,
        vault: Vault,
        amount: Uint128,
    },
    /// Used to update the account balance of vault coins after a vault action has taken place
    UpdateVaultCoinBalance {
        vault: Vault,
        /// Account that needs vault coin balance adjustment
        account_id: String,
        /// Total vault coin balance in Rover
        previous_total_balance: Uint128,
    },
    /// Executes a perp order against the given market. If no position exists, a position is opened.
    ExecutePerpOrder {
        account_id: String,
        denom: String,
        size: Int128,
        reduce_only: Option<bool>,
    },
    /// Executes a perp order against the given market for the current position size.
    ClosePerpPosition {
        account_id: String,
        denom: String,
    },
    /// Requests unlocking of shares for a vault with a lock period
    RequestVaultUnlock {
        account_id: String,
        vault: Vault,
        amount: Uint128,
    },
    /// Withdraws assets from vault for a locked position having a lockup period that has been fulfilled
    ExitVaultUnlocked {
        account_id: String,
        vault: Vault,
        position_id: u64,
    },
    /// Close all perp positions before liquidation
    CloseAllPerps {
        account_id: String,
    },
    /// Pay back debts of a liquidatable rover account for a bonus
    Liquidate {
        liquidator_account_id: String,
        liquidatee_account_id: String,
        debt_coin: Coin,
        request: LiquidateRequest<Vault>,
        prev_health: HealthValuesResponse,
    },
    /// Perform a swapper with an exact-in amount. Requires slippage allowance %.
    /// If `coin_in.amount: AccountBalance`, the accounts entire balance of `coin_in.denom` will be used.
    SwapExactIn {
        account_id: String,
        coin_in: ActionCoin,
        denom_out: String,
        min_receive: Uint128,
        route: Option<SwapperRoute>,
    },
    /// Used to update the coin balance of account after an async action
    UpdateCoinBalance {
        /// Account that needs coin balance adjustment
        account_id: String,
        /// Total balance for coin in Rover prior to withdraw
        previous_balance: Coin,
        /// The kind of change that is anticipated to balance of coin.
        /// If does not match expectation, an error is raised.
        change: ChangeExpected,
    },
    /// Used to update the coin balance of account after an async action
    UpdateCoinBalanceAfterVaultLiquidation {
        /// Account that needs coin balance adjustment
        account_id: String,
        /// Total balance for coin in Rover prior to withdraw
        previous_balance: Coin,
        /// Protocol fee percentage transfered to rewards-collector account
        protocol_fee: Decimal,
    },
    /// Add Vec<Coin> to liquidity pool in exchange for LP tokens.
    ProvideLiquidity {
        account_id: String,
        coins_in: Vec<ActionCoin>,
        lp_token_out: String,
        slippage: Decimal,
    },
    /// Stake lp token in astroport incentives contract via mars incentives
    StakeAstroLp {
        // Account id staking the LP
        account_id: String,
        // Amount / denom to stake
        lp_token: ActionCoin,
    },
    /// Unstake lp token from astroport incentives contract via mars incentives.
    UnstakeAstroLp {
        // account id  unstaking the LP
        account_id: String,
        // lp coin to unstake
        lp_token: ActionCoin,
    },
    /// Claim all accrued rewards for LP position in astroport incentives
    ClaimAstroLpRewards {
        account_id: String,
        lp_denom: String,
    },
    /// Send LP token and withdraw corresponding reserve assets from pool.
    /// If `lp_token.amount: AccountBalance`, the account balance of `lp_token.denom` will be used.
    WithdrawLiquidity {
        account_id: String,
        lp_token: ActionCoin,
        slippage: Decimal,
    },
    /// Refunds all coin balances back to user wallet
    RefundAllCoinBalances {
        account_id: String,
    },
    /// Ensures that HLS accounts abide by specific rules
    AssertHlsRules {
        account_id: String,
    },
    /// At the end of the execution of dispatched actions, this callback removes the guard
    /// and allows subsequent dispatches.
    RemoveReentrancyGuard {},
}

impl CallbackMsg {
    pub fn into_cosmos_msg(&self, contract_addr: &Addr) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: contract_addr.to_string(),
            msg: to_json_binary(&ExecuteMsg::Callback(self.clone()))?,
            funds: vec![],
        }))
    }
}

#[cw_serde]
pub enum CreateTriggerOrderType {
    /// Marks the order to have no relation to another trigger order.
    Default,
    /// Marks the order to have exactly 1 parent trigger order. This order can only be executed
    /// when the parent has been executed prior.
    Parent,
    /// Marks the order as a parent order. This means 1+ trigger orders depend on this order to
    /// be executed before they are considered executable.
    Child,
}

#[cw_serde]
pub enum ExecutePerpOrderType {
    /// Marks the perp order as default, without any trigger orders depending on it.
    Default,
    /// Marks the perp order as parent. This means that 1+ trigger orders depend on this perp order
    /// to be executed before they are considered executable. A perp order can not be a child, as
    /// a perp order is executed directly, since it is NOT a trigger order.
    Parent,
}
