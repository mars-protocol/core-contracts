import useSWR from 'swr'
import { AssetParamsBaseForAddr } from '../../../types/generated/mars-params/MarsParams.types.ts'
import useChainConfig from './useChainConfig.ts'
import useClients from './useClients.ts'

export default function useAssetParams() {
  const clients = useClients()
  const chainConfig = useChainConfig()

  return useSWR(clients && `chains/${chainConfig.chain}/assets/params`, async () => {
    if (!clients) return
    const result = await clients.params.allAssetParams({limit: 100})

    const assetParams: { [key: string]: AssetParamsBaseForAddr } = {}
    result.forEach((asset) => {
      assetParams[asset.denom] = asset
      if(!asset.close_factor) assetParams[asset.denom].close_factor = '0'
  })
    return assetParams
  },
  {
    revalidateOnFocus: false,
    revalidateOnReconnect: false,
    revalidateIfStale: false,
    keepPreviousData: false,
  })
}
