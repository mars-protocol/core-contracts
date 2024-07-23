use std::str::FromStr;

use cosmwasm_std::{from_json, testing::mock_env, Decimal, DepsMut, Int128};
use mars_oracle_base::{ContractError, MAX_DENOM_DECIMALS};
use mars_oracle_wasm::{
    contract::entry::{self, execute},
    slinky::SLINKY_MAX_BLOCKS_OLD,
    WasmPriceSourceChecked, WasmPriceSourceUnchecked,
};
use mars_testing::{mock_env_at_block_height, mock_info};
use mars_types::oracle::{ExecuteMsg, PriceResponse, PriceSourceResponse, QueryMsg};
use neutron_sdk::bindings::{
    marketmap::types::{CurrencyPair as MarketCurrencyPair, Market, Ticker},
    oracle::{
        query::GetPriceResponse,
        types::{CurrencyPair as OracleCurrencyPair, QuotePrice},
    },
};
use test_case::test_case;

use super::helpers;

fn set_usd_price(deps: DepsMut) {
    // price source used to convert USD to base_denom
    helpers::set_price_source(
        deps,
        "usd",
        WasmPriceSourceUnchecked::Fixed {
            price: Decimal::from_str("1000000").unwrap(),
        },
    );
}

fn create_slinky_market(cp: &OracleCurrencyPair, decimals: u64, enabled: bool) -> Market {
    Market {
        ticker: Ticker {
            currency_pair: MarketCurrencyPair {
                base: cp.base.clone(),
                quote: cp.quote.clone(),
            },
            decimals,
            enabled,
            min_provider_count: 0, // don't care about the rest of the fields
            metadata_json: "".to_string(),
        },
        provider_configs: vec![],
    }
}

fn create_slinky_price(
    price: u128,
    decimals: u64,
    nonce: u64,
    block_height: Option<u64>,
) -> GetPriceResponse {
    GetPriceResponse {
        price: QuotePrice {
            price: Int128::from(price as i128),
            block_timestamp: "2024-07-17T14:30:51.771052356Z".to_string(),
            block_height,
        },
        nonce,
        decimals,
        id: 43,
    }
}

#[test]
fn display_slinky_price_source() {
    let ps = WasmPriceSourceChecked::Slinky {
        base_symbol: "ETH".to_string(),
        denom_decimals: 18,
        max_blocks_old: 2,
    };
    assert_eq!(ps.to_string(), "slinky:ETH:18:2");

    let ps = WasmPriceSourceChecked::Slinky {
        base_symbol: "TIA".to_string(),
        denom_decimals: 8,
        max_blocks_old: 4,
    };
    assert_eq!(ps.to_string(), "slinky:TIA:8:4");
}

#[test]
fn setting_invalid_usd_price_source() {
    let mut deps = helpers::setup_test("astroport_factory");

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner"),
        ExecuteMsg::SetPriceSource {
            denom: "usd".to_string(),
            price_source: WasmPriceSourceUnchecked::Slinky {
                base_symbol: "NTRN".to_string(),
                denom_decimals: 8,
                max_blocks_old: 2,
            },
        },
    )
    .unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "cannot set price source for USD other than 'Fixed' with price value 1 followed by zeros".to_string()
        }
    );

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner"),
        ExecuteMsg::SetPriceSource {
            denom: "usd".to_string(),
            price_source: WasmPriceSourceUnchecked::Fixed {
                price: Decimal::from_str("1000001").unwrap(),
            },
        },
    )
    .unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "cannot set price source for USD other than 'Fixed' with price value 1 followed by zeros".to_string()
        }
    );
}

#[test]
fn setting_slinky_price_source_if_missing_usd() {
    let mut deps = helpers::setup_test("astroport_factory");

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner"),
        ExecuteMsg::SetPriceSource {
            denom: "untrn".to_string(),
            price_source: WasmPriceSourceUnchecked::Slinky {
                base_symbol: "NTRN".to_string(),
                denom_decimals: 8,
                max_blocks_old: 2,
            },
        },
    )
    .unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "missing price source for usd".to_string()
        }
    );
}

#[test]
fn setting_slinky_price_source_if_denom_decimals_too_big() {
    let mut deps = helpers::setup_test("astroport_factory");

    set_usd_price(deps.as_mut());

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner"),
        ExecuteMsg::SetPriceSource {
            denom: "untrn".to_string(),
            price_source: WasmPriceSourceUnchecked::Slinky {
                base_symbol: "NTRN".to_string(),
                denom_decimals: MAX_DENOM_DECIMALS + 1,
                max_blocks_old: 2,
            },
        },
    )
    .unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "denom_decimals must be <= 18".to_string()
        }
    );
}

