use cosmwasm_std::{attr, testing::mock_env, Addr, Decimal, Event};
use cw2::{ContractVersion, VersionError};
use mars_rewards_collector_base::ContractError;
use mars_rewards_collector_neutron::{
    entry::{migrate, NeutronCollector},
    migrations::v2_2_2::previous_state,
};
use mars_testing::mock_dependencies;
use mars_types::rewards_collector::{NeutronMigrateMsg, TransferType};

#[test]
fn wrong_contract_name() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "contract_xyz", "2.2.0").unwrap();

    let err = migrate(deps.as_mut(), mock_env(), NeutronMigrateMsg::V2_2_0ToV2_2_2 {}).unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongContract {
            expected: "crates.io:mars-rewards-collector-neutron".to_string(),
            found: "contract_xyz".to_string()
        })
    );
}

#[test]
fn wrong_contract_version() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(
        deps.as_mut().storage,
        "crates.io:mars-rewards-collector-neutron",
        "4.1.0",
    )
    .unwrap();

    let err = migrate(deps.as_mut(), mock_env(), NeutronMigrateMsg::V2_2_0ToV2_2_2 {}).unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongVersion {
            expected: "2.2.0".to_string(),
            found: "4.1.0".to_string()
        })
    );
}

#[test]
fn successful_migration() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(
        deps.as_mut().storage,
        "crates.io:mars-rewards-collector-neutron",
        "2.2.0",
    )
    .unwrap();

    let old_config = previous_state::Config {
        address_provider: Addr::unchecked("address_provider"),
        safety_tax_rate: Decimal::percent(50),
        safety_fund_denom: "ibc/6F34E1BD664C36CE49ACC28E60D62559A5F96C4F9A6CCE4FC5A67B2852E24CFE"
            .to_string(),
        fee_collector_denom: "ibc/2E7368A14AC9AB7870F32CFEA687551C5064FA861868EDF7437BC877358A81F9"
            .to_string(),
        channel_id: "channel-2083".to_string(),
        timeout_seconds: 600,
        slippage_tolerance: Decimal::percent(1),
        neutron_ibc_config: None,
    };
    previous_state::CONFIG.save(deps.as_mut().storage, &old_config).unwrap();

    let res = migrate(deps.as_mut(), mock_env(), NeutronMigrateMsg::V2_2_0ToV2_2_2 {}).unwrap();

    assert_eq!(res.messages, vec![]);
    assert_eq!(res.events, vec![] as Vec<Event>);
    assert!(res.data.is_none());
    assert_eq!(
        res.attributes,
        vec![attr("action", "migrate"), attr("from_version", "2.2.0"), attr("to_version", "2.2.2")]
    );

    let new_contract_version = ContractVersion {
        contract: "crates.io:mars-rewards-collector-neutron".to_string(),
        version: "2.2.2".to_string(),
    };

    assert_eq!(cw2::get_contract_version(deps.as_ref().storage).unwrap(), new_contract_version);

    // ensure state is correct
    let collector = NeutronCollector::default();
    let updated_config = collector.config.load(deps.as_ref().storage).unwrap();

    assert_eq!(updated_config.channel_id, "".to_string());
    assert_eq!(updated_config.safety_tax_rate, Decimal::percent(45));
    assert_eq!(updated_config.revenue_share_tax_rate, Decimal::percent(10));
    assert_eq!(updated_config.safety_fund_config.target_denom, old_config.safety_fund_denom);
    assert_eq!(updated_config.safety_fund_config.transfer_type, TransferType::Bank);
    assert_eq!(updated_config.revenue_share_config.target_denom, old_config.safety_fund_denom);
    assert_eq!(updated_config.revenue_share_config.transfer_type, TransferType::Bank);
    assert_eq!(updated_config.fee_collector_config.target_denom, old_config.fee_collector_denom);
    assert_eq!(updated_config.fee_collector_config.transfer_type, TransferType::Bank);
}
