use cosmwasm_std::{Coin, Uint128};
use mars_active_delta_neutral::order_creation::build_trade_actions;
use mars_types::{
    credit_manager::{Action as CreditAction, ActionAmount, ActionCoin},
    swapper::{DualityRoute, SwapperRoute},
};
use test_case::test_case;

const ASSET_IN_DENOM: &str = "asset_in";
const ASSET_OUT_DENOM: &str = "asset_out";

#[test_case(
    100,
    50,
    vec![
        CreditAction::Borrow(Coin {
            amount: Uint128::from(50u128),
            denom: ASSET_IN_DENOM.to_string(),
        }),
        CreditAction::SwapExactIn {
            coin_in: ActionCoin {
                amount: ActionAmount::Exact(Uint128::from(100u128)),
                denom: ASSET_IN_DENOM.to_string(),
            },
            denom_out: ASSET_OUT_DENOM.to_string(),
            min_receive: Uint128::zero(),
            route: Some(SwapperRoute::Duality(DualityRoute {
                from: ASSET_IN_DENOM.to_string(),
                to: ASSET_OUT_DENOM.to_string(),
                swap_denoms: vec![ASSET_IN_DENOM.to_string(), ASSET_OUT_DENOM.to_string()],
            })),
        },
    ],
    "Borrow is required"
)]
#[test_case(
    50,
    100,
    vec![
        CreditAction::SwapExactIn {
            coin_in: ActionCoin {
                amount: ActionAmount::Exact(Uint128::from(50u128)),
                denom: ASSET_IN_DENOM.to_string(),
            },
            denom_out: ASSET_OUT_DENOM.to_string(),
            min_receive: Uint128::zero(),
            route: Some(SwapperRoute::Duality(DualityRoute {
                from: ASSET_IN_DENOM.to_string(),
                to: ASSET_OUT_DENOM.to_string(),
                swap_denoms: vec![ASSET_IN_DENOM.to_string(), ASSET_OUT_DENOM.to_string()],
            })),
        },
    ],
    "No borrow is required"
)]
#[test_case(
    100,
    100,
    vec![
        CreditAction::SwapExactIn {
            coin_in: ActionCoin {
                amount: ActionAmount::Exact(Uint128::from(100u128)),
                denom: ASSET_IN_DENOM.to_string(),
            },
            denom_out: ASSET_OUT_DENOM.to_string(),
            min_receive: Uint128::zero(),
            route: Some(SwapperRoute::Duality(DualityRoute {
                from: ASSET_IN_DENOM.to_string(),
                to: ASSET_OUT_DENOM.to_string(),
                swap_denoms: vec![ASSET_IN_DENOM.to_string(), ASSET_OUT_DENOM.to_string()],
            })),
        },
    ],
    "No borrow is required when amount equals balance"
)]
#[test_case(
    0,
    100,
    vec![
        CreditAction::SwapExactIn {
            coin_in: ActionCoin {
                amount: ActionAmount::Exact(Uint128::from(0u128)),
                denom: ASSET_IN_DENOM.to_string(),
            },
            denom_out: ASSET_OUT_DENOM.to_string(),
            min_receive: Uint128::zero(),
            route: Some(SwapperRoute::Duality(DualityRoute {
                from: ASSET_IN_DENOM.to_string(),
                to: ASSET_OUT_DENOM.to_string(),
                swap_denoms: vec![ASSET_IN_DENOM.to_string(), ASSET_OUT_DENOM.to_string()],
            })),
        },
    ],
    "Zero amount"
)]
#[test_case(
    100,
    0,
    vec![
        CreditAction::Borrow(Coin {
            amount: Uint128::from(100u128),
            denom: ASSET_IN_DENOM.to_string(),
        }),
        CreditAction::SwapExactIn {
            coin_in: ActionCoin {
                amount: ActionAmount::Exact(Uint128::from(100u128)),
                denom: ASSET_IN_DENOM.to_string(),
            },
            denom_out: ASSET_OUT_DENOM.to_string(),
            min_receive: Uint128::zero(),
            route: Some(SwapperRoute::Duality(DualityRoute {
                from: ASSET_IN_DENOM.to_string(),
                to: ASSET_OUT_DENOM.to_string(),
                swap_denoms: vec![ASSET_IN_DENOM.to_string(), ASSET_OUT_DENOM.to_string()],
            })),
        },
    ],
    "Zero balance"
)]
fn test_build_trade_actions(
    amount: u128,
    available_balance: u128,
    expected_actions: Vec<CreditAction>,
    _name: &str,
) {
    let swapper_route = SwapperRoute::Duality(DualityRoute {
        from: ASSET_IN_DENOM.to_string(),
        to: ASSET_OUT_DENOM.to_string(),
        swap_denoms: vec![ASSET_IN_DENOM.to_string(), ASSET_OUT_DENOM.to_string()],
    });

    let actions = build_trade_actions(
        Uint128::from(amount),
        Uint128::from(available_balance),
        ASSET_IN_DENOM,
        ASSET_OUT_DENOM,
        &swapper_route,
    );

    assert_eq!(actions, expected_actions);
}