#[test]
fn setting_slinky_price_source_if_max_blocks_old_too_big() {
    let mut deps = helpers::setup_test("astroport_factory");

    set_usd_price(deps.as_mut());

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner"),
        ExecuteMsg::SetPriceSource {
            denom: "untrn".to_string(),
            price_source: WasmPriceSourceUnchecked::Slinky {
                base_symbol: "NTRN".to_string(),
                denom_decimals: 8,
                max_blocks_old: SLINKY_MAX_BLOCKS_OLD + 1,
            },
        },
    )
    .unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "max_blocks_old must be <= 5".to_string()
        }
    );
}

#[test]
fn setting_slinky_price_source_if_missing_currency_pair() {
    let mut deps = helpers::setup_test("astroport_factory");

    set_usd_price(deps.as_mut());

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner"),
        ExecuteMsg::SetPriceSource {
            denom: "untrn".to_string(),
            price_source: WasmPriceSourceUnchecked::Slinky {
                base_symbol: "NTRN".to_string(),
                denom_decimals: 8,
                max_blocks_old: 2,
            },
        },
    )
    .unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "Slinky Market NTRN/USD not found in x/oracle module".to_string()
        }
    );
}

#[test]
fn setting_slinky_price_source_if_missing_or_disabled_market() {
    let mut deps = helpers::setup_test("astroport_factory");

    set_usd_price(deps.as_mut());

    let cp = OracleCurrencyPair {
        base: "NTRN".to_string(),
        quote: "USD".to_string(),
    };
    deps.querier.set_slinky_currency_pair(cp.clone());

    let execute_fun = |deps: DepsMut| {
        execute(
            deps,
            mock_env(),
            mock_info("owner"),
            ExecuteMsg::SetPriceSource {
                denom: "untrn".to_string(),
                price_source: WasmPriceSourceUnchecked::Slinky {
                    base_symbol: "NTRN".to_string(),
                    denom_decimals: 8,
                    max_blocks_old: 2,
                },
            },
        )
    };

    let err = execute_fun(deps.as_mut()).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "Slinky Market NTRN/USD not found in x/marketmap module".to_string()
        }
    );

    deps.querier.set_slinky_market(cp.clone(), create_slinky_market(&cp, 8, false));

    let err = execute_fun(deps.as_mut()).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "Slinky Market NTRN/USD not enabled in x/marketmap module".to_string()
        }
    );
}

#[test]
fn setting_slinky_price_source_successfully() {
    let mut deps = helpers::setup_test("astroport_factory");

    set_usd_price(deps.as_mut());

    let cp = OracleCurrencyPair {
        base: "NTRN".to_string(),
        quote: "USD".to_string(),
    };
    deps.querier.set_slinky_currency_pair(cp.clone());
    deps.querier.set_slinky_market(cp.clone(), create_slinky_market(&cp, 8, true));

    helpers::set_price_source(
        deps.as_mut(),
        "untrn",
        WasmPriceSourceUnchecked::Slinky {
            base_symbol: "NTRN".to_string(),
            denom_decimals: 8,
            max_blocks_old: 4,
        },
    );

    let res: PriceSourceResponse<WasmPriceSourceChecked> = helpers::query(
        deps.as_ref(),
        QueryMsg::PriceSource {
            denom: "untrn".to_string(),
        },
    );
    assert_eq!(
        res,
        PriceSourceResponse {
            denom: "untrn".to_string(),
            price_source: WasmPriceSourceChecked::Slinky {
                base_symbol: "NTRN".to_string(),
                denom_decimals: 8,
                max_blocks_old: 4,
            }
        }
    );
}

#[test]
fn quering_slinky_price_if_missing_currency_pair() {
    let mut deps = helpers::setup_test("astroport_factory");

    set_usd_price(deps.as_mut());

    let cp = OracleCurrencyPair {
        base: "NTRN".to_string(),
        quote: "USD".to_string(),
    };
    deps.querier.set_slinky_currency_pair(cp.clone());
    deps.querier.set_slinky_market(cp.clone(), create_slinky_market(&cp, 8, true));

    helpers::set_price_source(
        deps.as_mut(),
        "untrn",
        WasmPriceSourceUnchecked::Slinky {
            base_symbol: "NTRN".to_string(),
            denom_decimals: 8,
            max_blocks_old: 4,
        },
    );

    deps.querier.remove_slinky_currency_pair(cp.clone());

    let err = entry::query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Price {
            denom: "untrn".to_string(),
            kind: None,
        },
    )
    .unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "Slinky Market NTRN/USD not found in x/oracle module".to_string()
        }
    );
}

