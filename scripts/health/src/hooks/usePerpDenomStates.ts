import useSWR from 'swr'

import { MarketStateResponse } from '../../../types/generated/mars-perps/MarsPerps.types.ts'
import useChainConfig from './useChainConfig.ts'
import useClients from './useClients.ts'
import usePerpsParams from './usePerpsParams.ts'

export default function useAllPerpsMarketStates() {
  const clients = useClients()
  const { data: perpsParams } = usePerpsParams()
  const chainConfig = useChainConfig()

  return useSWR(
    clients && perpsParams && `chains/${chainConfig.chain}/perps/state`,
    async () => {
      if (!perpsParams) return
      const promises = [] as Promise<MarketStateResponse>[]
      Object.keys(perpsParams)!.forEach((perp) => {
        if (!chainConfig?.addresses?.perps) return
        promises.push(clients!.perps.marketState({ denom: perp }))
      })

      const result = await Promise.all(promises)
      const perpMarketStates: { [key: string]: MarketStateResponse } = {}

      result.forEach((perpState) => (perpMarketStates[perpState.denom] = perpState))

      return perpMarketStates
    },
    {
      revalidateOnFocus: false,
      revalidateOnReconnect: false,
      revalidateIfStale: false,
      keepPreviousData: false,
    },
  )
}
