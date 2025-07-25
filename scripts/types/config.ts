import { OsmosisPriceSourceForString } from './generated/mars-oracle-osmosis/MarsOracleOsmosis.types'
import {
  DualityRoute,
  OsmosisRoute,
} from './generated/mars-swapper-osmosis/MarsSwapperOsmosis.types'
import { AstroportRoute } from './generated/mars-swapper-astroport/MarsSwapperAstroport.types'
import {
  WasmOracleCustomInitParams,
  WasmPriceSourceForString,
} from './generated/mars-oracle-wasm/MarsOracleWasm.types'
import {
  CmSettingsForString,
  Decimal,
  LiquidationBonus,
  RedBankSettings,
  VaultConfigBaseForString,
} from './generated/mars-params/MarsParams.types'
import { Uint128 } from './generated/mars-red-bank/MarsRedBank.types'
import { Duration, VaultInfoResponse } from './generated/mars-mock-vault/MarsMockVault.types'
import { RewardConfig } from './generated/mars-rewards-collector-base/MarsRewardsCollectorBase.types'

type SwapRoute = {
  denom_in: string
  denom_out: string
  route: OsmosisRoute | AstroportRoute | DualityRoute
}

export type SwapperExecuteMsg = {
  set_route: SwapRoute
}

export function isOsmosisRoute(
  route: OsmosisRoute | AstroportRoute | DualityRoute,
): route is OsmosisRoute {
  return Array.isArray(route)
}

export function isAstroportRoute(
  route: OsmosisRoute | AstroportRoute | DualityRoute,
): route is AstroportRoute {
  return !isOsmosisRoute(route) && !isDualityRoute(route)
}

export function isDualityRoute(
  route: OsmosisRoute | AstroportRoute | DualityRoute,
): route is DualityRoute {
  return (
    typeof route === 'object' &&
    !Array.isArray(route) &&
    'from' in route &&
    'to' in route &&
    'swap_denoms' in route
  )
}
export interface AstroportConfig {
  factory: string
  router: string
  incentives: string
}

export interface DeploymentConfig {
  mainnet: boolean
  deployerMnemonic: string
  marsDenom: string
  atomDenom: string
  safetyFundAddr: string
  protocolAdminAddr: string
  feeCollectorAddr: string
  chain: {
    prefix: string
    id: string
    rpcEndpoint: string
    defaultGasPrice: number
    baseDenom: string
  }
  oracle: {
    name: string
    baseDenom: string
    customInitParams?: WasmOracleCustomInitParams
  }
  rewardsCollector: {
    name: string
    timeoutSeconds: number
    channelId: string
    safetyFundFeeShare: string
    revenueShare: string
    revenueShareConfig: RewardConfig
    safetyFundConfig: RewardConfig
    feeCollectorConfig: RewardConfig
    slippageTolerance: string
  }
  incentives: {
    epochDuration: number
    maxWhitelistedIncentiveDenoms: number
  }
  swapper: {
    name: string
    routes: SwapRoute[]
  }
  dualitySwapper?: {
    name: string
    routes: SwapRoute[]
  }
  maxValueForBurn: string
  maxTriggerOrders: number
  maxUnlockingPositions: string
  maxSlippage: string
  runTests: boolean
  testActions?: TestActions
  zapperContractName: string
  multisigAddr?: string
  assets: AssetConfig[]
  vaults: VaultConfig[]
  oracleConfigs: OracleConfig[]
  keeperFeeConfig: {
    min_fee: { amount: string; denom: string }
  }
  astroportConfig?: AstroportConfig
  perps?: {
    baseDenom: string
    cooldownPeriod: number
    maxPositions: number
    denoms: PerpDenom[]
    protocolFeeRate: Decimal
    targetCollaterizationRatio: Decimal
    deleverageEnabled: boolean
    vaultWithdrawEnabled: boolean
    maxUnlocks: number
  }
  maxPerpParams: number
  perpsLiquidationBonusRatio: Decimal
}

export interface AssetConfig {
  symbol: string
  credit_manager: CmSettingsForString
  denom: string
  liquidation_bonus: LiquidationBonus
  liquidation_threshold: Decimal
  max_loan_to_value: Decimal
  protocol_liquidation_fee: Decimal
  red_bank: RedBankSettings
  deposit_cap: Uint128
  reserve_factor: string
  interest_rate_model: {
    optimal_utilization_rate: string
    base: string
    slope_1: string
    slope_2: string
  }
  close_factor: Decimal
}

export enum VaultType {
  LOCKED,
  UNLOCKED,
}

export interface VaultInfo {
  lockup: { time: number } | undefined
  tokens: VaultInfoResponse
}

export interface VaultConfig {
  symbol: string
  vault: VaultConfigBaseForString
}

export interface OracleConfig {
  denom: string
  price_source: OsmosisPriceSourceForString | WasmPriceSourceForString
}

export interface TestActions {
  vault: {
    depositAmount: string
    withdrawAmount: string
    mock: {
      type: VaultType
      config: Omit<VaultConfigBaseForString, 'addr'>
      vaultTokenDenom: string
      lockup?: Duration
      baseToken: string
    }
  }
  secondaryDenom: string
  startingAmountForTestUser: string
  depositAmount: string
  lendAmount: string
  borrowAmount: string
  repayAmount: string
  reclaimAmount: string
  swap: {
    amount: string
    slippage: string
    route: { token_out_denom: string; pool_id: string }[]
  }
  withdrawAmount: string
  zap: {
    coinsIn: { amount: string; denom: string }[]
    denomOut: string
  }
  unzapAmount: string
}

export interface PerpDenom {
  denom: string
  maxFundingVelocity: string
  skewScale: string
  maxNetOiValue: string
  maxLongOiValue: string
  maxShortOiValue: string
  closingFeeRate: string
  openingFeeRate: string
  liquidationThreshold: string
  maxLoanToValue: string
  maxLoanToValueUsdc: string | null
  liquidationThresholdUsdc: string | null
  maxPositionValue?: string
  minPositionValue: string
}
