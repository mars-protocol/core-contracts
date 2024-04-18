import useSWR from 'swr'

import { PerpDenomState } from '../../../types/generated/mars-perps/MarsPerps.types.ts'
import useChainConfig from './useChainConfig.ts'
import useClients from './useClients.ts'
import usePerpsParams from './usePerpsParams.ts'

export default function useAllPerpsDenomStates() {
  const clients = useClients()
  const { data: perpsParams } = usePerpsParams()
  const chainConfig = useChainConfig()

  return useSWR(
    clients && perpsParams && `chains/${chainConfig.chain}/perps/state`,
    async () => {
      if (!perpsParams) return
      const promises = [] as Promise<PerpDenomState>[]
      Object.keys(perpsParams)!.forEach((perp) => {
        if (!chainConfig?.addresses?.perps) return
        promises.push(clients!.perps.perpDenomState({ denom: perp }))
      })

      const result = await Promise.all(promises)
      const perpDenomStates: { [key: string]: PerpDenomState } = {}

      result.forEach((perpState) => (perpDenomStates[perpState.denom] = perpState))

      return perpDenomStates
    },
    {
      revalidateOnFocus: false,
      revalidateOnReconnect: false,
      revalidateIfStale: false,
      keepPreviousData: false,
    },
  )
}
