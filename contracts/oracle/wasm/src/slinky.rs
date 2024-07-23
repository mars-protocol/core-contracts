use cosmwasm_std::{Decimal, Deps, Empty, Env, Int128, QuerierWrapper, StdError, Uint128};
use cw_storage_plus::Map;
use mars_oracle_base::{
    ContractError, ContractResult, PriceSourceChecked, MAX_DENOM_DECIMALS, USD_DENOM,
};
use mars_types::oracle::{ActionKind, Config};
use neutron_sdk::bindings::{
    marketmap::query::{MarketMapQuery, MarketMapResponse},
    oracle::{
        query::{GetAllCurrencyPairsResponse, GetPriceResponse, OracleQuery},
        types::CurrencyPair,
    },
    query::NeutronQuery,
};

pub const SLINKY_QUOTE_CURRENCY: &str = "USD";

/// Maximum number of blocks that the price can be old.
/// The value is checked when setting up the price source.
pub const SLINKY_MAX_BLOCKS_OLD: u8 = 5;

pub trait CurrencyPairExt {
    fn key(&self) -> String;
}

impl CurrencyPairExt for CurrencyPair {
    /// Market key is a combination of base and quote currency symbols separated by a slash (e.g. BTC/USD).
    fn key(&self) -> String {
        format!("{}/{}", self.base, self.quote)
    }
}

/// Assert Slinky configuration
pub fn assert_slinky(
    deps: &Deps,
    base_symbol: &str,
    denom_decimals: u8,
    max_blocks_old: u8,
) -> ContractResult<()> {
    if denom_decimals > MAX_DENOM_DECIMALS {
        return Err(ContractError::InvalidPriceSource {
            reason: format!("denom_decimals must be <= {}", MAX_DENOM_DECIMALS),
        });
    }

    if max_blocks_old > SLINKY_MAX_BLOCKS_OLD {
        return Err(ContractError::InvalidPriceSource {
            reason: format!("max_blocks_old must be <= {}", SLINKY_MAX_BLOCKS_OLD),
        });
    }

    let currency_pair: CurrencyPair = CurrencyPair {
        base: base_symbol.to_string(),
        quote: SLINKY_QUOTE_CURRENCY.to_string(),
    };

    let ntrn_querier = QuerierWrapper::<NeutronQuery>::new(&*deps.querier);
    assert_currency_pair_in_oracle_module(&ntrn_querier, &currency_pair)?;
    assert_currency_pair_in_market_module(&ntrn_querier, &currency_pair)?;

    Ok(())
}

/// Assert that the currency pair exists in the x/oracle module
fn assert_currency_pair_in_oracle_module(
    querier: &QuerierWrapper<NeutronQuery>,
    currency_pair: &CurrencyPair,
) -> ContractResult<()> {
    // fetch all supported currency pairs in x/oracle module
    let oracle_currency_pairs_query: OracleQuery = OracleQuery::GetAllCurrencyPairs {};
    let oracle_currency_pairs_response: GetAllCurrencyPairsResponse =
        querier.query(&oracle_currency_pairs_query.into())?;
    if !oracle_currency_pairs_response.currency_pairs.contains(currency_pair) {
        return Err(ContractError::InvalidPriceSource {
            reason: format!(
                "Slinky Market {}/{} not found in x/oracle module",
                currency_pair.base, currency_pair.quote
            ),
        });
    }

    Ok(())
}

/// Assert that the currency pair exists in the x/marketmap module and is enabled
fn assert_currency_pair_in_market_module(
    querier: &QuerierWrapper<NeutronQuery>,
    currency_pair: &CurrencyPair,
) -> ContractResult<()> {
    // fetch all supported currency pairs in x/marketmap module
    // TODO: use MarketMapQuery::Market instead of MarketMapQuery::MarketMap when it returns Option<Market>
    let market_map_currency_pairs_query: MarketMapQuery = MarketMapQuery::MarketMap {};
    let market_map_currency_pairs_response: MarketMapResponse =
        querier.query(&market_map_currency_pairs_query.into())?;
    let market = market_map_currency_pairs_response.market_map.markets.get(&currency_pair.key());
    match market {
        None => {
            return Err(ContractError::InvalidPriceSource {
                reason: format!(
                    "Slinky Market {}/{} not found in x/marketmap module",
                    currency_pair.base, currency_pair.quote
                ),
            });
        }
        Some(market) => {
            if !market.ticker.enabled {
                return Err(ContractError::InvalidPriceSource {
                    reason: format!(
                        "Slinky Market {}/{} not enabled in x/marketmap module",
                        currency_pair.base, currency_pair.quote
                    ),
                });
            }
        }
    }

    Ok(())
}

