// @ts-nocheck
/**
 * This file was automatically generated by @cosmwasm/ts-codegen@0.24.0.
 * DO NOT MODIFY IT BY HAND. Instead, modify the source JSONSchema file,
 * and run the @cosmwasm/ts-codegen generate command to regenerate this file.
 */

import { MsgExecuteContractEncodeObject } from 'cosmwasm'
import { MsgExecuteContract } from 'cosmjs-types/cosmwasm/wasm/v1/tx'
import { toUtf8 } from '@cosmjs/encoding'
import {
  Decimal,
  Uint128,
  OracleBaseForString,
  RedBankBaseForString,
  SwapperBaseForString,
  ZapperBaseForString,
  InstantiateMsg,
  VaultInstantiateConfig,
  VaultConfig,
  Coin,
  VaultBaseForString,
  ExecuteMsg,
  Action,
  ActionAmount,
  VaultPositionType,
  OwnerUpdate,
  CallbackMsg,
  Addr,
  ActionCoin,
  ConfigUpdates,
  NftConfigUpdates,
  VaultBaseForAddr,
  QueryMsg,
  ArrayOfCoinBalanceResponseItem,
  CoinBalanceResponseItem,
  ArrayOfSharesResponseItem,
  SharesResponseItem,
  ArrayOfDebtShares,
  DebtShares,
  ArrayOfVaultWithBalance,
  VaultWithBalance,
  VaultPositionAmount,
  VaultAmount,
  VaultAmount1,
  UnlockingPositions,
  ArrayOfVaultPositionResponseItem,
  VaultPositionResponseItem,
  VaultPosition,
  LockingVaultAmount,
  VaultUnlockingPosition,
  ArrayOfString,
  ConfigResponse,
  ArrayOfCoin,
  HealthResponse,
  Positions,
  DebtAmount,
  ArrayOfVaultInfoResponse,
  VaultInfoResponse,
} from './MarsCreditManager.types'
export interface MarsCreditManagerMessage {
  contractAddress: string
  sender: string
  createCreditAccount: (funds?: Coin[]) => MsgExecuteContractEncodeObject
  updateCreditAccount: (
    {
      accountId,
      actions,
    }: {
      accountId: string
      actions: Action[]
    },
    funds?: Coin[],
  ) => MsgExecuteContractEncodeObject
  updateConfig: (
    {
      updates,
    }: {
      updates: ConfigUpdates
    },
    funds?: Coin[],
  ) => MsgExecuteContractEncodeObject
  updateOwner: (funds?: Coin[]) => MsgExecuteContractEncodeObject
  updateNftConfig: (
    {
      updates,
    }: {
      updates: NftConfigUpdates
    },
    funds?: Coin[],
  ) => MsgExecuteContractEncodeObject
  callback: (funds?: Coin[]) => MsgExecuteContractEncodeObject
}
export class MarsCreditManagerMessageComposer implements MarsCreditManagerMessage {
  sender: string
  contractAddress: string

  constructor(sender: string, contractAddress: string) {
    this.sender = sender
    this.contractAddress = contractAddress
    this.createCreditAccount = this.createCreditAccount.bind(this)
    this.updateCreditAccount = this.updateCreditAccount.bind(this)
    this.updateConfig = this.updateConfig.bind(this)
    this.updateOwner = this.updateOwner.bind(this)
    this.updateNftConfig = this.updateNftConfig.bind(this)
    this.callback = this.callback.bind(this)
  }

  createCreditAccount = (funds?: Coin[]): MsgExecuteContractEncodeObject => {
    return {
      typeUrl: '/cosmwasm.wasm.v1.MsgExecuteContract',
      value: MsgExecuteContract.fromPartial({
        sender: this.sender,
        contract: this.contractAddress,
        msg: toUtf8(
          JSON.stringify({
            create_credit_account: {},
          }),
        ),
        funds,
      }),
    }
  }
  updateCreditAccount = (
    {
      accountId,
      actions,
    }: {
      accountId: string
      actions: Action[]
    },
    funds?: Coin[],
  ): MsgExecuteContractEncodeObject => {
    return {
      typeUrl: '/cosmwasm.wasm.v1.MsgExecuteContract',
      value: MsgExecuteContract.fromPartial({
        sender: this.sender,
        contract: this.contractAddress,
        msg: toUtf8(
          JSON.stringify({
            update_credit_account: {
              account_id: accountId,
              actions,
            },
          }),
        ),
        funds,
      }),
    }
  }
  updateConfig = (
    {
      updates,
    }: {
      updates: ConfigUpdates
    },
    funds?: Coin[],
  ): MsgExecuteContractEncodeObject => {
    return {
      typeUrl: '/cosmwasm.wasm.v1.MsgExecuteContract',
      value: MsgExecuteContract.fromPartial({
        sender: this.sender,
        contract: this.contractAddress,
        msg: toUtf8(
          JSON.stringify({
            update_config: {
              updates,
            },
          }),
        ),
        funds,
      }),
    }
  }
  updateOwner = (funds?: Coin[]): MsgExecuteContractEncodeObject => {
    return {
      typeUrl: '/cosmwasm.wasm.v1.MsgExecuteContract',
      value: MsgExecuteContract.fromPartial({
        sender: this.sender,
        contract: this.contractAddress,
        msg: toUtf8(
          JSON.stringify({
            update_owner: {},
          }),
        ),
        funds,
      }),
    }
  }
  updateNftConfig = (
    {
      updates,
    }: {
      updates: NftConfigUpdates
    },
    funds?: Coin[],
  ): MsgExecuteContractEncodeObject => {
    return {
      typeUrl: '/cosmwasm.wasm.v1.MsgExecuteContract',
      value: MsgExecuteContract.fromPartial({
        sender: this.sender,
        contract: this.contractAddress,
        msg: toUtf8(
          JSON.stringify({
            update_nft_config: {
              updates,
            },
          }),
        ),
        funds,
      }),
    }
  }
  callback = (funds?: Coin[]): MsgExecuteContractEncodeObject => {
    return {
      typeUrl: '/cosmwasm.wasm.v1.MsgExecuteContract',
      value: MsgExecuteContract.fromPartial({
        sender: this.sender,
        contract: this.contractAddress,
        msg: toUtf8(
          JSON.stringify({
            callback: {},
          }),
        ),
        funds,
      }),
    }
  }
}
