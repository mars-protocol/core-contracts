import { DeploymentConfig } from '../../types/config'

const nobleUsdcDenom = 'ibc/B559A80D62249C8AA07A380E2A2BEA6E5CA9A6F079C912C3A9E9B494105E4F81'
const atomDenom = 'ibc/C4CFF46FD6DE35CA4CF4CE031E643C8FDC9BA4B99AE598E9B0ED98FE3A2319F9'
const marsDenom = 'factory/neutron1ndu2wvkrxtane8se2tr48gv7nsm46y5gcqjhux/MARS'

const protocolAdminAddr = 'neutron1pup4hxx879r2wreylmmtxlqz8va3kk2wcgd7w2'

const chainId = 'neutron-1'
const rpcEndpoint =
  'https://rpc-lb.neutron.org'

// Astroport configuration https://github.com/astroport-fi/astroport-changelog/blob/main/neutron/neutron-1/core_mainnet.json
const astroportFactory = 'neutron1hptk0k5kng7hjy35vmh009qd5m6l33609nypgf2yc6nqnewduqasxplt4e'
const astroportRouter = 'neutron1rwj6mfxzzrwskur73v326xwuff52vygqk73lr7azkehnfzz5f5wskwekf4'
const astroportIncentives = 'neutron173fd8wpfzyqnfnpwq2zhtgdstujrjz2wkprkjfr6gqg4gknctjyq6m3tch'

// note the following three addresses are all 'mars' bech32 prefix
const safetyFundAddr = 'neutron1pxjszcmmdxwtw9kv533u3hcudl6qahsa42chcs24gervf4ge40usaw3pcr'
const feeCollectorAddr = 'neutron1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqhufaa6'
const revShareAddr = 'neutron1d0h5pk9ffmewrs8aksdp4whxjrfr44xzfc03uanmhuj6fdjs8yqsg6rcpx'

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
    min_fee: { amount: '200000', denom: nobleUsdcDenom },
  },
  rewardsCollector: {
    name: 'neutron',
    timeoutSeconds: 600,
    channelId: '',
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
      transfer_type: 'bank',
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
  maxUnlockingPositions: '1',
  maxSlippage: '0.2',
  zapperContractName: 'mars_zapper_astroport',
  runTests: false,
  assets: [],
  vaults: [],
  oracleConfigs: [],
  astroportConfig: {
    factory: astroportFactory,
    router: astroportRouter,
    incentives: astroportIncentives,
  },
  perps: {
    baseDenom: nobleUsdcDenom,
    cooldownPeriod: 300, // 5 min
    maxPositions: 10,
    maxUnlocks: 4,
    protocolFeeRate: '0.25',
    targetCollaterizationRatio: '1.5',
    deleverageEnabled: true,
    vaultWithdrawEnabled: true,
    denoms: [],
  },
  maxPerpParams: 30,
  perpsLiquidationBonusRatio: '0.6',
}
