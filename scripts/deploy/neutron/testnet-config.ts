import { DeploymentConfig, AssetConfig, OracleConfig, PerpDenom } from '../../types/config'
import { NeutronIbcConfig } from '../../types/generated/mars-rewards-collector-base/MarsRewardsCollectorBase.types'

const nobleUsdcDenom = 'ibc/4C19E7EC06C1AB2EC2D70C6855FEB6D48E9CE174913991DA0A517D21978E7E42'
const atomDenom = 'ibc/C4CFF46FD6DE35CA4CF4CE031E643C8FDC9BA4B99AE598E9B0ED98FE3A2319F9'
const marsDenom = 'ibc/584A4A23736884E0C198FD1EE932455A9357A492A7B94324E4A02B5628687831'

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
const rpcEndpoint = 'https://rpc-palvus.pion-1.ntrn.tech'

// Astroport configuration
const astroportFactory = 'neutron1jj0scx400pswhpjes589aujlqagxgcztw04srynmhf0f6zplzn2qqmhwj7'
const astroportRouter = 'neutron12jm24l9lr9cupufqjuxpdjnnweana4h66tsx5cl800mke26td26sq7m05p'
// const astroportNtrnAtomPair = 'neutron1sm23jnz4lqd88etklvwlm66a0x6mhflaqlv65wwr7nwwxa6258ks6nshpq'

// note the following three addresses are all 'mars' bech32 prefix
const safetyFundAddr = 'mars1s4hgh56can3e33e0zqpnjxh0t5wdf7u3pze575'
const feeCollectorAddr = 'mars17xpfvakm2amg962yls6f84z3kell8c5ldy6e7x'

// Pyth configuration
const pythAddr = 'neutron15ldst8t80982akgr8w8ekcytejzkmfpgdkeq4xgtge48qs7435jqp87u3t'
const pythAtomID = 'b00b60f88b03a6a625a8d1c048c3f66653edf217439983d037e7222c4e612819'
const pythUsdcID = 'eaa020c61cc479712813461ce153894a96a6c00b21ed0cfc2798d1f9a9e9c94a'
const pythNtrnID = 'a8e6517966a52cb1df864b2764f3629fde3f21d2b640b5c572fcd654cbccd65e'
const pythBtcID = 'e62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43'
const pythEthID = 'ff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace'
const pythInjID = '7a5bc1d2b56ad029048cd63964b3ad2776eadf812edc1a43a31406cb54bff592'
const pythDydxID = '6489800bb8974169adfe35937bf6736507097d13c190d760c557108c7e93a81b'
const pythTiaID = '09f7c1d7dfbb7df2b8fe3d3d87ee94a2259d212da4f30c1f0540d066dfa44723'
const pythSolID = 'ef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d'

const defaultCreditLine = '100000000000000'

// IBC config for rewards-collector. See https://rest-palvus.pion-1.ntrn.tech/neutron-org/neutron/feerefunder/params
export const neutronIbcConfig: NeutronIbcConfig = {
  source_port: 'transfer',
  acc_fee: [
    {
      denom: 'untrn',
      amount: '1000',
    },
  ],
  timeout_fee: [
    {
      denom: 'untrn',
      amount: '1000',
    },
  ],
}

// Oracle configurations
export const ntrnOracle: OracleConfig = {
  denom: 'untrn',
  price_source: {
    pyth: {
      contract_addr: pythAddr,
      price_feed_id: pythNtrnID,
      denom_decimals: 6,
      max_staleness: 300, // 5 minutes
      max_confidence: '0.1',
      max_deviation: '0.15',
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
      max_staleness: 300, // 5 minutes
      max_confidence: '0.1',
      max_deviation: '0.15',
    },
  },
}

export const nobleUSDCOracle: OracleConfig = {
  denom: nobleUsdcDenom,
  price_source: {
    pyth: {
      contract_addr: pythAddr,
      price_feed_id: pythUsdcID,
      denom_decimals: 6,
      max_staleness: 300, // 5 minutes
      max_confidence: '0.1',
      max_deviation: '0.15',
    },
  },
}

export const btcOracle: OracleConfig = {
  denom: btcDenom,
  price_source: {
    pyth: {
      contract_addr: pythAddr,
      price_feed_id: pythBtcID,
      denom_decimals: 8,
      max_staleness: 300, // 5 minutes
      max_confidence: '0.1',
      max_deviation: '0.15',
    },
  },
}

