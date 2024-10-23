use std::fmt;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    coin, Addr, Api, CheckedFromRatioError, CheckedMultiplyFractionError,
    CheckedMultiplyRatioError, Coin, ConversionOverflowError, Decimal, DecimalRangeExceeded,
    Fraction, Int128, Int256, OverflowError, SignedDecimal, SignedDecimalRangeExceeded, StdError,
    StdResult, Uint128, Uint256,
};
use mars_owner::OwnerUpdate;
use thiserror::Error;

use crate::{error::MarsError, oracle::ActionKind, params::PerpParams};

// ------------------------------- message types -------------------------------

/// The perp protocol's global configuration
#[cw_serde]
pub struct Config<T> {
    /// Address provider returns addresses for all protocol contracts
    pub address_provider: T,

    /// The token used to settle perp trades.
    ///
    /// Typically, this is be a stablecoin such as USDC (more precisely, the IBC
    /// voucher denom of USDC).
    ///
    /// Liquidity providers deposit this token to the vault.
    /// Traders deposit this token as collateral when opening perp positions.
    ///
    /// When closing a winning perp position (one that has a positive unrealized
    /// PnL), this token (of the amount corresponding to the PnL) is transferred
    /// from the vault to the user's credit account, together with the
    /// originally deposited collateral.
    ///
    /// Conversely, when closing a losing position, this token (of amount
    /// corresponding to the PnL) is transferred from the user's position to the
    /// vault. The remaining amount is refunded to the uesr's credit account.
    pub base_denom: String,

    /// Stakers need to wait a cooldown period before being able to withdraw USDC from the vault.
    /// Value defined in seconds.
    pub cooldown_period: u64,

    /// The maximum number of positions that can be opened by a single user
    pub max_positions: u8,

    /// The percentage of fees that is directed to the protocol
    pub protocol_fee_rate: Decimal,

    /// The target collateralization ratio of the vault
    pub target_vault_collateralization_ratio: Decimal,

    /// If the collateralization ratio of the vault falls below the
    /// target_vault_collateralization_ratio, it is eligible to be deleveraged
    /// when this parameter is true.
    pub deleverage_enabled: bool,

    /// True by default, it can be set to false to disable perp counterparty vault withdrawals
    pub vault_withdraw_enabled: bool,

    /// The maximum number of unlocks that can be requested by a single user
    pub max_unlocks: u8,
}

impl Config<String> {
    pub fn check(self, api: &dyn Api) -> StdResult<Config<Addr>> {
        Ok(Config {
            address_provider: api.addr_validate(&self.address_provider)?,
            base_denom: self.base_denom,
            cooldown_period: self.cooldown_period,
            max_positions: self.max_positions,
            protocol_fee_rate: self.protocol_fee_rate,
            target_vault_collateralization_ratio: self.target_vault_collateralization_ratio,
            deleverage_enabled: self.deleverage_enabled,
            vault_withdraw_enabled: self.vault_withdraw_enabled,
            max_unlocks: self.max_unlocks,
        })
    }
}

impl From<Config<Addr>> for Config<String> {
    fn from(cfg: Config<Addr>) -> Self {
        Config {
            address_provider: cfg.address_provider.into(),
            base_denom: cfg.base_denom,
            cooldown_period: cfg.cooldown_period,
            max_positions: cfg.max_positions,
            protocol_fee_rate: cfg.protocol_fee_rate,
            target_vault_collateralization_ratio: cfg.target_vault_collateralization_ratio,
            deleverage_enabled: cfg.deleverage_enabled,
            vault_withdraw_enabled: cfg.vault_withdraw_enabled,
            max_unlocks: cfg.max_unlocks,
        }
    }
}
#[cw_serde]
#[derive(Default)]
pub struct ConfigUpdates {
    pub address_provider: Option<String>,
    pub cooldown_period: Option<u64>,
    pub max_positions: Option<u8>,
    pub protocol_fee_rate: Option<Decimal>,
    pub target_vault_collateralization_ratio: Option<Decimal>,
    pub deleverage_enabled: Option<bool>,
    pub vault_withdraw_enabled: Option<bool>,
    pub max_unlocks: Option<u8>,
}

