pub mod contract;
pub mod emergency_powers;
pub mod error;
pub mod execute;
pub mod migrations;
pub mod query;
pub mod state;

/// Minimum amount of UUSD (oracle base denom) required to create a vault.
/// This is to prevent spamming of vault creation.
/// 50 UUSD is the minimum amount of UUSD required to create a vault.
pub const MIN_VAULT_FEE_CREATION_IN_UUSD: u128 = 50000000u128;
