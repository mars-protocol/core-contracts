use cosmwasm_std::{
    CheckedFromRatioError, CheckedMultiplyFractionError, ConversionOverflowError,
    DecimalRangeExceeded, DivideByZeroError, DivisionError, OverflowError,
    SignedDecimalRangeExceeded, StdError,
};
use mars_delta_neutral_position::types::Side;
use thiserror::Error;

pub type ContractResult<T> = Result<T, ContractError>;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Account ID not owned by this contract")]
    NotOwned {},

    #[error("Profitability validation failed")]
    ProfitabilityValidationFailed {},

    #[error("{0}")]
    DivideByZeroError(#[from] DivideByZeroError),

    #[error("{0}")]
    OverflowError(#[from] OverflowError),

    #[error("{0}")]
    CheckedFromRatioError(#[from] CheckedFromRatioError),

    #[error("{0}")]
    CheckedMultiplyFractionError(#[from] CheckedMultiplyFractionError),

    #[error(
        "The minimum hedge deviation was exceeded. Min acceptable: {min}, result was: {actual}"
    )]
    ExecutionDeviationExceeded {
        min: String,
        actual: String,
    },

    #[error("Invalid decrease or position size")]
    InvalidDecreaseOrPositionSize {},

    #[error("{0}")]
    SignedDecimalRangeExceeded(#[from] SignedDecimalRangeExceeded),

    #[error("{0}")]
    DecimalRangeExceeded(#[from] DecimalRangeExceeded),

    #[error("{0}")]
    DivisionError(#[from] DivisionError),

    #[error("{0}")]
    ConversionOverflowError(#[from] ConversionOverflowError),

    #[error("Invalid amount: {reason}")]
    InvalidAmount {
        reason: String,
    },

    #[error("Direction mismatch. Existing direction: {existing_direction}, new direction: {new_direction}")]
    DirectionMismatch {
        existing_direction: Side,
        new_direction: Side,
    },

    #[error("{0}")]
    PositionContractError(#[from] mars_delta_neutral_position::error::ContractError),
}
