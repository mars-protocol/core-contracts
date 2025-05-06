#[cfg(not(feature = "library"))]
pub mod contract;
pub mod error;
pub mod execute;
pub mod instantiate;
pub mod msg;
pub mod performance_fee;
pub mod query;
pub mod state;
pub mod token_factory;
pub mod vault_token;

/// Minimum amount of UUSD (oracle base denom) required to create a vault.
/// This is to prevent spamming of vault creation.
/// 50 UUSD is the minimum amount of UUSD required to create a vault.
pub const MIN_VAULT_FEE_CREATION_IN_UUSD: u128 = 50000000u128;
