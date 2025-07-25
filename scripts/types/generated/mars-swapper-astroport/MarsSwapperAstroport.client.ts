// @ts-nocheck
/**
 * This file was automatically generated by @cosmwasm/ts-codegen@1.10.0.
 * DO NOT MODIFY IT BY HAND. Instead, modify the source JSONSchema file,
 * and run the @cosmwasm/ts-codegen generate command to regenerate this file.
 */

import { CosmWasmClient, SigningCosmWasmClient, ExecuteResult } from '@cosmjs/cosmwasm-stargate'
import { StdFee } from '@cosmjs/amino'
import {
  InstantiateMsg,
  ExecuteMsg,
  OwnerUpdate,
  SwapOperation,
  AssetInfo,
  Addr,
  Uint128,
  SwapperRoute,
  AstroportRoute,
  Coin,
  AstroRoute,
  AstroSwap,
  DualityRoute,
  OsmoRoute,
  OsmoSwap,
  AstroportConfig,
  QueryMsg,
  Empty,
  EstimateExactInSwapResponse,
  OwnerResponse,
  RouteResponseForEmpty,
  ArrayOfRouteResponseForEmpty,
} from './MarsSwapperAstroport.types'
export interface MarsSwapperAstroportReadOnlyInterface {
  contractAddress: string
  owner: () => Promise<OwnerResponse>
  route: ({
    denomIn,
    denomOut,
  }: {
    denomIn: string
    denomOut: string
  }) => Promise<RouteResponseForEmpty>
  routes: ({
    limit,
    startAfter,
  }: {
    limit?: number
    startAfter?: string[][]
  }) => Promise<ArrayOfRouteResponseForEmpty>
  estimateExactInSwap: ({
    coinIn,
    denomOut,
    route,
  }: {
    coinIn: Coin
    denomOut: string
    route?: SwapperRoute
  }) => Promise<EstimateExactInSwapResponse>
  config: () => Promise<Empty>
}
export class MarsSwapperAstroportQueryClient implements MarsSwapperAstroportReadOnlyInterface {
  client: CosmWasmClient
  contractAddress: string
  constructor(client: CosmWasmClient, contractAddress: string) {
    this.client = client
    this.contractAddress = contractAddress
    this.owner = this.owner.bind(this)
    this.route = this.route.bind(this)
    this.routes = this.routes.bind(this)
    this.estimateExactInSwap = this.estimateExactInSwap.bind(this)
    this.config = this.config.bind(this)
  }
  owner = async (): Promise<OwnerResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      owner: {},
    })
  }
  route = async ({
    denomIn,
    denomOut,
  }: {
    denomIn: string
    denomOut: string
  }): Promise<RouteResponseForEmpty> => {
    return this.client.queryContractSmart(this.contractAddress, {
      route: {
        denom_in: denomIn,
        denom_out: denomOut,
      },
    })
  }
  routes = async ({
    limit,
    startAfter,
  }: {
    limit?: number
    startAfter?: string[][]
  }): Promise<ArrayOfRouteResponseForEmpty> => {
    return this.client.queryContractSmart(this.contractAddress, {
      routes: {
        limit,
        start_after: startAfter,
      },
    })
  }
  estimateExactInSwap = async ({
    coinIn,
    denomOut,
    route,
  }: {
    coinIn: Coin
    denomOut: string
    route?: SwapperRoute
  }): Promise<EstimateExactInSwapResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      estimate_exact_in_swap: {
        coin_in: coinIn,
        denom_out: denomOut,
        route,
      },
    })
  }
  config = async (): Promise<Empty> => {
    return this.client.queryContractSmart(this.contractAddress, {
      config: {},
    })
  }
}
export interface MarsSwapperAstroportInterface extends MarsSwapperAstroportReadOnlyInterface {
  contractAddress: string
  sender: string
  updateOwner: (
    ownerUpdate: OwnerUpdate,
    fee?: number | StdFee | 'auto',
    memo?: string,
    _funds?: Coin[],
  ) => Promise<ExecuteResult>
  setRoute: (
    {
      denomIn,
      denomOut,
      route,
    }: {
      denomIn: string
      denomOut: string
      route: AstroportRoute
    },
    fee?: number | StdFee | 'auto',
    memo?: string,
    _funds?: Coin[],
  ) => Promise<ExecuteResult>
  swapExactIn: (
    {
      coinIn,
      denomOut,
      minReceive,
      route,
    }: {
      coinIn: Coin
      denomOut: string
      minReceive: Uint128
      route?: SwapperRoute
    },
    fee?: number | StdFee | 'auto',
    memo?: string,
    _funds?: Coin[],
  ) => Promise<ExecuteResult>
  transferResult: (
    {
      denomIn,
      denomOut,
      recipient,
    }: {
      denomIn: string
      denomOut: string
      recipient: Addr
    },
    fee?: number | StdFee | 'auto',
    memo?: string,
    _funds?: Coin[],
  ) => Promise<ExecuteResult>
  updateConfig: (
    {
      config,
    }: {
      config: AstroportConfig
    },
    fee?: number | StdFee | 'auto',
    memo?: string,
    _funds?: Coin[],
  ) => Promise<ExecuteResult>
}
export class MarsSwapperAstroportClient
  extends MarsSwapperAstroportQueryClient
  implements MarsSwapperAstroportInterface
{
  client: SigningCosmWasmClient
  sender: string
  contractAddress: string
  constructor(client: SigningCosmWasmClient, sender: string, contractAddress: string) {
    super(client, contractAddress)
    this.client = client
    this.sender = sender
    this.contractAddress = contractAddress
    this.updateOwner = this.updateOwner.bind(this)
    this.setRoute = this.setRoute.bind(this)
    this.swapExactIn = this.swapExactIn.bind(this)
    this.transferResult = this.transferResult.bind(this)
    this.updateConfig = this.updateConfig.bind(this)
  }
  updateOwner = async (
    ownerUpdate: OwnerUpdate,
    fee: number | StdFee | 'auto' = 'auto',
    memo?: string,
    _funds?: Coin[],
  ): Promise<ExecuteResult> => {
    return await this.client.execute(
      this.sender,
      this.contractAddress,
      {
        update_owner: ownerUpdate,
      },
      fee,
      memo,
      _funds,
    )
  }
  setRoute = async (
    {
      denomIn,
      denomOut,
      route,
    }: {
      denomIn: string
      denomOut: string
      route: AstroportRoute
    },
    fee: number | StdFee | 'auto' = 'auto',
    memo?: string,
    _funds?: Coin[],
  ): Promise<ExecuteResult> => {
    return await this.client.execute(
      this.sender,
      this.contractAddress,
      {
        set_route: {
          denom_in: denomIn,
          denom_out: denomOut,
          route,
        },
      },
      fee,
      memo,
      _funds,
    )
  }
  swapExactIn = async (
    {
      coinIn,
      denomOut,
      minReceive,
      route,
    }: {
      coinIn: Coin
      denomOut: string
      minReceive: Uint128
      route?: SwapperRoute
    },
    fee: number | StdFee | 'auto' = 'auto',
    memo?: string,
    _funds?: Coin[],
  ): Promise<ExecuteResult> => {
    return await this.client.execute(
      this.sender,
      this.contractAddress,
      {
        swap_exact_in: {
          coin_in: coinIn,
          denom_out: denomOut,
          min_receive: minReceive,
          route,
        },
      },
      fee,
      memo,
      _funds,
    )
  }
  transferResult = async (
    {
      denomIn,
      denomOut,
      recipient,
    }: {
      denomIn: string
      denomOut: string
      recipient: Addr
    },
    fee: number | StdFee | 'auto' = 'auto',
    memo?: string,
    _funds?: Coin[],
  ): Promise<ExecuteResult> => {
    return await this.client.execute(
      this.sender,
      this.contractAddress,
      {
        transfer_result: {
          denom_in: denomIn,
          denom_out: denomOut,
          recipient,
        },
      },
      fee,
      memo,
      _funds,
    )
  }
  updateConfig = async (
    {
      config,
    }: {
      config: AstroportConfig
    },
    fee: number | StdFee | 'auto' = 'auto',
    memo?: string,
    _funds?: Coin[],
  ): Promise<ExecuteResult> => {
    return await this.client.execute(
      this.sender,
      this.contractAddress,
      {
        update_config: {
          config,
        },
      },
      fee,
      memo,
      _funds,
    )
  }
}
