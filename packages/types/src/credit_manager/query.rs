use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Decimal, Uint128};
use mars_owner::OwnerResponse;

use super::{Action, Condition};
use crate::{
    adapters::{
        rewards_collector::RewardsCollector,
        vault::{Vault, VaultPosition, VaultUnchecked},
    },
    health::AccountKind,
    oracle::ActionKind,
    perps::PerpPosition,
    traits::Coins,
};

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AccountKind)]
    AccountKind {
        account_id: String,
    },
    #[returns(Vec<Account>)]
    Accounts {
        owner: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Rover contract-level config
    #[returns(ConfigResponse)]
    Config {},
    /// The amount the vault has been utilized,
    /// denominated in the same denom set in the vault config's deposit cap
    #[returns(VaultUtilizationResponse)]
    VaultUtilization {
        vault: VaultUnchecked,
    },
    /// Enumerate the amounts the vaults have been utilized,
    /// denominated in the same denom set in the vault config's deposit cap
    #[returns(cw_paginate::PaginationResponse<VaultUtilizationResponse>)]
    AllVaultUtilizations {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// All positions represented by token with value
    #[returns(Positions)]
    Positions {
        account_id: String,
        action: Option<ActionKind>,
    },
    /// Enumerate coin balances for all token positions; start_after accepts (account_id, denom)
    #[returns(Vec<CoinBalanceResponseItem>)]
    AllCoinBalances {
        start_after: Option<(String, String)>,
        limit: Option<u32>,
    },
    /// Enumerate debt shares for all token positions; start_after accepts (account_id, denom)
    #[returns(Vec<SharesResponseItem>)]
    AllDebtShares {
        start_after: Option<(String, String)>,
        limit: Option<u32>,
    },
    /// Total debt shares issued for Coin
    #[returns(DebtShares)]
    TotalDebtShares(String),
    /// Enumerate total debt shares for all supported coins; start_after accepts denom string
    #[returns(Vec<DebtShares>)]
    AllTotalDebtShares {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Enumerate all vault positions; start_after accepts (account_id, addr)
    #[returns(Vec<VaultPositionResponseItem>)]
    AllVaultPositions {
        start_after: Option<(String, String)>,
        limit: Option<u32>,
    },
    /// Estimate how many LP tokens received in exchange for coins provided for liquidity
    #[returns(Uint128)]
    EstimateProvideLiquidity {
        lp_token_out: String,
        coins_in: Vec<Coin>,
    },
    /// Estimate coins withdrawn if exchanged for LP tokens
    #[returns(Vec<Coin>)]
    EstimateWithdrawLiquidity {
        lp_token: Coin,
    },
    /// Returns the value of the a vault coin position.
    /// Given the extremely low price-per-coin and lack of precision, individual vault
    /// coins cannot be priced, hence you must send the whole amount you want priced.
    #[returns(crate::adapters::vault::VaultPositionValue)]
    VaultPositionValue {
        vault_position: VaultPosition,
    },
    /// Return all trigger orders.
    #[returns(cw_paginate::PaginationResponse<TriggerOrderResponse>)]
    AllTriggerOrders {
        start_after: Option<(String, String)>,
        limit: Option<u32>,
    },
    /// Return all trigger orders for an account.
    #[returns(cw_paginate::PaginationResponse<TriggerOrderResponse>)]
    AllAccountTriggerOrders {
        account_id: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },

    /// Enumerate all vault bindings; start_after accepts account_id
    #[returns(Vec<VaultBinding>)]
    VaultBindings {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct VaultUtilizationResponse {
    pub vault: VaultUnchecked,
    pub utilization: Coin,
}

#[cw_serde]
pub struct CoinBalanceResponseItem {
    pub account_id: String,
    pub denom: String,
    pub amount: Uint128,
}

#[cw_serde]
pub struct SharesResponseItem {
    pub account_id: String,
    pub denom: String,
    pub shares: Uint128,
}

#[cw_serde]
pub struct DebtShares {
    pub denom: String,
    pub shares: Uint128,
}

#[cw_serde]
pub struct LentShares {
    pub denom: String,
    pub shares: Uint128,
}

#[cw_serde]
pub struct DebtAmount {
    pub denom: String,
    /// number of shares in debt pool
    pub shares: Uint128,
    /// amount of coins
    pub amount: Uint128,
}

#[cw_serde]
#[derive(Default)]
pub struct KeeperFeeConfig {
    pub min_fee: Coin,
}

#[cw_serde]
pub struct TriggerOrder {
    pub order_id: String,
    pub actions: Vec<Action>,
    pub conditions: Vec<Condition>,
    pub keeper_fee: Coin,
}

impl Coins for Vec<DebtAmount> {
    fn to_coins(&self) -> Vec<Coin> {
        self.iter()
            .map(|d| Coin {
                denom: d.denom.to_string(),
                amount: d.amount,
            })
            .collect()
    }
}

#[cw_serde]
pub struct Positions {
    pub account_id: String,
    pub account_kind: AccountKind,
    pub deposits: Vec<Coin>,
    pub debts: Vec<DebtAmount>,
    pub lends: Vec<Coin>,
    pub vaults: Vec<VaultPosition>,
    pub staked_astro_lps: Vec<Coin>,
    pub perps: Vec<PerpPosition>,
}

#[cw_serde]
pub struct VaultPositionResponseItem {
    pub account_id: String,
    pub position: VaultPosition,
}

#[cw_serde]
pub struct TriggerOrderResponse {
    pub account_id: String,
    pub order: TriggerOrder,
}

#[cw_serde]
pub struct VaultWithBalance {
    pub vault: Vault,
    pub balance: Uint128,
}

#[cw_serde]
pub struct ConfigResponse {
    pub ownership: OwnerResponse,
    pub account_nft: Option<String>,
    pub red_bank: String,
    pub incentives: String,
    pub oracle: String,
    pub params: String,
    pub perps: String,
    pub max_unlocking_positions: Uint128,
    pub max_slippage: Decimal,
    pub swapper: String,
    pub zapper: String,
    pub health_contract: String,
    pub rewards_collector: Option<RewardsCollector>,
    pub keeper_fee_config: KeeperFeeConfig,
    pub perps_liquidation_bonus_ratio: Decimal,
}

#[cw_serde]
pub struct Account {
    pub id: String,
    pub kind: AccountKind,
}

#[cw_serde]
pub struct VaultBinding {
    pub account_id: String,
    pub vault_address: String,
}
