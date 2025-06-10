import { taskRunner } from '../base/index-mirror-mainnet.js'
import { neutronMainnetConfig } from './mainnet-config'

void (async function () {
  await taskRunner({
    config: neutronMainnetConfig,
    label: 'deployer-owner',
  })
})()
