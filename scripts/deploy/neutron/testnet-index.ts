import { taskRunner } from '../base/index.js'
import { neutronTestnetConfig } from './testnet-config.js'

void (async function () {
  await taskRunner({
    config: neutronTestnetConfig,
    label: 'deployer-owner',
  })
})()
