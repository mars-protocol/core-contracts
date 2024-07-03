import { setupDeployer } from './setup-deployer'
import { printRed, printYellow } from '../../utils/chalk'
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
    await deployer.instantiatePerps(0)
    await deployer.setConfigOnHealthContract()
    await deployer.transferNftContractOwnership()
    await deployer.setConfigOnCreditManagerContract()
    await deployer.saveDeploymentAddrsToFile(label)

    await deployer.updateAddressProvider()

    // setup
    for (const asset of config.assets) {
      await deployer.updateAssetParams(asset)
      await deployer.initializeMarket(asset)
    }
    for (const vault of config.vaults) {
      await deployer.updateVaultConfig(vault)
    }
    if (config.perps) {
      for (const perp of config.perps?.denoms) {
        await deployer.initializePerpDenom(perp, 0)
      }
    }
    for (const oracleConfig of config.oracleConfigs) {
      await deployer.setOracle(oracleConfig)
    }
    await deployer.setRoutes()

    // User flows with gas usage
    const ntrnDenom = 'untrn'
    const nobleUsdcDenom = 'ibc/4C19E7EC06C1AB2EC2D70C6855FEB6D48E9CE174913991DA0A517D21978E7E42'
    const atomDenom = 'ibc/C4CFF46FD6DE35CA4CF4CE031E643C8FDC9BA4B99AE598E9B0ED98FE3A2319F9'
    const tiaDenom = 'factory/neutron166t9ww3p6flv7c86376fy0r92r88t3492xxj2h/utia'
    const solDenom = 'factory/neutron166t9ww3p6flv7c86376fy0r92r88t3492xxj2h/usol'

    const depositor = await deployer.deployerAsRoverClient()
    await depositor.createCreditAccount()
    const ntrnDepositAmt = '5000000'
    await depositor.deposit(ntrnDenom, ntrnDepositAmt)
    await depositor.lend(ntrnDenom, ntrnDepositAmt)
    const usdcDepositAmt = '10000000'
    await depositor.deposit(nobleUsdcDenom, usdcDepositAmt)
    await depositor.depositToPerpVault(nobleUsdcDenom, usdcDepositAmt)

    const liquidator = await deployer.deployerAsRoverClient()
    await liquidator.createCreditAccount()
    await liquidator.deposit(ntrnDenom, '5000000')

    const trader = await deployer.deployerAsRoverClient()
    await trader.createCreditAccount()
    await trader.deposit(nobleUsdcDenom, '22000000')
    await trader.borrow(ntrnDenom, '2000000')
    await trader.openPerp(atomDenom, 10000)
    await trader.openPerp(tiaDenom, 10000)
    await trader.openPerp(solDenom, 10000)

    // prepare for liquidation
    await deployer.setOracle({ denom: ntrnDenom, price_source: { fixed: { price: '15' } } }, true)
    await liquidator.liquidateDeposit(trader.accountId!, nobleUsdcDenom, {
      denom: ntrnDenom,
      amount: '1000000',
    })

    // reset price after liquidation
    await deployer.setOracle({ denom: ntrnDenom, price_source: { fixed: { price: '1' } } }, true)

    // close trader's positions
    await trader.repayFullBalance(ntrnDenom)
    await trader.refundAllBalances()

    // close liquidator's positions
    await liquidator.refundAllBalances()

    // close depositor's positions
    await depositor.reclaim(ntrnDenom, ntrnDepositAmt)
    await depositor.unlockFromPerpVault()
    await depositor.withdrawFromPerpVault()
    await depositor.refundAllBalances()

    printYellow('COMPLETE')
  } catch (e) {
    printRed(e)
  } finally {
    await deployer.saveStorage()
  }
}
