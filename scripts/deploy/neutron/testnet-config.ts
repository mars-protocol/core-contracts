import { DeploymentConfig, AssetConfig, OracleConfig, PerpDenom } from '../../types/config'

const nobleUsdcDenom = 'ibc/4C19E7EC06C1AB2EC2D70C6855FEB6D48E9CE174913991DA0A517D21978E7E42'
const atomDenom = 'ibc/C4CFF46FD6DE35CA4CF4CE031E643C8FDC9BA4B99AE598E9B0ED98FE3A2319F9'
const marsDenom = 'ibc/584A4A23736884E0C198FD1EE932455A9357A492A7B94324E4A02B5628687831'

const pclLpMarsUsdcDenom =
  'factory/neutron1sf456kx85dz0wfjs4sx0s80dyzmc360pfc0rdzactxt8xrse9ykqsdpy2y/astroport/share'
const pclLpMarsDenom = 'factory/neutron1wm8jd0hrw79pfhhm9xmuq43jwz4wtukvxfgkkw/mars'
const pclLpUsdcDenom = 'factory/neutron1wm8jd0hrw79pfhhm9xmuq43jwz4wtukvxfgkkw/usdc'
const pclLpMarsUsdcPairAddr = 'neutron1sf456kx85dz0wfjs4sx0s80dyzmc360pfc0rdzactxt8xrse9ykqsdpy2y'

// dummy denoms for testing
const btcDenom = 'factory/neutron166t9ww3p6flv7c86376fy0r92r88t3492xxj2h/ubtc'
const ethDenom = 'factory/neutron166t9ww3p6flv7c86376fy0r92r88t3492xxj2h/ueth'
const injDenom = 'factory/neutron166t9ww3p6flv7c86376fy0r92r88t3492xxj2h/uinj'
const dydxDenom = 'factory/neutron166t9ww3p6flv7c86376fy0r92r88t3492xxj2h/udydx'
const tiaDenom = 'factory/neutron166t9ww3p6flv7c86376fy0r92r88t3492xxj2h/utia'
const solDenom = 'factory/neutron166t9ww3p6flv7c86376fy0r92r88t3492xxj2h/usol'

const protocolAdminAddr = 'neutron1ke0vqqzyymlp5esr8gjwuzh94ysnpvj8er5hm7'

const marsNeutronChannelId = 'channel-97'
const chainId = 'pion-1'
const rpcEndpoint = 'https://neutron-testnet-rpc.polkachu.com'

// Astroport configuration
const astroportFactory = 'neutron1jj0scx400pswhpjes589aujlqagxgcztw04srynmhf0f6zplzn2qqmhwj7'
const astroportRouter = 'neutron12jm24l9lr9cupufqjuxpdjnnweana4h66tsx5cl800mke26td26sq7m05p'
const astroportIncentives = 'neutron1slxs8heecwyw0n6zmj7unj3nenrfhk2zpagfz2lt87dnevmksgwsq9adkn'

// note the following three addresses are all 'mars' bech32 prefix
const safetyFundAddr = 'mars1s4hgh56can3e33e0zqpnjxh0t5wdf7u3pze575'
const feeCollectorAddr = 'mars17xpfvakm2amg962yls6f84z3kell8c5ldy6e7x'

// Oracle configurations
export const ntrnOracle: OracleConfig = {
  denom: 'untrn',
  price_source: {
    slinky: {
      base_symbol: 'NTRN',
      denom_decimals: 6,
      max_blocks_old: 5,
    },
  },
}

export const atomOracle: OracleConfig = {
  denom: atomDenom,
  price_source: {
    slinky: {
      base_symbol: 'ATOM',
      denom_decimals: 6,
      max_blocks_old: 5,
    },
  },
}

export const nobleUSDCOracle: OracleConfig = {
  denom: nobleUsdcDenom,
  price_source: {
    slinky: {
      base_symbol: 'USDT', // TODO: change to USDC when available on Slinky
      denom_decimals: 6,
      max_blocks_old: 5,
    },
  },
}