/// Global state of the counterparty vault
#[cw_serde]
#[derive(Default)]
pub struct VaultState {
    /// Value of the total balance in the base denom. This is the total amount
    /// of the base denom deposited to the vault by liquidity providers.
    /// The value is updated when a user deposits or withdraws from the vault.
    /// The value can be negative if the liquidity providers withdraw more than
    /// the total balance. This can happen if the vault earns a profit from trading.
    pub total_balance: Int128,

    /// Total shares minted to liquidity providers
    pub total_shares: Uint128,
}

#[cw_serde]
#[derive(Default)]
pub struct VaultResponse {
    /// Value of the total balance in the base denom. This is the total amount
    /// of the base denom deposited to the vault by liquidity providers.
    /// The value is updated when a user deposits or withdraws from the vault.
    /// The value can be negative if the liquidity providers withdraw more than
    /// the total balance. This can happen if the vault earns a profit from trading.
    pub total_balance: Int128,

    /// Total shares minted to liquidity providers.
    pub total_shares: Uint128,

    /// The total number of shares that are either currently unlocking or already unlocked but not withdrawn.
    /// This includes both shares still within the unlocking period and shares that have completed unlocking.
    pub total_unlocking_or_unlocked_shares: Uint128,

    /// The total amount (in base currency) corresponding to shares that are either unlocking or already unlocked but not withdrawn.
    /// This amount is proportional to the total unlocking or unlocked shares and is calculated based on the Vault share price.
    pub total_unlocking_or_unlocked_amount: Uint128,

    /// Total withdrawal balance in the base denom aggregated across all markets.
    /// `total_withdrawal_balance = max(total_balance + accounting.withdrawal_balance.total, 0)`
    /// See [`Accounting`] for more details regarding the calculation of `accounting.withdrawal_balance.total`.
    pub total_withdrawal_balance: Uint128,

    /// Vault share price is calculated directly from the total withdrawal balance and the shares supply.
    /// `share_price = total_withdrawal_balance / total_shares`
    /// None if `total_shares` is zero.
    pub share_price: Option<Decimal>,

    /// Total liquidity in the base denom aggregated across all markets.
    /// `total_liquidity = max(total_balance + accounting.cash_flow.total, 0)`
    /// See [`Accounting`] for more details regarding the calculation of `accounting.cash_flow.total`.
    pub total_liquidity: Uint128,

    /// Positive total unrealized PnL that the vault owes to the users.
    /// `total_debt = max(total_unrealized_pnl, 0)`
    pub total_debt: Uint128,

    /// Collateralization ratio of the vault.
    /// `collateralization_ratio = total_liquidity / total_debt`
    /// None if `total_debt` is zero.
    pub collateralization_ratio: Option<Decimal>,
}

/// Unlock state for a single user
#[cw_serde]
#[derive(Default)]
pub struct UnlockState {
    pub created_at: u64,
    pub cooldown_end: u64,
    pub shares: Uint128,
}

/// Global state of a single denom
#[cw_serde]
#[derive(Default)]
pub struct MarketState {
    /// Whether the denom is enabled for trading
    pub enabled: bool,

    /// Total LONG open interest
    pub long_oi: Uint128,

    /// Total SHORT open interest
    pub short_oi: Uint128,

    /// The accumulated entry cost, calculated for open positions as:
    /// pos_1_size * pos_1_entry_exec_price + pos_2_size * pos_2_entry_exec_price + ...
    /// if a position is closed, the accumulated entry cost is removed from the accumulator:
    /// pos_1_size * pos_1_entry_exec_price + pos_2_size * pos_2_entry_exec_price + ... - pos_1_size * pos_1_entry_exec_price
    /// pos_2_size * pos_2_entry_exec_price + ...
    pub total_entry_cost: Int128,

