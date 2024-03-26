// @ts-nocheck
/**
 * This file was automatically generated by @cosmwasm/ts-codegen@0.35.3.
 * DO NOT MODIFY IT BY HAND. Instead, modify the source JSONSchema file,
 * and run the @cosmwasm/ts-codegen generate command to regenerate this file.
 */

export type Decimal = string
export type HlsAssetTypeForAddr =
  | {
      coin: {
        denom: string
      }
    }
  | {
      vault: {
        addr: Addr
      }
    }
export type Addr = string
export type Uint128 = string
export type AccountKind = 'default' | 'high_levered_strategy'
export type VaultPositionAmount =
  | {
      unlocked: VaultAmount
    }
  | {
      locking: LockingVaultAmount
    }
export type VaultAmount = string
export type VaultAmount1 = string
export type UnlockingPositions = VaultUnlockingPosition[]
export interface HealthComputer {
  asset_params: {
    [k: string]: AssetParamsBaseForAddr
  }
  kind: AccountKind
  oracle_prices: {
    [k: string]: Decimal
  }
  perps_data: PerpsData
  positions: Positions
  vaults_data: VaultsData
}
export interface AssetParamsBaseForAddr {
  close_factor: Decimal
  credit_manager: CmSettingsForAddr
  denom: string
  deposit_cap: Uint128
  liquidation_bonus: LiquidationBonus
  liquidation_threshold: Decimal
  max_loan_to_value: Decimal
  protocol_liquidation_fee: Decimal
  red_bank: RedBankSettings
}
export interface CmSettingsForAddr {
  hls?: HlsParamsBaseForAddr | null
  whitelisted: boolean
}
export interface HlsParamsBaseForAddr {
  correlations: HlsAssetTypeForAddr[]
  liquidation_threshold: Decimal
  max_loan_to_value: Decimal
}
export interface LiquidationBonus {
  max_lb: Decimal
  min_lb: Decimal
  slope: Decimal
  starting_lb: Decimal
}
export interface RedBankSettings {
  borrow_enabled: boolean
  deposit_enabled: boolean
}
export interface PerpsData {
  denom_states: {
    [k: string]: PerpDenomState
  }
  params: {
    [k: string]: PerpParams
  }
}
export interface PerpDenomState {
  denom: string
  enabled: boolean
  funding: Funding
  long_oi: Uint128
  pnl_values: PnlValues
  rate: SignedDecimal
  short_oi: Uint128
  total_entry_cost: SignedUint
  total_entry_funding: SignedUint
}
export interface Funding {
  last_funding_accrued_per_unit_in_base_denom: SignedUint
  last_funding_rate: SignedDecimal
  max_funding_velocity: Decimal
  skew_scale: Uint128
}
export interface SignedUint {
  abs: Uint128
  negative: boolean
  [k: string]: unknown
}
export interface SignedDecimal {
  abs: Decimal
  negative: boolean
  [k: string]: unknown
}
export interface PnlValues {
  accrued_funding: SignedUint
  closing_fee: SignedUint
  pnl: SignedUint
  price_pnl: SignedUint
}
export interface PerpParams {
  closing_fee_rate: Decimal
  denom: string
  liquidation_threshold: Decimal
  max_loan_to_value: Decimal
  max_long_oi_value: Uint128
  max_net_oi_value: Uint128
  max_position_value?: Uint128 | null
  max_short_oi_value: Uint128
  min_position_value: Uint128
  opening_fee_rate: Decimal
}
export interface Positions {
  account_id: string
  debts: DebtAmount[]
  deposits: Coin[]
  lends: Coin[]
  perp_vault?: PerpVaultPosition | null
  perps: PerpPosition[]
  vaults: VaultPosition[]
}
export interface DebtAmount {
  amount: Uint128
  denom: string
  shares: Uint128
}
export interface Coin {
  amount: Uint128
  denom: string
  [k: string]: unknown
}
export interface PerpVaultPosition {
  denom: string
  deposit: PerpVaultDeposit
  unlocks: UnlockState[]
}
export interface PerpVaultDeposit {
  amount: Uint128
  shares: Uint128
}
export interface UnlockState {
  amount: Uint128
  cooldown_end: number
  created_at: number
}
export interface PerpPosition {
  base_denom: string
  closing_fee_rate: Decimal
  current_exec_price: Decimal
  current_price: Decimal
  denom: string
  entry_exec_price: Decimal
  entry_price: Decimal
  realised_pnl: PnlAmounts
  size: SignedUint
  unrealised_pnl: PnlAmounts
}
export interface PnlAmounts {
  accrued_funding: SignedUint
  closing_fee: SignedUint
  opening_fee: SignedUint
  pnl: SignedUint
  price_pnl: SignedUint
}
export interface VaultPosition {
  amount: VaultPositionAmount
  vault: VaultBaseForAddr
}
export interface LockingVaultAmount {
  locked: VaultAmount1
  unlocking: UnlockingPositions
}
export interface VaultUnlockingPosition {
  coin: Coin
  id: number
}
export interface VaultBaseForAddr {
  address: Addr
}
export interface VaultsData {
  vault_configs: {
    [k: string]: VaultConfigBaseForAddr
  }
  vault_values: {
    [k: string]: VaultPositionValue
  }
}
export interface VaultConfigBaseForAddr {
  addr: Addr
  deposit_cap: Coin
  hls?: HlsParamsBaseForAddr | null
  liquidation_threshold: Decimal
  max_loan_to_value: Decimal
  whitelisted: boolean
}
export interface VaultPositionValue {
  base_coin: CoinValue
  vault_coin: CoinValue
}
export interface CoinValue {
  amount: Uint128
  denom: string
  value: Uint128
}
