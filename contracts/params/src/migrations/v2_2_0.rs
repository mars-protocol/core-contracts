use cosmwasm_std::{Addr, Decimal, DepsMut, Order, Response, StdResult};
use cw2::{assert_contract_version, set_contract_version};
use mars_owner::OwnerInit::SetInitialOwner;
use mars_types::params::{
    AssetParams, CmSettings, HlsAssetType, HlsParams, LiquidationBonus, MigrateMsg, RedBankSettings,
};

use crate::{
    contract::{CONTRACT_NAME, CONTRACT_VERSION},
    error::ContractError,
    state::{ASSET_PARAMS, OWNER, RISK_MANAGER},
};

const FROM_VERSION: &str = "2.1.0";

/// Copy paste of the state structs from the v2.1.0 of the contract (https://github.com/mars-protocol/contracts/tree/v2.1.0).
pub mod v2_1_0_state {
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::{Addr, Decimal, Uint128};
    use cw_storage_plus::Map;

    #[cw_serde]
    pub enum HlsAssetType<T> {
        Coin {
            denom: String,
        },
        Vault {
            addr: T,
        },
    }

    #[cw_serde]
    pub struct HlsParamsBase<T> {
        pub max_loan_to_value: Decimal,
        pub liquidation_threshold: Decimal,
        pub correlations: Vec<HlsAssetType<T>>,
    }

    pub type HlsParams = HlsParamsBase<Addr>;
    pub type HlsParamsUnchecked = HlsParamsBase<String>;

    #[cw_serde]
    pub struct CmSettings<T> {
        pub whitelisted: bool,
        pub hls: Option<HlsParamsBase<T>>,
    }

    #[cw_serde]
    pub struct RedBankSettings {
        pub deposit_enabled: bool,
        pub borrow_enabled: bool,
    }

    #[cw_serde]
    pub struct LiquidationBonus {
        pub starting_lb: Decimal,
        pub slope: Decimal,
        pub min_lb: Decimal,
        pub max_lb: Decimal,
    }

    #[cw_serde]
    pub struct AssetParamsBase<T> {
        pub denom: String,
        pub credit_manager: CmSettings<T>,
        pub red_bank: RedBankSettings,
        pub max_loan_to_value: Decimal,
        pub liquidation_threshold: Decimal,
        pub liquidation_bonus: LiquidationBonus,
        pub protocol_liquidation_fee: Decimal,
        pub deposit_cap: Uint128,
    }

    pub type AssetParams = AssetParamsBase<Addr>;
    pub type AssetParamsUnchecked = AssetParamsBase<String>;

    pub const ASSET_PARAMS: Map<&str, AssetParams> = Map::new("asset_params");
}

pub fn migrate(deps: DepsMut, msg: MigrateMsg) -> Result<Response, ContractError> {
    // Make sure we're migrating the correct contract and from the correct version.
    assert_contract_version(deps.storage, &format!("crates.io:{CONTRACT_NAME}"), FROM_VERSION)?;

    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;

    // Since version <= 2.1.0 of the contract didn't have the risk manager storage item, that is initialised in the instantiate function.
    // We need to initialise the risk manager to the default owner of the contract here in the migration.
    let owner = OWNER.query(deps.storage)?.owner.unwrap();
    RISK_MANAGER.initialize(
        deps.storage,
        deps.api,
        SetInitialOwner {
            owner,
        },
    )?;

    // Migrate assets
    let asset_params = v2_1_0_state::ASSET_PARAMS
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;
    v2_1_0_state::ASSET_PARAMS.clear(deps.storage);
    for (denom, asset_param) in asset_params.into_iter() {
        ASSET_PARAMS.save(
            deps.storage,
            &denom,
            &from_v2_1_0_to_v2_2_0_asset_param(asset_param, msg.close_factor),
        )?;
    }

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("from_version", FROM_VERSION)
        .add_attribute("to_version", CONTRACT_VERSION))
}

fn from_v2_1_0_to_v2_2_0_asset_param(
    value: v2_1_0_state::AssetParams,
    close_factor: Decimal,
) -> AssetParams {
    AssetParams {
        denom: value.denom,
        credit_manager: CmSettings {
            whitelisted: value.credit_manager.whitelisted,
            hls: value.credit_manager.hls.map(Into::into),
            withdraw_enabled: true, // New field
        },
        red_bank: RedBankSettings {
            deposit_enabled: value.red_bank.deposit_enabled,
            borrow_enabled: value.red_bank.borrow_enabled,
            withdraw_enabled: value.red_bank.deposit_enabled, // New field, make it dependent on deposit_enabled
        },
        max_loan_to_value: value.max_loan_to_value,
        liquidation_threshold: value.liquidation_threshold,
        liquidation_bonus: LiquidationBonus {
            starting_lb: value.liquidation_bonus.starting_lb,
            slope: value.liquidation_bonus.slope,
            min_lb: value.liquidation_bonus.min_lb,
            max_lb: value.liquidation_bonus.max_lb,
        },
        protocol_liquidation_fee: value.protocol_liquidation_fee,
        deposit_cap: value.deposit_cap,
        close_factor, // New field
    }
}

impl From<v2_1_0_state::HlsAssetType<Addr>> for HlsAssetType<Addr> {
    fn from(value: v2_1_0_state::HlsAssetType<Addr>) -> Self {
        match value {
            v2_1_0_state::HlsAssetType::Coin {
                denom,
            } => HlsAssetType::Coin {
                denom,
            },
            v2_1_0_state::HlsAssetType::Vault {
                addr,
            } => HlsAssetType::Vault {
                addr,
            },
        }
    }
}

impl From<v2_1_0_state::HlsParams> for HlsParams {
    fn from(value: v2_1_0_state::HlsParams) -> Self {
        Self {
            max_loan_to_value: value.max_loan_to_value,
            liquidation_threshold: value.liquidation_threshold,
            correlations: value.correlations.into_iter().map(Into::into).collect(),
        }
    }
}