pub fn query_slinky_price<P: PriceSourceChecked<Empty>>(
    deps: &Deps,
    env: &Env,
    config: &Config,
    price_sources: &Map<&str, P>,
    kind: ActionKind,
    base_symbol: &str,
    denom_decimals: u8,
    max_blocks_old: u8,
) -> ContractResult<Decimal> {
    let ntrn_querier = QuerierWrapper::<NeutronQuery>::new(&*deps.querier);

    let currency_pair: CurrencyPair = CurrencyPair {
        base: base_symbol.to_string(),
        quote: SLINKY_QUOTE_CURRENCY.to_string(),
    };

    assert_currency_pair_in_oracle_module(&ntrn_querier, &currency_pair)?;
    assert_currency_pair_in_market_module(&ntrn_querier, &currency_pair)?;

    // fetch price for currency_pair from x/oracle module
    let oracle_price_query: OracleQuery = OracleQuery::GetPrice {
        currency_pair: currency_pair.clone(),
    };
    let oracle_price_response: GetPriceResponse = ntrn_querier.query(&oracle_price_query.into())?;

    assert_oracle_price_validity(
        env.block.height,
        max_blocks_old,
        &currency_pair,
        &oracle_price_response,
    )?;

    // Use current price source for USD to check how much 1 USD is worth in base_denom
    let usd_price = price_sources
        .load(deps.storage, USD_DENOM)
        .map_err(|_| StdError::generic_err("Price source not found for denom 'usd'"))?
        .query_price(deps, env, USD_DENOM, config, price_sources, kind.clone())?;

    let scaled_price = scale_slinky_price(
        oracle_price_response.price.price,
        oracle_price_response.decimals,
        denom_decimals,
        usd_price,
    )?;

    Ok(scaled_price)
}

/// Assert validity of the price from x/oracle module
fn assert_oracle_price_validity(
    current_block_height: u64,
    max_blocks_old: u8,
    currency_pair: &CurrencyPair,
    price_response: &GetPriceResponse,
) -> ContractResult<()> {
    // check if block_height is not too old
    if (current_block_height
        - price_response.price.block_height.ok_or(ContractError::InvalidPrice {
            reason: "block_height is not available in Slinky OracleQuery response".to_string(),
        })?)
        > max_blocks_old as u64
    {
        return Err(ContractError::InvalidPrice {
            reason: format!(
                "Slinky Market {}/{} price is older than {} blocks",
                currency_pair.base, currency_pair.quote, max_blocks_old
            ),
        });
    }

    // make sure the price value is not None (i.e. has not been initialized)
    if price_response.nonce == 0 {
        return Err(ContractError::InvalidPrice {
            reason: format!(
                "Slinky Market {}/{} price is nil",
                currency_pair.base, currency_pair.quote
            ),
        });
    }

    Ok(())
}

