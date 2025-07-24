use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, CosmosMsg, Decimal, Empty, Env, QuerierWrapper, Uint128, Uint256};
use mars_swapper_base::{ContractError, ContractResult, Route};
use mars_types::swapper::{EstimateExactInSwapResponse, SwapperRoute};
use neutron_sdk::{
    bindings::msg::NeutronMsg,
    stargate::dex::{
        msg::msg_multi_hop_swap,
        types::{LimitOrderType, MultiHopSwapRequest, PlaceLimitOrderRequest},
    },
};

use crate::{
    config::DualityConfig,
    helpers::{hashset, msg_place_limit_order},
};

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

        // There must be at least two denoms in the route
        if swap_denoms.len() < 2 {
            return Err(ContractError::InvalidRoute {
                reason: "the route must contain at least one pair".to_string(),
            });
        }

        // Ensure the first denom in the route is the input denom
        if swap_denoms.first() != Some(&denom_in.to_string()) {
            return Err(ContractError::InvalidRoute {
                reason: format!(
                    "the route's first denom {} does not match the input denom {}",
                    swap_denoms.first().unwrap_or(&"none".to_string()),
                    denom_in
                ),
            });
        }

        // Ensure the last denom in the route is the output denom
        if swap_denoms.last() != Some(&denom_out.to_string()) {
            return Err(ContractError::InvalidRoute {
                reason: format!(
                    "the route's last denom {} does not match the output denom {}",
                    swap_denoms.last().unwrap_or(&"none".to_string()),
                    denom_out
                ),
            });
        }

        // Check for loops - each denom should only appear once in the route
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

        // If we have more than two denoms, we need to do a multi-hop swap.
        let swap_msg: CosmosMsg<NeutronMsg> = if swap_denoms.len() > 2 {
            // PrecDec (neutrons decimal implementation) uses fixed-point precision of 27 decimal places.
            let exponent = Uint256::from(10u128.pow(27));
            let min_receive_scaled = Uint256::from(min_receive).checked_mul(exponent)?;

            // Our limit sell price is the worst price we are willing to accept.
            // Note that MultiHopSwapRequest msg requires the raw integer string value of the price, not a decimal string.
            // This means that 1.0 will be 1^27 (1000000000000000000000000000)
            let exit_limit_price = min_receive_scaled.checked_div(coin_in.amount.into())?;
            msg_multi_hop_swap(MultiHopSwapRequest {
                sender: env.contract.address.to_string(),
                receiver: env.contract.address.to_string(),
                routes: vec![swap_denoms.clone()],
                amount_in: coin_in.amount.to_string(),
                exit_limit_price: exit_limit_price.to_string(),
                pick_best_route: true,
            })
        } else {
            // The PlaceLimitOrderRequest msg requires the decimal, not the integer value.
            let limit_sell_price = Decimal::from_ratio(min_receive, coin_in.amount).to_string();

            msg_place_limit_order(PlaceLimitOrderRequest {
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
                limit_sell_price,
            })?
        };

        Ok(swap_msg)
    }

    fn estimate_exact_in_swap(
        &self,
        _: &QuerierWrapper,
        _: &Env,
        _: &Coin,
    ) -> ContractResult<EstimateExactInSwapResponse> {
        unimplemented!("Duality does not support estimate_exact_in_swap")
    }
}
