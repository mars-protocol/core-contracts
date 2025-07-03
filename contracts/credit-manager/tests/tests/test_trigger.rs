use std::str::FromStr;

use anyhow::Error;
use cosmwasm_std::{Addr, Coin, Decimal, Int128, OverflowError, OverflowOperation, Uint128};
use cw_multi_test::AppResponse;
use mars_credit_manager::error::ContractError;
use mars_mock_oracle::msg::CoinPrice;
use mars_testing::multitest::helpers::AccountToFund;
use mars_types::{
    credit_manager::{
        Action::{
            self, ClosePerpPosition, CreateTriggerOrder, DeleteTriggerOrder, Deposit,
            ExecutePerpOrder, Lend, Liquidate,
        },
        ActionAmount, ActionCoin, Comparison,
        Condition::{HealthFactor, OraclePrice},
        CreateTriggerOrderType, ExecutePerpOrderType, LiquidateRequest, TriggerOrder,
        TriggerOrderResponse,
    },
    oracle::ActionKind,
    params::PerpParamsUpdate,
};
use test_case::test_case;

use super::helpers::MockEnv;
use crate::tests::helpers::{coin_info, default_perp_params, uatom_info, uosmo_info};

#[test]
fn error_when_exceeding_max_trigger_orders() {
    let user = Addr::unchecked("user");
    let keeper_fee = Coin {
        denom: "uusdc".to_string(),
        amount: Uint128::new(1000000),
    };
    let mut mock = MockEnv::new()
        .max_trigger_orders(1)
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![Coin {
                denom: "uusdc".to_string(),
                amount: Uint128::new(10000000000000),
            }],
        })
        .build()
        .unwrap();

    let account_id = mock.create_credit_account(&user).unwrap();

    // Create the first trigger order. This should work fine.
    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(keeper_fee.clone()),
            CreateTriggerOrder {
                order_type: Some(CreateTriggerOrderType::Default),
                actions: vec![ExecutePerpOrder {
                    denom: "perp1".to_string(),
                    order_size: Int128::from_str("10").unwrap(),
                    reduce_only: None,
                    order_type: Some(ExecutePerpOrderType::Default),
                }],
                conditions: vec![OraclePrice {
                    denom: "perp1".to_string(),
                    price: Decimal::from_str("100").unwrap(),
                    comparison: Comparison::GreaterThan,
                }],
                keeper_fee: keeper_fee.clone(),
            },
        ],
        &[keeper_fee.clone()],
    )
    .unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(keeper_fee.clone()),
            CreateTriggerOrder {
                order_type: Some(CreateTriggerOrderType::Default),
                actions: vec![ExecutePerpOrder {
                    denom: "perp1".to_string(),
                    order_size: Int128::from_str("10").unwrap(),
                    reduce_only: None,
                    order_type: Some(ExecutePerpOrderType::Default),
                }],
                conditions: vec![OraclePrice {
                    denom: "perp1".to_string(),
                    price: Decimal::from_str("100").unwrap(),
                    comparison: Comparison::GreaterThan,
                }],
                keeper_fee: keeper_fee.clone(),
            },
        ],
        &[keeper_fee.clone()],
    );

    check_result_for_expected_error(
        res,
        Some(ContractError::MaxTriggerOrdersReached {
            max_trigger_orders: 1,
        }),
    );
}

