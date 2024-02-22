use cosmwasm_std::{Decimal, Uint128};
use mars_owner::OwnerResponse;
use mars_types::{
    adapters::{oracle::OracleBase, params::ParamsBase},
    perps::Config,
};

use super::helpers::MockEnv;

#[test]
fn proper_initialization() {
    let mock = MockEnv::new()
        .perps_base_denom("uusdc")
        .min_position_value(Uint128::new(5_000_000))
        .max_position_value(Some(Uint128::new(6_000_000)))
        .cooldown_period(3688)
        .opening_fee_rate(Decimal::from_ratio(12u128, 100u128))
        .closing_fee_rate(Decimal::from_ratio(16u128, 100u128))
        .build()
        .unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let oracle = mock.oracle.clone();
    let params = mock.params.clone();

    let owner_res = mock.query_ownership();
    assert_eq!(
        owner_res,
        OwnerResponse {
            owner: Some(owner.into()),
            proposed: None,
            initialized: true,
            abolished: false,
            emergency_owner: None,
        },
    );

    let config = mock.query_config();
    assert_eq!(
        config,
        Config {
            credit_manager,
            oracle: OracleBase::new(oracle),
            params: ParamsBase::new(params),
            base_denom: "uusdc".to_string(),
            min_position_value: Uint128::new(5_000_000),
            max_position_value: Some(Uint128::new(6_000_000)),
            cooldown_period: 3688,
            opening_fee_rate: Decimal::from_ratio(12u128, 100u128),
            closing_fee_rate: Decimal::from_ratio(16u128, 100u128)
        }
    );
}
