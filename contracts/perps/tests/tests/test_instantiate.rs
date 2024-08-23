use cosmwasm_std::Decimal;
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
        .cooldown_period(3688)
        .max_positions(9)
        .protocol_fee_rate(Decimal::percent(25))
        .build()
        .unwrap();

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();
    let address_provider = mock.address_provider.clone();
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
            address_provider,
            credit_manager,
            oracle: OracleBase::new(oracle),
            params: ParamsBase::new(params),
            base_denom: "uusdc".to_string(),
            cooldown_period: 3688,
            max_positions: 9,
            protocol_fee_rate: Decimal::percent(25),
            target_vault_collaterization_ratio: Decimal::percent(125)
        }
    );
}