#[test]
fn lend_action_whitelisted_in_trigger_orders() {
    let user = Addr::unchecked("user");
    let keeper_fee = Coin {
        denom: "uusdc".to_string(),
        amount: Uint128::new(1000000),
    };
    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![Coin {
                denom: "uusdc".to_string(),
                amount: Uint128::new(10000000000000),
            }],
        })
        .build()
        .unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    // Should be able to create a trigger order with a lend action
    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(keeper_fee.clone()),
            CreateTriggerOrder {
                order_type: Some(CreateTriggerOrderType::Default),
                actions: vec![
                    ExecutePerpOrder {
                        denom: "perp1".to_string(),
                        order_size: Int128::from_str("10").unwrap(),
                        reduce_only: None,
                        order_type: Some(ExecutePerpOrderType::Default),
                    },
                    Lend(ActionCoin {
                        denom: keeper_fee.denom.clone(),
                        amount: ActionAmount::AccountBalance,
                    }),
                ],
                conditions: vec![OraclePrice {
                    denom: "perp1".to_string(),
                    price: Decimal::from_str("100").unwrap(),
                    comparison: Comparison::GreaterThan,
                }],
                keeper_fee: keeper_fee.clone(),
            },
        ],
        &[keeper_fee.clone()],
    )
    .unwrap();
}

#[test]
fn close_perp_position_action_whitelisted_in_trigger_orders() {
    let user = Addr::unchecked("user");
    let keeper_fee = Coin {
        denom: "uusdc".to_string(),
        amount: Uint128::new(1000000),
    };

    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![Coin {
                denom: "uusdc".to_string(),
                amount: Uint128::new(10000000000000),
            }],
        })
        .build()
        .unwrap();

    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: "perp1".to_string(),
        price: Decimal::from_str("100").unwrap(),
    });

    mock.update_perp_params(PerpParamsUpdate::AddOrUpdate {
        params: default_perp_params("perp1"),
    });

    let account_id = mock.create_credit_account(&user).unwrap();

    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(keeper_fee.clone()),
            CreateTriggerOrder {
                order_type: Some(CreateTriggerOrderType::Default),
                actions: vec![ClosePerpPosition {
                    denom: "perp1".to_string(),
                }],
                conditions: vec![OraclePrice {
                    denom: "perp1".to_string(),
                    price: Decimal::from_str("120").unwrap(),
                    comparison: Comparison::GreaterThan,
                }],
                keeper_fee: keeper_fee.clone(),
            },
        ],
        &[keeper_fee.clone()],
    )
    .unwrap();
}

#[test]
fn query_all_trigger_orders() {
    // create mock
    let user = Addr::unchecked("user");
    let keeper_fee = Coin {
        denom: "uusdc".to_string(),
        amount: Uint128::new(1000000),
    };
    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![Coin {
                denom: "uusdc".to_string(),
                amount: Uint128::new(10000000000000),
            }],
        })
        .build()
        .unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    // create 9 trigger orders
    let mut orders = vec![];
    for num in 1..10 {
        let order_size = Int128::from_str("-10")
            .unwrap()
            .checked_mul(Int128::from_str(&num.to_string()).unwrap())
            .unwrap();
        orders.push(TriggerOrderResponse {
            account_id: account_id.clone(),
            order: TriggerOrder {
                order_id: num.to_string(),
                actions: vec![ExecutePerpOrder {
                    denom: "perp1".to_string(),
                    order_size,
                    reduce_only: None,
                    order_type: Some(ExecutePerpOrderType::Default),
                }],
                conditions: vec![OraclePrice {
                    denom: "perp1".to_string(),
                    price: Decimal::from_str("100").unwrap(),
                    comparison: Comparison::GreaterThan,
                }],
                keeper_fee: keeper_fee.clone(),
            },
        });
    }

    // create trigger orders
    for order in &orders {
        mock.update_credit_account(
            &order.account_id,
            &user,
            vec![
                Deposit(order.order.keeper_fee.clone()),
                CreateTriggerOrder {
                    order_type: Some(CreateTriggerOrderType::Default),
                    actions: order.order.actions.clone(),
                    conditions: order.order.conditions.clone(),
                    keeper_fee: order.order.keeper_fee.clone(),
                },
            ],
            &[order.order.keeper_fee.clone()],
        )
        .unwrap();
    }
    let response = mock.query_all_trigger_orders(None, Some(5));

    assert_eq!(response.data.len(), 5);
    assert_eq!(response.data, orders[0..5].to_vec());

    // Check we can paginate
    let last_item = response.data.last().unwrap();
    let start_after = (last_item.account_id.clone(), last_item.order.order_id.clone());
    let next_response = mock.query_all_trigger_orders(Some(start_after), Some(5));
    assert_eq!(next_response.data.len(), 4);
}

