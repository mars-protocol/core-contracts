import useSWR from 'swr'
import useChainConfig from './useChainConfig.ts'
import useClients from './useClients.ts'

export default function usePositions(accountId: string) {
  const clients = useClients()
  const chainConfig = useChainConfig()

  return useSWR(
    accountId && `accounts/${accountId}/positions`,
    async () => {
      const result = await clients?.creditManager.positions({ accountId })
      if(chainConfig.addresses?.perps) return result
      
      return {...result, perps: []}
  },
  )
}
