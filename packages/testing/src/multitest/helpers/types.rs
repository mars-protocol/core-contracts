use cosmwasm_schema::cw_serde;
use cosmwasm_std::{coin, Addr, Coin, Decimal, Uint128};
use cw_utils::Duration;
use mars_types::{
    credit_manager::{ActionAmount, ActionCoin},
    params::{
        AssetParamsUnchecked, CmSettings, HlsParamsUnchecked, LiquidationBonus, RedBankSettings,
    },
    red_bank::InterestRateModel,
};

#[cw_serde]
pub struct AccountToFund {
    pub addr: Addr,
    pub funds: Vec<Coin>,
}

#[cw_serde]
pub struct CoinInfo {
    pub denom: String,
    pub price: Decimal,
    pub max_ltv: Decimal,
    pub liquidation_threshold: Decimal,
    pub liquidation_bonus: LiquidationBonus,
    pub whitelisted: bool,
    pub withdraw_enabled: bool,
    pub hls: Option<HlsParamsUnchecked>,
    pub protocol_liquidation_fee: Decimal,
    pub close_factor: Decimal,
}

#[cw_serde]
pub struct LpCoinInfo {
    pub denom: String,
    pub price: Decimal,
    pub max_ltv: Decimal,
    pub liquidation_threshold: Decimal,
    pub underlying_pair: (String, String),
}

#[cw_serde]
pub struct VaultTestInfo {
    pub vault_token_denom: String,
    pub base_token_denom: String,
    pub lockup: Option<Duration>,
    pub deposit_cap: Coin,
    pub max_ltv: Decimal,
    pub liquidation_threshold: Decimal,
    pub whitelisted: bool,
    pub hls: Option<HlsParamsUnchecked>,
}

impl CoinInfo {
    pub fn to_coin(&self, amount: u128) -> Coin {
        coin(amount, self.denom.clone())
    }

    pub fn to_action_coin(&self, amount: u128) -> ActionCoin {
        ActionCoin {
            denom: self.denom.clone(),
            amount: ActionAmount::Exact(Uint128::new(amount)),
        }
    }

    pub fn to_action_coin_full_balance(&self) -> ActionCoin {
        ActionCoin {
            denom: self.denom.clone(),
            amount: ActionAmount::AccountBalance,
        }
    }
}

impl From<CoinInfo> for AssetParamsUnchecked {
    fn from(c: CoinInfo) -> Self {
        Self {
            denom: c.denom.clone(),
            credit_manager: CmSettings {
                whitelisted: c.whitelisted,
                withdraw_enabled: c.withdraw_enabled,
                hls: c.hls,
            },
            red_bank: RedBankSettings {
                deposit_enabled: true,
                borrow_enabled: true,
                withdraw_enabled: c.withdraw_enabled,
            },
            max_loan_to_value: c.max_ltv,
            liquidation_threshold: c.liquidation_threshold,
            liquidation_bonus: c.liquidation_bonus,
            protocol_liquidation_fee: c.protocol_liquidation_fee,
            deposit_cap: Uint128::MAX,
            close_factor: c.close_factor,
            reserve_factor: Decimal::percent(10u64),
            interest_rate_model: InterestRateModel {
                optimal_utilization_rate: Decimal::percent(80u64),
                base: Decimal::zero(),
                slope_1: Decimal::percent(7u64),
                slope_2: Decimal::percent(45u64),
            },
        }
    }
}
