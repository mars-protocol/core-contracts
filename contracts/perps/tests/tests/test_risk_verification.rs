use std::{fs::File, io::Read, str::FromStr};

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{coin, Decimal, Int128, Uint128};
use mars_types::{
    params::{PerpParams, PerpParamsUpdate},
    perps::Balance,
};

use super::helpers::MockEnv;

#[cw_serde]
struct AssetConfig {
    name: String,
    initial_price: f64,
    initial_funding_rate: f64,
}

#[cw_serde]
struct PerpConfig {
    initial_price: Decimal,

    #[serde(flatten)]
    perps_params: PerpParams,
}

#[cw_serde]
struct TestConfig {
    target_collaterization_ratio: Decimal,
    protocol_fee_rate: Decimal,
    perps: Vec<PerpConfig>,
}

#[cw_serde]
enum ActionType {
    ExecutePerpOrder {
        account_id: String,
        denom: String,
        order_size: Int128,
        reduce_only: Option<bool>,
    },
    ChangePrice {
        denom: String,
        new_price: Decimal,
    },
    DepositToVault {
        account_id: String,
        amount: Uint128,
    },
    SnapshotState {},
}

#[cw_serde]
struct Action {
    block_time: u64,
    action: ActionType,
}

#[cw_serde]
struct SnapshotStateResponse {
    block_time: u64,
    accounting: Accounting,
    vault: Vault,
}

#[cw_serde]
pub struct Accounting {
    /// The actual amount of money, includes only realized payments
    pub cash_flow: Balance,

    /// The actual amount of money + unrealized payments
    pub balance: Balance,

    /// The amount of money available for withdrawal by LPs (in this type of balance we cap some unrealized payments)
    pub withdrawal_balance: Balance,
}

#[cw_serde]
struct Vault {
    total_withdrawal_balance: Uint128,
}

fn load_test_config(path: &str) -> TestConfig {
    let mut file = File::open(path).expect("Unable to open config file");
    let mut data = String::new();
    file.read_to_string(&mut data).expect("Unable to read file");
    serde_json::from_str(&data).expect("JSON was not well-formatted")
}

fn load_actions(path: &str) -> Vec<Action> {
    let mut file = File::open(path).expect("Unable to open input file");
    let mut data = String::new();
    file.read_to_string(&mut data).expect("Unable to read file");
    serde_json::from_str(&data).expect("JSON was not well-formatted")
}

/// Loads the test configuration and input actions, then processes the actions to generate snapshots.
/// The generated snapshots are saved to a file for further verification.
/// The `sc_snapshot_state.json` file produced by this test is compared with the expected `risk_snapshot_state.json`
/// file provided by the Risk team to ensure accuracy.
#[test]
fn verify_accounting_with_input_actions() {
    // Load the test configuration and input actions from the specified files.
    let config = load_test_config("tests/tests/files/risk/config.json");
    let input = load_actions("tests/tests/files/risk/input_actions.json");

    // Initialize the mock environment with the target vault collateralization ratio and protocol fee rate.
    let mut mock = MockEnv::new()
        .target_vault_collaterization_ratio(config.target_collaterization_ratio)
        .protocol_fee_rate(config.protocol_fee_rate)
        .build()
        .unwrap();

    mock.set_block_time(0);

    let owner = mock.owner.clone();
    let credit_manager = mock.credit_manager.clone();

    // Fund the credit manager account, as it needs to call the perps contract and cover any losses from positions.
    mock.fund_accounts(&[&credit_manager], 1_000_000_000_000_000u128, &["uusdc"]);
    mock.set_price(&owner, "uusdc", Decimal::from_str("1").unwrap()).unwrap();

    let mut perp_denoms = vec![];

    // Initialize each perpetual market defined in the config.
    for perp in config.perps {
        println!("Configure perp: {:?}", perp);

        perp_denoms.push(perp.perps_params.denom.clone());

        // Add or update the perpetual market parameters.
        mock.update_perp_params(
            &owner,
            PerpParamsUpdate::AddOrUpdate {
                params: perp.perps_params.clone(),
            },
        );

        // Set the initial price for the perpetual market.
        mock.set_price(&owner, &perp.perps_params.denom, perp.initial_price).unwrap();
    }

    let mut snapshots = vec![];

    // Process each action defined in the input file.
    for action in &input {
        // Set the block time according to the action's specified time.
        mock.set_block_time(action.block_time);

        match &action.action {
            ActionType::DepositToVault {
                account_id,
                amount,
            } => {
                println!("Deposit to vault: {:?}", action);

                // Execute a deposit to the vault with the specified amount and account ID.
                mock.deposit_to_vault(
                    &credit_manager,
                    Some(account_id),
                    None,
                    &[coin(amount.u128(), "uusdc")],
                )
                .unwrap();
            }
            ActionType::ExecutePerpOrder {
                account_id,
                denom,
                order_size,
                reduce_only,
            } => {
                println!("Execute perp order: {:?}", action);

                // Query the position with the specified order size to determine if additional funds are needed.
                let pos_res =
                    mock.query_position_with_order_size(account_id, denom, Some(*order_size));
                let funds = if let Some(pos) = pos_res.position {
                    if pos.unrealized_pnl.pnl < Int128::zero() {
                        let fund =
                            coin(pos.unrealized_pnl.pnl.unsigned_abs().u128(), pos.base_denom);
                        vec![fund]
                    } else {
                        vec![]
                    }
                } else {
                    let opening_fee = mock.query_opening_fee(denom, *order_size).fee;
                    vec![opening_fee]
                };

                // Execute the perpetual order with the determined funds.
                mock.execute_perp_order(
                    &credit_manager,
                    account_id,
                    denom,
                    *order_size,
                    *reduce_only,
                    &funds,
                )
                .unwrap();
            }
            ActionType::ChangePrice {
                denom,
                new_price,
            } => {
                println!("Change price: {:?}", action);

                // Update the price for the specified denomination.
                mock.set_price(&owner, denom, *new_price).unwrap();
            }
            ActionType::SnapshotState {} => {
                println!("Snapshot state: {:?}", action);

                // Capture the current state as a snapshot for comparison.
                let block_time = mock.query_block_time();

                let accounting = mock.query_total_accounting().accounting;
                let vault = mock.query_vault();

                let snapshot_state = SnapshotStateResponse {
                    block_time,
                    accounting: Accounting {
                        cash_flow: Balance {
                            price_pnl: accounting.cash_flow.price_pnl,
                            opening_fee: accounting.cash_flow.opening_fee,
                            closing_fee: accounting.cash_flow.closing_fee,
                            accrued_funding: accounting.cash_flow.accrued_funding,
                            total: accounting.cash_flow.total().unwrap(),
                        },
                        balance: accounting.balance,
                        withdrawal_balance: accounting.withdrawal_balance,
                    },
                    vault: Vault {
                        total_withdrawal_balance: vault.total_withdrawal_balance,
                    },
                };
                snapshots.push(snapshot_state);
            }
        }
    }

    // Save the collected snapshots to a JSON file for further verification.
    let snapshot_state_json = serde_json::to_string_pretty(&snapshots).unwrap();
    let snapshot_state_file = "tests/tests/files/risk/sc_snapshot_state.json";
    std::fs::write(snapshot_state_file, &snapshot_state_json).unwrap();

    // Compare the generated snapshot state with the expected state from the Risk team.
    let expected_snapshot_state_file = "tests/tests/files/risk/risk_snapshot_state.json";
    let expected_snapshot_state = std::fs::read_to_string(expected_snapshot_state_file).unwrap();
    assert_eq!(snapshot_state_json, expected_snapshot_state);
}