/// We have to represent the price for utoken in base_denom.
/// Slinky price should be normalized with token decimals.
///
/// Let's try to convert BTC/USD reported by Slinky to ubtc/base_denom:
/// - base_denom = uusd
/// - price source set for usd (e.g. FIXED price source where 1 usd = 1000000 uusd = 10^6 uusd)
/// - denom_decimals (BTC) = 8
///
/// 1 BTC = 10^8 ubtc
///
/// 1 BTC = price * 10^(-slinky_decimals) USD
/// 10^8 ubtc = price * 10^(-slinky_decimals) * 10^6 uusd
/// ubtc = price * 10^(-slinky_decimals) * 10^6 / 10^8 uusd
/// ubtc = price * 10^(-slinky_decimals) * 10^6 * 10^(-8) uusd
/// ubtc/uusd = 6470160093122 * 10^(-8) * 10^6 * 10^(-8)
/// ubtc/uusd = 6470160093122 * 10^(-10) = 647.0160093122
///
/// Generalized formula:
/// utoken/uusd = price * 10^(-slinky_decimals) * usd_price_in_base_denom * 10^(-denom_decimals)
pub fn scale_slinky_price(
    slinky_value: Int128,
    slinky_decimals: u64,
    denom_decimals: u8,
    usd_price: Decimal,
) -> ContractResult<Decimal> {
    // Slinky price should be above 0
    if slinky_value <= Int128::zero() {
        return Err(ContractError::InvalidPrice {
            reason: "Slinky price should be greater than 0".to_string(),
        });
    }
    let value = slinky_value.unsigned_abs();

    // Slinky decimals should be 8 in most cases (see doc for `GetPriceResponse`).
    // This check is to prevent overflow in the calculation.
    let slinky_decimals = if slinky_decimals > u8::MAX as u64 {
        return Err(ContractError::InvalidPrice {
            reason: format!(
                "Slinky decimals {} too big (should be <= {})",
                slinky_decimals,
                u8::MAX
            ),
        });
    } else {
        slinky_decimals as u8
    };

    // USD price is expected to be represented as: 10^decimals (it is validated when setting usd price source).
    // Example:
    // 1 USD = 10^6 uusd = 1000000 uusd
    // We subtract 1 from the length of the string representation of the price to get the number of decimals.
    let usd_decimals = usd_price.to_string().len() as u8 - 1;

    let decimal_places = usd_decimals as i32 - denom_decimals as i32 - slinky_decimals as i32;
    let price = if decimal_places <= 0 {
        Decimal::from_atomics(value, decimal_places.unsigned_abs())?
    } else {
        // Impossible for current Slinky and USD price setup:
        // - Slinky price is 8 decimals
        // - USD price is 6 decimals
        // denom_decimals should be < -2 to get here but it is not possible.
        let target_expo = Uint128::from(10u8).checked_pow(decimal_places.unsigned_abs())?;
        let res = value.checked_mul(target_expo)?;
        Decimal::from_ratio(res, 1u128)
    };

    if price.is_zero() {
        return Err(ContractError::InvalidPrice {
            reason: "price is zero".to_string(),
        });
    }

    Ok(price)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn return_error_if_slinky_price_less_than_or_equal_to_zero() {
        let usd_price = Decimal::from_str("1000000").unwrap();

        // slinky price is 0
        let price_err = scale_slinky_price(Int128::zero(), 8, 6, usd_price).unwrap_err();
        assert_eq!(
            price_err,
            ContractError::InvalidPrice {
                reason: "Slinky price should be greater than 0".to_string(),
            }
        );

        // slinky price below 0
        let price_err = scale_slinky_price(Int128::from(-1), 8, 6, usd_price).unwrap_err();
        assert_eq!(
            price_err,
            ContractError::InvalidPrice {
                reason: "Slinky price should be greater than 0".to_string(),
            }
        );
    }

    #[test]
    fn return_error_if_slinky_decimals_too_big() {
        let price_err =
            scale_slinky_price(Int128::one(), 256, 6, Decimal::from_str("1000000").unwrap())
                .unwrap_err();
        assert_eq!(
            price_err,
            ContractError::InvalidPrice {
                reason: "Slinky decimals 256 too big (should be <= 255)".to_string(),
            }
        );
    }

    #[test]
    fn return_error_if_scaled_price_is_zero() {
        let price_err =
            scale_slinky_price(Int128::from(1), 18, 18, Decimal::from_str("1000000").unwrap())
                .unwrap_err();
        assert_eq!(
            price_err,
            ContractError::InvalidPrice {
                reason: "price is zero".to_string()
            }
        );
    }

    #[test]
    fn scale_slinky_price_if_decimal_places_less_than_or_equal_to_zero() {
        let usd_price = Decimal::from_str("1000000").unwrap();

        // slinky ETH price with 6 decimals
        let ueth_price_in_uusd =
            scale_slinky_price(Int128::from(3486881068i128), 6, 18, usd_price).unwrap();
        let exptected_price = Decimal::from_atomics(3486881068u128, 18u32).unwrap();
        assert_eq!(ueth_price_in_uusd, exptected_price);
        assert_eq!(ueth_price_in_uusd.to_string(), "0.000000003486881068".to_string());

        // slinky ETH price with 8 decimals
        let ueth_price_in_uusd =
            scale_slinky_price(Int128::from(348688106812i128), 8, 18, usd_price).unwrap();
        let exptected_price = Decimal::from_atomics(348688106812u128, 20u32).unwrap();
        assert_eq!(ueth_price_in_uusd, exptected_price);
        assert_eq!(ueth_price_in_uusd.to_string(), "0.000000003486881068".to_string()); // lost 2 digits precision

        // slinky bigger ETH price with 8 decimals
        let ueth_price_in_uusd =
            scale_slinky_price(Int128::from(1248688106812i128), 8, 18, usd_price).unwrap();
        let exptected_price = Decimal::from_atomics(1248688106812u128, 20u32).unwrap();
        assert_eq!(ueth_price_in_uusd, exptected_price);
        assert_eq!(ueth_price_in_uusd.to_string(), "0.000000012486881068".to_string()); // lost 2 digits precision

        // slinky TIA price with 8 decimals
        let utia_price_in_uusd =
            scale_slinky_price(Int128::from(652586790i128), 8, 6, usd_price).unwrap();
        let exptected_price = Decimal::from_atomics(652586790u128, 8u32).unwrap();
        assert_eq!(utia_price_in_uusd, exptected_price);
        assert_eq!(utia_price_in_uusd.to_string(), "6.5258679".to_string());

        // slinky DYDX price with 8 decimals
        let udydx_price_in_uusd =
            scale_slinky_price(Int128::from(142437588i128), 8, 18, usd_price).unwrap();
        let exptected_price = Decimal::from_atomics(142437588u128, 20u32).unwrap();
        assert_eq!(udydx_price_in_uusd, exptected_price);
        assert_eq!(udydx_price_in_uusd.to_string(), "0.000000000001424375".to_string());
        // lost 2 digits precision
    }

    #[test]
    fn scale_slinky_price_if_decimal_places_more_than_zero() {
        let usd_price = Decimal::from_str("10000000000").unwrap();

        let price_in_uusd = scale_slinky_price(Int128::from(15612i128), 2, 6, usd_price).unwrap();
        let exptected_price = Decimal::from_atomics(1561200u128, 0u32).unwrap();
        assert_eq!(price_in_uusd, exptected_price);
        assert_eq!(price_in_uusd.to_string(), "1561200".to_string());
    }
}
