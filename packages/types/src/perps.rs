use std::{fmt, str::FromStr};

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{coin, Addr, Api, Coin, Decimal, StdResult, Uint128};
use mars_owner::OwnerUpdate;

use crate::{
    adapters::{
        oracle::{OracleBase, OracleUnchecked},
        params::{ParamsBase, ParamsUnchecked},
    },
    error::MarsError,
    math::SignedDecimal,
    oracle::ActionKind,
    params::PerpParams,
    signed_uint::SignedUint,
};

// ------------------------------- message types -------------------------------

/// The perp protocol's global configuration
#[cw_serde]
pub struct Config<T> {
    /// Address of the Mars Rover credit manager (CM) contract.
    ///
    /// Users open, modify, or close perp positions by interacting with the CM.
    /// The CM then invokes the appropriate execute method(s) on the perps
    /// contract to fulfill the user requests.
    pub credit_manager: T,

    /// Adapter for interacting with the Mars oracle contract
    pub oracle: OracleBase<T>,

    /// Adapter for interacting with the Mars params contract
    pub params: ParamsBase<T>,

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
}

impl Config<String> {
    pub fn check(self, api: &dyn Api) -> StdResult<Config<Addr>> {
        Ok(Config {
            address_provider: api.addr_validate(&self.address_provider)?,
            credit_manager: api.addr_validate(&self.credit_manager)?,
            oracle: self.oracle.check(api)?,
            params: self.params.check(api)?,
            base_denom: self.base_denom,
            cooldown_period: self.cooldown_period,
            max_positions: self.max_positions,
            protocol_fee_rate: self.protocol_fee_rate,
            target_vault_collateralization_ratio: self.target_vault_collateralization_ratio,
            deleverage_enabled: self.deleverage_enabled,
            vault_withdraw_enabled: self.vault_withdraw_enabled,
        })
    }
}

impl From<Config<Addr>> for Config<String> {
    fn from(cfg: Config<Addr>) -> Self {
        Config {
            address_provider: cfg.address_provider.into(),
            credit_manager: cfg.credit_manager.into(),
            oracle: cfg.oracle.into(),
            params: cfg.params.into(),
            base_denom: cfg.base_denom,
            cooldown_period: cfg.cooldown_period,
            max_positions: cfg.max_positions,
            protocol_fee_rate: cfg.protocol_fee_rate,
            target_vault_collateralization_ratio: cfg.target_vault_collateralization_ratio,
            deleverage_enabled: cfg.deleverage_enabled,
            vault_withdraw_enabled: cfg.vault_withdraw_enabled,
        }
    }
}
#[cw_serde]
#[derive(Default)]
pub struct ConfigUpdates {
    pub address_provider: Option<String>,
    pub credit_manager: Option<String>,
    pub oracle: Option<OracleUnchecked>,
    pub params: Option<ParamsUnchecked>,
    pub cooldown_period: Option<u64>,
    pub max_positions: Option<u8>,
    pub protocol_fee_rate: Option<Decimal>,
    pub target_vault_collateralization_ratio: Option<Decimal>,
    pub deleverage_enabled: Option<bool>,
    pub vault_withdraw_enabled: Option<bool>,
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
    pub total_balance: SignedUint,

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
    pub total_balance: SignedUint,

    /// Total shares minted to liquidity providers.
    pub total_shares: Uint128,

    /// Total withdrawal balance in the base denom aggregated across all markets.
    /// `total_withdrawal_balance = max(total_balance + accounting.withdrawal_balance.total, 0)`
    /// See [`Accounting`] for more details regarding the calculation of `accounting.withdrawal_balance.total`.
    pub total_withdrawal_balance: Uint128,

    /// Vault share price is calculated directly from the total withdrawal balance and the shares supply.
    /// `share_price = total_withdrawal_balance / total_shares`
    /// None if `total_shares` is zero.
    pub share_price: Option<Decimal>,

    /// Total liquidity in the base denom aggregated across all markets.
    /// `total_liquidity = max(total_balance + accounting.liquidity.total, 0)`
    /// See [`Accounting`] for more details regarding the calculation of `accounting.liquidity.total`.
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
pub struct DenomState {
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
    pub total_entry_cost: SignedUint,

    /// The accumulated entry funding, calculated for open positions as:
    /// pos_1_size * pos_1_entry_funding + pos_2_size * pos_2_entry_funding + ...
    /// if a position is closed, the accumulated entry funding is removed from the accumulator:
    /// pos_1_size * pos_1_entry_funding + pos_2_size * pos_2_entry_funding + ... - pos_1_size * pos_1_entry_funding
    /// pos_2_size * pos_2_entry_funding + ...
    pub total_entry_funding: SignedUint,

    /// The accumulated squared positions, calculated for open positions as:
    /// pos_1_size^2 + pos_2_size^2 + ...
    /// if a position is closed, the accumulated squared position is removed from the accumulator:
    /// pos_1_size^2 + pos_2_size^2 + ... - pos_1_size^2
    pub total_squared_positions: SignedUint, // TODO consider Uint256

