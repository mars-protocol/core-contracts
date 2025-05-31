use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, CosmosMsg, Decimal, Empty, Env, QuerierWrapper, Uint128};
use mars_swapper_base::{ContractError, ContractResult, Route};
use mars_types::swapper::{EstimateExactInSwapResponse, SwapperRoute};
use neutron_sdk::{
    bindings::msg::NeutronMsg,
    stargate::dex::types::{LimitOrderType, MultiHopSwapRequest, PlaceLimitOrderRequest},
};

use crate::{config::DualityConfig, helpers::hashset};

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

        // ensure the first denom in the route is the input denom
        if swap_denoms.first() != Some(&denom_in.to_string()) {
            return Err(ContractError::InvalidRoute {
                reason: format!(
                    "the route's first denom {} does not match the input denom {}",
                    swap_denoms.first().unwrap_or(&"none".to_string()),
                    denom_in
                ),
            });
        }

        // ensure the last denom in the route is the output denom
        if swap_denoms.last() != Some(&denom_out.to_string()) {
            return Err(ContractError::InvalidRoute {
                reason: format!(
                    "the route's last denom {} does not match the output denom {}",
                    swap_denoms.last().unwrap_or(&"none".to_string()),
                    denom_out
                ),
            });
        }

        // check for loops - each denom should only appear once in the route
        let mut seen_denoms = hashset(&[]);
        for denom in swap_denoms.iter() {
            if seen_denoms.contains(denom) {
                return Err(ContractError::InvalidRoute {
                    reason: format!("route contains a loop: denom {} seen twice", denom),
                });
            }
            seen_denoms.insert(denom.to_string());
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
        let limit_sell_price = Decimal::from_ratio(min_receive, coin_in.amount);

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
        _: &QuerierWrapper,
        _: &Env,
        _: &Coin,
    ) -> ContractResult<EstimateExactInSwapResponse> {
        unimplemented!("Duality does not yet support estimate_exact_in_swap")
    }
}
