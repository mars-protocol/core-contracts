use cosmwasm_std::{Coin, Uint128};
use mars_types::{
    credit_manager::{Action as CreditAction, ActionAmount, ActionCoin},
    swapper::SwapperRoute,
};

/// Constructs a sequence of credit manager actions for a buy or sell operation.
///
/// This helper will:
/// - Check if the `amount` to trade exceeds `available_balance`.
/// - If so, add a borrow action for the difference.
/// - Always add a swap action for the full `amount`.
///
/// # Parameters
/// - `amount`: The amount to buy or sell.
/// - `available_balance`: The current balance of the asset to use for the trade.
/// - `asset_in_denom`: The denomination of the asset being sold (for buy: stable, for sell: volatile).
/// - `asset_out_denom`: The denomination to receive in the swap (for buy: volatile, for sell: perp).
/// - `swapper_route`: The route for the swap.
///
/// # Returns
/// - `Vec<CreditAction>`: The sequence of actions to perform.
///
pub fn build_trade_actions(
    amount: Uint128,
    available_balance: Uint128,
    asset_in_denom: &str,
    asset_out_denom: &str,
    swapper_route: &SwapperRoute,
) -> Vec<CreditAction> {
    let mut actions = Vec::new();
    let additional_debt = if amount > available_balance {
        amount.checked_sub(available_balance).unwrap_or(Uint128::zero())
    } else {
        Uint128::zero()
    };
    if additional_debt > Uint128::zero() {
        actions.push(CreditAction::Borrow(Coin {
            amount: additional_debt,
            denom: asset_in_denom.to_string(),
        }));
    }
    actions.push(CreditAction::SwapExactIn {
        coin_in: ActionCoin {
            amount: ActionAmount::Exact(amount),
            denom: asset_in_denom.to_string(),
        },
        denom_out: asset_out_denom.to_string(),
        min_receive: Uint128::new(100), // todo
        route: Some(swapper_route.clone()),
    });
    actions
}
