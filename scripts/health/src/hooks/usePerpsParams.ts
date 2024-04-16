import useSWR from 'swr'
import useClients from './useClients.ts'
import { PerpParams } from '../../../types/generated/mars-params/MarsParams.types.ts'

export default function usePerpsParams() {
  const clients = useClients()

  return useSWR(clients && `chains/pion-1/perps/params`, async () => {
    if (!clients) return
    const result = await clients.params.allPerpParams({})
    const perpParams: { [key: string]: PerpParams } = {}
    result.forEach((perpState) => (perpParams[perpState.denom] = perpState))

    return perpParams
  })
}
