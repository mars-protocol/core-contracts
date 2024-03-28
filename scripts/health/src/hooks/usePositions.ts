import useClients from './useClients.ts'
import useSWR from 'swr'

export default function usePositions(accountId: string) {
  const clients = useClients()

  return useSWR(
    accountId && `accounts/${accountId}/positions`,
    () => clients?.creditManager.positions({ accountId }),
  )
}