#[test]
fn delete_trigger_order() {
    // create mock
    let user = Addr::unchecked("user");
    let keeper_fee = Coin {
        denom: "uusdc".to_string(),
        amount: Uint128::new(10000000),
    };
    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![keeper_fee.clone()],
        })
        .build()
        .unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    // create trigger order
    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(keeper_fee.clone()),
            CreateTriggerOrder {
                order_type: Some(CreateTriggerOrderType::Default),
                actions: vec![ExecutePerpOrder {
                    denom: "perp1".to_string(),
                    order_size: Int128::from_str("-10").unwrap(),
                    reduce_only: None,
                    order_type: Some(ExecutePerpOrderType::Default),
                }],
                conditions: vec![OraclePrice {
                    denom: "perp1".to_string(),
                    price: Decimal::from_str("100").unwrap(),
                    comparison: Comparison::GreaterThan,
                }],
                keeper_fee: keeper_fee.clone(),
            },
        ],
        &[keeper_fee.clone()],
    )
    .unwrap();

    let response = mock.query_trigger_orders_for_account(account_id.clone(), None, None);

    assert_eq!(response.data.len(), 1);
    assert_eq!(response.data[0].account_id, account_id.clone());
    assert_eq!(response.data[0].order.keeper_fee, keeper_fee.clone());
    assert_eq!(
        response.data[0].order.actions,
        [ExecutePerpOrder {
            denom: "perp1".to_string(),
            order_size: Int128::from_str("-10").unwrap(),
            reduce_only: None,
            order_type: Some(ExecutePerpOrderType::Default)
        }]
    );
    assert_eq!(
        response.data[0].order.conditions,
        [OraclePrice {
            denom: "perp1".to_string(),
            price: Decimal::from_str("100").unwrap(),
            comparison: Comparison::GreaterThan,
        }]
    );
    assert_eq!(response.data[0].order.order_id, "1");

    // Assert keeper fee deducted from account
    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 0);

    mock.update_credit_account(
        &account_id,
        &user,
        vec![DeleteTriggerOrder {
            trigger_order_id: 1.to_string(),
        }],
        &[],
    )
    .unwrap();

    // Assert no remaining trigger orders
    let response = mock.query_trigger_orders_for_account(account_id.clone(), None, None);
    assert_eq!(response.data.len(), 0);

    // Assert keeper fee returned to user
    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 1);
    assert_eq!(position.deposits[0].amount, keeper_fee.amount);
    assert_eq!(position.deposits[0].denom, keeper_fee.denom);
}

