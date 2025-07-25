import { setupDeployer } from './setup-deployer'
import { printGreen, printRed, printYellow } from '../../utils/chalk'
import { DeploymentConfig } from '../../types/config'
import { wasmFile } from '../../utils/environment'

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
    await deployer.upload('dualitySwapper', `mars_swapper_${config.dualitySwapper?.name}.wasm`)
    await deployer.upload('params', wasmFile(`mars_params`))
    await deployer.upload('accountNft', wasmFile('mars_account_nft'))
    await deployer.upload('mockVault', wasmFile('mars_mock_vault'))
    await deployer.upload('zapper', wasmFile(config.zapperContractName))
    await deployer.upload('creditManager', wasmFile('mars_credit_manager'))
    await deployer.upload('health', wasmFile('mars_rover_health'))
    await deployer.upload('perps', wasmFile('mars_perps'))
    await deployer.upload('vault', wasmFile('mars_vault'))

    // Instantiate contracts
    await deployer.instantiateAddressProvider()
    await deployer.instantiateRedBank()
    await deployer.instantiateIncentives()
    await deployer.instantiateOracle(config.oracle.customInitParams)
    await deployer.instantiateRewards()
    await deployer.instantiateSwapper()
    await deployer.instantiateDualitySwapper()
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

    if (config.dualitySwapper) {
      // Set up LP
      await deployer.setDualityRoutes()
      await deployer.setDualitySwapperLP()
    }
    // setup

    for (const oracleConfig of config.oracleConfigs) {
      await deployer.setOracle(oracleConfig)
    }

    for (const asset of config.assets) {
      await deployer.updateAssetParams(asset)
    }
    for (const vault of config.vaults) {
      await deployer.updateVaultConfig(vault)
    }
    if (config.perps) {
      for (const perp of config.perps?.denoms) {
        await deployer.initializePerpDenom(perp)
      }
    }

    // Test basic user flows
    if (config.runTests && config.testActions) {
      await deployer.executeDeposit()
      await deployer.executeBorrow()
      await deployer.executeRepay()
      await deployer.executeWithdraw()
      // await deployer.executeRewardsSwap()

      const rover = await deployer.newUserRoverClient(config.testActions)
      await rover.createCreditAccount()
      await rover.depositWithTestParams()
      await rover.lendWithTestParams()
      await rover.borrowWithTestParams()
      await rover.swapWithTestParams()
      await rover.repayWithTestParams()
      await rover.reclaimWithTestParams()
      await rover.withdrawWithTestParams()

      const vaultConfig = config.vaults[0].vault
      const info = await rover.getVaultInfo(vaultConfig)
      await rover.zapWithTestParams(info.tokens.base_token)
      await rover.vaultDepositWithTestParams(vaultConfig, info)
      if (info.lockup) {
        await rover.vaultRequestUnlockWithTestParams(vaultConfig, info)
      } else {
        await rover.vaultWithdrawWithTestParams(vaultConfig, info)
        await rover.unzapWithTestParams(info.tokens.base_token)
      }
      await rover.refundAllBalances()
    }

    // If multisig is set, transfer ownership from deployer to multisig
    if (config.multisigAddr) {
      await deployer.updateIncentivesContractOwner()
      await deployer.updateRedBankContractOwner()
      await deployer.updateOracleContractOwner()
      await deployer.updateRewardsContractOwner()
      await deployer.updateSwapperContractOwner()
      await deployer.updateParamsContractOwner()
      await deployer.updateAddressProviderContractOwner()
      await deployer.updateCreditManagerOwner()
      await deployer.updateHealthOwner()
      printGreen('It is confirmed that all contracts have transferred ownership to the Multisig')
    } else {
      printGreen('Owner remains the deployer address.')
    }

    printYellow('COMPLETE')
  } catch (e) {
    printRed(e)
  } finally {
    await deployer.saveStorage()
  }
}