#[test]
fn quering_slinky_price_if_missing_or_disabled_market() {
    let mut deps = helpers::setup_test("astroport_factory");

    set_usd_price(deps.as_mut());

    let cp = OracleCurrencyPair {
        base: "NTRN".to_string(),
        quote: "USD".to_string(),
    };
    deps.querier.set_slinky_currency_pair(cp.clone());
    deps.querier.set_slinky_market(cp.clone(), create_slinky_market(&cp, 8, true));

    helpers::set_price_source(
        deps.as_mut(),
        "untrn",
        WasmPriceSourceUnchecked::Slinky {
            base_symbol: "NTRN".to_string(),
            denom_decimals: 8,
            max_blocks_old: 4,
        },
    );

    deps.querier.remove_slinky_market(cp.clone());

    let err = entry::query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Price {
            denom: "untrn".to_string(),
            kind: None,
        },
    )
    .unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "Slinky Market NTRN/USD not found in x/marketmap module".to_string()
        }
    );

    deps.querier.set_slinky_market(cp.clone(), create_slinky_market(&cp, 8, false));

    let err = entry::query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Price {
            denom: "untrn".to_string(),
            kind: None,
        },
    )
    .unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidPriceSource {
            reason: "Slinky Market NTRN/USD not enabled in x/marketmap module".to_string()
        }
    );
}

#[test_case(1u64, None, 0u64, ContractError::InvalidPrice { reason: "block_height is not available in Slinky OracleQuery response".to_string()}; "missing block height")]
#[test_case(1u64, Some(987654321), 1u64, ContractError::InvalidPrice { reason: "Slinky Market NTRN/USD price is older than 4 blocks".to_string()}; "price is too old")]
#[test_case(0u64, Some(987654321), 0u64, ContractError::InvalidPrice { reason: "Slinky Market NTRN/USD price is nil".to_string()}; "price is nil")]
fn quering_slinky_price_if_invalid_price_response(
    nonce: u64,
    block_height: Option<u64>,
    block_offset: u64,
    expected_err: ContractError,
) {
    let mut deps = helpers::setup_test("astroport_factory");

    set_usd_price(deps.as_mut());

    let max_blocks_old = 4u8;
    let cp = OracleCurrencyPair {
        base: "NTRN".to_string(),
        quote: "USD".to_string(),
    };

    deps.querier.set_slinky_currency_pair(cp.clone());
    deps.querier.set_slinky_market(cp.clone(), create_slinky_market(&cp, 8, true));

    helpers::set_price_source(
        deps.as_mut(),
        "untrn",
        WasmPriceSourceUnchecked::Slinky {
            base_symbol: "NTRN".to_string(),
            denom_decimals: 8,
            max_blocks_old,
        },
    );

    deps.querier.set_slinky_price(cp.clone(), create_slinky_price(124u128, 8, nonce, block_height));

    let env_block = if let Some(bh) = block_height {
        bh + max_blocks_old as u64 + block_offset
    } else {
        0u64
    };

    let err = entry::query(
        deps.as_ref(),
        mock_env_at_block_height(env_block),
        QueryMsg::Price {
            denom: "untrn".to_string(),
            kind: None,
        },
    )
    .unwrap_err();
    assert_eq!(err, expected_err);
}

#[test]
fn querying_slinky_price_successfully() {
    let mut deps = helpers::setup_test("astroport_factory");

    set_usd_price(deps.as_mut());

    let publish_block_height = 987654321u64;
    let max_blocks_old = 4u8;
    let cp = OracleCurrencyPair {
        base: "NTRN".to_string(),
        quote: "USD".to_string(),
    };

    deps.querier.set_slinky_currency_pair(cp.clone());
    deps.querier.set_slinky_market(cp.clone(), create_slinky_market(&cp, 8, true));

    helpers::set_price_source(
        deps.as_mut(),
        "untrn",
        WasmPriceSourceUnchecked::Slinky {
            base_symbol: "NTRN".to_string(),
            denom_decimals: 8,
            max_blocks_old,
        },
    );

    deps.querier.set_slinky_price(
        cp.clone(),
        create_slinky_price(52630171u128, 8, 1, Some(publish_block_height)),
    );

    let res = entry::query(
        deps.as_ref(),
        mock_env_at_block_height(publish_block_height + 1u64),
        QueryMsg::Price {
            denom: "untrn".to_string(),
            kind: None,
        },
    )
    .unwrap();
    let res: PriceResponse = from_json(res).unwrap();
    assert_eq!(
        res,
        PriceResponse {
            denom: "untrn".to_string(),
            price: Decimal::from_str("0.0052630171").unwrap(),
        }
    );
}