export const btcOracle: OracleConfig = {
  denom: btcDenom,
  price_source: {
    slinky: {
      base_symbol: 'BTC',
      denom_decimals: 8,
      max_blocks_old: 5,
    },
  },
}

export const ethOracle: OracleConfig = {
  denom: ethDenom,
  price_source: {
    slinky: {
      base_symbol: 'ETH',
      denom_decimals: 18,
      max_blocks_old: 5,
    },
  },
}

export const injOracle: OracleConfig = {
  denom: injDenom,
  price_source: {
    slinky: {
      base_symbol: 'INJ',
      denom_decimals: 18,
      max_blocks_old: 5,
    },
  },
}

export const dydxOracle: OracleConfig = {
  denom: dydxDenom,
  price_source: {
    slinky: {
      base_symbol: 'DYDX',
      denom_decimals: 18,
      max_blocks_old: 5,
    },
  },
}

export const tiaOracle: OracleConfig = {
  denom: tiaDenom,
  price_source: {
    slinky: {
      base_symbol: 'TIA',
      denom_decimals: 6,
      max_blocks_old: 5,
    },
  },
}

export const solOracle: OracleConfig = {
  denom: solDenom,
  price_source: {
    slinky: {
      base_symbol: 'SOL',
      denom_decimals: 9,
      max_blocks_old: 5,
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

export const pclLpMarsUsdcOracle: OracleConfig = {
  denom: pclLpMarsUsdcDenom,
  price_source: {
    pcl_liquidity_token: {
      pair_address: pclLpMarsUsdcPairAddr,
    },
  },
}

export const pclLpMarsOracle: OracleConfig = {
  denom: pclLpMarsDenom,
  price_source: {
    fixed: {
      price: '0.84',
    },
  },
}

export const pclLpUsdcOracle: OracleConfig = {
  denom: pclLpUsdcDenom,
  price_source: {
    fixed: {
      price: '1.05',
    },
  },
}

// Router configurations
export const atomUsdcRoute = {
  denom_in: atomDenom,
  denom_out: nobleUsdcDenom,
  route: {
    factory: astroportFactory,
    operations: [
      {
        astro_swap: {
          ask_asset_info: {
            native_token: {
              denom: nobleUsdcDenom,
            },
          },
          offer_asset_info: {
            native_token: {
              denom: atomDenom,
            },
          },
        },
      },
    ],
    oracle: '', // Will be filled in by deploy script
    router: astroportRouter,
  },
}

export const ntrnUsdcRoute = {
  denom_in: 'untrn',
  denom_out: nobleUsdcDenom,
  route: {
    factory: astroportFactory,
    operations: [
      {
        astro_swap: {
          ask_asset_info: {
            native_token: {
              denom: nobleUsdcDenom,
            },
          },
          offer_asset_info: {
            native_token: {
              denom: 'untrn',
            },
          },
        },
      },
    ],
    oracle: '', // Will be filled in by deploy script
    router: astroportRouter,
  },
}

export const atomMarsRoute = {
  denom_in: atomDenom,
  denom_out: marsDenom,
  route: {
    factory: astroportFactory,
    operations: [
      {
        astro_swap: {
          ask_asset_info: {
            native_token: {
              denom: marsDenom,
            },
          },
          offer_asset_info: {
            native_token: {
              denom: atomDenom,
            },
          },
        },
      },
    ],
    oracle: '', // Will be filled in by deploy script
    router: astroportRouter,
  },
}

export const ntrnMarsRoute = {
  denom_in: 'untrn',
  denom_out: marsDenom,
  route: {
    factory: astroportFactory,
    operations: [
      {
        astro_swap: {
          ask_asset_info: {
            native_token: {
              denom: marsDenom,
            },
          },
          offer_asset_info: {
            native_token: {
              denom: 'untrn',
            },
          },
        },
      },
    ],
    oracle: '', // Will be filled in by deploy script
    router: astroportRouter,
  },
}

export const usdcMarsRoute = {
  denom_in: nobleUsdcDenom,
  denom_out: marsDenom,
  route: {
    factory: astroportFactory,
    operations: [
      {
        astro_swap: {
          ask_asset_info: {
            native_token: {
              denom: marsDenom,
            },
          },
          offer_asset_info: {
            native_token: {
              denom: nobleUsdcDenom,
            },
          },
        },
      },
    ],
    oracle: '', // Will be filled in by deploy script
    router: astroportRouter,
  },
}

// Asset configurations
export const ntrnAsset: AssetConfig = {
  denom: 'untrn',
  max_loan_to_value: '0.54',
  liquidation_threshold: '0.55',
  liquidation_bonus: {
    max_lb: '0.2',
    min_lb: '0.05',
    slope: '1',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.5',
  symbol: 'NTRN',
  credit_manager: {
    whitelisted: true,
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
  max_loan_to_value: '0.74',
  liquidation_threshold: '0.75',
  liquidation_bonus: {
    max_lb: '0.2',
    min_lb: '0.05',
    slope: '1',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.5',
  symbol: 'ATOM',
  credit_manager: {
    whitelisted: true,
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
    optimal_utilization_rate: '0.8',
    base: '0',
    slope_1: '0.14',
    slope_2: '3',
  },
  close_factor: '0.9',
}

export const nobleUSDCAsset: AssetConfig = {
  denom: nobleUsdcDenom,
  max_loan_to_value: '0.795',
  liquidation_threshold: '0.8',
  liquidation_bonus: {
    max_lb: '0.2',
    min_lb: '0.05',
    slope: '1',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.5',
  symbol: 'nobleUSDC',
  credit_manager: {
    whitelisted: true,
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
    slope_1: '0.2',
    slope_2: '2',
  },
  close_factor: '0.9',
}

export const pclLpMarsUsdcAsset: AssetConfig = {
  denom: pclLpMarsUsdcDenom,
  max_loan_to_value: '0.35',
  liquidation_threshold: '0.40',
  liquidation_bonus: {
    max_lb: '0.2',
    min_lb: '0.05',
    slope: '1',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.5',
  symbol: 'PCL_LP_MARS_USDC',
  credit_manager: {
    whitelisted: true,
    withdraw_enabled: true,
  },
  red_bank: {
    borrow_enabled: false,
    deposit_enabled: true,
    withdraw_enabled: true,
  },
  deposit_cap: '1000000000000000000',
  reserve_factor: '0.1',
  interest_rate_model: {
    optimal_utilization_rate: '0.6',
    base: '0',
    slope_1: '0.15',
    slope_2: '3',
  },
  close_factor: '0.9',
}

export const pclLpMarsAsset: AssetConfig = {
  denom: pclLpMarsDenom,
  max_loan_to_value: '0.74',
  liquidation_threshold: '0.75',
  liquidation_bonus: {
    max_lb: '0.2',
    min_lb: '0.05',
    slope: '1',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.5',
  symbol: 'PCL_LP_MARS',
  credit_manager: {
    whitelisted: true,
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

export const pclLpUsdcAsset: AssetConfig = {
  denom: pclLpUsdcDenom,
  max_loan_to_value: '0.74',
  liquidation_threshold: '0.75',
  liquidation_bonus: {
    max_lb: '0.2',
    min_lb: '0.05',
    slope: '1',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.5',
  symbol: 'PCL_LP_USDC',
  credit_manager: {
    whitelisted: true,
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

export const ntrnPerpDenom: PerpDenom = {
  denom: 'untrn',
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

export const btcPerpDenom: PerpDenom = {
  denom: btcDenom,
  maxFundingVelocity: '36',
  skewScale: '8892400000000',
  maxNetOiValue: '88882000000',
  maxLongOiValue: '28135198400000',
  maxShortOiValue: '28135198400000',
  closingFeeRate: '0.00075',
  openingFeeRate: '0.00075',
  minPositionValue: '500000000',
  liquidationThreshold: '0.91',
  maxLoanToValue: '0.90',
}

export const ethPerpDenom: PerpDenom = {
  denom: ethDenom,
  maxFundingVelocity: '36',
  skewScale: '1186268000000000000000000',
  maxNetOiValue: '86049000000',
  maxLongOiValue: '19093576000000',
  maxShortOiValue: '19093576000000',
  closingFeeRate: '0.00075',
  openingFeeRate: '0.00075',
  minPositionValue: '500000000',
  liquidationThreshold: '0.91',
  maxLoanToValue: '0.90',
}

export const injPerpDenom: PerpDenom = {
  denom: injDenom,
  maxFundingVelocity: '36',
  skewScale: '1805314000000000000000000',
  maxNetOiValue: '33496000000',
  maxLongOiValue: '400314200000',
  maxShortOiValue: '400314200000',
  closingFeeRate: '0.00075',
  openingFeeRate: '0.00075',
  minPositionValue: '500000000',
  liquidationThreshold: '0.88',
  maxLoanToValue: '0.83',
}

export const dydxPerpDenom: PerpDenom = {
  denom: dydxDenom,
  maxFundingVelocity: '36',
  skewScale: '1462272000000000000000000',
  maxNetOiValue: '32529000000',
  maxLongOiValue: '221088700000',
  maxShortOiValue: '221088700000',
  closingFeeRate: '0.00075',
  openingFeeRate: '0.00075',
  minPositionValue: '500000000',
  liquidationThreshold: '0.82',
  maxLoanToValue: '0.80',
}

export const tiaPerpDenom: PerpDenom = {
  denom: tiaDenom,
  maxFundingVelocity: '36',
  skewScale: '4504227000000',
  maxNetOiValue: '22081000000',
  maxLongOiValue: '316093400000',
  maxShortOiValue: '316093400000',
  closingFeeRate: '0.00075',
  openingFeeRate: '0.00075',
  minPositionValue: '500000000',
  liquidationThreshold: '0.76',
  maxLoanToValue: '0.71',
}

export const solPerpDenom: PerpDenom = {
  denom: solDenom,
  maxFundingVelocity: '36',
  skewScale: '3954627000000000',
  maxNetOiValue: '36869000000',
  maxLongOiValue: '3396453000000',
  maxShortOiValue: '3396453000000',
  closingFeeRate: '0.00075',
  openingFeeRate: '0.00075',
  minPositionValue: '500000000',
  liquidationThreshold: '0.88',
  maxLoanToValue: '0.85',
}

export const neutronTestnetConfig: DeploymentConfig = {
  mainnet: false,
  deployerMnemonic: 'TO BE INSERTED AT TIME OF DEPLOYMENT',
  marsDenom: marsDenom,
  atomDenom: atomDenom,
  safetyFundAddr: safetyFundAddr,
  protocolAdminAddr: protocolAdminAddr,
  feeCollectorAddr: feeCollectorAddr,
  keeperFeeConfig: {
    min_fee: { amount: '1000000', denom: nobleUsdcDenom },
  },
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
    routes: [atomUsdcRoute, atomMarsRoute, ntrnUsdcRoute, ntrnMarsRoute, usdcMarsRoute],
  },
  maxValueForBurn: '10000',
  maxUnlockingPositions: '1',
  maxSlippage: '0.2',
  zapperContractName: 'mars_zapper_astroport',
  runTests: false,
  assets: [
    ntrnAsset,
    atomAsset,
    nobleUSDCAsset,
    pclLpMarsUsdcAsset,
    pclLpMarsAsset,
    pclLpUsdcAsset,
  ],
  vaults: [],
  oracleConfigs: [
    usdOracle,
    nobleUSDCOracle,
    atomOracle,
    ntrnOracle,
    pclLpMarsOracle,
    pclLpUsdcOracle,
    pclLpMarsUsdcOracle,
    btcOracle,
    ethOracle,
    injOracle,
    dydxOracle,
    tiaOracle,
    solOracle,
  ],
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
    denoms: [
      atomPerpDenom,
      ntrnPerpDenom,
      btcPerpDenom,
      ethPerpDenom,
      injPerpDenom,
      dydxPerpDenom,
      tiaPerpDenom,
      solPerpDenom,
    ],
  },
  maxPerpParams: 20,
  perpsLiquidationBonusRatio: '0.6',
}