    /// The accumulated absolute multiplied positions, calculated for open positions as:
    /// pos_1_size * |pos_1_size| + pos_2_size * |pos_2_size| + ...
    /// if a position is closed, the accumulated absolute multiplied position is removed from the accumulator:
    /// pos_1_size * |pos_1_size| + pos_2_size * |pos_2_size| + ... - pos_1_size * |pos_1_size|
    pub total_abs_multiplied_positions: SignedUint,

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
    pub price_pnl: SignedUint,
    pub opening_fee: SignedUint,
    pub closing_fee: SignedUint,
    pub accrued_funding: SignedUint,
}

impl CashFlow {
    pub fn total(&self) -> Result<SignedUint, MarsError> {
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
    pub price_pnl: SignedUint,
    pub opening_fee: SignedUint,
    pub closing_fee: SignedUint,
    pub accrued_funding: SignedUint,
    pub total: SignedUint,
}

/// Accounting in the base denom (e.g. UUSDC)
#[cw_serde]
#[derive(Default)]
pub struct Accounting {
    /// The actual amount of money, includes only realized payments
    pub cash_flow: CashFlow,

    /// The actual amount of money + unrealized payments
    pub balance: Balance,

    /// The amount of money available for withdrawal by LPs (in this type of balance we cap some unrealized payments)
    pub withdrawal_balance: Balance,
}

/// This is the denom data to be returned in a query. It includes current
/// price, PnL and funding.
#[cw_serde]
#[derive(Default)]
pub struct PerpDenomState {
    pub denom: String,
    pub enabled: bool,
    pub long_oi: Uint128,
    pub short_oi: Uint128,
    pub total_entry_cost: SignedUint,
    pub total_entry_funding: SignedUint,
    pub rate: SignedDecimal,
    pub pnl_values: PnlValues,
    pub funding: Funding,
}

/// This is the position data to be stored in the contract state. It does not
/// include PnL, which is to be calculated according to the price at query time.
/// It also does not include the denom, which is indicated by the Map key.
#[cw_serde]
#[derive(Default)]
pub struct Position {
    pub size: SignedUint,
    pub entry_price: Decimal,
    pub entry_exec_price: Decimal,
    pub entry_accrued_funding_per_unit_in_base_denom: SignedDecimal,
    pub initial_skew: SignedUint,
    pub realized_pnl: PnlAmounts,
}

/// This is the position data to be returned in a query. It includes current
/// price and PnL.
#[cw_serde]
pub struct PerpPosition {
    pub denom: String,
    pub base_denom: String,
    pub size: SignedUint,
    pub entry_price: Decimal,
    pub current_price: Decimal,
    pub entry_exec_price: Decimal,
    pub current_exec_price: Decimal,
    pub unrealised_pnl: PnlAmounts,
    pub realised_pnl: PnlAmounts,
}

/// The profit-and-loss of a perp position, denominated in the base currency.
#[cw_serde]
pub enum PnL {
    Profit(Coin),
    Loss(Coin),
    BreakEven,
}

impl PnL {
    pub fn from_signed_uint(denom: impl Into<String>, amount: SignedUint) -> Self {
        if amount.is_positive() {
            PnL::Profit(Coin {
                denom: denom.into(),
                amount: amount.abs,
            })
        } else if amount.is_negative() {
            PnL::Loss(Coin {
                denom: denom.into(),
                amount: amount.abs,
            })
        } else {
            PnL::BreakEven
        }
    }
}

#[cw_serde]
pub struct PositionPnl {
    pub values: PnlValues,
    pub amounts: PnlAmounts,
    pub coins: PnlCoins,
}

/// Values denominated in the Oracle base currency (uusd)
#[cw_serde]
#[derive(Default)]
pub struct PnlValues {
    pub price_pnl: SignedUint,
    pub accrued_funding: SignedUint,
    pub closing_fee: SignedUint,

    /// PnL: price PnL + accrued funding + closing fee
    pub pnl: SignedUint,
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
    pub price_pnl: SignedUint,
    pub accrued_funding: SignedUint,
    pub opening_fee: SignedUint,
    pub closing_fee: SignedUint,

    /// PnL: price PnL + accrued funding + opening fee + closing fee
    pub pnl: SignedUint,
}

impl PnlAmounts {
    /// Create a new PnL amounts from the opening fee.
    /// It can be used when opening a new position.
    pub fn from_opening_fee(opening_fee: Uint128) -> StdResult<Self> {
        // make opening fee negative to show that it's a cost for the user
        let opening_fee = SignedUint::zero().checked_sub(opening_fee.into())?;
        Ok(PnlAmounts {
            opening_fee,
            pnl: opening_fee,
            ..Default::default()
        })
    }

    pub fn add_opening_fee(&mut self, opening_fee: Uint128) -> StdResult<()> {
        // make opening fee negative to show that it's a cost for the user
        let opening_fee = SignedUint::zero().checked_sub(opening_fee.into())?;
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
            closing_fee: coin(self.closing_fee.abs.u128(), base_denom),
            pnl: PnL::from_signed_uint(base_denom, self.pnl),
        }
    }

