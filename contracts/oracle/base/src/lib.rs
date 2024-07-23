mod contract;
mod error;
mod traits;

pub mod lp_pricing;
pub mod pyth;
pub mod redemption_rate;

pub use contract::*;
use cosmwasm_std::{Deps, Empty};
use cw_storage_plus::Map;
pub use error::*;
pub use traits::*;

/// Denom for USD, used in price source to get USD price in uusd
pub const USD_DENOM: &str = "usd";

/// We don't support any denom with more than 18 decimals
pub const MAX_DENOM_DECIMALS: u8 = 18;

/// Assert availability of usd price source
pub fn assert_usd_price_source<P: PriceSourceChecked<Empty>>(
    deps: &Deps,
    price_sources: &Map<&str, P>,
) -> ContractResult<()> {
    if !price_sources.has(deps.storage, USD_DENOM) {
        return Err(ContractError::InvalidPriceSource {
            reason: "missing price source for usd".to_string(),
        });
    }

    Ok(())
}

/// Check if usd price is 1 followed by zeros (e.g. 1000000 = 10^6)
pub fn is_one_followed_by_zeros(price: &str) -> bool {
    price.starts_with('1') && price.chars().skip(1).all(|c| c == '0')
}

#[cfg(test)]
mod tests {
    use crate::is_one_followed_by_zeros;

    #[test]
    fn one_and_zeros_for_usd_price() {
        assert!(!is_one_followed_by_zeros("0"));
        assert!(is_one_followed_by_zeros("1"));
        assert!(is_one_followed_by_zeros("10"));
        assert!(is_one_followed_by_zeros("1000000"));
        assert!(!is_one_followed_by_zeros("1100"));
        assert!(!is_one_followed_by_zeros("1001"));
        assert!(!is_one_followed_by_zeros("2000"));
    }
}
