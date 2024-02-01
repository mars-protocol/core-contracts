use std::{fmt, str::FromStr};

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Api, Coin, Decimal, StdResult, Uint128};
use mars_owner::OwnerUpdate;

use crate::{
    adapters::{oracle::OracleBase, params::ParamsBase},
    math::SignedDecimal,
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

    /// The minimum value of a position, in the base asset (i.e. USDC).
    pub min_position_in_base_denom: Uint128,

    /// The maximum value of a position, in the base asset (i.e. USDC).
    pub max_position_in_base_denom: Option<Uint128>,

    /// Stakers need to wait a cooldown period before being able to withdraw USDC from the vault.
    /// Value defined in seconds.
    pub cooldown_period: u64,

    /// The fee rate charged when opening a new position
    pub opening_fee_rate: Decimal,

    /// The fee rate charged when closing a position
    pub closing_fee_rate: Decimal,
}

impl Config<String> {
    pub fn check(self, api: &dyn Api) -> StdResult<Config<Addr>> {
        Ok(Config {
            credit_manager: api.addr_validate(&self.credit_manager)?,
            oracle: self.oracle.check(api)?,
            params: self.params.check(api)?,
            base_denom: self.base_denom,
            min_position_in_base_denom: self.min_position_in_base_denom,
            max_position_in_base_denom: self.max_position_in_base_denom,
            cooldown_period: self.cooldown_period,
            opening_fee_rate: self.opening_fee_rate,
            closing_fee_rate: self.closing_fee_rate,
        })
    }
}

impl From<Config<Addr>> for Config<String> {
    fn from(cfg: Config<Addr>) -> Self {
        Config {
            credit_manager: cfg.credit_manager.into(),
            oracle: cfg.oracle.into(),
            params: cfg.params.into(),
            base_denom: cfg.base_denom,
            min_position_in_base_denom: cfg.min_position_in_base_denom,
            max_position_in_base_denom: cfg.max_position_in_base_denom,
            cooldown_period: cfg.cooldown_period,
            opening_fee_rate: cfg.opening_fee_rate,
            closing_fee_rate: cfg.closing_fee_rate,
        }
    }
}

/// Global state of the counterparty vault
#[cw_serde]
#[derive(Default)]
pub struct VaultState {
    pub total_liquidity: Uint128,
    pub total_shares: Uint128,
}

/// Unlock state for a single user
#[cw_serde]
#[derive(Default)]
pub struct UnlockState {
    pub created_at: u64,
    pub cooldown_end: u64,
    pub amount: Uint128,
}

/// Global state of a single denom
#[cw_serde]
#[derive(Default)]
pub struct DenomState {
    /// Whether the denom is enabled for trading
    pub enabled: bool,

    /// Total LONG open interest
    pub long_oi: Decimal,

    /// Total SHORT open interest
    pub short_oi: Decimal,

    /// The accumulated entry cost, calculated for open positions as:
    /// pos_1_size * pos_1_entry_exec_price + pos_2_size * pos_2_entry_exec_price + ...
    /// if a position is closed, the accumulated entry cost is removed from the accumulator:
    /// pos_1_size * pos_1_entry_exec_price + pos_2_size * pos_2_entry_exec_price + ... - pos_1_size * pos_1_entry_exec_price
    /// pos_2_size * pos_2_entry_exec_price + ...
    pub total_entry_cost: SignedDecimal,

    /// The accumulated entry funding, calculated for open positions as:
    /// pos_1_size * pos_1_entry_funding + pos_2_size * pos_2_entry_funding + ...
    /// if a position is closed, the accumulated entry funding is removed from the accumulator:
    /// pos_1_size * pos_1_entry_funding + pos_2_size * pos_2_entry_funding + ... - pos_1_size * pos_1_entry_funding
    /// pos_2_size * pos_2_entry_funding + ...
    pub total_entry_funding: SignedDecimal,

