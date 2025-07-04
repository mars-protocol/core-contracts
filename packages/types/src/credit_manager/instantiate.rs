use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, Uint128};

use super::KeeperFeeConfig;
use crate::adapters::{
    account_nft::AccountNftUnchecked, health::HealthContractUnchecked,
    incentives::IncentivesUnchecked, oracle::OracleUnchecked, params::ParamsUnchecked,
    perps::PerpsUnchecked, red_bank::RedBankUnchecked, swapper::SwapperUnchecked,
    zapper::ZapperUnchecked,
};

#[cw_serde]
pub struct InstantiateMsg {
    /// The address with privileged access to update config
    pub owner: String,
    /// The Mars Protocol money market contract where we borrow assets from
    pub red_bank: RedBankUnchecked,
    /// The Mars Protocol oracle contract. We read prices of assets here.
    pub oracle: OracleUnchecked,
    /// The maximum number of unlocking positions an account can have simultaneously
    /// Note: As health checking requires looping through each, this number must not be too large.
    ///       If so, having too many could prevent the account from being liquidated due to gas constraints.
    pub max_unlocking_positions: Uint128,
    /// The maximum slippage allowed for swaps, provide liquidity and withdraw liquidity
    pub max_slippage: Decimal,
    /// Helper contract for making swaps
    pub swapper: SwapperUnchecked,
    /// Helper contract for making swaps
    pub duality_swapper: SwapperUnchecked,
    /// Helper contract for adding/removing liquidity
    pub zapper: ZapperUnchecked,
    /// Helper contract for calculating health factor
    pub health_contract: HealthContractUnchecked,
    /// Contract that stores asset and vault params
    pub params: ParamsUnchecked,
    /// Contract that handles lending incentive rewards
    pub incentives: IncentivesUnchecked,
    /// Configuration for the keeper fee for trigger orders
    pub keeper_fee_config: KeeperFeeConfig,
    /// This variable represents the percentage of the original Liquidation Bonus (LB)
    /// applied to negative PnL when liquidating (closing) perps positions. It serves as
    /// a reward for the liquidator for closing perps in a loss and improving the account’s
    /// Health Factor (HF). This modified LB specifically applies in perps liquidation cases,
    /// allowing for a reduced bonus proportion when compared to standard spot liquidation.
    /// For example, if set to 0.60, 60% of the original LB will be applied to the perps
    /// PnL loss as follows:
    /// `bonus applied to liquidation = perps_liquidation_bonus_ratio * original LB * PnL loss`
    pub perps_liquidation_bonus_ratio: Decimal,
}

/// Used when you want to update fields on Instantiate config
#[cw_serde]
#[derive(Default)]
pub struct ConfigUpdates {
    pub account_nft: Option<AccountNftUnchecked>,
    pub oracle: Option<OracleUnchecked>,
    pub red_bank: Option<RedBankUnchecked>,
    pub incentives: Option<IncentivesUnchecked>,
    pub max_unlocking_positions: Option<Uint128>,
    pub max_slippage: Option<Decimal>,
    pub swapper: Option<SwapperUnchecked>,
    pub duality_swapper: Option<SwapperUnchecked>,
    pub zapper: Option<ZapperUnchecked>,
    pub health_contract: Option<HealthContractUnchecked>,
    pub params: Option<ParamsUnchecked>,
    /// The Mars Protocol rewards-collector contract. We collect protocol fee for its account.
    pub rewards_collector: Option<String>,
    pub perps: Option<PerpsUnchecked>,
    pub keeper_fee_config: Option<KeeperFeeConfig>,
    pub perps_liquidation_bonus_ratio: Option<Decimal>,
}