    /// The accumulated entry funding, calculated for open positions as:
    /// pos_1_size * pos_1_entry_funding + pos_2_size * pos_2_entry_funding + ...
    /// if a position is closed, the accumulated entry funding is removed from the accumulator:
    /// pos_1_size * pos_1_entry_funding + pos_2_size * pos_2_entry_funding + ... - pos_1_size * pos_1_entry_funding
    /// pos_2_size * pos_2_entry_funding + ...
    pub total_entry_funding: Int128,

    /// The accumulated squared positions, calculated for open positions as:
    /// pos_1_size^2 + pos_2_size^2 + ...
    /// if a position is closed, the accumulated squared position is removed from the accumulator:
    /// pos_1_size^2 + pos_2_size^2 + ... - pos_1_size^2
    pub total_squared_positions: Uint256,

    /// The accumulated absolute multiplied positions, calculated for open positions as:
    /// pos_1_size * |pos_1_size| + pos_2_size * |pos_2_size| + ...
    /// if a position is closed, the accumulated absolute multiplied position is removed from the accumulator:
    /// pos_1_size * |pos_1_size| + pos_2_size * |pos_2_size| + ... - pos_1_size * |pos_1_size|
    pub total_abs_multiplied_positions: Int256,

    /// The actual amount of money, includes only realized payments
    pub cash_flow: CashFlow,

    /// Funding parameters for this denom
    pub funding: Funding,

    /// The last time this denom was updated
    pub last_updated: u64,
}

/// Funding parameters for a single denom.
///
/// The role of funding rates is generally to balance long and short demand.
/// Traders will either pay or receive funding rates, depending on their positions.
/// If the funding rate is positive, long position holders will pay the funding rate to those holding short positions, and vice versa.
#[cw_serde]
pub struct Funding {
    /// Determines the maximum rate at which funding can be adjusted
    pub max_funding_velocity: Decimal,

    /// Determines the funding rate for a given level of skew.
    /// The lower the skew_scale the higher the funding rate.
    pub skew_scale: Uint128,

    /// The current funding rate calculated as an 24-hour rate
    pub last_funding_rate: SignedDecimal,

    /// Last funding accrued per unit
    pub last_funding_accrued_per_unit_in_base_denom: SignedDecimal,
}

impl Default for Funding {
    fn default() -> Self {
        Funding {
            max_funding_velocity: Decimal::zero(),
            skew_scale: Uint128::one(),
            last_funding_rate: SignedDecimal::zero(),
            last_funding_accrued_per_unit_in_base_denom: SignedDecimal::zero(),
        }
    }
}

/// The actual amount of money denominated in the base denom (e.g. UUSDC), includes only realized payments
#[cw_serde]
#[derive(Default)]
pub struct CashFlow {
    pub price_pnl: Int128,
    pub opening_fee: Int128, // This is without the protocol fee
    pub closing_fee: Int128, // This is without the protocol fee
    pub accrued_funding: Int128,
    pub protocol_fee: Uint128, // Used to track the protocol fee. Excluded from the total
}

impl CashFlow {
    /// Calculates the net cashflow for the vault. This is the sum of all cashflows except
    /// the protocol fee.
    pub fn total(&self) -> Result<Int128, MarsError> {
        Ok(self
            .price_pnl
            .checked_add(self.opening_fee)?
            .checked_add(self.closing_fee)?
            .checked_add(self.accrued_funding)?)
    }
}

/// Amount of money denominated in the base denom (e.g. UUSDC) used for accounting
#[cw_serde]
#[derive(Default)]
pub struct Balance {
    pub price_pnl: Int128,
    pub opening_fee: Int128,
    pub closing_fee: Int128,
    pub accrued_funding: Int128,
    pub total: Int128,
}

