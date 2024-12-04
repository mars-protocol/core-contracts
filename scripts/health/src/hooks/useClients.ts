import { CosmWasmClient } from '@cosmjs/cosmwasm-stargate'
import useSWR from 'swr'
import { MarsAccountNftQueryClient } from '../../../types/generated/mars-account-nft/MarsAccountNft.client.ts'
import { MarsCreditManagerQueryClient } from '../../../types/generated/mars-credit-manager/MarsCreditManager.client.ts'
import { MarsIncentivesQueryClient } from '../../../types/generated/mars-incentives/MarsIncentives.client.ts'
import { MarsOracleOsmosisQueryClient } from '../../../types/generated/mars-oracle-osmosis/MarsOracleOsmosis.client.ts'
import { MarsParamsQueryClient } from '../../../types/generated/mars-params/MarsParams.client.ts'
import { MarsPerpsQueryClient } from '../../../types/generated/mars-perps/MarsPerps.client.ts'
import { MarsRedBankQueryClient } from '../../../types/generated/mars-red-bank/MarsRedBank.client.ts'
import { MarsSwapperOsmosisQueryClient } from '../../../types/generated/mars-swapper-osmosis/MarsSwapperOsmosis.client.ts'
import useChainConfig from './useChainConfig.ts'

export default function useClients() {
  const chainConfig = useChainConfig()

  const swr = useSWR(
    chainConfig.addresses && `chains/${chainConfig.chain}/clients`,
    async () => {
      const client = (await CosmWasmClient.connect(
        `${chainConfig.rpc}?x-apikey=${import.meta.env.VITE_API_KEY}`,
      )) as never
      if (!chainConfig.addresses) return
      return {
        creditManager: new MarsCreditManagerQueryClient(
          client,
          chainConfig.addresses.creditManager,
        ),
        accountNft: new MarsAccountNftQueryClient(client, chainConfig.addresses.accountNft),
        oracle: new MarsOracleOsmosisQueryClient(client, chainConfig.addresses.oracle),
        params: new MarsParamsQueryClient(client, chainConfig.addresses.params),
        redBank: new MarsRedBankQueryClient(client, chainConfig.addresses.redBank),
        swapper: new MarsSwapperOsmosisQueryClient(client, chainConfig.addresses.swapper),
        incentives: new MarsIncentivesQueryClient(client, chainConfig.addresses.incentives),
        perps: new MarsPerpsQueryClient(client, chainConfig.addresses.perps),
      }
    },
    {
      revalidateOnFocus: false,
      revalidateOnReconnect: false,
      revalidateIfStale: false,
      keepPreviousData: false,
    },
  )

  return swr.data
}