    /// The accumulated squared positions, calculated for open positions as:
    /// pos_1_size^2 + pos_2_size^2 + ...
    /// if a position is closed, the accumulated squared position is removed from the accumulator:
    /// pos_1_size^2 + pos_2_size^2 + ... - pos_1_size^2
    pub total_squared_positions: SignedDecimal,

    /// The accumulated absolute multiplied positions, calculated for open positions as:
    /// pos_1_size * |pos_1_size| + pos_2_size * |pos_2_size| + ...
    /// if a position is closed, the accumulated absolute multiplied position is removed from the accumulator:
    /// pos_1_size * |pos_1_size| + pos_2_size * |pos_2_size| + ... - pos_1_size * |pos_1_size|
    pub total_abs_multiplied_positions: SignedDecimal,

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
    pub skew_scale: Decimal,

    /// The current funding rate calculated as an 24-hour rate
    pub last_funding_rate: SignedDecimal,

    /// Last funding accrued per unit
    pub last_funding_accrued_per_unit_in_base_denom: SignedDecimal,
}

impl Default for Funding {
    fn default() -> Self {
        Funding {
            max_funding_velocity: Decimal::zero(),
            skew_scale: Decimal::one(),
            last_funding_rate: SignedDecimal::zero(),
            last_funding_accrued_per_unit_in_base_denom: SignedDecimal::zero(),
        }
    }
}

/// The actual amount of money denominated in the base denom (e.g. UUSDC), includes only realized payments
#[cw_serde]
#[derive(Default)]
pub struct CashFlow {
    pub price_pnl: SignedDecimal,
    pub opening_fees: SignedDecimal,
    pub closing_fees: SignedDecimal,
    pub accrued_funding: SignedDecimal,
}

/// Amount of money denominated in the base denom (e.g. UUSDC) used for accounting
#[cw_serde]
#[derive(Default)]
pub struct Balance {
    pub price_pnl: SignedDecimal,
    pub opening_fees: SignedDecimal,
    pub closing_fees: SignedDecimal,
    pub accrued_funding: SignedDecimal,
    pub total: SignedDecimal,
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
pub struct PerpDenomState {
    pub denom: String,
    pub enabled: bool,
    pub total_entry_cost: SignedDecimal,
    pub total_entry_funding: SignedDecimal,
    pub rate: SignedDecimal,
    pub pnl_values: DenomPnlValues,
}

/// This is the position data to be stored in the contract state. It does not
/// include PnL, which is to be calculated according to the price at query time.
/// It also does not include the denom, which is indicated by the Map key.
#[cw_serde]
pub struct Position {
    pub size: SignedDecimal,
    pub entry_price: Decimal,
    pub entry_accrued_funding_per_unit_in_base_denom: SignedDecimal,
    pub initial_skew: SignedDecimal,
    pub opening_fee_in_base_denom: Uint128,
}

/// This is the position data to be returned in a query. It includes current
/// price and PnL.
#[cw_serde]
pub struct PerpPosition {
    pub denom: String,
    pub base_denom: String,
    pub size: SignedDecimal,
    pub entry_price: Decimal,
    pub current_price: Decimal,
    pub pnl: PositionPnl,
    pub closing_fee_rate: Decimal,
}

/// The profit-and-loss of a perp position, denominated in the base currency.
#[cw_serde]
pub enum PnL {
    Profit(Coin),
    Loss(Coin),
    BreakEven,
}

impl PnL {
    pub fn from_signed_decimal(denom: impl Into<String>, amount: SignedDecimal) -> Self {
        if amount.is_positive() {
            PnL::Profit(Coin {
                denom: denom.into(),
                amount: amount.abs.to_uint_floor(),
            })
        } else if amount.is_negative() {
            PnL::Loss(Coin {
                denom: denom.into(),
                amount: amount.abs.to_uint_floor(),
            })
        } else {
            PnL::BreakEven
        }
    }
}

#[cw_serde]
pub struct PositionPnl {
    pub values: PnlValues,
    pub coins: PnlCoins,
}

/// Values denominated in the Oracle base currency (uusd)
#[cw_serde]
pub struct PnlValues {
    pub price_pnl: SignedDecimal,
    pub accrued_funding: SignedDecimal,
    pub closing_fee: SignedDecimal,

