use cosmwasm_std::Uint128;
use mars_types::params::PerpParams;

pub fn default_perp_params(denom: &str) -> PerpParams {
    PerpParams {
        denom: denom.to_string(),
        max_net_oi_value: Uint128::new(1_000_000_000),
        max_long_oi_value: Uint128::new(1_000_000_000),
        max_short_oi_value: Uint128::new(1_000_000_000),
    }
}
