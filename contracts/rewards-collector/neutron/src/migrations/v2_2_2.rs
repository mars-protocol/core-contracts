use cosmwasm_std::{Decimal, DepsMut, Response, Storage};
use cw2::{assert_contract_version, set_contract_version};
use mars_rewards_collector_base::ContractError;
use mars_types::rewards_collector::{Config, RewardConfig, TransferType};

use crate::entry::{NeutronCollector, CONTRACT_NAME, CONTRACT_VERSION};

pub mod previous_state {
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::{Addr, Coin, Decimal};
    use cw_storage_plus::Item;

    pub const CONFIG: Item<Config> = Item::new("config");

    #[cw_serde]
    pub struct Config {
        /// Address provider returns addresses for all protocol contracts
        pub address_provider: Addr,
        /// Percentage of fees that are sent to the safety fund
        pub safety_tax_rate: Decimal,
        /// The asset to which the safety fund share is converted
        pub safety_fund_denom: String,
        /// The asset to which the fee collector share is converted
        pub fee_collector_denom: String,
        /// The channel ID of the mars hub
        pub channel_id: String,
        /// Number of seconds after which an IBC transfer is to be considered failed, if no acknowledgement is received
        pub timeout_seconds: u64,
        /// Maximum percentage of price movement (minimum amount you accept to receive during swap)
        pub slippage_tolerance: Decimal,
        /// Neutron IBC config
        pub neutron_ibc_config: Option<NeutronIbcConfig>,
    }

    #[cw_serde]
    pub struct NeutronIbcConfig {
        pub source_port: String,
        pub acc_fee: Vec<Coin>,
        pub timeout_fee: Vec<Coin>,
    }
}

const FROM_VERSION: &str = "2.2.0";

pub fn migrate(deps: DepsMut) -> Result<Response, ContractError> {
    let storage: &mut dyn Storage = deps.storage;
    let collector = NeutronCollector::default();

    // make sure we're migrating the correct contract and from the correct version
    assert_contract_version(storage, &format!("crates.io:{CONTRACT_NAME}"), FROM_VERSION)?;
    // Load the existing config
    let existing_config = previous_state::CONFIG.load(storage)?;

    previous_state::CONFIG.remove(storage);

    let new_config = Config {
        // old, unchanged values
        address_provider: existing_config.address_provider,
        timeout_seconds: existing_config.timeout_seconds,

        // set as empty so any ibc transfers error. This prevents mistakenly sending funds somewhere
        channel_id: "".to_string(),

        // update tax_rate to account for the new revenue share
        // breakdown is now 45% safety fund, 10% revenue share, remaining 45% fee collector
        safety_tax_rate: Decimal::percent(45),
        revenue_share_tax_rate: Decimal::percent(10), // New revenue share tax rate

        // safety fund set to same denom as before. Bank transfer, not IBC
        safety_fund_config: RewardConfig {
            target_denom: existing_config.safety_fund_denom.clone(),
            transfer_type: TransferType::Bank,
        },

        // revenue share set to same denom as safety fund. Bank transfer, not IBC
        revenue_share_config: RewardConfig {
            target_denom: existing_config.safety_fund_denom,
            transfer_type: TransferType::Bank,
        },

        // fee collector set to same denom as before. Bank transfer, not IBC
        fee_collector_config: RewardConfig {
            target_denom: existing_config.fee_collector_denom,
            transfer_type: TransferType::Bank,
        },
    };

    // ensure our new config is legal
    new_config.validate()?;

    // save our updated
    collector.config.save(storage, &new_config)?;

    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("from_version", FROM_VERSION)
        .add_attribute("to_version", CONTRACT_VERSION))
}