    pub fn from_pnl_values(
        pnl_values: PnlValues,
        base_denom_price: Decimal,
    ) -> Result<Self, MarsError> {
        let price_pnl = pnl_values.price_pnl.checked_div_floor(base_denom_price.into())?;
        let accrued_funding =
            pnl_values.accrued_funding.checked_div_floor(base_denom_price.into())?;
        let closing_fee = pnl_values.closing_fee.checked_div_floor(base_denom_price.into())?;
        let pnl = price_pnl.checked_add(accrued_funding)?.checked_add(closing_fee)?;
        Ok(PnlAmounts {
            price_pnl,
            accrued_funding,
            opening_fee: SignedUint::zero(),
            closing_fee,
            pnl,
        })
    }
}

impl PnL {
    pub fn to_signed_uint(&self) -> StdResult<SignedUint> {
        let value = match self {
            PnL::Profit(c) => SignedUint::from_str(c.amount.to_string().as_str())?,
            PnL::Loss(c) => SignedUint::zero().checked_sub(c.amount.into())?,
            PnL::BreakEven => SignedUint::zero(),
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
    },

    /// Execute a perp order against a perp market for a given account.
    /// If the position in that market for that account id exists, it is modified.
    /// If no position exists, a position is created (providing reduce_only is none or false)
    ExecutePerpOrder {
        account_id: String,
        denom: String,

        // The amount of size to execute against the position.
        // Positive numbers will increase longs and decrease shorts
        // Negative numbers will decrease longs and increase shorts
        size: SignedUint,

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
    UpdateParams {
        params: PerpParams,
    },

    UpdateConfig {
        updates: ConfigUpdates,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(mars_owner::OwnerResponse)]
    Owner {},

    #[returns(Config<String>)]
    Config {},

    #[returns(VaultResponse)]
    Vault {
        action: Option<ActionKind>,
    },

    #[returns(DenomStateResponse)]
    DenomState {
        denom: String,
    },

    /// Query a single perp denom state with current calculated PnL, funding etc.
    #[returns(PerpDenomState)]
    PerpDenomState {
        denom: String,
    },

    /// Query a single perp denom state with current calculated PnL, funding etc.
    #[returns(cw_paginate::PaginationResponse<PerpDenomState>)]
    PerpDenomStates {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    /// List all denoms enabled for trading
    #[returns(Vec<DenomStateResponse>)]
    DenomStates {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    #[returns(Option<PerpVaultPosition>)]
    PerpVaultPosition {
        /// User address calling the contract.
        /// It can be the Credit Manager contract or a wallet.
        user_address: String,
        /// The user's credit account token ID.
        /// If account id is provided Credit Manager calls the contract, otherwise a wallet.
        account_id: Option<String>,
    },

    /// Query the amount of deposit made to the vault by a single user
    #[returns(PerpVaultDeposit)]
    Deposit {
        /// User address calling the contract.
        /// It can be the Credit Manager contract or a wallet.
        user_address: String,
        /// The user's credit account token ID.
        /// If account id is provided Credit Manager calls the contract, otherwise a wallet.
        account_id: Option<String>,
    },

    #[returns(Vec<PerpVaultUnlock>)]
    Unlocks {
        /// User address calling the contract.
        /// It can be the Credit Manager contract or a wallet.
        user_address: String,
        /// The user's credit account token ID.
        /// If account id is provided Credit Manager calls the contract, otherwise a wallet.
        account_id: Option<String>,
    },

    /// Query a single perp position by ID
    #[returns(PositionResponse)]
    Position {
        account_id: String,
        denom: String,
        order_size: Option<SignedUint>,
    },

    /// List positions of all accounts and denoms
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

    /// Compute the total PnL of all perp positions, denominated in uusd (USD = 1e6 uusd, configured in Oracle)
    #[returns(SignedDecimal)]
    TotalPnl {},

    #[returns(TradingFee)]
    OpeningFee {
        denom: String,
        size: SignedUint,
    },

    #[returns(Accounting)]
    DenomAccounting {
        denom: String,
    },

    #[returns(Accounting)]
    TotalAccounting {},

    #[returns(PnlAmounts)]
    DenomRealizedPnlForAccount {
        account_id: String,
        denom: String,
    },

    #[returns(PositionFeesResponse)]
    PositionFees {
        account_id: String,
        denom: String,
        new_size: SignedUint,
    },
}

#[cw_serde]
#[derive(Default)]
pub struct DenomStateResponse {
    pub denom: String,
    pub enabled: bool,
    pub total_cost_base: SignedUint,
    pub funding: Funding,
    pub last_updated: u64,
}

#[cw_serde]
pub struct PerpVaultPosition {
    pub denom: String,
    pub deposit: PerpVaultDeposit,
    pub unlocks: Vec<PerpVaultUnlock>,
}

#[cw_serde]
#[derive(Default)]
pub struct PerpVaultDeposit {
    pub shares: Uint128,
    pub amount: Uint128,
}

#[cw_serde]
#[derive(Default)]
pub struct PerpVaultUnlock {
    pub created_at: u64,
    pub cooldown_end: u64,
    pub shares: Uint128,
    pub amount: Uint128,
}

#[cw_serde]
pub struct DebtResponse {
    pub account_id: String,
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
