#[cfg(test)]
mod tests {
    use cosmwasm_std::{testing::mock_env, Coin, CosmosMsg, Uint128, WasmMsg};
    use mars_rewards_collector_base::{ContractError, SwapMsg};
    use mars_rewards_collector_neutron::NeutronMsgFactory;
    use mars_types::{
        address_provider::{AddressResponseItem, MarsAddressType},
        swapper::{DualityRoute, SwapperRoute},
    };

    #[test]
    fn test_neutron_swap_msg_with_duality_route() {
        let env = mock_env();

        let coin_in = Coin {
            denom: "untrn".to_string(),
            amount: Uint128::new(1000),
        };

        let duality_route = Some(SwapperRoute::Duality(DualityRoute {
            from: "untrn".to_string(),
            to: "uusdc".to_string(),
            swap_denoms: vec!["untrn".to_string(), "uusdc".to_string()],
        }));

        let default_swapper = AddressResponseItem {
            address: "swapper_contract".to_string(),
            address_type: MarsAddressType::Swapper,
        };
        let duality_swapper = Some(AddressResponseItem {
            address: "duality_swapper".to_string(),
            address_type: MarsAddressType::DualitySwapper,
        });

        let result = NeutronMsgFactory::swap_msg(
            &env,
            &default_swapper,
            &duality_swapper,
            coin_in.clone(),
            "uusdc",
            Uint128::new(950),
            duality_route,
        );

        assert!(result.is_ok());
        let msg = result.unwrap();

        match msg {
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                funds,
                ..
            }) => {
                assert_eq!(contract_addr, "duality_swapper");
                assert_eq!(funds, vec![coin_in]);
            }
            _ => panic!("Expected WasmMsg::Execute"),
        }
    }

    #[test]
    fn test_neutron_swap_msg_with_default_swapper() {
        let env = mock_env();

        let coin_in = Coin {
            denom: "untrn".to_string(),
            amount: Uint128::new(1000),
        };

        let default_swapper = AddressResponseItem {
            address: "swapper_contract".to_string(),
            address_type: MarsAddressType::Swapper,
        };
        let duality_swapper = Some(AddressResponseItem {
            address: "duality_swapper".to_string(),
            address_type: MarsAddressType::DualitySwapper,
        });

        // Test with no route (should use default swapper)
        let result = NeutronMsgFactory::swap_msg(
            &env,
            &default_swapper,
            &duality_swapper,
            coin_in.clone(),
            "uusdc",
            Uint128::new(950),
            None,
        );

        assert!(result.is_ok());
        let msg = result.unwrap();

        match msg {
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                funds,
                ..
            }) => {
                assert_eq!(contract_addr, "swapper_contract");
                assert_eq!(funds, vec![coin_in]);
            }
            _ => panic!("Expected WasmMsg::Execute"),
        }
    }

    #[test]
    fn test_neutron_swap_msg_with_astro_route() {
        let env = mock_env();

        let coin_in = Coin {
            denom: "untrn".to_string(),
            amount: Uint128::new(1000),
        };

        // Test with osmosis route (should use default swapper)
        let route = Some(SwapperRoute::Astro(mars_types::swapper::AstroRoute {
            swaps: vec![mars_types::swapper::AstroSwap {
                from: "untrn".to_string(),
                to: "uusdc".to_string(),
            }],
        }));

        let default_swapper = AddressResponseItem {
            address: "swapper_contract".to_string(),
            address_type: MarsAddressType::Swapper,
        };

        let duality_swapper = Some(AddressResponseItem {
            address: "duality_swapper".to_string(),
            address_type: MarsAddressType::DualitySwapper,
        });

        let result = NeutronMsgFactory::swap_msg(
            &env,
            &default_swapper,
            &duality_swapper,
            coin_in.clone(),
            "uusdc",
            Uint128::new(950),
            route,
        );

        assert!(result.is_ok());
        let msg = result.unwrap();

        match msg {
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                funds,
                ..
            }) => {
                assert_eq!(contract_addr, "swapper_contract");
                assert_eq!(funds, vec![coin_in]);
            }
            _ => panic!("Expected WasmMsg::Execute"),
        }
    }

    #[test]
    fn test_neutron_swap_msg_no_duality_swapper() {
        let env = mock_env();
        let swapper_address = AddressResponseItem {
            address: "swapper_contract".to_string(),
            address_type: MarsAddressType::Swapper,
        };

        let coin_in = Coin {
            denom: "untrn".to_string(),
            amount: Uint128::new(1000),
        };

        let duality_route = Some(SwapperRoute::Duality(DualityRoute {
            from: "untrn".to_string(),
            to: "uusdc".to_string(),
            swap_denoms: vec!["untrn".to_string(), "uusdc".to_string()],
        }));

        let result = NeutronMsgFactory::swap_msg(
            &env,
            &swapper_address,
            &None,
            coin_in,
            "uusdc",
            Uint128::new(950),
            duality_route,
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            ContractError::NoSwapper {
                required,
            } => {
                assert_eq!(required, MarsAddressType::DualitySwapper.to_string());
            }
            _ => panic!("Expected NoSwapper error"),
        }
    }
}
