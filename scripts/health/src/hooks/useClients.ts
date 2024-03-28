import { CosmWasmClient } from '@cosmjs/cosmwasm-stargate'
import useSWR from 'swr'
import { MarsCreditManagerQueryClient } from '../../../types/generated/mars-credit-manager/MarsCreditManager.client.ts'
import { MarsAccountNftQueryClient } from '../../../types/generated/mars-account-nft/MarsAccountNft.client.ts'
import { MarsOracleOsmosisQueryClient } from '../../../types/generated/mars-oracle-osmosis/MarsOracleOsmosis.client.ts'
import { MarsParamsQueryClient } from '../../../types/generated/mars-params/MarsParams.client.ts'
import { MarsRedBankQueryClient } from '../../../types/generated/mars-red-bank/MarsRedBank.client.ts'
import { MarsSwapperOsmosisQueryClient } from '../../../types/generated/mars-swapper-osmosis/MarsSwapperOsmosis.client.ts'
import { MarsIncentivesQueryClient } from '../../../types/generated/mars-incentives/MarsIncentives.client.ts'
import { MarsPerpsQueryClient } from '../../../types/generated/mars-perps/MarsPerps.client.ts'
import { useEffect, useState } from 'react'

export default function useClients() {
  const [addresses, setAddresses] = useState<{ [key: string]: string }>()

  useEffect(() => {
    import('../../../deploy/addresses/pion-1-deployer-owner.json').then((json) =>
      setAddresses(json.default),
    )
  }, [])

  const swr = useSWR(
    addresses && `chains/pion-1/clients`,
    async () => {
      const client = (await CosmWasmClient.connect(
        `https://rpc-palvus.pion-1.ntrn.tech?x-apikey=${import.meta.env.VITE_API_KEY}`,
      )) as never
      if (!addresses) return
      return {
        creditManager: new MarsCreditManagerQueryClient(client, addresses.creditManager),
        accountNft: new MarsAccountNftQueryClient(client, addresses.accountNft),
        oracle: new MarsOracleOsmosisQueryClient(client, addresses.oracle),
        params: new MarsParamsQueryClient(client, addresses.params),
        redBank: new MarsRedBankQueryClient(client, addresses.redBank),
        swapper: new MarsSwapperOsmosisQueryClient(client, addresses.swapper),
        incentives: new MarsIncentivesQueryClient(client, addresses.incentives),
        perps: new MarsPerpsQueryClient(client, addresses.perps),
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
