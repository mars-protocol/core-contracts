use cosmwasm_std::{
    CheckedFromRatioError, CheckedMultiplyFractionError, CheckedMultiplyRatioError,
    DecimalRangeExceeded, OverflowError, StdError, Uint128,
};
use cw_utils::PaymentError;
use mars_owner::OwnerError;

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
    Owner(#[from] OwnerError),

    #[error(transparent)]
    Payment(#[from] PaymentError),

    #[error("denom `{denom}` is already enabled")]
    DenomEnabled {
        denom: String,
    },

    #[error("denom `{denom}` exists but is not enabled")]
    DenomNotEnabled {
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

    #[error("positive opening value is too small: min {min}, found {found}")]
    PositionTooSmall {
        min: Uint128,
        found: Uint128,
    },

    #[error("only the credit manager can modify perp positions")]
    SenderIsNotCreditManager,

    #[error("cannot compute deposit amount when there is zero total shares")]
    ZeroTotalShares,

    #[error("cannot unlock when there is zero shares")]
    ZeroShares,

    #[error("Invalid param: {reason}")]
    InvalidParam {
        reason: String,
    },

    #[error("Unlocked positions not found")]
    UnlockedPositionsNotFound {},
}

pub type ContractResult<T> = Result<T, ContractError>;
