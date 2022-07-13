#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use mars_outpost::error::MarsError;
use osmo_bindings::OsmosisQuery;

use mars_outpost::asset::Asset;
use mars_outpost::helpers::option_string_to_addr;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{CONFIG, PRICE_SOURCES};
use crate::{Config, PriceSourceChecked, PriceSourceUnchecked};

use self::helpers::*;

// INIT

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut<OsmosisQuery>,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let config = Config {
        owner: deps.api.addr_validate(&msg.owner)?,
    };
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::default())
}

// HANDLERS

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut<OsmosisQuery>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig { owner } => execute_update_config(deps, env, info, owner),
        ExecuteMsg::SetAsset {
            asset,
            price_source,
        } => execute_set_asset(deps, env, info, asset, price_source),
    }
}

pub fn execute_update_config(
    deps: DepsMut<OsmosisQuery>,
    _env: Env,
    info: MessageInfo,
    owner: Option<String>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner {
        return Err(MarsError::Unauthorized {}.into());
    };

    config.owner = option_string_to_addr(deps.api, owner, config.owner)?;

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

pub fn execute_set_asset(
    deps: DepsMut<OsmosisQuery>,
    _env: Env,
    info: MessageInfo,
    asset: Asset,
    price_source_unchecked: PriceSourceUnchecked,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner {
        return Err(MarsError::Unauthorized {}.into());
    }

    let (asset_label, asset_reference, _) = asset.get_attributes();
    let price_source = price_source_unchecked.to_checked(deps.api)?;
    PRICE_SOURCES.save(deps.storage, &asset_reference, &price_source)?;

    // for spot we must make sure: the osmosis pool indicated by `pool_id`
    // consists of OSMO and the asset of interest
    if let PriceSourceChecked::OsmosisSpot { pool_id } = &price_source {
        assert_osmosis_pool_assets(deps.as_ref(), &asset, *pool_id)?;
    }

    Ok(Response::new()
        .add_attribute("action", "set_asset")
        .add_attribute("asset", asset_label)
        .add_attribute("price_source", price_source_unchecked.to_string()))
}

// QUERIES

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<OsmosisQuery>, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps, env)?),
        QueryMsg::AssetPriceSource { asset } => {
            to_binary(&query_asset_price_source(deps, env, asset)?)
        }
        QueryMsg::AssetPrice { asset } => {
            to_binary(&query_asset_price(deps, env, asset.get_reference())?)
        }
        QueryMsg::AssetPriceByReference { asset_reference } => {
            to_binary(&query_asset_price(deps, env, asset_reference)?)
        }
    }
}

fn query_config(deps: Deps<OsmosisQuery>, _env: Env) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}

fn query_asset_price_source(
    deps: Deps<OsmosisQuery>,
    _env: Env,
    asset: Asset,
) -> StdResult<PriceSourceChecked> {
    PRICE_SOURCES.load(deps.storage, &asset.get_reference())
}

