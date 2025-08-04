import { DeploymentConfig, AssetConfig, OracleConfig, PerpDenom } from '../../types/config'

const nobleUsdcDenom = 'ibc/B559A80D62249C8AA07A380E2A2BEA6E5CA9A6F079C912C3A9E9B494105E4F81'
const axlUsdcDenom = 'ibc/F082B65C88E4B6D5EF1DB243CDA1D331D002759E938A0F5CD3FFDC5D53B3E349'
const atomDenom = 'ibc/C4CFF46FD6DE35CA4CF4CE031E643C8FDC9BA4B99AE598E9B0ED98FE3A2319F9'
const marsDenom = 'ibc/9598CDEB7C6DB7FC21E746C8E0250B30CD5154F39CA111A9D4948A4362F638BD'

const protocolAdminAddr = 'neutron1ltzuv25ltw9mkwuvvmt7e54a6ene283hfj7l0c'

const marsNeutronChannelId = 'channel-16'
const chainId = 'neutron-1'
const rpcEndpoint =
  'https://neutron.rpc.p2p.world:443/qgrnU6PsQZA8F9S5Fb8Fn3tV3kXmMBl2M9bcc9jWLjQy8p'

// Astroport configuration https://github.com/astroport-fi/astroport-changelog/blob/main/neutron/neutron-1/core_mainnet.json
const astroportFactory = 'neutron1hptk0k5kng7hjy35vmh009qd5m6l33609nypgf2yc6nqnewduqasxplt4e'
const astroportRouter = 'neutron1rwj6mfxzzrwskur73v326xwuff52vygqk73lr7azkehnfzz5f5wskwekf4'
const astroportIncentives = 'neutron173fd8wpfzyqnfnpwq2zhtgdstujrjz2wkprkjfr6gqg4gknctjyq6m3tch'
const astroportNtrnAtomPair = 'neutron1e22zh5p8meddxjclevuhjmfj69jxfsa8uu3jvht72rv9d8lkhves6t8veq'
const astroportMarsUsdcPair = 'neutron165m0r6rkhqxs30wch00t7mkykxxvgve9yyu254wknwhhjn34rmqsh6vfcj'

// note the following three addresses are all 'mars' bech32 prefix
const safetyFundAddr = 'mars1s4hgh56can3e33e0zqpnjxh0t5wdf7u3pze575'
const feeCollectorAddr = 'mars17xpfvakm2amg962yls6f84z3kell8c5ldy6e7x'
const revShareAddr = 'mars17xpfvakm2amg962yls6f84z3kell8c5ldy6e7x'

// Pyth configuration
const pythAddr = 'neutron1m2emc93m9gpwgsrsf2vylv9xvgqh654630v7dfrhrkmr5slly53spg85wv'
const pythAtomID = 'b00b60f88b03a6a625a8d1c048c3f66653edf217439983d037e7222c4e612819'
const pythUsdcID = 'eaa020c61cc479712813461ce153894a96a6c00b21ed0cfc2798d1f9a9e9c94a'

// Oracle configurations
export const marsOracle: OracleConfig = {
  denom: marsDenom,
  price_source: {
    astroport_twap: {
      window_size: 1800, // 30 minutes
      tolerance: 120, // 2 minutes
      pair_address: astroportMarsUsdcPair,
    },
  },
}

export const ntrnOracle: OracleConfig = {
  denom: 'untrn',
  price_source: {
    astroport_twap: {
      window_size: 1800, // 30 minutes
      tolerance: 120, // 2 minutes
      pair_address: astroportNtrnAtomPair,
    },
  },
}

export const atomOracle: OracleConfig = {
  denom: atomDenom,
  price_source: {
    pyth: {
      contract_addr: pythAddr,
      price_feed_id: pythAtomID,
      denom_decimals: 6,
      max_staleness: 60,
      max_confidence: '0.1',
      max_deviation: '0.15',
    },
  },
}

export const axlUSDCOracle: OracleConfig = {
  denom: axlUsdcDenom,
  price_source: {
    pyth: {
      contract_addr: pythAddr,
      price_feed_id: pythUsdcID,
      denom_decimals: 6,
      max_staleness: 60,
      max_confidence: '0.1',
      max_deviation: '0.15',
    },
  },
}

export const usdOracle: OracleConfig = {
  denom: 'usd',
  price_source: {
    fixed: {
      price: '1000000',
    },
  },
}

