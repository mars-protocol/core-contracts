use cosmwasm_std::{
    CheckedFromRatioError, CheckedMultiplyFractionError, CheckedMultiplyRatioError, Coin, Coins,
    CoinsError, Decimal, DecimalRangeExceeded, OverflowError, StdError, Uint128,
};
use cw2::VersionError;
use cw_utils::PaymentError;
use mars_liquidation::error::LiquidationError;
use mars_owner::OwnerError;
use mars_types::{
    adapters::{oracle::OracleError, vault::VaultError},
    health::HealthError,
};
use mars_utils::error::GuardError;
use thiserror::Error;

pub type ContractResult<T> = Result<T, ContractError>;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("Actions resulted in exceeding maximum allowed loan-to-value. Max LTV health factor: {max_ltv_health_factor:?}")]
    AboveMaxLTV {
        account_id: String,
        max_ltv_health_factor: String,
    },

    #[error("Asset deposit would result in exceeding limit. With deposit: {new_value:?}, maximum: {maximum}")]
    AboveAssetDepositCap {
        new_value: Coin,
        maximum: Uint128,
    },

    #[error("Vault deposit would result in exceeding limit. With deposit: {new_value:?}, Maximum: {maximum:?}")]
    AboveVaultDepositCap {
        new_value: String,
        maximum: String,
    },

    #[error("Illegal deposit of {denom:?}. Expected: {expected_denom:?}")]
    IllegalDepositDenom {
        denom: String,
        expected_denom: String,
    },

    #[error("{0}")]
    Owner(#[from] OwnerError),

    #[error(
        "{denom:?} balance change was unexpected. Prev: {prev_amount:?}, Curr: {curr_amount:?}."
    )]
    BalanceChange {
        denom: String,
        prev_amount: Uint128,
        curr_amount: Uint128,
    },

    #[error("{0} is not an available coin to request")]
    CoinNotAvailable(String),

    #[error("{0}")]
    CheckedFromRatioError(#[from] CheckedFromRatioError),

    #[error("{0}")]
    CheckedMultiply(#[from] CheckedMultiplyRatioError),

    #[error("{0}")]
    CheckedMultiplyFraction(#[from] CheckedMultiplyFractionError),

    #[error("{0}")]
    DecimalRangeExceeded(#[from] DecimalRangeExceeded),

    #[error("New unlocking positions: {new_amount:?}. Maximum: {maximum:?}.")]
    ExceedsMaxUnlockingPositions {
        new_amount: Uint128,
        maximum: Uint128,
    },

    #[error("Callbacks cannot be invoked externally")]
    ExternalInvocation,

    #[error("Extra funds received: {0}")]
    ExtraFundsReceived(Coins),

    #[error("Sent fund mismatch. Expected: {expected:?}, received {received:?}")]
    FundsMismatch {
        expected: Uint128,
        received: Uint128,
    },

    #[error(
        "Actions did not result in improved health factor: before: {prev_hf:?}, after: {new_hf:?}"
    )]
    HealthNotImproved {
        prev_hf: String,
        new_hf: String,
    },

    #[error(
        "Actions result in a lower liquidation health factor: before: {prev_hf:?}, after: {new_hf:?}"
    )]
    UnhealthyLiquidationHfDecrease {
        prev_hf: String,
        new_hf: String,
    },

    #[error("{reason:?}")]
    HLS {
        reason: String,
    },

    #[error("Insufficient funds. Requested {requested:?}, available {available:?}")]
    InsufficientFunds {
        requested: Uint128,
        available: Uint128,
    },

    #[error("{reason:?}")]
    InvalidConfig {
        reason: String,
    },

    #[error("Paying down {debt_coin:?} for {request_coin:?} does not result in a profit for the liquidator")]
    LiquidationNotProfitable {
        debt_coin: Coin,
        request_coin: Coin,
    },

    #[error("Maximum number of trigger orders reached. Unable to create more than {max_trigger_orders:?} trigger orders.")]
    MaxTriggerOrdersReached {
        max_trigger_orders: u8,
    },

    #[error("No coin amount set for action")]
    NoAmount,

    #[error("No debt to repay")]
    NoDebt,

    #[error("Nothing lent to reclaim")]
    NoneLent,

    #[error("No Astro LP available")]
    NoAstroLp,

    #[error(
        "{account_id:?} is not a liquidatable credit account. Health factor: {lqdt_health_factor:?}."
    )]
    NotLiquidatable {
        account_id: String,
        lqdt_health_factor: String,
    },

    #[error("{user:?} is not the owner of {account_id:?}")]
    NotTokenOwner {
        user: String,
        account_id: String,
    },

    #[error("{0} is not whitelisted")]
    NotWhitelisted(String),

    #[error("Expected vault coins in exchange for deposit, but none were sent")]
    NoVaultCoinsReceived,

    #[error("No more than one vault positions is allowed")]
    OnlyOneVaultPositionAllowed,

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("{0}")]
    Payment(#[from] PaymentError),

    #[error("{0}")]
    ReentrancyGuard(String),

    #[error("Reply id: {0} not valid")]
    ReplyIdError(u64),

    #[error("{0}")]
    RequirementsNotMet(String),

    #[error("Cannot request liquidation on own credit account")]
    SelfLiquidation,

    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{user:?} is not authorized to {action:?}")]
    Unauthorized {
        user: String,
        action: String,
    },

    #[error("{user:?} is not allowed to {action:?}")]
    IllegalAction {
        user: String,
        action: String,
    },

    #[error("There is more time left on the lock period")]
    UnlockNotReady,

    #[error("{0}")]
    Version(#[from] VersionError),

    #[error("Withdraw for {denom:?} not enabled")]
    WithdrawNotEnabled {
        denom: String,
    },

    #[error("{0}")]
    Liquidation(#[from] LiquidationError),

    #[error("Slippage {slippage:?} exceeded max slippage {max_slippage:?}")]
    SlippageExceeded {
        slippage: Decimal,
        max_slippage: Decimal,
    },

    #[error(transparent)]
    Coins(#[from] CoinsError),

    #[error(transparent)]
    Guard(#[from] GuardError),

    #[error(transparent)]
    Vault(#[from] VaultError),

    #[error(transparent)]
    Oracle(#[from] OracleError),

    #[error("Order conditions not met")]
    IllegalExecuteTriggerOrder,

    #[error("Debt cannot be represented by zero debt shares")]
    ZeroDebtShares,

    #[error("{0} asset params not found")]
    AssetParamsNotFound(String),

    #[error("{0}")]
    Health(#[from] HealthError),

    #[error("Trigger order with id {order_id:?} for account id {account_id:?} not found")]
    TriggerOrderNotFound {
        order_id: String,
        account_id: String,
    },

    #[error("Received keeper fee is less than the min required fee. Expected: {expected_min_amount:?}, received: {received_amount:?}")]
    KeeperFeeTooSmall {
        expected_min_amount: Uint128,
        received_amount: Uint128,
    },

    #[error("Received invalid keeper fee denom. Expected: {expected_denom:?}, received: {received_denom:?}")]
    InvalidKeeperFeeDenom {
        expected_denom: String,
        received_denom: String,
    },

    #[error(
        "Illegal trigger action. Trigger actions may only contain execute_perp_order and lend"
    )]
    IllegalTriggerAction,

    #[error("Trigger conditions may only have one OrderExecuted")]
    MultipleOrderExecutedConditions,

    #[error("Unable to find a valid parent order")]
    NoValidParentOrderFound,

    #[error("No child orders found for parent order")]
    NoChildOrdersFound,

    #[error("Invalid order conditions. Reason: {reason}")]
    InvalidOrderConditions {
        reason: String,
    },

    #[error("Cannot have a default and parent/child CreateTriggerOrder in the same transaction")]
    InvalidCreateTriggerOrderType,

    #[error("Parent order has to be the first order in the transaction")]
    InvalidParentOrderPosition,

    #[error("Perp position not found for denom {denom:?}")]
    NoPerpPosition {
        denom: String,
    },
    #[error("Invalid vault code id. Allowed code ids configured in params")]
    InvalidVaultCodeId {},

    #[error("Vault {vault:?} is blacklisted. No actions can be performed on this vault")]
    BlacklistedVault {
        vault: String,
    },
    #[error("Vault has an admin; vaults cannot be managed with an admin set.")]
    VaultHasAdmin {},
}
