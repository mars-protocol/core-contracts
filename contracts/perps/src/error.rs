use cosmwasm_std::{
    CheckedFromRatioError, CheckedMultiplyFractionError, CheckedMultiplyRatioError, Decimal,
    DecimalRangeExceeded, DivideByZeroError, OverflowError, StdError, Uint128,
};
use cw_utils::PaymentError;
use mars_owner::OwnerError;
use mars_types::error::MarsError;

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Overflow(#[from] OverflowError),

    #[error(transparent)]
    CheckedFromRatio(#[from] CheckedFromRatioError),

    #[error(transparent)]
    CheckedMultiplyRatio(#[from] CheckedMultiplyRatioError),

    #[error(transparent)]
    CheckedMultiplyFraction(#[from] CheckedMultiplyFractionError),

    #[error(transparent)]
    DecimalRangeExceeded(#[from] DecimalRangeExceeded),

    #[error(transparent)]
    DivideByZeroError(#[from] DivideByZeroError),

    #[error(transparent)]
    Owner(#[from] OwnerError),

    #[error(transparent)]
    Payment(#[from] PaymentError),

    #[error(transparent)]
    Mars(#[from] MarsError),

    #[error("Cannot deleverage - deleveraging is disabled")]
    DeleverageDisabled,

    #[error("denom `{denom}` is already enabled")]
    DenomEnabled {
        denom: String,
    },

    #[error("denom `{denom}` exists but is not enabled")]
    DenomNotEnabled {
        denom: String,
    },

    #[error(
        "Position can not be modified if denom `{denom}` is disabled. Only closing is allowed."
    )]
    PositionCannotBeModifiedIfDenomDisabled {
        denom: String,
    },

    #[error("denom `{denom}` is not found")]
    DenomNotFound {
        denom: String,
    },

    #[error("denom `{denom}` already exists")]
    DenomAlreadyExists {
        denom: String,
    },

    #[error("account `{account_id}` already has a position in denom `{denom}`")]
    PositionExists {
        account_id: String,
        denom: String,
    },

    #[error("position opening size is too small: min {min} uusd, found {found} uusd")]
    PositionTooSmall {
        min: Uint128,
        found: Uint128,
    },

    #[error("position opening size is too big: max {max} uusd, found {found} uusd")]
    PositionTooBig {
        max: Uint128,
        found: Uint128,
    },

    #[error("only the credit manager can modify perp positions")]
    SenderIsNotCreditManager,

    #[error("withdrawing from the counterparty vault is currently disabled")]
    VaultWithdrawDisabled,

    #[error("cannot compute deposit amount when there is zero total shares")]
    ZeroTotalShares,

    #[error("cannot unlock when there is zero shares")]
    ZeroShares,

    #[error("cannot unlock with zero withdrawal balance")]
    ZeroWithdrawalBalance,

    #[error("Invalid param: {reason}")]
    InvalidParam {
        reason: String,
    },

    #[error("Unlocked positions not found")]
    UnlockedPositionsNotFound {},

    #[error("Net OI reached: max {max}, found {found}")]
    NetOpenInterestReached {
        max: Uint128,
        found: Uint128,
    },

    #[error("Long OI reached: max {max}, found {found}")]
    LongOpenInterestReached {
        max: Uint128,
        found: Uint128,
    },

    #[error("Short OI reached: max {max}, found {found}")]
    ShortOpenInterestReached {
        max: Uint128,
        found: Uint128,
    },

    #[error("Invalid payment: required {required} {denom}, received {received} {denom}")]
    InvalidPayment {
        denom: String,
        required: Uint128,
        received: Uint128,
    },

    #[error("Illegal position modification: {reason}")]
    IllegalPositionModification {
        reason: String,
    },

    #[error(
        "Account `{account_id}` has reached the maximum number of open positions: {max_positions}"
    )]
    MaxPositionsReached {
        account_id: String,
        max_positions: u8,
    },

    #[error("Invalid position flip: {reason}")]
    InvalidPositionFlip {
        reason: String,
    },

    #[error("Position not found: account_id {account_id}, denom {denom}")]
    PositionNotFound {
        account_id: String,
        denom: String,
    },

    #[error("Invalid deleverage position: {reason}")]
    DeleverageInvalidPosition {
        reason: String,
    },

    #[error("Reply id: {0} not valid")]
    ReplyIdError(u64),

    #[error("Invalid amount sent by credit manager after deleverage: expected {expected}, received {received}")]
    InvalidFundsAfterDeleverage {
        expected: Uint128,
        received: Uint128,
    },

    #[error("Collateralization ratio of the vault below threshold: {current_cr} < {threshold_cr}")]
    VaultUndercollateralized {
        current_cr: Decimal,
        threshold_cr: Decimal,
    },
}

pub type ContractResult<T> = Result<T, ContractError>;
