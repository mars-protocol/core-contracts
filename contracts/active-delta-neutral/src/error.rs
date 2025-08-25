use cosmwasm_std::{
    CheckedFromRatioError, CheckedMultiplyFractionError, ConversionOverflowError,
    DecimalRangeExceeded, DivideByZeroError, DivisionError, OverflowError,
    SignedDecimalRangeExceeded, StdError,
};
use mars_delta_neutral_position::types::Side;
use mars_owner::OwnerError;
use mars_types::perps::PerpsError;
use mars_utils::error::ValidationError;
use thiserror::Error;

pub type ContractResult<T> = Result<T, ContractError>;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Account ID not owned by this contract")]
    NotOwned {},

    #[error("{0}")]
    Owner(#[from] OwnerError),

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

    #[error("{0}")]
    ValidationError(#[from] ValidationError),

    #[error("Credit account not initialized")]
    CreditAccountNotInitialized {},

    #[error("No collateral found for denom: {denom}")]
    NoCollateralForDenom {
        denom: String,
    },

    #[error("Invalid funds: Received multiple coins. May only send one coin of denom: {denom}")]
    ExcessAssets {
        denom: String,
    },

    #[error("Invalid funds. Received {denom} but contract may only receive {base_denom}")]
    IncorrectDenom {
        denom: String,
        base_denom: String,
    },

    #[error("This msg cannot receive funds")]
    IllegalFundsSent {},

    #[error("{0}")]
    PerpsError(#[from] PerpsError),
    
}
