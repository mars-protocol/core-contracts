import { taskRunner } from '../base/index-mainnet-mirror.js'
import { neutronMainnetConfig } from './mainnet-mirror-config'

void (async function () {
  await taskRunner({
    config: neutronMainnetConfig,
    label: 'deployer-owner',
  })
})()
