use cosmwasm_std::{
    CheckedFromRatioError, CheckedMultiplyFractionError, CheckedMultiplyRatioError,
    ConversionOverflowError, Decimal, DecimalRangeExceeded, DivideByZeroError, DivisionError,
    OverflowError, SignedDecimalRangeExceeded, StdError,
};
use cw_utils::PaymentError;
use mars_owner::OwnerError;
use mars_utils::error::ValidationError;

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
    Validation(#[from] ValidationError),

    #[error("{0}")]
    Generic(String),

    #[error("Caller is not the Credit Manager contract")]
    NotCreditManager {},

    #[error("Vault account not found")]
    VaultAccountNotFound {},

    #[error("Vault account exists. Only one binding allowed between Credit Manager and Vault contracts.")]
    VaultAccountExists {},

    #[error("{reason:?}")]
    InvalidAmount {
        reason: String,
    },

    #[error("Unlocked positions not found")]
    UnlockedPositionsNotFound {},

    #[error("{user:?} is not the owner of {account_id:?}")]
    NotTokenOwner {
        user: String,
        account_id: String,
    },

    #[error("Invalid performance fee, expected less than {expected:?}, got {actual:?}")]
    InvalidPerformanceFee {
        expected: Decimal,
        actual: Decimal,
    },

    #[error("Zero performance fee")]
    ZeroPerformanceFee {},

    #[error("Zero shares")]
    ZeroShares {},

    #[error("Withdrawal interval not passed")]
    WithdrawalIntervalNotPassed {},

    #[error("Invalid cooldown period, expected value greater than 0")]
    ZeroCooldownPeriod,

    #[error("Contract owner not set")]
    NoOwner {},

    #[error(transparent)]
    DivisionError(#[from] DivisionError),

    #[error(transparent)]
    ConversionOverflowError(#[from] ConversionOverflowError),

    #[error(transparent)]
    SignedDecimalRangeExceeded(#[from] SignedDecimalRangeExceeded),
    #[error(
        "Vault with credit account id {vault_account_id:?} is bankrupt. Actions on this vault are not allowed."
    )]
    VaultBankrupt {
        vault_account_id: String,
    },

    #[error("Minimum amount of {denom} required to create a vault is {min_value:?} uusd, got {actual_value:?} uusd")]
    MinAmountRequired {
        min_value: u128,
        actual_value: u128,
        denom: String,
    },

    #[error("Rewards collector not set in Credit Manager")]
    RewardsCollectorNotSet {},
}

pub type ContractResult<T> = Result<T, ContractError>;

impl From<&str> for ContractError {
    fn from(val: &str) -> Self {
        ContractError::Generic(val.into())
    }
}
