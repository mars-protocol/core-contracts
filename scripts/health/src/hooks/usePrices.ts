import BigNumber from 'bignumber.js'
import useSWR from 'swr'
import useChainConfig from './useChainConfig'
import useClients from './useClients.ts'
import { MarsOracleWasmQueryClient } from '../../../types/generated/mars-oracle-wasm/MarsOracleWasm.client.ts'
import { ArrayOfPriceResponse } from '../../../types/generated/mars-oracle-osmosis/MarsOracleOsmosis.types.ts'

const LIMIT = 10
BigNumber.config({ EXPONENTIAL_AT: 1e9 })

export default function usePrices() {
  const chainConfig = useChainConfig()
  const clients = useClients()

  return useSWR(clients && `chains/${chainConfig.chain}/prices`, () => {
    if (!clients?.oracle) return
    return getOraclePrices(clients.oracle)
  })
}

async function getOraclePrices(
  oracleClient: MarsOracleWasmQueryClient,
  previousPrices: ArrayOfPriceResponse = [],
): Promise<{ [key: string]: string }> {
  const startAfter = previousPrices.at(-1)?.denom
  const response = await oracleClient.prices({ limit: LIMIT, startAfter })

  previousPrices = previousPrices.concat(response)

  if (Object.keys(response).length === LIMIT) {
    return getOraclePrices(oracleClient, previousPrices)
  }

  const prices: { [key: string]: string } = {}
  previousPrices.forEach((price) => {
    prices[price.denom] = price.price
  })

  return prices
}