/// Represents the accounting data for the vault, denominated in the base currency (e.g. UUSDC).
/// If the values are negative, it indicates the vault is losing money.
#[cw_serde]
#[derive(Default)]
pub struct Accounting {
    /// The realized amount of money, only includes completed payments.
    pub cash_flow: CashFlow,

    /// The total balance, which includes both realized and unrealized amounts.
    pub balance: Balance,

    /// The amount available for withdrawal by liquidity providers (LPs).
    /// This value may cap certain unrealized payments.
    pub withdrawal_balance: Balance,
}

/// Represents the vault's accounting data and unrealized profit and loss (PnL), all denominated in the base currency (e.g. UUSDC).
#[cw_serde]
#[derive(Default)]
pub struct AccountingResponse {
    /// The vault's accounting data.
    /// Negative values indicate the vault is losing money, meaning traders are in profit.
    pub accounting: Accounting,

    /// Unrealized PnL amounts for all open positions.
    /// Negative unrealized PnL indicates traders are losing money, meaning the vault is in profit.
    pub unrealized_pnl: PnlAmounts,
}

/// Market state for a single denom
#[cw_serde]
#[derive(Default)]
pub struct MarketResponse {
    pub denom: String,
    pub enabled: bool,
    pub long_oi: Uint128,
    pub short_oi: Uint128,
    pub current_funding_rate: SignedDecimal,
}

/// This is the position data to be stored in the contract state. It does not
/// include PnL, which is to be calculated according to the price at query time.
/// It also does not include the denom, which is indicated by the Map key.
#[cw_serde]
#[derive(Default)]
pub struct Position {
    pub size: Int128,
    pub entry_price: Decimal,
    pub entry_exec_price: Decimal,
    pub entry_accrued_funding_per_unit_in_base_denom: SignedDecimal,
    pub initial_skew: Int128,
    pub realized_pnl: PnlAmounts,
}

/// This is the position data to be returned in a query. It includes current
/// price and PnL.
#[cw_serde]
pub struct PerpPosition {
    pub denom: String,
    pub base_denom: String,
    pub size: Int128,
    pub entry_price: Decimal,
    pub current_price: Decimal,
    pub entry_exec_price: Decimal,
    pub current_exec_price: Decimal,
    pub unrealized_pnl: PnlAmounts,
    pub realized_pnl: PnlAmounts,
}

/// The profit-and-loss of a perp position, denominated in the base currency.
#[cw_serde]
pub enum PnL {
    Profit(Coin),
    Loss(Coin),
    BreakEven,
}

impl PnL {
    pub fn from_signed_uint(denom: impl Into<String>, amount: Int128) -> Self {
        if amount.is_zero() {
            PnL::BreakEven
        } else if amount.is_negative() {
            PnL::Loss(Coin {
                denom: denom.into(),
                amount: amount.unsigned_abs(),
            })
        } else {
            PnL::Profit(Coin {
                denom: denom.into(),
                amount: amount.unsigned_abs(),
            })
        }
    }
}

/// Values denominated in the Oracle base currency (uusd)
#[cw_serde]
#[derive(Default)]
pub struct PnlValues {
    pub price_pnl: Int128,
    pub accrued_funding: Int128,
    pub closing_fee: Int128,

    /// PnL: price PnL + accrued funding + closing fee
    pub pnl: Int128,
}

/// Coins with Perp Vault base denom (uusdc) as a denom
#[cw_serde]
pub struct PnlCoins {
    pub closing_fee: Coin,
    pub pnl: PnL,
}

/// Amounts denominated in the Perp Vault base denom (uusdc)
#[cw_serde]
#[derive(Default)]
pub struct PnlAmounts {
    pub price_pnl: Int128,
    pub accrued_funding: Int128,
    pub opening_fee: Int128,
    pub closing_fee: Int128,

    /// PnL: price PnL + accrued funding + opening fee + closing fee
    pub pnl: Int128,
}