    /// PnL: price PnL + accrued funding + closing fee
    pub pnl: SignedDecimal,
}

/// Coins with Perp Vault base denom (uusdc) as a denom
#[cw_serde]
pub struct PnlCoins {
    pub closing_fee: Coin,
    pub pnl: PnL,
}

/// Amounts denominated in the Perp Vault base denom (uusdc)
#[cw_serde]
pub struct PnlAmounts {
    pub price_pnl: SignedDecimal,
    pub accrued_funding: SignedDecimal,
    pub closing_fee: SignedDecimal,

    /// PnL: price PnL + accrued funding + closing fee
    pub pnl: SignedDecimal,
}

/// Amounts denominated in the Perp Vault base denom (uusdc)
#[cw_serde]
#[derive(Default)]
pub struct RealizedPnlAmounts {
    pub price_pnl: SignedDecimal,
    pub accrued_funding: SignedDecimal,
    pub opening_fee: SignedDecimal,
    pub closing_fee: SignedDecimal,

    /// PnL: price PnL + accrued funding + opening fee + closing fee
    pub pnl: SignedDecimal,
}

impl RealizedPnlAmounts {
    pub fn from_pnl_amounts(amounts: &PnlAmounts, opening_fee: Uint128) -> StdResult<Self> {
        // make opening fee negative to show that it's a cost for the user
        let opening_fee = SignedDecimal::zero().checked_sub(opening_fee.into())?;
        Ok(RealizedPnlAmounts {
            price_pnl: amounts.price_pnl,
            accrued_funding: amounts.accrued_funding,
            opening_fee,
            closing_fee: amounts.closing_fee,
            pnl: amounts.pnl.checked_add(opening_fee)?,
        })
    }

