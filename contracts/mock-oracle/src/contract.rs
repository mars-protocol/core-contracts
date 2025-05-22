use std::collections::HashMap;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use mars_types::oracle::{ActionKind, HasPriceSourceResponse, PriceResponse};

use crate::{
    msg::{CoinPrice, ExecuteMsg, InstantiateMsg, QueryMsg},
    state::{DEFAULT_COIN_PRICE, LIQUIDATION_COIN_PRICE},
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    for item in msg.prices {
        DEFAULT_COIN_PRICE.save(deps.storage, item.denom.clone(), &item.price)?;
        LIQUIDATION_COIN_PRICE.save(deps.storage, item.denom, &item.price)?;
    }
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::ChangePrice(coin) => change_price(deps, coin),
        ExecuteMsg::RemovePrice {
            denom,
            pricing,
        } => remove_price(deps, denom, pricing),
    }
}

fn change_price(deps: DepsMut, coin: CoinPrice) -> StdResult<Response> {
    match coin.pricing {
        ActionKind::Default => {
            DEFAULT_COIN_PRICE.save(deps.storage, coin.denom, &coin.price)?;
        }
        ActionKind::Liquidation => {
            LIQUIDATION_COIN_PRICE.save(deps.storage, coin.denom, &coin.price)?
        }
    }
    Ok(Response::new())
}

fn remove_price(deps: DepsMut, denom: String, pricing: ActionKind) -> StdResult<Response> {
    match pricing {
        ActionKind::Default => DEFAULT_COIN_PRICE.remove(deps.storage, denom),
        ActionKind::Liquidation => LIQUIDATION_COIN_PRICE.remove(deps.storage, denom),
    }
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Price {
            denom,
            kind,
        } => to_json_binary(&query_price(deps, denom, kind)?),
        QueryMsg::PricesByDenoms {
            denoms,
            kind,
        } => to_json_binary(&query_prices_by_denoms(deps, denoms, kind)?),
        QueryMsg::HasPriceSource {
            denom,
        } => to_json_binary(&query_has_price_source(deps, denom)?),
    }
}

fn query_price(
    deps: Deps,
    denom: String,
    kind_opt: Option<ActionKind>,
) -> StdResult<PriceResponse> {
    let price = match kind_opt.unwrap_or(ActionKind::Default) {
        ActionKind::Default => DEFAULT_COIN_PRICE.load(deps.storage, denom.clone())?,
        ActionKind::Liquidation => LIQUIDATION_COIN_PRICE.load(deps.storage, denom.clone())?,
    };

    Ok(PriceResponse {
        denom,
        price,
    })
}

fn query_prices_by_denoms(
    deps: Deps,
    denoms: Vec<String>,
    kind_opt: Option<ActionKind>,
) -> StdResult<HashMap<String, Decimal>> {
    let mut prices: HashMap<String, Decimal> = HashMap::new();

    for denom in denoms {
        let price = match kind_opt.clone().unwrap_or(ActionKind::Default) {
            ActionKind::Default => DEFAULT_COIN_PRICE.load(deps.storage, denom.clone())?,
            ActionKind::Liquidation => LIQUIDATION_COIN_PRICE.load(deps.storage, denom.clone())?,
        };

        prices.insert(denom, price);
    }

    Ok(prices)
}

fn query_has_price_source(deps: Deps, denom: String) -> StdResult<HasPriceSourceResponse> {
    let default_price_source = DEFAULT_COIN_PRICE.may_load(deps.storage, denom.clone())?;
    let liquidation_price_source = LIQUIDATION_COIN_PRICE.may_load(deps.storage, denom.clone())?;
    let has_price_source = default_price_source.is_some() || liquidation_price_source.is_some();
    Ok(HasPriceSourceResponse {
        denom,
        has_price_source,
    })
}
