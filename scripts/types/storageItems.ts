export interface StorageItems {
  codeIds: {
    accountNft?: number
    addressProvider?: number
    creditManager?: number
    health?: number
    incentives?: number
    mockVault?: number
    oracle?: number
    params?: number
    swapper?: number
    dualitySwapper?: number
    redBank?: number
    rewardsCollector?: number
    zapper?: number
    perps?: number
    vault?: number
  }

  addresses: {
    accountNft?: string
    addressProvider?: string
    creditManager?: string
    health?: string
    incentives?: string
    mockVault?: string
    oracle?: string
    params?: string
    swapper?: string
    dualitySwapper?: string
    redBank?: string
    rewardsCollector?: string
    zapper?: string
    perps?: string
  }

  actions: {
    addressProviderSet: Record<string, boolean>
    proposedNewOwner?: boolean
    acceptedOwnership?: boolean
    seedMockVault?: boolean
    grantedCreditLines?: boolean
    redBankMarketsSet: string[]
    assetsSet: string[]
    vaultsSet: string[]
    perpsSet: string[]
    oraclePricesSet: string[]
    routesSet: string[]
    dualityRoutesSet: string[]
    dualityLpProvided?: boolean
    healthContractConfigUpdate?: boolean
    creditManagerContractConfigUpdate?: boolean
  }
}