    pub fn update(&mut self, amounts: &PnlAmounts, opening_fee: Uint128) -> StdResult<()> {
        let realized_amounts = Self::from_pnl_amounts(amounts, opening_fee)?;
        self.price_pnl = self.price_pnl.checked_add(realized_amounts.price_pnl)?;
        self.accrued_funding =
            self.accrued_funding.checked_add(realized_amounts.accrued_funding)?;
        self.opening_fee = self.opening_fee.checked_add(realized_amounts.opening_fee)?;
        self.closing_fee = self.closing_fee.checked_add(realized_amounts.closing_fee)?;
        self.pnl = self.pnl.checked_add(realized_amounts.pnl)?;
        Ok(())
    }
}

impl PnL {
    pub fn to_signed_decimal(&self) -> StdResult<SignedDecimal> {
        let value = match self {
            PnL::Profit(c) => SignedDecimal::from_str(c.amount.to_string().as_str())?,
            PnL::Loss(c) => SignedDecimal::zero()
                .checked_sub(SignedDecimal::from_str(c.amount.to_string().as_str())?)?,
            PnL::BreakEven => SignedDecimal::zero(),
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

/// PnL values denominated in the base currency
#[cw_serde]
#[derive(Default)]
pub struct DenomPnlValues {
    pub price_pnl: SignedDecimal,
    pub closing_fees: SignedDecimal,
    pub accrued_funding: SignedDecimal,

    /// The total PnL: price_pnl + closing_fees + accrued_funding
    pub pnl: SignedDecimal,
}

pub type InstantiateMsg = Config<String>;

#[cw_serde]
pub enum ExecuteMsg {
    UpdateOwner(OwnerUpdate),

    /// Init a denom to be traded.
    ///
    /// Only callable by the owner.
    InitDenom {
        denom: String,
        max_funding_velocity: Decimal,
        skew_scale: Decimal,
    },

    /// Enable a denom to be traded.
    ///
    /// Only callable by the owner.
    EnableDenom {
        denom: String,
    },

    /// Disable a denom from being traded.
    ///
    /// Once disabled, perp positions with this denom can only be closed.
    ///
    /// Only callable by the owner.
    DisableDenom {
        denom: String,
    },

    /// Provide liquidity of the base token to the vault.
    ///
    /// Must send exactly one coin of `base_denom`.
    ///
    /// The deposited tokens will be used to settle perp trades. liquidity
    /// providers win if traders have negative PnLs, or loss if traders have
    /// positive PnLs.
    Deposit {},

    /// Unlock liquidity from the vault. The unlocked tokens will have to wait
    /// a cooldown period before they can be withdrawn.
    Unlock {
        shares: Uint128,
    },

    /// Withdraw liquidity from the vault.
    Withdraw {},

    /// Open a new perp position.
    ///
    /// Only callable by Rover credit manager.
    ///
    /// Must send exactly one coin of `base_denom`.
    OpenPosition {
        /// The user's credit account token ID
        account_id: String,

        /// Name of the trading pair
        denom: String,

        /// Size of the position, denominated in the traded asset.
        ///
        /// A positive number means the position is long, a negative number
        /// means it's short.
        ///
        /// Must be greater than the minimum position size set at the protocol
        /// level.
        size: SignedDecimal,
    },

    /// Close a perp position. Return collateral + unrealized PnL to the user's
    /// credit account.
    ///
    /// Only callable by Rover credit manager.
    ClosePosition {
        account_id: String,
        denom: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(mars_owner::OwnerResponse)]
    Owner {},

    #[returns(Config<String>)]
    Config {},

    #[returns(VaultState)]
    VaultState {},

    // TODO: in case a denom is not found, should we throw an error (the current
    // behavior) or return a None?
    #[returns(DenomStateResponse)]
    DenomState {
        denom: String,
    },

    /// Query a single perp denom state with current calculated PnL, funding etc.
    #[returns(PerpDenomState)]
    PerpDenomState {
        denom: String,
    },

    /// List all denoms enabled for trading
    #[returns(Vec<DenomStateResponse>)]
    DenomStates {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    /// Query the amount of deposit made to the vault by a single user
    //
    // TODO: in case a deposit is not found, should we return zero (the current
    // behavior) or throw an error?
    #[returns(DepositResponse)]
    Deposit {
        depositor: String,
    },

    /// List all deposits to the vault
    #[returns(Vec<DepositResponse>)]
    Deposits {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    #[returns(Vec<UnlockState>)]
    Unlocks {
        depositor: String,
    },

    /// Query a single perp position by ID
    #[returns(PositionResponse)]
    Position {
        account_id: String,
        denom: String,
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
    },

    /// Compute the total PnL of all perp positions, denominated in uusd (USD = 1e6 uusd, configured in Oracle)
    #[returns(SignedDecimal)]
    TotalPnl {},

    #[returns(TradingFee)]
    OpeningFee {
        denom: String,
        size: SignedDecimal,
    },

    #[returns(Accounting)]
    DenomAccounting {
        denom: String,
    },

    #[returns(Accounting)]
    TotalAccounting {},

    #[returns(RealizedPnlAmounts)]
    DenomRealizedPnlForAccount {
        account_id: String,
        denom: String,
    },
}

#[cw_serde]
#[derive(Default)]
pub struct DenomStateResponse {
    pub denom: String,
    pub enabled: bool,
    pub total_cost_base: SignedDecimal,
    pub funding: Funding,
    pub last_updated: u64,
}

#[cw_serde]
pub struct DepositResponse {
    pub depositor: String,
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
    pub position: PerpPosition,
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