impl PnlAmounts {
    /// Create a new PnL amounts from the opening fee.
    /// It can be used when opening a new position.
    pub fn from_opening_fee(opening_fee: Uint128) -> StdResult<Self> {
        // make opening fee negative to show that it's a cost for the user
        let opening_fee = Int128::zero().checked_sub(opening_fee.try_into()?)?;
        Ok(PnlAmounts {
            opening_fee,
            pnl: opening_fee,
            ..Default::default()
        })
    }

    pub fn add_opening_fee(&mut self, opening_fee: Uint128) -> StdResult<()> {
        // make opening fee negative to show that it's a cost for the user
        let opening_fee = Int128::zero().checked_sub(opening_fee.try_into()?)?;
        self.opening_fee = self.opening_fee.checked_add(opening_fee)?;
        self.pnl = self.pnl.checked_add(opening_fee)?;
        Ok(())
    }

    pub fn add(&mut self, amounts: &PnlAmounts) -> StdResult<()> {
        self.price_pnl = self.price_pnl.checked_add(amounts.price_pnl)?;
        self.accrued_funding = self.accrued_funding.checked_add(amounts.accrued_funding)?;
        self.opening_fee = self.opening_fee.checked_add(amounts.opening_fee)?;
        self.closing_fee = self.closing_fee.checked_add(amounts.closing_fee)?;
        self.pnl = self.pnl.checked_add(amounts.pnl)?;
        Ok(())
    }

    pub fn to_coins(&self, base_denom: &str) -> PnlCoins {
        PnlCoins {
            closing_fee: coin(self.closing_fee.unsigned_abs().u128(), base_denom),
            pnl: PnL::from_signed_uint(base_denom, self.pnl),
        }
    }

    pub fn from_pnl_values(
        pnl_values: PnlValues,
        base_denom_price: Decimal,
    ) -> Result<Self, MarsError> {
        let price_pnl_int256 = Int256::from(pnl_values.price_pnl)
            .checked_multiply_ratio(base_denom_price.denominator(), base_denom_price.numerator())?;
        let price_pnl = Int128::try_from(price_pnl_int256)?;
        let accrued_funding_int256 = Int256::from(pnl_values.accrued_funding)
            .checked_multiply_ratio(base_denom_price.denominator(), base_denom_price.numerator())?;
        let accrued_funding = Int128::try_from(accrued_funding_int256)?;
        let closing_fee_int256 = Int256::from(pnl_values.closing_fee)
            .checked_multiply_ratio(base_denom_price.denominator(), base_denom_price.numerator())?;
        let closing_fee = Int128::try_from(closing_fee_int256)?;
        let pnl = price_pnl.checked_add(accrued_funding)?.checked_add(closing_fee)?;
        Ok(PnlAmounts {
            price_pnl,
            accrued_funding,
            opening_fee: Int128::zero(),
            closing_fee,
            pnl,
        })
    }
}

impl PnL {
    pub fn to_signed_uint(&self) -> StdResult<Int128> {
        let value = match self {
            PnL::Profit(c) => Int128::try_from(c.amount)?,
            PnL::Loss(c) => Int128::zero().checked_sub(Int128::try_from(c.amount)?)?,
            PnL::BreakEven => Int128::zero(),
        };
        Ok(value)
    }
}

impl fmt::Display for PnL {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PnL::Profit(Coin {
                denom,
                amount,
            }) => write!(f, "profit:{denom}:{amount}"),
            PnL::Loss(Coin {
                denom,
                amount,
            }) => write!(f, "loss:{denom}:{amount}"),
            PnL::BreakEven => write!(f, "break_even"),
        }
    }
}

pub type InstantiateMsg = Config<String>;

#[cw_serde]
pub enum ExecuteMsg {
    UpdateOwner(OwnerUpdate),

