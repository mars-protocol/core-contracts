import useSWR from 'swr'
import { PerpParams } from '../../../types/generated/mars-params/MarsParams.types.ts'
import useClients from './useClients.ts'
import useChainConfig from './useChainConfig.ts'

export default function usePerpsParams() {
  const clients = useClients()
  const chainConfig = useChainConfig()

  return useSWR(clients?.perps && `chains/${chainConfig.chain}/perps/params`, async () => {
    if (!clients) return
    const result = await clients.params.allPerpParams({})
    const perpParams: { [key: string]: PerpParams } = {}
    result.forEach((perpState) => (perpParams[perpState.denom] = perpState))

    return perpParams
  })
}