#[test_case(
    coin_info("uusdc").to_coin(1),
    vec![
        CreateTriggerOrder {
            order_type: Some(CreateTriggerOrderType::Default),
            actions: vec![ExecutePerpOrder {
                denom: uatom_info().denom.to_string(),
                order_size: Int128::from_str("-10").unwrap(),
                reduce_only: None,
                order_type: Some(ExecutePerpOrderType::Default)
            }],
            conditions: vec![OraclePrice {
                denom: uatom_info().denom.to_string(),
                price: Decimal::from_str("1.5").unwrap(),
                comparison: Comparison::GreaterThan,
            }],
            keeper_fee: coin_info("uusdc").to_coin(1),
        }
    ],
    None,
    vec![],
    Some(ContractError::KeeperFeeTooSmall { expected_min_amount: Uint128::new(1000000), received_amount: Uint128::new(1) }),
    None
    ;
    "Keeper fee too low"
)]
#[test_case(
    coin_info("uusdc").to_coin(1),
    vec![
        Deposit(coin_info("uusdc").to_coin(1)),
        CreateTriggerOrder {
            order_type: Some(CreateTriggerOrderType::Default),
            actions: vec![ExecutePerpOrder {
                denom: uatom_info().denom.to_string(),
                order_size: Int128::from_str("-10").unwrap(),
                reduce_only: None,
                order_type: Some(ExecutePerpOrderType::Default)
            }],
            conditions: vec![OraclePrice {
                denom: uatom_info().denom.to_string(),
                price: Decimal::from_str("1.5").unwrap(),
                comparison: Comparison::GreaterThan,
            }],
            keeper_fee: coin_info("untrn").to_coin(100000000),
        }
    ],
    None,
    vec![
        coin_info("uusdc").to_coin(1),
    ],
    Some(ContractError::InvalidKeeperFeeDenom { expected_denom: "uusdc".to_string(), received_denom: "untrn".to_string() }),
    None
    ;
    "Keeper fee incorrect denom"
)]
#[test_case(
    coin_info("uusdc").to_coin(1),
    vec![
        CreateTriggerOrder {
            order_type: Some(CreateTriggerOrderType::Default),
            actions: vec![ExecutePerpOrder {
                denom: uatom_info().denom.to_string(),
                order_size: Int128::from_str("-10").unwrap(),
                reduce_only: None,
                order_type: Some(ExecutePerpOrderType::Default)
            }],
            conditions: vec![OraclePrice {
                denom: uatom_info().denom.to_string(),
                price: Decimal::from_str("1.5").unwrap(),
                comparison: Comparison::GreaterThan,
            }],
            keeper_fee: coin_info("uusdc").to_coin(100000000),
        }
    ],
    None,
    vec![],
    Some(ContractError::Overflow(OverflowError {
        operation: OverflowOperation::Sub,
        operand1: "0".to_string(),
        operand2: "100000000".to_string(),
    })),
    None
    ;
    "Insufficient funds to pay keeper fee"
)]
#[test_case(
    coin_info("uusdc").to_coin(1000000),
    vec![
        Deposit(coin_info("uusdc").to_coin(1000000)),
        CreateTriggerOrder {
            order_type: Some(CreateTriggerOrderType::Default),
            actions: vec![ExecutePerpOrder {
                denom: uatom_info().denom.to_string(),
                order_size: Int128::from_str("-10").unwrap(),
                reduce_only: None,
                order_type: Some(ExecutePerpOrderType::Default)
            }],
            conditions: vec![OraclePrice {
                denom: uatom_info().denom.to_string(),
                price: Decimal::from_str("1.5").unwrap(),
                comparison: Comparison::GreaterThan,
            }],
            keeper_fee: coin_info("uusdc").to_coin(1000000),
        },
    ],
    Some(12347),
    vec![coin_info("uusdc").to_coin(1000000)],
    None,
    Some(ContractError::TriggerOrderNotFound { order_id: "12347".to_string(), account_id: "2".to_string() });
    "Error when no trigger id found"
)]
#[test_case(
    coin_info("uusdc").to_coin(1000000),
    vec![
        Deposit(coin_info("uusdc").to_coin(1000000)),
        CreateTriggerOrder {
            order_type: Some(CreateTriggerOrderType::Default),
            actions: vec![ExecutePerpOrder {
                denom: uatom_info().denom.to_string(),
                order_size: Int128::from_str("-10").unwrap(),
                reduce_only: None,
                order_type: Some(ExecutePerpOrderType::Default)
            }],
            conditions: vec![OraclePrice {
                denom: uatom_info().denom.to_string(),
                price: Decimal::from_str("1.5").unwrap(),
                comparison: Comparison::GreaterThan,
            }],
            keeper_fee: coin_info("uusdc").to_coin(1000000),
        },
    ],
    Some(1),
    vec![coin_info("uusdc").to_coin(1000000)],
    None,
    Some(ContractError::IllegalExecuteTriggerOrder);
    "Error when price condition not met on execute"
)]
#[test_case(
    coin_info("uusdc").to_coin(1000000),
    vec![
        Deposit(coin_info("uusdc").to_coin(100000000)),
        CreateTriggerOrder {
            order_type: Some(CreateTriggerOrderType::Default),
            actions: vec![ExecutePerpOrder {
                denom: uatom_info().denom.to_string(),
                order_size: Int128::from_str("-1").unwrap(),
                reduce_only: None,
                order_type: Some(ExecutePerpOrderType::Default)
            }],
            conditions: vec![OraclePrice {
                denom: uatom_info().denom.to_string(),
                price: Decimal::from_str("1.5").unwrap(),
                comparison: Comparison::LessThan,
            }],
            keeper_fee: coin_info("uusdc").to_coin(1000000),
        },
    ],
    Some(1),
    vec![coin_info("uusdc").to_coin(100000000)],
    None,
    None;
    "Succeed when price condition met on execute"
)]
#[test_case(
    coin_info("uusdc").to_coin(1000000),
    vec![
        Deposit(coin_info("uusdc").to_coin(100000000)),
        CreateTriggerOrder {
            order_type: Some(CreateTriggerOrderType::Default),
            actions: vec![ExecutePerpOrder {
                denom: uatom_info().denom.to_string(),
                order_size: Int128::from_str("-1").unwrap(),
                reduce_only: None,
                order_type: Some(ExecutePerpOrderType::Default)
            }],
            conditions: vec![
                OraclePrice {
                    denom: uatom_info().denom.to_string(),
                    price: Decimal::from_str("1.5").unwrap(),
                    comparison: Comparison::LessThan,
                },
                HealthFactor {
                    comparison: Comparison::GreaterThan,
                    threshold: Decimal::from_str("1.2").unwrap()
                }
            ],
            keeper_fee: coin_info("uusdc").to_coin(1000000),
        },
    ],
    Some(1),
    vec![coin_info("uusdc").to_coin(100000000)],
    None,
    None;
    "Succeed when price condition and HF met on execute"
)]
#[test_case(
    coin_info("uusdc").to_coin(1000000),
    vec![
        Deposit(coin_info("uusdc").to_coin(100000000)),
        CreateTriggerOrder {
            order_type: Some(CreateTriggerOrderType::Default),
            actions: vec![Deposit(coin_info("uusdc").to_coin(100000000))],
            conditions: vec![],
            keeper_fee: coin_info("uusdc").to_coin(1000000),
        },
    ],
    None,
    vec![coin_info("uusdc").to_coin(100000000)],
    Some(ContractError::IllegalTriggerAction),
    None;
    "Error when illegal trigger actions used (deposit)"
)]
#[test_case(
    coin_info("uusdc").to_coin(1000000),
    vec![
        Deposit(coin_info("uusdc").to_coin(100000000)),
        CreateTriggerOrder {
            order_type: Some(CreateTriggerOrderType::Default),
            actions: vec![Liquidate{
                debt_coin: coin_info("uusdc").to_coin(100000000),
                liquidatee_account_id: "1".to_string(),
                request: LiquidateRequest::Deposit("uusdc".to_string()),
            }],
            conditions: vec![],
            keeper_fee: coin_info("uusdc").to_coin(1000000),
        },
    ],
    None,
    vec![coin_info("uusdc").to_coin(100000000)],
    Some(ContractError::IllegalTriggerAction),
    None;
    "Error when illegal trigger actions used (liquidate)"
)]
#[test_case(
    coin_info("uusdc").to_coin(1000000),
    vec![
        Deposit(coin_info("uusdc").to_coin(100000000)),
        CreateTriggerOrder {
            order_type: Some(CreateTriggerOrderType::Default),
            actions: vec![ExecutePerpOrder {
                denom: uatom_info().denom.to_string(),
                order_size: Int128::from_str("-1").unwrap(),
                reduce_only: None,
                order_type: Some(ExecutePerpOrderType::Default)
            }],
            conditions: vec![
                OraclePrice {
                    denom: uatom_info().denom.to_string(),
                    price: Decimal::from_str("1.5").unwrap(),
                    comparison: Comparison::LessThan,
                },
                HealthFactor {
                    comparison: Comparison::LessThan,
                    threshold: Decimal::from_str("1.2").unwrap()
                }
            ],
            keeper_fee: coin_info("uusdc").to_coin(1000000),
        },
    ],
    Some(1),
    vec![coin_info("uusdc").to_coin(100000000)],
    None,
    Some(ContractError::IllegalExecuteTriggerOrder);
    "Error when price condition met and HF not met on execute"
)]
fn verify_trigger_orders(
    keeper_fee: Coin,
    actions: Vec<Action>,
    maybe_order_to_execute: Option<u32>,
    funds: Vec<Coin>,
    maybe_expected_error_on_create: Option<ContractError>,
    maybe_expected_error_on_execute: Option<ContractError>,
) {
    // create mock
    let osmo_info = uosmo_info();
    let atom_info = uatom_info();
    let usdc_info = coin_info("uusdc");
    let ntrn_info = coin_info("untrn");

    // Addresses
    let contract_owner = Addr::unchecked("owner");
    let cm_user = Addr::unchecked("user");
    let keeper_bot = Addr::unchecked("keeper");
    let vault_depositor = Addr::unchecked("vault_depositor");

    // Funds given to each address
    let osmo_coin = osmo_info.to_coin(100000000000);
    let usdc_coin = usdc_info.to_coin(100000000000);
    let ntrn_coin = ntrn_info.to_coin(100000000000);

    // Create mock env
    let mut mock = MockEnv::new()
        .owner(contract_owner.as_str())
        .set_params(&[osmo_info, atom_info.clone(), usdc_info.clone()])
        .fund_accounts(
            vec![cm_user.clone(), vault_depositor.clone()],
            vec![osmo_coin.clone(), usdc_coin.clone(), usdc_coin.clone(), ntrn_coin.clone()],
        )
        .build()
        .unwrap();

    mock.update_perp_params(PerpParamsUpdate::AddOrUpdate {
        params: default_perp_params(&atom_info.denom),
    });

    let account_id = mock.create_credit_account(&cm_user).unwrap();

    let result = mock.update_credit_account(&account_id, &cm_user, actions, &funds);

    // Keep record of whether the trigger order was successfully placed
    let mut successful_execute = result.is_ok();

    check_result_for_expected_error(result, maybe_expected_error_on_create);

    if let Some(order) = maybe_order_to_execute {
        let execute_result =
            mock.execute_trigger_order(&keeper_bot, &account_id, &order.to_string());
        successful_execute = execute_result.is_ok() && successful_execute;
        check_result_for_expected_error(execute_result, maybe_expected_error_on_execute);
    }

    if successful_execute {
        // Check that our keeper fee was deducted from the account
        assert_eq!(mock.query_balance(&keeper_bot, &keeper_fee.denom), keeper_fee);
        // Check that the trigger order was removed
        assert_eq!(
            mock.query_trigger_orders_for_account(account_id.clone(), None, None).data.len(),
            0
        );
    }
}

fn check_result_for_expected_error(
    result: Result<AppResponse, Error>,
    expected_error: Option<ContractError>,
) {
    // check result
    match (result, expected_error) {
        (Err(err), Some(exp_err)) => {
            let err: ContractError = err.downcast().unwrap();
            assert_eq!(err, exp_err);
        }
        (Err(err), None) => {
            panic!("unexpected error: {:?}", err);
        }
        (Ok(_), Some(_)) => panic!("expected error, but got success"),
        (Ok(_), None) => {}
    }
}