// Asset configurations
export const ntrnAsset: AssetConfig = {
  denom: 'untrn',
  max_loan_to_value: '0.35',
  liquidation_threshold: '0.40',
  liquidation_bonus: {
    max_lb: '0.05',
    min_lb: '0',
    slope: '2',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.5',
  // liquidation_bonus: '0.15',
  symbol: 'NTRN',
  credit_manager: {
    whitelisted: false,
    withdraw_enabled: true,
  },
  red_bank: {
    borrow_enabled: true,
    deposit_enabled: true,
    withdraw_enabled: true,
  },
  deposit_cap: '5000000000000',
  reserve_factor: '0.1',
  interest_rate_model: {
    optimal_utilization_rate: '0.6',
    base: '0',
    slope_1: '0.15',
    slope_2: '3',
  },
  close_factor: '0.9',
}

export const atomAsset: AssetConfig = {
  denom: atomDenom,
  max_loan_to_value: '0.68',
  liquidation_threshold: '0.7',
  liquidation_bonus: {
    max_lb: '0.05',
    min_lb: '0',
    slope: '2',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.5',
  // liquidation_bonus: '0.1',
  symbol: 'ATOM',
  credit_manager: {
    whitelisted: false,
    withdraw_enabled: true,
  },
  red_bank: {
    borrow_enabled: true,
    deposit_enabled: true,
    withdraw_enabled: true,
  },
  deposit_cap: '150000000000',
  reserve_factor: '0.1',
  interest_rate_model: {
    optimal_utilization_rate: '0.7',
    base: '0',
    slope_1: '0.2',
    slope_2: '3',
  },
  close_factor: '0.9',
}

export const axlUSDCAsset: AssetConfig = {
  denom: axlUsdcDenom,
  max_loan_to_value: '0.74',
  liquidation_threshold: '0.75',
  liquidation_bonus: {
    max_lb: '0.05',
    min_lb: '0',
    slope: '2',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.5',
  // liquidation_bonus: '0.1',
  symbol: 'axlUSDC',
  credit_manager: {
    whitelisted: false,
    withdraw_enabled: true,
  },
  red_bank: {
    borrow_enabled: true,
    deposit_enabled: true,
    withdraw_enabled: true,
  },
  deposit_cap: '500000000000',
  reserve_factor: '0.1',
  interest_rate_model: {
    optimal_utilization_rate: '0.8',
    base: '0',
    slope_1: '0.125',
    slope_2: '2',
  },
  close_factor: '0.9',
}

// Perps configurations
export const atomPerpDenom: PerpDenom = {
  denom: atomDenom,
  maxFundingVelocity: '36',
  skewScale: '7227323000000',
  maxNetOiValue: '45591000000',
  maxLongOiValue: '490402700000',
  maxShortOiValue: '490402700000',
  closingFeeRate: '0.00075',
  openingFeeRate: '0.00075',
  minPositionValue: '500000000',
  liquidationThreshold: '0.86',
  maxLoanToValue: '0.85',
}

export const neutronMainnetConfig: DeploymentConfig = {
  mainnet: true,
  deployerMnemonic: 'TO BE INSERTED AT TIME OF DEPLOYMENT',
  marsDenom: marsDenom,
  atomDenom: atomDenom,
  safetyFundAddr: safetyFundAddr,
  protocolAdminAddr: protocolAdminAddr,
  feeCollectorAddr: feeCollectorAddr,
  revShareAddr: revShareAddr,
  chain: {
    baseDenom: 'untrn',
    defaultGasPrice: 0.02,
    id: chainId,
    prefix: 'neutron',
    rpcEndpoint: rpcEndpoint,
  },
  oracle: {
    name: 'wasm',
    baseDenom: 'uusd',
    customInitParams: {
      astroport_factory: astroportFactory,
    },
  },
  keeperFeeConfig: {
    min_fee: { amount: '1000000', denom: nobleUsdcDenom },
  },
  rewardsCollector: {
    name: 'neutron',
    timeoutSeconds: 600,
    channelId: marsNeutronChannelId,
    safetyFundFeeShare: '0.45',
    revenueShare: '0.1',
    revenueShareConfig: {
      target_denom: nobleUsdcDenom,
      transfer_type: 'bank',
    },
    safetyFundConfig: {
      target_denom: nobleUsdcDenom,
      transfer_type: 'bank',
    },
    feeCollectorConfig: {
      target_denom: marsDenom,
      transfer_type: 'ibc',
    },
    slippageTolerance: '0.01',
  },
  incentives: {
    epochDuration: 604800, // 1 week
    maxWhitelistedIncentiveDenoms: 10,
  },
  swapper: {
    name: 'astroport',
    routes: [],
  },
  maxValueForBurn: '10000',
  maxTriggerOrders: 50,
  maxUnlockingPositions: '1',
  maxSlippage: '0.2',
  zapperContractName: 'mars_zapper_astroport',
  runTests: false,
  assets: [ntrnAsset, atomAsset, axlUSDCAsset],
  vaults: [],
  oracleConfigs: [usdOracle, axlUSDCOracle, marsOracle, atomOracle, ntrnOracle],
  astroportConfig: {
    factory: astroportFactory,
    router: astroportRouter,
    incentives: astroportIncentives,
  },
  perps: {
    baseDenom: nobleUsdcDenom,
    cooldownPeriod: 300, // 5 min
    maxPositions: 4,
    maxUnlocks: 5,
    protocolFeeRate: '0.1',
    targetCollaterizationRatio: '1.2',
    deleverageEnabled: true,
    vaultWithdrawEnabled: true,
    denoms: [atomPerpDenom],
  },
  maxPerpParams: 20,
  perpsLiquidationBonusRatio: '0.6',
}
