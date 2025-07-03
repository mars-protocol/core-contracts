use cosmwasm_std::{Decimal, DepsMut, Order, Response, StdResult};
use cw2::{assert_contract_version, set_contract_version};
use mars_owner::OwnerInit::SetInitialOwner;
use mars_types::{
    address_provider::{self, MarsAddressType},
    params::AssetParams,
    red_bank::{self, InterestRateModel, Market},
};

use crate::{
    contract::{CONTRACT_NAME, CONTRACT_VERSION},
    error::ContractError,
    state::{ADDRESS_PROVIDER, ASSET_PARAMS, OWNER, RISK_MANAGER},
};

const FROM_VERSION: &str = "2.2.0";

/// Copy paste of the state structs from the v2.2.0 of the contract (https://github.com/mars-protocol/core-contracts/releases/tag/v2.2.0-perps).
pub mod v2_2_0_state {
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::{Addr, Decimal, Uint128};
    use cw_storage_plus::Map;
    use mars_types::params::{CmSettings, LiquidationBonus, RedBankSettings};

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
        pub close_factor: Decimal,
    }

    pub type AssetParams = AssetParamsBase<Addr>;

    pub const ASSET_PARAMS: Map<&str, AssetParams> = Map::new("asset_params");
}

pub fn migrate(
    deps: DepsMut,
    reserve_factor: Decimal,
    interest_rate_model: InterestRateModel,
) -> Result<Response, ContractError> {
    // Make sure we're migrating the correct contract and from the correct version.
    assert_contract_version(deps.storage, &format!("crates.io:{CONTRACT_NAME}"), FROM_VERSION)?;

    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;

    // Since version <= 2.2.0 of the contract didn't have the risk manager storage item, that is initialised in the instantiate function.
    // We need to initialise the risk manager to the default owner of the contract here in the migration.
    let owner = OWNER.query(deps.storage)?.owner.unwrap();
    RISK_MANAGER.initialize(
        deps.storage,
        deps.api,
        SetInitialOwner {
            owner,
        },
    )?;

    // Get the address of the Red Bank contract
    let ap_addr = ADDRESS_PROVIDER.load(deps.storage)?;
    let rb_addr = address_provider::helpers::query_contract_addr(
        deps.as_ref(),
        &ap_addr,
        MarsAddressType::RedBank,
    )?;

    // Migrate assets
    let asset_params = v2_2_0_state::ASSET_PARAMS
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;
    v2_2_0_state::ASSET_PARAMS.clear(deps.storage);
    for (denom, asset_param) in asset_params.into_iter() {
        // Query the market to get the reserve factor and interest rate model
        let rb_market_opt = deps.querier.query_wasm_smart::<Option<Market>>(
            rb_addr.clone(),
            &red_bank::QueryMsg::Market {
                denom: denom.clone(),
            },
        )?;
        let (reserve_factor, interest_rate_model) = match rb_market_opt {
            Some(market) => (market.reserve_factor, market.interest_rate_model),
            None => {
                // If the market doesn't exist, use the default values
                (reserve_factor, interest_rate_model.clone())
            }
        };

        ASSET_PARAMS.save(
            deps.storage,
            &denom,
            &from_v2_2_0_to_v2_2_1_asset_param(asset_param, reserve_factor, interest_rate_model),
        )?;

        // We don't have to initialize the market in the Red Bank contract now in the migration process, as the market will be created when it will be needed (for example if we change red bank settings)
    }

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("from_version", FROM_VERSION)
        .add_attribute("to_version", CONTRACT_VERSION))
}

fn from_v2_2_0_to_v2_2_1_asset_param(
    value: v2_2_0_state::AssetParams,
    reserve_factor: Decimal,
    interest_rate_model: InterestRateModel,
) -> AssetParams {
    AssetParams {
        denom: value.denom.clone(),
        credit_manager: value.credit_manager,
        red_bank: value.red_bank,
        max_loan_to_value: value.max_loan_to_value,
        liquidation_threshold: value.liquidation_threshold,
        liquidation_bonus: value.liquidation_bonus,
        protocol_liquidation_fee: value.protocol_liquidation_fee,
        deposit_cap: value.deposit_cap,
        close_factor: value.close_factor,
        reserve_factor,
        interest_rate_model,
    }
}