export const ethOracle: OracleConfig = {
  denom: ethDenom,
  price_source: {
    pyth: {
      contract_addr: pythAddr,
      price_feed_id: pythEthID,
      denom_decimals: 18,
      max_staleness: 300, // 5 minutes
      max_confidence: '0.1',
      max_deviation: '0.15',
    },
  },
}

export const injOracle: OracleConfig = {
  denom: injDenom,
  price_source: {
    pyth: {
      contract_addr: pythAddr,
      price_feed_id: pythInjID,
      denom_decimals: 18,
      max_staleness: 300, // 5 minutes
      max_confidence: '0.1',
      max_deviation: '0.15',
    },
  },
}

export const dydxOracle: OracleConfig = {
  denom: dydxDenom,
  price_source: {
    pyth: {
      contract_addr: pythAddr,
      price_feed_id: pythDydxID,
      denom_decimals: 18,
      max_staleness: 300, // 5 minutes
      max_confidence: '0.1',
      max_deviation: '0.15',
    },
  },
}

export const tiaOracle: OracleConfig = {
  denom: tiaDenom,
  price_source: {
    pyth: {
      contract_addr: pythAddr,
      price_feed_id: pythTiaID,
      denom_decimals: 6,
      max_staleness: 300, // 5 minutes
      max_confidence: '0.1',
      max_deviation: '0.15',
    },
  },
}

export const solOracle: OracleConfig = {
  denom: solDenom,
  price_source: {
    pyth: {
      contract_addr: pythAddr,
      price_feed_id: pythSolID,
      denom_decimals: 9,
      max_staleness: 300, // 5 minutes
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
  max_loan_to_value: '0.35',
  liquidation_threshold: '0.40',
  liquidation_bonus: {
    max_lb: '0.05',
    min_lb: '0',
    slope: '2',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.5',
  symbol: 'NTRN',
  credit_manager: {
    whitelisted: true,
  },
  red_bank: {
    borrow_enabled: true,
    deposit_enabled: true,
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
  symbol: 'ATOM',
  credit_manager: {
    whitelisted: true,
  },
  red_bank: {
    borrow_enabled: true,
    deposit_enabled: true,
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

export const nobleUSDCAsset: AssetConfig = {
  denom: nobleUsdcDenom,
  max_loan_to_value: '0.74',
  liquidation_threshold: '0.75',
  liquidation_bonus: {
    max_lb: '0.05',
    min_lb: '0',
    slope: '2',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.5',
  symbol: 'axlUSDC',
  credit_manager: {
    whitelisted: true,
  },
  red_bank: {
    borrow_enabled: true,
    deposit_enabled: true,
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
  deployerMnemonic: 'TODO',
  marsDenom: marsDenom,
  atomDenom: atomDenom,
  safetyFundAddr: safetyFundAddr,
  protocolAdminAddr: protocolAdminAddr,
  feeCollectorAddr: feeCollectorAddr,
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
    safetyFundFeeShare: '0.5',
    feeCollectorDenom: marsDenom,
    safetyFundDenom: nobleUsdcDenom,
    slippageTolerance: '0.01',
    neutronIbcConfig: neutronIbcConfig,
  },
  incentives: {
    epochDuration: 604800, // 1 week
    maxWhitelistedIncentiveDenoms: 10,
  },
  swapper: {
    name: 'astroport',
    routes: [atomUsdcRoute, atomMarsRoute, ntrnUsdcRoute, ntrnMarsRoute, usdcMarsRoute],
  },
  creditLineCoins: [
    { denom: 'untrn', creditLine: defaultCreditLine },
    { denom: nobleUsdcDenom, creditLine: defaultCreditLine },
    { denom: atomDenom, creditLine: defaultCreditLine },
  ],
  maxValueForBurn: '10000',
  maxUnlockingPositions: '1',
  maxSlippage: '0.2',
  zapperContractName: 'mars_zapper_osmosis',
  runTests: false,
  assets: [ntrnAsset, atomAsset, nobleUSDCAsset],
  vaults: [],
  oracleConfigs: [
    usdOracle,
    nobleUSDCOracle,
    atomOracle,
    ntrnOracle,
    btcOracle,
    ethOracle,
    injOracle,
    dydxOracle,
    tiaOracle,
    solOracle,
  ],
  perps: {
    baseDenom: nobleUsdcDenom,
    cooldownPeriod: 300, // 5 min
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
}
