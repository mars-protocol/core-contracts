use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, CosmosMsg, Empty, Env, QuerierWrapper, QueryRequest, Uint128};
use mars_swapper_base::{ContractError, ContractResult, Route};
use mars_types::swapper::{EstimateExactInSwapResponse, SwapperRoute};
use neutron_sdk::{
    bindings::msg::NeutronMsg,
    proto_types::neutron::dex::{
        QueryEstimateMultiHopSwapRequest, QueryEstimatePlaceLimitOrderRequest,
    },
    stargate::dex::
        types::{
            EstimateMultiHopSwapRequest, EstimateMultiHopSwapResponse,
            EstimatePlaceLimitOrderRequest, EstimatePlaceLimitOrderResponse, LimitOrderType,
            MultiHopSwapRequest, PlaceLimitOrderRequest,
        },
};
use prost::Message;
use crate::{config::DualityConfig, helpers::hashset};

const ESTIMATE_MULTI_HOP_SWAP_QUERY_PATH: &str = "/neutron.dex.Query/EstimateMultiHopSwap";
const ESTIMATE_PLACE_LIMIT_ORDER_QUERY_PATH: &str = "/neutron.dex.Query/EstimatePlaceLimitOrder";

#[cw_serde]
pub struct DualityRoute {
    pub from: String,
    pub to: String,
    pub swap_denoms: Vec<String>,
}

impl std::fmt::Display for DualityRoute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let swap_denoms =
            self.swap_denoms.iter().map(|d| d.to_string()).collect::<Vec<_>>().join(", ");
        write!(
            f,
            "DualityRoute{{ from: {}, to: {}, swap_denoms: [{}] }}",
            self.from, self.to, swap_denoms
        )
    }
}

impl Route<NeutronMsg, Empty, DualityConfig> for DualityRoute {
    fn from(route: SwapperRoute, _: Option<DualityConfig>) -> ContractResult<Self> {
        match route {
            SwapperRoute::Duality(route) => Ok(Self {
                from: route.from,
                to: route.to,
                swap_denoms: route.swap_denoms,
            }),
            _ => Err(ContractError::InvalidRoute {
                reason: "Invalid route type. Route must be of type DualityRoute".to_string(),
            }),
        }
    }

    fn validate(
        &self,
        _querier: &QuerierWrapper,
        denom_in: &str,
        denom_out: &str,
    ) -> ContractResult<()> {
        let swap_denoms = &self.swap_denoms;

        // there must be at least two denoms in the route
        if swap_denoms.len() < 2 {
            return Err(ContractError::InvalidRoute {
                reason: "the route must contain at least one pair".to_string(),
            });
        }

        // for each denom:
        // - the denom must not have been seen before
        let mut prev_denom_out = denom_in.to_string();
        let mut seen_denoms = hashset(&[prev_denom_out.clone()]);
        for denom in swap_denoms.iter() {
            if seen_denoms.contains(denom) {
                return Err(ContractError::InvalidRoute {
                    reason: format!("route contains a loop: denom {} seen twice", denom),
                });
            }

            prev_denom_out = denom.to_string();
            seen_denoms.insert(denom.to_string());
        }

        // the route's final output denom must match the desired output denom
        if prev_denom_out != denom_out {
            return Err(ContractError::InvalidRoute {
                reason: format!(
                    "the route's output denom {prev_denom_out} does not match the desired output {denom_out}",
                ),
            });
        }

        Ok(())
    }

    fn build_exact_in_swap_msg(
        &self,
        _querier: &QuerierWrapper,
        env: &Env,
        coin_in: &Coin,
        min_receive: Uint128,
    ) -> ContractResult<CosmosMsg<NeutronMsg>> {
        let swap_denoms = &self.swap_denoms;

        if swap_denoms.len() < 2 {
            return Err(ContractError::InvalidRoute {
                reason: "the route must contain at least two denoms".to_string(),
            });
        }

        // our limit sell price is the worst price we are willing to accept.
        let limit_sell_price = coin_in.amount.checked_div(min_receive)?;

        // if we have more than two denoms, we need to do a multi-hop swap
        let swap_msg: CosmosMsg<NeutronMsg> = if swap_denoms.len() > 2 {
            neutron_sdk::stargate::dex::msg::msg_multi_hop_swap(MultiHopSwapRequest {
                sender: env.contract.address.to_string(),
                receiver: env.contract.address.to_string(),
                routes: vec![swap_denoms.clone()],
                amount_in: coin_in.amount.to_string(),
                    exit_limit_price: limit_sell_price.to_string(),
                    pick_best_route: true,
                })
        } else {
            neutron_sdk::stargate::dex::msg::msg_place_limit_order(PlaceLimitOrderRequest {
                    order_type: LimitOrderType::FillOrKill,
                    sender: env.contract.address.to_string(),
                    receiver: env.contract.address.to_string(),
                    token_in: coin_in.denom.to_string(),
                    token_out: self.to.to_string(),
                    // tick_index_in_to_out is depreciated in favor of limit_sell_price
                    tick_index_in_to_out: 0,
                    amount_in: coin_in.amount.to_string(),
                    expiration_time: None,
                    max_amount_out: None,
                    limit_sell_price: limit_sell_price.to_string(),
                })
        };

        Ok(swap_msg)
    }

    fn estimate_exact_in_swap(
        &self,
        querier: &QuerierWrapper,
        env: &Env,
        coin_in: &Coin,
    ) -> ContractResult<EstimateExactInSwapResponse> {
        let swap_denoms = &self.swap_denoms;

        if swap_denoms.len() < 2 {
            return Err(ContractError::InvalidRoute {
                reason: "the route must contain at least two denoms".to_string(),
            });
        }

        // if we have more than two denoms, we need to do a multi-hop swap
        let amount_out = if swap_denoms.len() > 2 {
            let path = ESTIMATE_MULTI_HOP_SWAP_QUERY_PATH;
            let query_data = QueryEstimateMultiHopSwapRequest::from(EstimateMultiHopSwapRequest {
                creator: env.contract.address.to_string(),
                receiver: env.contract.address.to_string(),
                routes: vec![swap_denoms.clone()],
                amount_in: coin_in.amount.to_string(),
                // TODO is this an issue?
                exit_limit_price: "0".to_string(),
                pick_best_route: true,
            });

            let res: EstimateMultiHopSwapResponse = querier.query(&QueryRequest::Stargate {
                path: path.to_string(),
                data: query_data.encode_to_vec().into(),
            })?;
            res.coin_out.amount
        } else {
            let path = ESTIMATE_PLACE_LIMIT_ORDER_QUERY_PATH;
            let query_data =
                QueryEstimatePlaceLimitOrderRequest::from(EstimatePlaceLimitOrderRequest {
                    order_type: LimitOrderType::FillOrKill,
                    creator: env.contract.address.to_string(),
                    receiver: env.contract.address.to_string(),
                    token_in: coin_in.denom.to_string(),
                    token_out: self.to.to_string(),
                    tick_index_in_to_out: 0,
                    amount_in: coin_in.amount.to_string(),
                    expiration_time: None,
                    max_amount_out: None,
                });

            let res: EstimatePlaceLimitOrderResponse = querier.query(&QueryRequest::Stargate {
                path: path.to_string(),
                data: query_data.encode_to_vec().into(),
            })?;

            res.swap_out_coin.amount
        };

        Ok(EstimateExactInSwapResponse {
            amount: amount_out,
        })
    }
}
