use cosmwasm_std::{from_json, testing::mock_env, Addr, Decimal};
use cw2::VersionError;
use mars_rewards_collector_base::ContractError;
use mars_rewards_collector_neutron::entry::{migrate, query, CONTRACT_NAME, CONTRACT_VERSION};
use mars_testing::mock_dependencies;
use mars_types::rewards_collector::{
    ConfigResponse, NeutronMigrateMsg, QueryMsg, RewardConfig, TransferType,
};

mod previous_state {
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::{Addr, Decimal};
    use cw_storage_plus::Item;
    use mars_types::rewards_collector::RewardConfig;

    #[cw_serde]
    pub struct Config {
        pub address_provider: Addr,
        pub safety_tax_rate: Decimal,
        pub revenue_share_tax_rate: Decimal,
        pub slippage_tolerance: Decimal,
        pub safety_fund_config: RewardConfig,
        pub revenue_share_config: RewardConfig,
        pub fee_collector_config: RewardConfig,
        pub channel_id: String,
        pub timeout_seconds: u64,
    }

    pub const CONFIG: Item<Config> = Item::new("config");
}

#[test]
fn test_successful_migration() {
    let mut deps = mock_dependencies(&[]);

    let old_config = previous_state::Config {
        address_provider: Addr::unchecked("address_provider"),
        safety_tax_rate: Decimal::percent(10),
        revenue_share_tax_rate: Decimal::percent(5),
        slippage_tolerance: Decimal::percent(1),
        safety_fund_config: RewardConfig {
            target_denom: "sf_denom".to_string(),
            transfer_type: TransferType::Bank,
        },
        revenue_share_config: RewardConfig {
            target_denom: "sf_denom".to_string(),
            transfer_type: TransferType::Bank,
        },
        fee_collector_config: RewardConfig {
            target_denom: "fc_denom".to_string(),
            transfer_type: TransferType::Bank,
        },
        channel_id: "channel-1".to_string(),
        timeout_seconds: 60,
    };

    previous_state::CONFIG.save(&mut deps.storage, &old_config).unwrap();

    // Set the contract version to the old version
    cw2::set_contract_version(&mut deps.storage, format!("crates.io:{}", CONTRACT_NAME), "2.2.2")
        .unwrap();

    // Perform the migration
    let msg = NeutronMigrateMsg::V2_2_2ToV2_3_1 {};
    let res = migrate(deps.as_mut(), mock_env(), msg).unwrap();

    // Check the response attributes
    assert_eq!(res.attributes[0].key, "action");
    assert_eq!(res.attributes[0].value, "migrate");
    assert_eq!(res.attributes[1].key, "from_version");
    assert_eq!(res.attributes[1].value, "2.2.2");
    assert_eq!(res.attributes[2].key, "to_version");
    assert_eq!(res.attributes[2].value, CONTRACT_VERSION);

    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_response: ConfigResponse = from_json(&query_res).unwrap();

    assert_eq!(config_response.address_provider, old_config.address_provider.to_string());
    assert_eq!(config_response.safety_tax_rate, old_config.safety_tax_rate);
    assert_eq!(config_response.revenue_share_tax_rate, old_config.revenue_share_tax_rate);
    assert_eq!(config_response.safety_fund_config, old_config.safety_fund_config);
    assert_eq!(config_response.revenue_share_config, old_config.revenue_share_config);
    assert_eq!(config_response.fee_collector_config, old_config.fee_collector_config);
    assert_eq!(config_response.channel_id, old_config.channel_id);
    assert_eq!(config_response.timeout_seconds, old_config.timeout_seconds);
    assert!(config_response.whitelisted_distributors.is_empty());

    // After migration, check that the contract version is updated
    let version = cw2::get_contract_version(&deps.storage).unwrap();
    assert_eq!(version.version, CONTRACT_VERSION);
    assert_eq!(version.contract, format!("crates.io:{}", CONTRACT_NAME));
}

#[test]
fn test_unsuccessful_migration_from_wrong_version() {
    let mut deps = mock_dependencies(&[]);

    // Set the contract version to a wrong version
    cw2::set_contract_version(&mut deps.storage, format!("crates.io:{}", CONTRACT_NAME), "2.0.0")
        .unwrap();

    // Perform the migration and expect an error
    let msg = NeutronMigrateMsg::V2_2_2ToV2_3_1 {};
    let err = migrate(deps.as_mut(), mock_env(), msg).unwrap_err();

    // Check that the error is a VersionError
    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongVersion {
            expected: "2.2.2".to_string(),
            found: "2.0.0".to_string(),
        })
    );
}

#[test]
fn test_unsuccessful_migration_from_wrong_contract_name() {
    let mut deps = mock_dependencies(&[]);

    // Set the contract version with a wrong contract name
    let wrong_contract_name = "wrong-contract-name";
    cw2::set_contract_version(&mut deps.storage, wrong_contract_name, "2.2.2").unwrap();

    // Perform the migration and expect an error
    let msg = NeutronMigrateMsg::V2_2_2ToV2_3_1 {};
    let err = migrate(deps.as_mut(), mock_env(), msg).unwrap_err();

    // Check that the error is a VersionError for wrong contract
    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongContract {
            expected: format!("crates.io:{}", CONTRACT_NAME),
            found: wrong_contract_name.to_string(),
        })
    );
}
