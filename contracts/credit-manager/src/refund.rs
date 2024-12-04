use cosmwasm_std::{to_json_binary, Addr, CosmosMsg, DepsMut, Env, Response, WasmMsg};
use mars_types::credit_manager::{ActionAmount, ActionCoin, CallbackMsg, ExecuteMsg};

use crate::{
    error::ContractResult,
    query::query_coin_balances,
    utils::{assert_withdraw_enabled, query_nft_token_owner},
};

pub fn refund_coin_balances(deps: DepsMut, env: Env, account_id: &str) -> ContractResult<Response> {
    let coins = query_coin_balances(deps.as_ref(), account_id)?;
    let account_nft_owner = query_nft_token_owner(deps.as_ref(), account_id)?;
    let mut frozen_coin_denoms = vec![];
    let withdraw_msgs = coins
        .into_iter()
        .filter(|coin| {
            if assert_withdraw_enabled(deps.storage, &deps.querier, &coin.denom).is_err() {
                frozen_coin_denoms.push(coin.denom.clone());
                return false;
            }
            true
        })
        .map(|coin| {
            let action_amount = ActionAmount::Exact(coin.amount);
            let action_coin = ActionCoin {
                denom: coin.denom,
                amount: action_amount,
            };
            Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                funds: vec![],
                msg: to_json_binary(&ExecuteMsg::Callback(CallbackMsg::Withdraw {
                    account_id: account_id.to_string(),
                    coin: action_coin,
                    recipient: Addr::unchecked(account_nft_owner.clone()),
                }))?,
            }))
        })
        .collect::<ContractResult<Vec<_>>>()?;

    let mut res = Response::new()
        .add_messages(withdraw_msgs)
        .add_attribute("action", "callback/refund_coin_balances")
        .add_attribute("account_id", account_id.to_string());

    if !frozen_coin_denoms.is_empty() {
        res = res.add_attribute("frozen_coins", frozen_coin_denoms.join(","))
    }

    Ok(res)
}
