import useSWR from 'swr'

import usePerpsParams from './usePerpsParams.ts'
import useClients from './useClients.ts'
import { PerpDenomState } from '../../../types/generated/mars-perps/MarsPerps.types.ts'

export default function useAllPerpsDenomStates() {
  const clients = useClients()
  const { data: perpsParams } = usePerpsParams()

  return useSWR(clients && perpsParams && `chains/pion-1/perps/state`, async () => {
    if (!perpsParams) return
    const promises = Object.keys(perpsParams)!.map((perp) =>
      clients!.perps.perpDenomState({ denom: perp }),
    )

    const result = await Promise.all(promises)
    const perpDenomStates: { [key: string]: PerpDenomState } = {}

    result.forEach((perpState) => (perpDenomStates[perpState.denom] = perpState))

    return perpDenomStates
  })
}
