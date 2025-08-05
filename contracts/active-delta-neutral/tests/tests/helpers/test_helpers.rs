use mars_active_delta_neutral::helpers::validate_swapper_route;
use mars_types::swapper::{AstroRoute, AstroSwap, SwapperRoute};
use test_case::test_case;
#[test_case(
    SwapperRoute::Astro(AstroRoute {
        swaps: vec![AstroSwap {
            from: "uusd".to_string(),
            to: "atom".to_string()
        }]
    }),
    "uusd",
    "atom",
    true;
    "valid_astro_swap"
)]
#[test_case(
    SwapperRoute::Astro(AstroRoute {
        swaps: vec![AstroSwap {
            from: "atom".to_string(),
            to: "uusd".to_string()
        }]
    }),
    "uusd",
    "atom",
    true;
    "valid_astro_swap_reversed"
)]
#[test_case(
    SwapperRoute::Astro(AstroRoute {
        swaps: vec![]
    }),
    "uusd",
    "atom",
    false;
    "astro_route_empty_swaps"
)]
#[test_case(
    SwapperRoute::Astro(AstroRoute {
        swaps: vec![AstroSwap {
            from: "luna".to_string(),
            to: "atom".to_string()
        }]
    }),
    "uusd",
    "atom",
    false;
    "invalid_from_token"
)]
#[test_case(
    SwapperRoute::Astro(AstroRoute {
        swaps: vec![AstroSwap {
            from: "uusd".to_string(),
            to: "luna".to_string()
        }]
    }),
    "uusd",
    "atom",
    false;
    "invalid_to_token"
)]
fn test_validate_swapper_route(
    route: SwapperRoute,
    denom_in: &str,
    denom_out: &str,
    should_pass: bool,
) {
    let result = std::panic::catch_unwind(|| validate_swapper_route(&route, denom_in, denom_out));

    assert_eq!(result.is_ok(), should_pass);
}
