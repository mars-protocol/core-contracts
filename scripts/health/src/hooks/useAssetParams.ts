import useSWR from 'swr'
import useClients from './useClients.ts'
import { AssetParamsBaseForAddr } from '../../../types/generated/mars-params/MarsParams.types.ts'

export default function useAssetParams() {
  const clients = useClients()

  return useSWR(clients && `chains/pion-1/assets/params`, async () => {
    if (!clients) return
    const result = await clients.params.allAssetParams({})

    const assetParams: { [key: string]: AssetParamsBaseForAddr } = {}
    result.forEach((perpState) => (assetParams[perpState.denom] = perpState))

    return assetParams
  })
}
