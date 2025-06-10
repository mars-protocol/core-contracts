import { setupDeployer } from './setup-deployer'
import { printRed, printYellow } from '../../utils/chalk'
import { DeploymentConfig } from '../../types/config'
import { PriceSourceResponseForString } from '../../types/generated/mars-oracle-osmosis/MarsOracleOsmosis.types'
import {
  AssetParamsBaseForString,
  PerpParams,
} from '../../types/generated/mars-params/MarsParams.types'
import { MarketV2Response } from '../../types/generated/mars-red-bank/MarsRedBank.types'
import { WasmPriceSourceForString } from '../../types/generated/mars-oracle-wasm/MarsOracleWasm.types'
import { wasmFile } from '../../utils/environment'

const marsOracleAddr = 'neutron1dwp6m7pdrz6rnhdyrx5ha0acsduydqcpzkylvfgspsz60pj2agxqaqrr7g'
const marsParamsAddr = 'neutron1x4rgd7ry23v2n49y7xdzje0743c5tgrnqrqsvwyya2h6m48tz4jqqex06x'
const marsRedBankAddr = 'neutron1n97wnm7q6d2hrcna3rqlnyqw2we6k0l8uqvmyqq6gsml92epdu7quugyph'

export interface TaskRunnerProps {
  config: DeploymentConfig
  label: string
}

export const taskRunner = async ({ config, label }: TaskRunnerProps) => {
  const deployer = await setupDeployer(config, label)

  try {
    await deployer.assertDeployerBalance()

    // Upload contracts
    await deployer.upload('redBank', wasmFile('mars_red_bank'))
    await deployer.upload('addressProvider', wasmFile('mars_address_provider'))
    await deployer.upload('incentives', wasmFile('mars_incentives'))
    await deployer.upload('oracle', wasmFile(`mars_oracle_${config.oracle.name}`))
    await deployer.upload(
      'rewardsCollector',
      wasmFile(`mars_rewards_collector_${config.rewardsCollector.name}`),
    )
    await deployer.upload('swapper', wasmFile(`mars_swapper_${config.swapper.name}`))
    await deployer.upload('params', wasmFile(`mars_params`))
    await deployer.upload('accountNft', wasmFile('mars_account_nft'))
    await deployer.upload('mockVault', wasmFile('mars_mock_vault'))
    await deployer.upload('zapper', wasmFile(config.zapperContractName))
    await deployer.upload('creditManager', wasmFile('mars_credit_manager'))
    await deployer.upload('health', wasmFile('mars_rover_health'))
    await deployer.upload('perps', wasmFile('mars_perps'))

    // Instantiate contracts
    await deployer.instantiateAddressProvider()
    await deployer.instantiateRedBank()
    await deployer.instantiateIncentives()
    await deployer.instantiateOracle(config.oracle.customInitParams)
    await deployer.instantiateRewards()
    await deployer.instantiateSwapper()
    await deployer.instantiateParams()
    await deployer.instantiateMockVault()
    await deployer.instantiateZapper()
    await deployer.instantiateHealthContract()
    await deployer.instantiateCreditManager()
    await deployer.instantiateNftContract()
    await deployer.instantiatePerps()
    await deployer.setConfigOnHealthContract()
    await deployer.transferNftContractOwnership()
    await deployer.setConfigOnCreditManagerContract()
    await deployer.saveDeploymentAddrsToFile(label)

    await deployer.updateAddressProvider()

    if (config.swapper.name == 'astroport') {
      await deployer.updateSwapperAstroportConfig(config.astroportConfig!)
      await deployer.setAstroportIncentivesAddress(config.astroportConfig!.incentives!)
    }

    const oraclePriceSources: Map<string, PriceSourceResponseForString> =
      await deployer.queryOraclePriceSources(marsOracleAddr)
    const assetParams: Map<string, AssetParamsBaseForString> =
      await deployer.queryAssetParams(marsParamsAddr)
    const redBankMarkets: Map<string, MarketV2Response> =
      await deployer.queryRedBankMarkets(marsRedBankAddr)
    const perpParams: Map<string, PerpParams> = await deployer.queryPerpParams(marsParamsAddr)

    // Create a Set of denoms that have pcl_liquidity_token, lsd, or astroport_twap price sources
    const excludedDenoms = new Set<string>()
    for (const [denom, priceSource] of oraclePriceSources.entries()) {
      const source = priceSource.price_source
      if (
        source &&
        typeof source === 'object' &&
        ('pcl_liquidity_token' in source || 'lsd' in source || 'astroport_twap' in source)
      ) {
        excludedDenoms.add(denom)
        console.log(`Found excluded price source for denom: ${denom}`)
      }
    }
    excludedDenoms.add('uusd')

    // Filter oraclePriceSources
    const filteredOraclePriceSources = new Map<string, WasmPriceSourceForString>()
    for (const [denom, priceSource] of oraclePriceSources.entries()) {
      if (!excludedDenoms.has(denom)) {
        filteredOraclePriceSources.set(
          denom,
          priceSource.price_source as unknown as WasmPriceSourceForString,
        )
      }
    }

    await deployer.setOracle({ denom: 'usd', price_source: { fixed: { price: '1000000' } } })

    // setup
    for (const [denom, priceSource] of filteredOraclePriceSources.entries()) {
      await deployer.setOracle({ denom, price_source: priceSource })
    }

    for (const [denom, asset] of assetParams.entries()) {
      if (!excludedDenoms.has(denom)) {
        await deployer.updateAssetParamsV2(asset)
      }
    }

    for (const [denom, market] of redBankMarkets.entries()) {
      if (!excludedDenoms.has(denom)) {
        await deployer.initializeMarketV2(market)
      }
    }

    for (const [denom, perp] of perpParams.entries()) {
      if (!excludedDenoms.has(denom)) {
        await deployer.initializePerpDenomV2(perp)
      }
    }

    printYellow('COMPLETE')
  } catch (e) {
    printRed(e)
  } finally {
    await deployer.saveStorage()
  }
}
