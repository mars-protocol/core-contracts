use cosmwasm_std::{
    CheckedFromRatioError, CheckedMultiplyFractionError, CheckedMultiplyRatioError,
    ConversionOverflowError, DecimalRangeExceeded, DivideByZeroError, OverflowError,
    SignedDecimalRangeExceeded, StdError,
};
use cw2::VersionError;
use mars_owner::OwnerError;
use thiserror::Error;

use crate::perps::PerpsError;

pub type HealthResult<T> = Result<T, HealthError>;

#[derive(Error, Debug, PartialEq)]
pub enum HealthError {
    #[error("{0}")]
    DivideByZeroError(#[from] DivideByZeroError),

    #[error("{0}")]
    CheckedFromRatio(#[from] CheckedFromRatioError),

    #[error("{0}")]
    CheckedMultiplyFraction(#[from] CheckedMultiplyFractionError),

    #[error("{0}")]
    SignedDecimalRangeExceeded(#[from] SignedDecimalRangeExceeded),

    #[error("{0} not found in account's positions")]
    DenomNotPresent(String),

    #[error("{0} address has not been set in config")]
    ContractNotSet(String),

    #[error("{0} amount was not provided or is 0")]
    MissingAmount(String),

    #[error(
        "Account is an HLS account, but {0} was not provided HLS params to compute health with"
    )]
    MissingHLSParams(String),

    #[error("{0} was not provided asset params to compute health with")]
    MissingAssetParams(String),

    #[error("{0} was not provided perp params to compute health with")]
    MissingPerpParams(String),

    #[error("Max LTV / Liquidation threshold not provided for USDC Margin Accounts")]
    MissingUSDCMarginParams(String),

    #[error("{0} was not provided market state to compute health with")]
    MissingDenomState(String),

    #[error("{0} was not provided a price to compute health with")]
    MissingPrice(String),

    #[error("{0} was not provided a vault config to compute health with")]
    MissingVaultConfig(String),

    #[error("{0} was not provided vault info to compute health with")]
    MissingVaultInfo(String),

    #[error("{0} was not provided vault coin + base coin values to compute health with")]
    MissingVaultValues(String),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("{0}")]
    Owner(#[from] OwnerError),

    #[error("{0}")]
    PerpsError(#[from] PerpsError),

    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    DecimalRangeExceeded(#[from] DecimalRangeExceeded),

    #[error("{0}")]
    ConversionOverflowError(#[from] ConversionOverflowError),

    #[error("{0}")]
    CheckedMultiplyRatio(#[from] CheckedMultiplyRatioError),

    #[error("{0}")]
    Version(#[from] VersionError),
}
