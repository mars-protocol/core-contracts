import useSWR from 'swr'
import { PerpParams } from '../../../types/generated/mars-params/MarsParams.types.ts'
import useChainConfig from './useChainConfig.ts'
import useClients from './useClients.ts'

export default function usePerpsParams() {
  const clients = useClients()
  const chainConfig = useChainConfig()

  return useSWR(clients && `chains/${chainConfig.chain}/perps/params`, async () => {
    if (!clients || !chainConfig?.addresses?.perps) return

    const result = await clients.params.allPerpParams({})
    const perpParams: { [key: string]: PerpParams } = {}
    result.forEach((perpState) => (perpParams[perpState.denom] = perpState))

    return perpParams
  },
  {
    revalidateOnFocus: false,
    revalidateOnReconnect: false,
    revalidateIfStale: false,
    keepPreviousData: false,
  },
)
}
