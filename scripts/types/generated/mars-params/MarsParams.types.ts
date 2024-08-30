// @ts-nocheck
/**
 * This file was automatically generated by @cosmwasm/ts-codegen@1.10.0.
 * DO NOT MODIFY IT BY HAND. Instead, modify the source JSONSchema file,
 * and run the @cosmwasm/ts-codegen generate command to regenerate this file.
 */

export interface InstantiateMsg {
  address_provider: string
  owner: string
}
export type ExecuteMsg =
  | {
      update_owner: OwnerUpdate
    }
  | {
      update_config: {
        address_provider?: string | null
      }
    }
  | {
      update_asset_params: AssetParamsUpdate
    }
  | {
      update_vault_config: VaultConfigUpdate
    }
  | {
      update_perp_params: PerpParamsUpdate
    }
  | {
      emergency_update: EmergencyUpdate
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
export type AssetParamsUpdate = {
  add_or_update: {
    params: AssetParamsBaseForString
  }
}
export type Decimal = string
export type HlsAssetTypeForString =
  | {
      coin: {
        denom: string
      }
    }
  | {
      vault: {
        addr: string
      }
    }
export type Uint128 = string
export type VaultConfigUpdate = {
  add_or_update: {
    config: VaultConfigBaseForString
  }
}
export type PerpParamsUpdate = {
  add_or_update: {
    params: PerpParams
  }
}
export type EmergencyUpdate =
  | {
      credit_manager: CmEmergencyUpdate
    }
  | {
      red_bank: RedBankEmergencyUpdate
    }
  | {
      perps: PerpsEmergencyUpdate
    }
export type CmEmergencyUpdate =
  | {
      set_zero_max_ltv_on_vault: string
    }
  | {
      set_zero_deposit_cap_on_vault: string
    }
  | {
      disallow_coin: string
    }
  | {
      disable_withdraw: string
    }
export type RedBankEmergencyUpdate =
  | {
      disable_borrowing: string
    }
  | {
      disable_withdraw: string
    }
export type PerpsEmergencyUpdate =
  | {
      disable_trading: string
    }
  | {
      disable_deleverage: []
    }
export interface AssetParamsBaseForString {
  close_factor: Decimal
  credit_manager: CmSettingsForString
  denom: string
  deposit_cap: Uint128
  liquidation_bonus: LiquidationBonus
  liquidation_threshold: Decimal
  max_loan_to_value: Decimal
  protocol_liquidation_fee: Decimal
  red_bank: RedBankSettings
}
export interface CmSettingsForString {
  hls?: HlsParamsBaseForString | null
  whitelisted: boolean
  withdraw_enabled: boolean
}
export interface HlsParamsBaseForString {
  correlations: HlsAssetTypeForString[]
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
  withdraw_enabled: boolean
}
export interface VaultConfigBaseForString {
  addr: string
  deposit_cap: Coin
  hls?: HlsParamsBaseForString | null
  liquidation_threshold: Decimal
  max_loan_to_value: Decimal
  whitelisted: boolean
}
export interface Coin {
  amount: Uint128
  denom: string
  [k: string]: unknown
}
export interface PerpParams {
  closing_fee_rate: Decimal
  denom: string
  enabled: boolean
  liquidation_threshold: Decimal
  max_funding_velocity: Decimal
  max_loan_to_value: Decimal
  max_long_oi_value: Uint128
  max_net_oi_value: Uint128
  max_position_value?: Uint128 | null
  max_short_oi_value: Uint128
  min_position_value: Uint128
  opening_fee_rate: Decimal
  skew_scale: Uint128
}
export type QueryMsg =
  | {
      owner: {}
    }
  | {
      config: {}
    }
  | {
      asset_params: {
        denom: string
      }
    }
  | {
      all_asset_params: {
        limit?: number | null
        start_after?: string | null
      }
    }
  | {
      vault_config: {
        address: string
      }
    }
  | {
      all_vault_configs: {
        limit?: number | null
        start_after?: string | null
      }
    }
  | {
      all_vault_configs_v2: {
        limit?: number | null
        start_after?: string | null
      }
    }
  | {
      perp_params: {
        denom: string
      }
    }
  | {
      all_perp_params: {
        limit?: number | null
        start_after?: string | null
      }
    }
  | {
      total_deposit: {
        denom: string
      }
    }
  | {
      all_total_deposits_v2: {
        limit?: number | null
        start_after?: string | null
      }
    }
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
export type ArrayOfAssetParamsBaseForAddr = AssetParamsBaseForAddr[]
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
  withdraw_enabled: boolean
}
export interface HlsParamsBaseForAddr {
  correlations: HlsAssetTypeForAddr[]
  liquidation_threshold: Decimal
  max_loan_to_value: Decimal
}
export type ArrayOfPerpParams = PerpParams[]
export interface PaginationResponseForTotalDepositResponse {
  data: TotalDepositResponse[]
  metadata: Metadata
}
export interface TotalDepositResponse {
  amount: Uint128
  cap: Uint128
  denom: string
}
export interface Metadata {
  has_more: boolean
}
export type ArrayOfVaultConfigBaseForAddr = VaultConfigBaseForAddr[]
export interface VaultConfigBaseForAddr {
  addr: Addr
  deposit_cap: Coin
  hls?: HlsParamsBaseForAddr | null
  liquidation_threshold: Decimal
  max_loan_to_value: Decimal
  whitelisted: boolean
}
export interface PaginationResponseForVaultConfigBaseForAddr {
  data: VaultConfigBaseForAddr[]
  metadata: Metadata
}
export type NullableAssetParamsBaseForAddr = AssetParamsBaseForAddr | null
export interface ConfigResponse {
  address_provider: string
}
export interface OwnerResponse {
  abolished: boolean
  emergency_owner?: string | null
  initialized: boolean
  owner?: string | null
  proposed?: string | null
}
