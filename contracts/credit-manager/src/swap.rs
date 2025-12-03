use cosmwasm_std::{Coin, Decimal, DepsMut, Env, Response, Uint128};
use mars_types::{
    credit_manager::{ActionAmount, ActionCoin, ChangeExpected},
    swapper::SwapperRoute,
};

use crate::{
    error::{ContractError, ContractResult},
    staking::get_account_tier_and_discount,
    state::{COIN_BALANCES, DUALITY_SWAPPER, REWARDS_COLLECTOR, SWAPPER, SWAP_FEE},
    utils::{
        assert_withdraw_enabled, decrement_coin_balance, increment_coin_balance, update_balance_msg,
    },
};

pub fn swap_exact_in(
    deps: DepsMut,
    env: Env,
    account_id: &str,
    coin_in: &ActionCoin,
    denom_out: &str,
    min_receive: Uint128,
    route: Option<SwapperRoute>,
) -> ContractResult<Response> {
    // Prevent swapping the asset if withdraw is disabled
    assert_withdraw_enabled(deps.storage, &deps.querier, &coin_in.denom)?;

    let mut coin_in_to_trade = Coin {
        denom: coin_in.denom.clone(),
        amount: match coin_in.amount {
            ActionAmount::Exact(a) => a,
            ActionAmount::AccountBalance => COIN_BALANCES
                .may_load(deps.storage, (account_id, &coin_in.denom))?
                .unwrap_or(Uint128::zero()),
        },
    };

    if coin_in_to_trade.amount.is_zero() {
        return Err(ContractError::NoAmount);
    }

    decrement_coin_balance(deps.storage, account_id, &coin_in_to_trade)?;

    // Get staking tier discount for this account
    let (tier, discount_pct, voting_power) =
        get_account_tier_and_discount(deps.as_ref(), account_id)?;

    // Apply discount to swap fee
    let base_swap_fee = SWAP_FEE.load(deps.storage)?;
    let effective_swap_fee =
        base_swap_fee.checked_mul(Decimal::one().checked_sub(discount_pct)?)?;
    let swap_fee_amount = coin_in_to_trade.amount.checked_mul_floor(effective_swap_fee)?;
    coin_in_to_trade.amount = coin_in_to_trade.amount.checked_sub(swap_fee_amount)?;

    // Send to Rewards collector
    let rc_coin = Coin {
        denom: coin_in.denom.clone(),
        amount: swap_fee_amount,
    };
    let rewards_collector_account = REWARDS_COLLECTOR.load(deps.storage)?.account_id;
    increment_coin_balance(deps.storage, &rewards_collector_account, &rc_coin)?;

    // Updates coin balances for account after the swap has taken place
    let update_coin_balance_msg = update_balance_msg(
        &deps.querier,
        &env.contract.address,
        account_id,
        denom_out,
        ChangeExpected::Increase,
    )?;

    // If this is a duality specific route, use the duality swapper, otherwise use the default swapper
    let swapper = match route {
        Some(SwapperRoute::Duality(_)) => DUALITY_SWAPPER.load(deps.storage)?,
        _ => SWAPPER.load(deps.storage)?,
    };

    Ok(Response::new()
        .add_message(swapper.swap_exact_in_msg(&coin_in_to_trade, denom_out, min_receive, route)?)
        .add_message(update_coin_balance_msg)
        .add_attribute("action", "swapper")
        .add_attribute("account_id", account_id)
        .add_attribute("coin_in", coin_in_to_trade.to_string())
        .add_attribute("denom_out", denom_out)
        .add_attribute("rewards_collector", rewards_collector_account)
        .add_attribute("rewards_collector_fee", rc_coin.to_string())
        .add_attribute("voting_power", voting_power.to_string())
        .add_attribute("tier_id", tier.id)
        .add_attribute("discount_pct", discount_pct.to_string())
        .add_attribute("base_swap_fee", base_swap_fee.to_string())
        .add_attribute("effective_swap_fee", effective_swap_fee.to_string()))
}