fn query_asset_price(
    deps: Deps<OsmosisQuery>,
    env: Env,
    asset_reference: Vec<u8>,
) -> Result<Decimal, ContractError> {
    let price_source = PRICE_SOURCES.load(deps.storage, &asset_reference)?;

    match price_source {
        PriceSourceChecked::Fixed { price } => Ok(price),

        PriceSourceChecked::OsmosisSpot { pool_id } => query_osmosis_spot_price(deps, pool_id),

        // The value of each unit of the liquidity token is the total value of pool's two assets
        // divided by the liquidity token's total supply
        //
        // NOTE: Price sources must exist for both assets in the pool
        PriceSourceChecked::OsmosisLiquidityToken { pool_id } => {
            let pool = query_osmosis_pool(deps, pool_id)?;

            let asset0: Asset = (&pool.assets[0]).into();
            let asset0_price = query_asset_price(deps, env.clone(), asset0.get_reference())?;
            let asset0_value = asset0_price * pool.assets[0].amount;

            let asset1: Asset = (&pool.assets[1]).into();
            let asset1_price = query_asset_price(deps, env, asset1.get_reference())?;
            let asset1_value = asset1_price * pool.assets[1].amount;

            let price = Decimal::from_ratio(asset0_value + asset1_value, pool.shares.amount);
            Ok(price)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}

// HELPERS

mod helpers {
    use cosmwasm_std::{Decimal, Deps, QueryRequest, StdResult};
    use osmo_bindings::{OsmosisQuery, PoolStateResponse, SpotPriceResponse};

    use mars_outpost::asset::Asset;

    use crate::error::ContractError;

    pub fn uosmo() -> Asset {
        Asset::Native {
            denom: "uosmo".to_string(),
        }
    }

    /// Assert the osmosis pool indicated by `pool_id` consists of OSMO and `asset`
    pub fn assert_osmosis_pool_assets(
        deps: Deps<OsmosisQuery>,
        asset: &Asset,
        pool_id: u64,
    ) -> Result<(), ContractError> {
        let pool = query_osmosis_pool(deps, pool_id)?;
        let asset0: Asset = (&pool.assets[0]).into();
        let asset1: Asset = (&pool.assets[1]).into();

        if (asset0 == uosmo() && &asset1 == asset) || (asset1 == uosmo() && &asset0 == asset) {
            Ok(())
        } else {
            Err(ContractError::InvalidPoolId {})
        }
    }

    pub fn query_osmosis_pool(
        deps: Deps<OsmosisQuery>,
        pool_id: u64,
    ) -> StdResult<PoolStateResponse> {
        let pool_query = OsmosisQuery::PoolState { id: pool_id };
        let query = QueryRequest::from(pool_query);
        let pool_info: PoolStateResponse = deps.querier.query(&query)?;
        Ok(pool_info)
    }

    pub fn query_osmosis_spot_price(
        deps: Deps<OsmosisQuery>,
        pool_id: u64,
    ) -> Result<Decimal, ContractError> {
        let pool_info = query_osmosis_pool(deps, pool_id)?; // FIXME: we can take asset from price sources (one of the asset is osmo)
        let asset_1 = &pool_info.assets[0];
        let asset_2 = &pool_info.assets[1];
        let spot_price =
            OsmosisQuery::spot_price(pool_id, asset_1.denom.as_str(), asset_2.denom.as_str());
        let query = QueryRequest::from(spot_price);
        let response: SpotPriceResponse = deps.querier.query(&query)?;
        Ok(response.price)
    }
}

// TESTS

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{
        mock_env, mock_info, MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR,
    };
    use cosmwasm_std::{from_binary, Addr, Coin, Decimal, OwnedDeps, Uint128};
    use mars_outpost::testing::MarsMockQuerier;
    use osmo_bindings::PoolStateResponse;
    use std::marker::PhantomData;

    #[test]
    fn test_proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            owner: String::from("owner"),
        };
        let info = mock_info("owner", &[]);

        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let config = CONFIG.load(&deps.storage).unwrap();
        assert_eq!(Addr::unchecked("owner"), config.owner);
    }

    #[test]
    fn test_update_config() {
        let mut deps = th_setup();

        // only owner can update
        {
            let msg = ExecuteMsg::UpdateConfig {
                owner: Some(String::from("new_owner")),
            };
            let info = mock_info("another_one", &[]);
            let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
            assert_eq!(err, MarsError::Unauthorized {}.into());
        }

        let info = mock_info("owner", &[]);
        // no change
        {
            let msg = ExecuteMsg::UpdateConfig { owner: None };
            execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

            let config = CONFIG.load(&deps.storage).unwrap();
            assert_eq!(config.owner, Addr::unchecked("owner"));
        }

        // new owner
        {
            let msg = ExecuteMsg::UpdateConfig {
                owner: Some(String::from("new_owner")),
            };
            execute(deps.as_mut(), mock_env(), info, msg).unwrap();

            let config = CONFIG.load(&deps.storage).unwrap();
            assert_eq!(config.owner, Addr::unchecked("new_owner"));
        }
    }

    #[test]
    fn test_set_asset_fixed() {
        let mut deps = th_setup();
        let info = mock_info("owner", &[]);

        let asset = Asset::Cw20 {
            contract_addr: String::from("cw20token"),
        };
        let reference = asset.get_reference();
        let msg = ExecuteMsg::SetAsset {
            asset,
            price_source: PriceSourceUnchecked::Fixed {
                price: Decimal::from_ratio(1_u128, 2_u128),
            },
        };
        execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let price_source = PRICE_SOURCES
            .load(&deps.storage, reference.as_slice())
            .unwrap();
        assert_eq!(
            price_source,
            PriceSourceChecked::Fixed {
                price: Decimal::from_ratio(1_u128, 2_u128)
            }
        );
    }

    #[test]
    fn test_set_asset_osmosis_spot() {
        let mut deps = th_setup();
        let info = mock_info("owner", &[]);

        deps.querier.set_pool_state(
            102,
            PoolStateResponse {
                assets: vec![
                    Coin {
                        denom: "sometoken".to_string(),
                        amount: Uint128::zero(),
                    },
                    Coin {
                        denom: "uosmo".to_string(),
                        amount: Uint128::zero(),
                    },
                ],
                shares: Coin {
                    denom: "lpsometoken".to_string(),
                    amount: Uint128::zero(),
                },
            },
        );

        let asset = Asset::Native {
            denom: "sometoken".to_string(),
        };
        let reference = asset.get_reference();
        let msg = ExecuteMsg::SetAsset {
            asset,
            price_source: PriceSourceUnchecked::OsmosisSpot { pool_id: 102 },
        };
        execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let price_source = PRICE_SOURCES
            .load(&deps.storage, reference.as_slice())
            .unwrap();
        assert_eq!(
            price_source,
            PriceSourceUnchecked::OsmosisSpot { pool_id: 102 }
        );
    }

    #[test]
    fn test_query_asset_price_source() {
        let mut deps = th_setup();
        let info = mock_info("owner", &[]);

        let asset = Asset::Cw20 {
            contract_addr: String::from("cw20token"),
        };

        let msg = ExecuteMsg::SetAsset {
            asset: asset.clone(),
            price_source: PriceSourceUnchecked::Fixed {
                price: Decimal::from_ratio(1_u128, 2_u128),
            },
        };

        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let price_source: PriceSourceChecked = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::AssetPriceSource { asset },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            price_source,
            PriceSourceChecked::Fixed {
                price: Decimal::from_ratio(1_u128, 2_u128),
            },
        );
    }

    #[test]
    fn test_query_asset_price_fixed() {
        let mut deps = th_setup();
        let asset = Asset::Cw20 {
            contract_addr: String::from("cw20token"),
        };
        let asset_reference = asset.get_reference();

        PRICE_SOURCES
            .save(
                &mut deps.storage,
                asset_reference.as_slice(),
                &PriceSourceChecked::Fixed {
                    price: Decimal::from_ratio(3_u128, 2_u128),
                },
            )
            .unwrap();

        let price: Decimal = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::AssetPriceByReference { asset_reference },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(price, Decimal::from_ratio(3_u128, 2_u128));
    }

    // TEST_HELPERS
    fn th_setup() -> OwnedDeps<MockStorage, MockApi, MarsMockQuerier, OsmosisQuery> {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            owner: String::from("owner"),
        };
        let info = mock_info("owner", &[]);
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        deps
    }

    fn mock_dependencies(
        contract_balance: &[Coin],
    ) -> OwnedDeps<MockStorage, MockApi, MarsMockQuerier, OsmosisQuery> {
        let contract_addr = Addr::unchecked(MOCK_CONTRACT_ADDR);
        let custom_querier: MarsMockQuerier = MarsMockQuerier::new(MockQuerier::new(&[(
            &contract_addr.to_string(),
            contract_balance,
        )]));

        OwnedDeps::<_, _, _, OsmosisQuery> {
            storage: MockStorage::default(),
            api: MockApi::default(),
            querier: custom_querier,
            custom_query_type: PhantomData,
        }
    }
}
