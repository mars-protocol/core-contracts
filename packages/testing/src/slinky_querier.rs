use std::collections::{BTreeMap, HashMap};

use cosmwasm_std::{to_json_binary, Binary, ContractResult, QuerierResult};
use mars_oracle_wasm::slinky::CurrencyPairExt;
use neutron_sdk::bindings::{
    marketmap::{
        query::{MarketMapQuery, MarketMapResponse},
        types::{Market, MarketMap},
    },
    oracle::{
        query::{GetAllCurrencyPairsResponse, GetPriceResponse, OracleQuery},
        types::CurrencyPair,
    },
    query::NeutronQuery,
};

#[derive(Default)]
pub struct SlinkyQuerier {
    pub currency_pairs: Vec<CurrencyPair>,
    pub markets: BTreeMap<String, Market>,
    pub prices: HashMap<String, GetPriceResponse>,
}

impl SlinkyQuerier {
    pub fn handle_query(&self, query: NeutronQuery) -> QuerierResult {
        let res: ContractResult<Binary> = match query {
            NeutronQuery::Oracle(oracle_query) => match oracle_query {
                OracleQuery::GetAllCurrencyPairs {} => {
                    let response = GetAllCurrencyPairsResponse {
                        currency_pairs: self.currency_pairs.clone(),
                    };
                    to_json_binary(&response).into()
                }
                OracleQuery::GetPrice {
                    currency_pair,
                } => {
                    let key = currency_pair.key();
                    let option_price = self.prices.get(&key);

                    if let Some(price) = option_price {
                        to_json_binary(price).into()
                    } else {
                        Err(format!("[mock]: could not find Slinky price for {key}")).into()
                    }
                }
                OracleQuery::GetPrices {
                    currency_pair_ids: _,
                } => Err("[mock]: Unsupported Slinky GetPrices query").into(),
            },
            NeutronQuery::MarketMap(market_map_query) => match market_map_query {
                MarketMapQuery::MarketMap {} => {
                    let response = MarketMapResponse {
                        market_map: MarketMap {
                            markets: self.markets.clone(),
                        },
                        last_updated: None,
                        chain_id: "neutron".to_string(),
                    };
                    to_json_binary(&response).into()
                }
                MarketMapQuery::Market {
                    currency_pair,
                } => {
                    let key = currency_pair.key();
                    let option_market = self.markets.get(&key);

                    if let Some(market) = option_market {
                        to_json_binary(market).into()
                    } else {
                        Err(format!("[mock]: could not find Slinky market for {key}")).into()
                    }
                }
                MarketMapQuery::Params {} => Err("[mock]: Unsupported Slinky Params query").into(),
                MarketMapQuery::LastUpdated {} => {
                    Err("[mock]: Unsupported Slinky LastUpdated query").into()
                }
            },

            _ => Err("[mock]: Unsupported Slinky query").into(),
        };

        Ok(res).into()
    }
}
