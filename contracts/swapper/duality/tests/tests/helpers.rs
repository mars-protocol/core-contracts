use mars_swapper_duality::DualityRoute;

/// helper function to create a simple direct route
pub fn create_direct_route(from: &str, to: &str) -> DualityRoute {
    DualityRoute {
        from: from.to_string(),
        to: to.to_string(),
        swap_denoms: vec![from.to_string(), to.to_string()],
    }
}

/// helper function to create a multi-hop route
pub fn create_multi_hop_route(from: &str, via: &[&str], to: &str) -> DualityRoute {
    let mut swap_denoms = vec![];
    swap_denoms.push(from.to_string());
    for denom in via {
        swap_denoms.push(denom.to_string());
    }
    swap_denoms.push(to.to_string());

    DualityRoute {
        from: from.to_string(),
        to: to.to_string(),
        swap_denoms,
    }
}