    /// Provide liquidity of the base token to the vault.
    ///
    /// Must send exactly one coin of `base_denom`.
    ///
    /// The deposited tokens will be used to settle perp trades. liquidity
    /// providers win if traders have negative PnLs, or loss if traders have
    /// positive PnLs.
    Deposit {
        /// The user's credit account token ID.
        /// If account id is provided Credit Manager calls the contract, otherwise a wallet.
        account_id: Option<String>,

        /// The maximum amount of shares received from the deposit action.
        /// This allows the user to protect themselves from unexpected slippage
        /// or directional exposure that can result from the vault having a
        /// negative PNL.
        /// If not provided, defaults to zero.
        max_shares_receivable: Option<Uint128>,
    },

    /// Unlock liquidity from the vault. The unlocked tokens will have to wait
    /// a cooldown period before they can be withdrawn.
    Unlock {
        /// The user's credit account token ID.
        /// If account id is provided Credit Manager calls the contract, otherwise a wallet.
        account_id: Option<String>,

        /// The amount of shares to unlock
        shares: Uint128,
    },

    /// Withdraw liquidity from the vault.
    Withdraw {
        /// The user's credit account token ID.
        /// If account id is provided Credit Manager calls the contract, otherwise a wallet.
        account_id: Option<String>,

        /// The minimum amount of base token to recieve from the withdraw action.
        /// Provided to protect user from unexpected slippage.
        /// If not provided, defaults to zero.
        min_receive: Option<Uint128>,
    },

    /// Execute a perp order against a perp market for a given account.
    /// If the position in that market for that account id exists, it is modified.
    /// If no position exists, a position is created (providing reduce_only is none or false)
    ExecuteOrder {
        account_id: String,
        denom: String,

        // The amount of size to execute against the position.
        // Positive numbers will increase longs and decrease shorts
        // Negative numbers will decrease longs and increase shorts
        size: Int128,

        // Reduce Only enforces a position size cannot increase in absolute terms, ensuring a position will never flip
        // from long to short or vice versa
        reduce_only: Option<bool>,
    },

    /// Close all perp positions. Use this to liquidate a user's credit account.
    ///
    ///
    /// Only callable by Rover credit manager.
    CloseAllPositions {
        account_id: String,
        action: Option<ActionKind>,
    },

    /// Deleveraging a vault by closing a position for an account.
    /// This process helps to increase the Collateralization Ratio (CR) of the vault and/or decrease the maximum Open Interest (max OI) values
    /// (`long_oi_value` and `short_oi_value`).
    ///
    /// The highest unrealized PnL should be closed first. In cases where the maximum OI is exceeded, prioritize closing
    /// the most profitable position that contributes to the exceeded OI (e.g., if long OI is exceeded, close the most profitable long position).
    Deleverage {
        account_id: String,
        denom: String,
    },

    /// Receive updated parameters from the params contract
    UpdateMarket {
        params: PerpParams,
    },

