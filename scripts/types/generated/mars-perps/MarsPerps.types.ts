// @ts-nocheck
/**
 * This file was automatically generated by @cosmwasm/ts-codegen@0.35.3.
 * DO NOT MODIFY IT BY HAND. Instead, modify the source JSONSchema file,
 * and run the @cosmwasm/ts-codegen generate command to regenerate this file.
 */

export type OracleBaseForString = string
export type ParamsBaseForString = string
export interface InstantiateMsg {
  base_denom: string
  cooldown_period: number
  credit_manager: string
  max_positions: number
  oracle: OracleBaseForString
  params: ParamsBaseForString
}
export type ExecuteMsg =
  | {
      update_owner: OwnerUpdate
    }
  | {
      init_denom: {
        denom: string
        max_funding_velocity: Decimal
        skew_scale: Uint128
      }
    }
  | {
      enable_denom: {
        denom: string
      }
    }
  | {
      disable_denom: {
        denom: string
      }
    }
  | {
      deposit: {
        account_id?: string | null
      }
    }
  | {
      unlock: {
        account_id?: string | null
        shares: Uint128
      }
    }
  | {
      withdraw: {
        account_id?: string | null
      }
    }
  | {
      open_position: {
        account_id: string
        denom: string
        size: SignedUint
      }
    }
  | {
      close_position: {
        account_id: string
        denom: string
      }
    }
  | {
      modify_position: {
        account_id: string
        denom: string
        new_size: SignedUint
      }
    }
  | {
      close_all_positions: {
        account_id: string
        action?: ActionKind | null
      }
    }
export type OwnerUpdate =
  | {
      propose_new_owner: {
        proposed: string
      }
    }
  | 'clear_proposed'
  | 'accept_proposed'
  | 'abolish_owner_role'
  | {
      set_emergency_owner: {
        emergency_owner: string
      }
    }
  | 'clear_emergency_owner'
export type Decimal = string
export type Uint128 = string
export type ActionKind = 'default' | 'liquidation'
export interface SignedUint {
  abs: Uint128
  negative: boolean
  [k: string]: unknown
}
export type QueryMsg =
  | {
      owner: {}
    }
  | {
      config: {}
    }
  | {
      vault_state: {}
    }
  | {
      denom_state: {
        denom: string
      }
    }
  | {
      perp_denom_state: {
        denom: string
      }
    }
  | {
      denom_states: {
        limit?: number | null
        start_after?: string | null
      }
    }
  | {
      perp_vault_position: {
        account_id?: string | null
        action?: ActionKind | null
        user_address: string
      }
    }
  | {
      deposit: {
        account_id?: string | null
        user_address: string
      }
    }
  | {
      unlocks: {
        account_id?: string | null
        user_address: string
      }
    }
  | {
      position: {
        account_id: string
        denom: string
        new_size?: SignedUint | null
      }
    }
  | {
      positions: {
        limit?: number | null
        start_after?: [string, string] | null
      }
    }
  | {
      positions_by_account: {
        account_id: string
        action?: ActionKind | null
      }
    }
  | {
      total_pnl: {}
    }
  | {
      opening_fee: {
        denom: string
        size: SignedUint
      }
    }
  | {
      denom_accounting: {
        denom: string
      }
    }
  | {
      total_accounting: {}
    }
  | {
      denom_realized_pnl_for_account: {
        account_id: string
        denom: string
      }
    }
  | {
      position_fees: {
        account_id: string
        denom: string
        new_size: SignedUint
      }
    }
export interface ConfigForString {
  base_denom: string
  cooldown_period: number
  credit_manager: string
  max_positions: number
  oracle: OracleBaseForString
  params: ParamsBaseForString
}
export interface Accounting {
  balance: Balance
  cash_flow: CashFlow
  withdrawal_balance: Balance
}
export interface Balance {
  accrued_funding: SignedUint
  closing_fee: SignedUint
  opening_fee: SignedUint
  price_pnl: SignedUint
  total: SignedUint
}
export interface CashFlow {
  accrued_funding: SignedUint
  closing_fee: SignedUint
  opening_fee: SignedUint
  price_pnl: SignedUint
}
export interface PnlAmounts {
  accrued_funding: SignedUint
  closing_fee: SignedUint
  opening_fee: SignedUint
  pnl: SignedUint
  price_pnl: SignedUint
}
export interface DenomStateResponse {
  denom: string
  enabled: boolean
  funding: Funding
  last_updated: number
  total_cost_base: SignedUint
}
export interface Funding {
  last_funding_accrued_per_unit_in_base_denom: SignedDecimal
  last_funding_rate: SignedDecimal
  max_funding_velocity: Decimal
  skew_scale: Uint128
}
export interface SignedDecimal {
  abs: Decimal
  negative: boolean
  [k: string]: unknown
}
export type ArrayOfDenomStateResponse = DenomStateResponse[]
export interface PerpVaultDeposit {
  amount: Uint128
  shares: Uint128
}
export interface TradingFee {
  fee: Coin
  rate: Decimal
}
export interface Coin {
  amount: Uint128
  denom: string
  [k: string]: unknown
}
export interface OwnerResponse {
  abolished: boolean
  emergency_owner?: string | null
  initialized: boolean
  owner?: string | null
  proposed?: string | null
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
export interface PnlValues {
  accrued_funding: SignedUint
  closing_fee: SignedUint
  pnl: SignedUint
  price_pnl: SignedUint
}
export type NullablePerpVaultPosition = PerpVaultPosition | null
export interface PerpVaultPosition {
  denom: string
  deposit: PerpVaultDeposit
  unlocks: PerpVaultUnlock[]
}
export interface PerpVaultUnlock {
  amount: Uint128
  cooldown_end: number
  created_at: number
  shares: Uint128
}
export interface PositionResponse {
  account_id: string
  position: PerpPosition
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
export interface PositionFeesResponse {
  base_denom: string
  closing_exec_price?: Decimal | null
  closing_fee: Uint128
  opening_exec_price?: Decimal | null
  opening_fee: Uint128
}
export type ArrayOfPositionResponse = PositionResponse[]
export interface PositionsByAccountResponse {
  account_id: string
  positions: PerpPosition[]
}
export type ArrayOfPerpVaultUnlock = PerpVaultUnlock[]
export interface VaultState {
  total_liquidity: Uint128
  total_shares: Uint128
}
