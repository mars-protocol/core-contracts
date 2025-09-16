use cosmwasm_std::{
    coin, from_json, testing::{mock_env, MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR}, Decimal, Deps, OwnedDeps
};

use mars_rewards_collector_neutron::entry;
use mars_testing::{mock_info, MarsMockQuerier};
use mars_types::{
    rewards_collector::{InstantiateMsg, QueryMsg, RewardConfig, TransferType},
};

pub fn mock_instantiate_msg() -> InstantiateMsg {
    InstantiateMsg {
        owner: "owner".to_string(),
        address_provider: "address_provider".to_string(),
        safety_tax_rate: Decimal::percent(25),
        revenue_share_tax_rate: Decimal::percent(10),
        safety_fund_config: RewardConfig {
            target_denom: "uusdc".to_string(),
            transfer_type: TransferType::Bank,
        },
        revenue_share_config: RewardConfig {
            target_denom: "uusdc".to_string(),
            transfer_type: TransferType::Bank,
        },
        fee_collector_config: RewardConfig {
            target_denom: "umars".to_string(),
            transfer_type: TransferType::Ibc,
        },
        channel_id: "channel-69".to_string(),
        timeout_seconds: 300,
        whitelisted_distributors: vec!["owner".to_string(), "jake".to_string()],
    }
}

pub fn setup_test_env() -> OwnedDeps<cosmwasm_std::MemoryStorage, MockApi, MarsMockQuerier> {

    let mut deps: OwnedDeps<cosmwasm_std::MemoryStorage, MockApi, MarsMockQuerier> =
    OwnedDeps::<_, _, _> {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: MarsMockQuerier::new(MockQuerier::new(&[(
            MOCK_CONTRACT_ADDR,
            &[coin(88888, "uatom"), coin(1234, "uusdc"), coin(8964, "umars")],
        )])),
        custom_query_type: Default::default(),
    };

    deps.querier.set_oracle_price("uatom", Decimal::one());
    
    let info = mock_info("deployer");
    let msg = mock_instantiate_msg();
    entry::instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    
    deps
}

pub fn query<T: serde::de::DeserializeOwned>(deps: Deps, msg: QueryMsg) -> T {
    from_json(entry::query(deps, mock_env(), msg).unwrap()).unwrap()
}
