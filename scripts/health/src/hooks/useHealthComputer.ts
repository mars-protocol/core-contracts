import { useMemo } from 'react'
import { HealthComputer } from '../../pkg-web'
import useAssetParams from './useAssetParams.ts'
import useChainConfig from './useChainConfig.ts'
import useAllPerpsDenomStates from './usePerpDenomStates.ts'
import usePerpsParams from './usePerpsParams.ts'
import usePositions from './usePositions.ts'
import usePrices from './usePrices.ts'

export default function useHealthComputer(accountId: string) {
  const { data: positions } = usePositions(accountId)
  const { data: prices } = usePrices()
  const { data: perpsDenomStates } = useAllPerpsDenomStates()
  const { data: assetParams } = useAssetParams()
  const { data: perpsParams } = usePerpsParams()
  const chainConfig = useChainConfig()
  const hasPerps = chainConfig.addresses?.perps

  return useMemo(() => {
    return {
      data: {
        kind: 'default',
        positions,
        oracle_prices: prices,
        asset_params: assetParams,
        vaults_data: {
          vault_configs: {},
          vault_values: {},
        },
        perps_data: {
          market_states: hasPerps ? perpsDenomStates : {},
          params: hasPerps ? perpsParams : {},
        },
      } as HealthComputer,
    }
  }, [positions, prices, assetParams, perpsDenomStates, perpsParams, hasPerps])
}