    /// Update the contract's global configuration
    UpdateConfig {
        updates: ConfigUpdates,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Query the owner of the contract.
    #[returns(mars_owner::OwnerResponse)]
    Owner {},

    /// Query the current configuration of the contract.
    #[returns(Config<String>)]
    Config {},

    /// Query the vault state, optionally for a specific action.
    #[returns(VaultResponse)]
    Vault {
        action: Option<ActionKind>,
    },

    /// Query the state of a specific market.
    #[returns(MarketStateResponse)]
    MarketState {
        denom: String,
    },

    /// Query a single market.
    #[returns(MarketResponse)]
    Market {
        denom: String,
    },

    /// Query markets with pagination.
    #[returns(cw_paginate::PaginationResponse<MarketResponse>)]
    Markets {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    /// Query the vault position for a specific user and optional account id.
    #[returns(Option<VaultPositionResponse>)]
    VaultPosition {
        /// User address calling the contract.
        /// It can be the Credit Manager contract or a wallet.
        user_address: String,
        /// The user's credit account token ID.
        /// If account id is provided Credit Manager calls the contract, otherwise a wallet.
        account_id: Option<String>,
    },

    /// Query a single perp position by account and denom.
    #[returns(PositionResponse)]
    Position {
        account_id: String,
        denom: String,
        order_size: Option<Int128>,
    },

    /// List positions of all accounts and denoms.
    #[returns(Vec<PositionResponse>)]
    Positions {
        start_after: Option<(String, String)>,
        limit: Option<u32>,
    },

    /// List positions of all denoms that belong to a specific credit account.
    ///
    /// NOTE: This query does not take a pagination parameter. It always returns
    /// _all_ perp positions that belong to the given account.
    #[returns(PositionsByAccountResponse)]
    PositionsByAccount {
        account_id: String,
        action: Option<ActionKind>,
    },

    /// Query realized PnL amounts for a specific account and market.
    #[returns(PnlAmounts)]
    RealizedPnlByAccountAndMarket {
        account_id: String,
        denom: String,
    },

    /// Query the accounting details for a specific market.
    #[returns(AccountingResponse)]
    MarketAccounting {
        denom: String,
    },

    /// Query the total accounting details across all markets.
    #[returns(AccountingResponse)]
    TotalAccounting {},

    /// Query the opening fee for a given market and position size.
    #[returns(TradingFee)]
    OpeningFee {
        denom: String,
        size: Int128,
    },

    /// Query the fees associated with modifying a specific position.
    #[returns(PositionFeesResponse)]
    PositionFees {
        account_id: String,
        denom: String,
        new_size: Int128,
    },
}

#[cw_serde]
#[derive(Default)]
pub struct MarketStateResponse {
    pub denom: String,

    #[serde(flatten)]
    pub market_state: MarketState,
}

#[cw_serde]
pub struct VaultPositionResponse {
    pub denom: String,
    pub deposit: VaultDeposit,
    pub unlocks: Vec<VaultUnlock>,
}

#[cw_serde]
#[derive(Default)]
pub struct VaultDeposit {
    pub shares: Uint128,
    pub amount: Uint128,
}

#[cw_serde]
#[derive(Default)]
pub struct VaultUnlock {
    pub created_at: u64,
    pub cooldown_end: u64,
    pub shares: Uint128,
    pub amount: Uint128,
}

#[cw_serde]
pub struct PositionResponse {
    pub account_id: String,
    pub position: Option<PerpPosition>,
}

#[cw_serde]
pub struct PositionsByAccountResponse {
    pub account_id: String,
    pub positions: Vec<PerpPosition>,
}

#[cw_serde]
pub struct TradingFee {
    pub rate: Decimal,
    pub fee: Coin,
}

#[cw_serde]
#[derive(Default)]
pub struct PositionFeesResponse {
    /// Denomination of the base asset
    pub base_denom: String,

    /// The fee charged when opening/increasing a position
    pub opening_fee: Uint128,

    /// The fee charged when closing/reducing a position
    pub closing_fee: Uint128,

    /// Opening execution price of the position calculated with:
    /// - entry size if the position is opened
    /// - new size if the position is increased or reduced
    pub opening_exec_price: Option<Decimal>,

    /// Closing execution price of the position calculated with:
    /// - entry size if the position is closed or reduced
    pub closing_exec_price: Option<Decimal>,
}

#[derive(Error, Debug, PartialEq)]
pub enum PerpsError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("{0}")]
    CheckedMultiplyRatio(#[from] CheckedMultiplyRatioError),

    #[error("{0}")]
    CheckedMultiplyFraction(#[from] CheckedMultiplyFractionError),

    #[error("{0}")]
    CheckedFromRatio(#[from] CheckedFromRatioError),

    #[error("{0}")]
    DecimalRangeExceeded(#[from] DecimalRangeExceeded),

    #[error("{0}")]
    ConversionOverflow(#[from] ConversionOverflowError),

    #[error("{0}")]
    SignedDecimalRangeExceeded(#[from] SignedDecimalRangeExceeded),
}
