pub use mars_testing::multitest::helpers;

mod test_borrow;
mod test_claim_astro_lp_rewards;
mod test_claim_rewards;
mod test_coin_balances;
mod test_create_credit_account;
mod test_deposit;
mod test_deposit_cap;
mod test_dispatch;
mod test_enumerate_accounts;
mod test_enumerate_coin_balances;
mod test_enumerate_debt_shares;
mod test_enumerate_total_debt_shares;
mod test_enumerate_vault_positions;
mod test_fund_manager_accounts;
mod test_health;
mod test_hls_accounts;
mod test_instantiate;
mod test_lend;
mod test_liquidate_deposit;
mod test_liquidate_guard;
mod test_liquidate_if_perps_open;
mod test_liquidate_lend;
mod test_liquidate_staked_astro_lp;
mod test_liquidate_vault;
mod test_liquidation_pricing;
mod test_migration_v2;
mod test_migration_v2_2_3;
mod test_no_health_check;
mod test_order_relations;
mod test_perp;
mod test_perp_vault;
mod test_perps_deleverage;
mod test_reclaim;
mod test_reentrancy_guard;
mod test_refund_balances;
mod test_repay;
mod test_repay_for_recipient;
mod test_repay_from_wallet;
mod test_stake_astro_lp;
mod test_swap;
mod test_trigger;
mod test_unstake_astro_lp;
mod test_update_admin;
mod test_update_config;
mod test_update_credit_account_with_new_acc;
mod test_update_nft_config;
mod test_usdc_accounts;
mod test_utilization_query;
mod test_utilizations_all_query;
mod test_vault_enter;
mod test_vault_exit;
mod test_vault_exit_unlocked;
mod test_vault_query_value;
mod test_vault_request_unlock;
mod test_withdraw;
mod test_zap_provide;
mod test_zap_withdraw;
