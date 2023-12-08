use std::{fmt, str::FromStr};

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Api, CheckedFromRatioError, Coin, Decimal, StdResult, Uint128};
use mars_owner::OwnerUpdate;

use crate::{adapters::oracle::OracleBase, math::SignedDecimal};

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
    pub min_position_value: Uint128,

    /// Stakers need to wait a cooldown period before being able to withdraw USDC from the vault.
    /// Value defined in seconds.
    pub cooldown_period: u64,
}

impl Config<String> {
    pub fn check(self, api: &dyn Api) -> StdResult<Config<Addr>> {
        Ok(Config {
            credit_manager: api.addr_validate(&self.credit_manager)?,
            oracle: self.oracle.check(api)?,
            base_denom: self.base_denom,
            min_position_value: self.min_position_value,
            cooldown_period: self.cooldown_period,
        })
    }
}

impl From<Config<Addr>> for Config<String> {
    fn from(cfg: Config<Addr>) -> Self {
        Config {
            credit_manager: cfg.credit_manager.into(),
            oracle: cfg.oracle.into(),
            base_denom: cfg.base_denom,
            min_position_value: cfg.min_position_value,
            cooldown_period: cfg.cooldown_period,
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
    pub enabled: bool,
    pub total_size: SignedDecimal,
    // this isn't really cost basis in the typical meaning, but I can't think of
    // a better term yet
    pub total_cost_base: SignedDecimal,
    pub funding: Funding,
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

    /// Determines the funding rate for a given level of skew, as per the function:
    /// funding_rate_velocity = max_funding_velocity * skew / skew_scale
    /// The lower the skew_scale the higher the funding rate.
    pub skew_scale: Decimal,

    /// The constant_factor: max_funding_velocity / skew_scale
    pub constant_factor: SignedDecimal,

    /// The current funding rate calculated as an 24-hour rate. It is calculated as:
    /// rate = prev_rate + (funding_rate_velocity * (current_time_sec - prev_time_sec) / (ONE_DAY_SEC))
    pub rate: SignedDecimal,

    /// The current funding index calculated as:
    /// index = prev_index * (1 + current_rate)
    pub index: SignedDecimal,

    /// The accumulated size weighted by the index, calculated for open positions as:
    /// pos_1_size / pos_1_index + pos_2_size / pos_2_index + ...
    /// if a position is closed, the accumulated position is removed from the accumulator:
    /// pos_1_size / pos_1_index + pos_2_size / pos_2_index + ... - pos_1_size / pos_1_index
    /// pos_2_size / pos_2_index + ...
    pub accumulated_size_weighted_by_index: SignedDecimal,
}

impl Default for Funding {
    fn default() -> Self {
        Funding {
            max_funding_velocity: Decimal::zero(),
            skew_scale: Decimal::one(),
            constant_factor: SignedDecimal::zero(),
            rate: SignedDecimal::zero(),
            index: SignedDecimal::one(),
            accumulated_size_weighted_by_index: SignedDecimal::zero(),
        }
    }
}

impl Funding {
    pub fn constant_factor(
        max_funding_velocity: Decimal,
        skew_scale: Decimal,
    ) -> Result<SignedDecimal, CheckedFromRatioError> {
        let c: SignedDecimal = max_funding_velocity.checked_div(skew_scale)?.into();
        Ok(c)
    }
}

/// This is the denom data to be returned in a query. It includes current
/// price, PnL and funding.
#[cw_serde]
pub struct PerpDenomState {
    pub denom: String,
    pub enabled: bool,
    pub total_size: SignedDecimal,
    pub total_cost_base: SignedDecimal,
    pub constant_factor: SignedDecimal,
    pub rate: SignedDecimal,
    pub index: SignedDecimal,
    pub pnl_values: PnlValues,
}

/// This is the position data to be stored in the contract state. It does not
/// include PnL, which is to be calculated according to the price at query time.
/// It also does not include the denom, which is indicated by the Map key.
#[cw_serde]
pub struct Position {
    pub size: SignedDecimal,
    pub entry_price: Decimal,
    pub entry_funding_index: SignedDecimal,
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
    pub pnl: PnL,
    pub unrealised_funding_accrued: SignedDecimal,
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
pub struct PnlValues {
    pub unrealized_pnl: SignedDecimal,
    pub accrued_funding: SignedDecimal,

    /// The total PnL: unrealized PnL - accrued funding
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
}

#[cw_serde]
#[derive(Default)]
pub struct DenomStateResponse {
    pub denom: String,
    pub enabled: bool,
    pub total_size: SignedDecimal,
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
