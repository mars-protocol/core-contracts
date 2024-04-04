import { Storage } from './storage'
import { DeploymentConfig, TestActions, VaultInfo } from '../../types/config'
import { difference } from 'lodash'
import assert from 'assert'
import { printBlue, printGreen, printYellow } from '../../utils/chalk'
import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate'
import {
  MarsCreditManagerClient,
  MarsCreditManagerQueryClient,
} from '../../types/generated/mars-credit-manager/MarsCreditManager.client'
import { MarsAccountNftQueryClient } from '../../types/generated/mars-account-nft/MarsAccountNft.client'
import {
  Action,
  Coin,
  ConfigUpdates,
  ExecuteMsg,
} from '../../types/generated/mars-credit-manager/MarsCreditManager.types'
import { MarsMockVaultQueryClient } from '../../types/generated/mars-mock-vault/MarsMockVault.client'
import { VaultConfigBaseForString } from '../../types/generated/mars-params/MarsParams.types'

export class Rover {
  private exec: MarsCreditManagerClient
  private query: MarsCreditManagerQueryClient
  private nft: MarsAccountNftQueryClient
  accountId?: string

  constructor(
    private userAddr: string,
    private storage: Storage,
    private config: DeploymentConfig,
    private cwClient: SigningCosmWasmClient,
    private actions?: TestActions,
  ) {
    this.exec = new MarsCreditManagerClient(cwClient, userAddr, storage.addresses.creditManager!)
    this.query = new MarsCreditManagerQueryClient(cwClient, storage.addresses.creditManager!)
    this.nft = new MarsAccountNftQueryClient(cwClient, storage.addresses.accountNft!)
  }

  async updateConfig(updates: ConfigUpdates) {
    await this.exec.updateConfig({ updates })
  }

  async createCreditAccount() {
    const before = await this.nft.tokens({ owner: this.userAddr })
    const executeMsg = { create_credit_account: 'default' } satisfies ExecuteMsg
    const response = await this.cwClient.execute(
      this.userAddr,
      this.storage.addresses.creditManager!,
      executeMsg,
      'auto',
    )
    printYellow(
      `Create credit account, gas used: ${response.gasUsed}, tx: ${response.transactionHash}`,
    )
    const after = await this.nft.tokens({ owner: this.userAddr })
    const diff = difference(after.tokens, before.tokens)
    assert.equal(diff.length, 1)
    this.accountId = diff[0]
    printGreen(`Newly created credit account id: #${diff[0]}`)
  }

  async reuseCreditAccount(accountId: string) {
    this.accountId = accountId
    printGreen(`Reuse credit account id: #${accountId}`)
  }

  async depositWithTestParams() {
    const denom = this.config.chain.baseDenom
    const amount = this.actions!.depositAmount
    await this.deposit(denom, amount)

    const positions = await this.query.positions({ accountId: this.accountId! })
    assert.equal(positions.deposits.length, 1)
    assert.equal(positions.deposits[0].amount, amount)
    assert.equal(positions.deposits[0].denom, denom)
    printGreen(`Deposited into credit account: ${amount} ${denom}`)
  }

  async deposit(denom: string, amount: string) {
    const response = await this.updateCreditAccount(
      [{ deposit: { amount, denom } }],
      [{ amount: amount, denom }],
    )
    printYellow(`Deposit, gas used: ${response.gasUsed}, tx: ${response.transactionHash}`)
  }

  async lendWithTestParams() {
    const denom = this.config.chain.baseDenom
    const amount = this.actions!.lendAmount
    await this.lend(denom, amount)

    const positions = await this.query.positions({ accountId: this.accountId! })
    assert.equal(positions.lends.length, 1)
    assert.equal(positions.lends[0].denom, denom)
    printGreen(`Lent to Red Bank: ${amount} ${denom}`)
  }

  async lend(denom: string, amount: string) {
    const response = await this.updateCreditAccount(
      [{ lend: { amount: { exact: amount }, denom } }],
      [],
    )
    printYellow(`Lend, gas used: ${response.gasUsed}, tx: ${response.transactionHash}`)
  }

  async withdrawWithTestParams() {
    const denom = this.config.chain.baseDenom
    const amount = this.actions!.withdrawAmount

    const positionsBefore = await this.query.positions({ accountId: this.accountId! })
    const beforeWithdraw = parseFloat(
      positionsBefore.deposits.find((c) => c.denom === denom)!.amount,
    )

    await this.withdraw(denom, amount)

    const positionsAfter = await this.query.positions({ accountId: this.accountId! })
    const afterWithdraw = parseFloat(positionsAfter.deposits.find((c) => c.denom === denom)!.amount)
    assert.equal(beforeWithdraw - afterWithdraw, amount)
    printGreen(`Withdrew: ${amount} ${denom}`)
  }

  async withdraw(denom: string, amount: string) {
    const response = await this.updateCreditAccount([
      { withdraw: { amount: { exact: amount }, denom } },
    ])
    printYellow(`Withdraw, gas used: ${response.gasUsed}, tx: ${response.transactionHash}`)
  }

  async borrowWithTestParams() {
    const denom = this.config.chain.baseDenom
    const amount = this.actions!.borrowAmount
    await this.borrow(denom, amount)

    const positions = await this.query.positions({ accountId: this.accountId! })
    assert.equal(positions.debts.length, 1)
    assert.equal(positions.debts[0].denom, denom)
    printGreen(`Borrowed from RedBank: ${amount} ${denom}`)
  }

  async borrow(denom: string, amount: string) {
    const response = await this.updateCreditAccount([{ borrow: { amount, denom } }])
    printYellow(`Borrow, gas used: ${response.gasUsed}, tx: ${response.transactionHash}`)
  }

  async repayWithTestParams() {
    const denom = this.config.chain.baseDenom
    const amount = this.actions!.repayAmount
    await this.repay(denom, amount)

    const positions = await this.query.positions({ accountId: this.accountId! })
    printGreen(
      `Repaid to RedBank: ${amount} ${denom}. Debt remaining: ${JSON.stringify(positions.debts)}`,
    )
  }

  async repayFullBalance(denom: string) {
    const response = await this.updateCreditAccount([
      { repay: { coin: { amount: 'account_balance', denom } } },
    ])
    printYellow(`Repay, gas used: ${response.gasUsed}, tx: ${response.transactionHash}`)
  }

  async repay(denom: string, amount: string) {
    const response = await this.updateCreditAccount([
      { repay: { coin: { amount: { exact: amount }, denom } } },
    ])
    printYellow(`Repay, gas used: ${response.gasUsed}, tx: ${response.transactionHash}`)
  }

  async reclaimWithTestParams() {
    const positions = await this.query.positions({ accountId: this.accountId! })

    const denom = this.config.chain.baseDenom
    const amount = this.actions!.reclaimAmount
    await this.reclaim(denom, amount)

    printGreen(
      `User reclaimed: ${amount} ${denom}. Lent amount remaining: ${JSON.stringify(
        positions.lends,
      )}`,
    )
  }

  async reclaim(denom: string, amount: string) {
    const response = await this.updateCreditAccount([
      { reclaim: { amount: { exact: amount }, denom } },
    ])
    printYellow(`Reclaim, gas used: ${response.gasUsed}, tx: ${response.transactionHash}`)
  }

  async swapWithTestParams() {
    const amount = this.actions!.swap.amount
    printBlue(
      `Swapping ${amount} ${this.config.chain.baseDenom} for ${this.actions!.secondaryDenom}`,
    )
    const prevPositions = await this.query.positions({ accountId: this.accountId! })
    printBlue(`Previous account balance: ${JSON.stringify(prevPositions.deposits)}`)
    const response = await this.updateCreditAccount([
      {
        swap_exact_in: {
          coin_in: { amount: { exact: amount }, denom: this.config.chain.baseDenom },
          denom_out: this.actions!.secondaryDenom,
          slippage: this.actions!.swap.slippage,
        },
      },
    ])
    printYellow(`Swap, gas used: ${response.gasUsed}, tx: ${response.transactionHash}`)
    printGreen(`Swap successful`)
    const newPositions = await this.query.positions({ accountId: this.accountId! })
    printGreen(`New account balance: ${JSON.stringify(newPositions.deposits)}`)
  }

  async zapWithTestParams(lp_token_out: string) {
    const response = await this.updateCreditAccount([
      {
        provide_liquidity: {
          coins_in: this.actions!.zap.coinsIn.map((c) => ({
            denom: c.denom,
            amount: { exact: c.amount },
          })),
          lp_token_out,
          slippage: '0.05',
        },
      },
    ])
    printYellow(`Zap, gas used: ${response.gasUsed}, tx: ${response.transactionHash}`)
    const positions = await this.query.positions({ accountId: this.accountId! })
    const lp_balance = positions.deposits.find((c) => c.denom === lp_token_out)!.amount
    printGreen(
      `Zapped ${this.actions!.zap.coinsIn.map((c) => c.denom).join(
        ', ',
      )} for LP token: ${lp_balance} ${lp_token_out}`,
    )
  }

  async unzapWithTestParams(lp_token_in: string) {
    const lpToken = {
      denom: lp_token_in,
      amount: this.actions!.unzapAmount,
    }
    const response = await this.updateCreditAccount([
      {
        withdraw_liquidity: {
          lp_token: { amount: { exact: lpToken.amount }, denom: lpToken.denom },
          slippage: '0.05',
        },
      },
    ])
    printYellow(`Unzap, gas used: ${response.gasUsed}, tx: ${response.transactionHash}`)
    const underlying = await this.query.estimateWithdrawLiquidity({ lpToken })
    printGreen(
      `Unzapped ${lp_token_in} ${this.actions!.unzapAmount} for underlying: ${underlying
        .map((c) => `${c.amount} ${c.denom}`)
        .join(', ')}`,
    )
  }

  async vaultDepositWithTestParams(v: VaultConfigBaseForString, info: VaultInfo) {
    const oldRoverBalance = await this.cwClient.getBalance(
      this.storage.addresses.creditManager!,
      info.tokens.vault_token,
    )
    printBlue('testing vault deposit')
    printGreen(v.addr)
    printGreen(this.actions!.vault.depositAmount)
    printGreen(info.tokens.base_token)
    const response = await this.updateCreditAccount([
      {
        enter_vault: {
          coin: {
            amount: { exact: this.actions!.vault.depositAmount },
            denom: info.tokens.base_token,
          },
          vault: { address: v.addr },
        },
      },
    ])
    printYellow(`Vault deposit, gas used: ${response.gasUsed}, tx: ${response.transactionHash}`)
    const positions = await this.query.positions({ accountId: this.accountId! })
    assert.equal(positions.vaults.length, 1)
    const state = await this.getVaultBalance(v.addr)
    assert(state.locked > 0 || state.unlocked > 0)
    const newRoverBalance = await this.cwClient.getBalance(
      this.storage.addresses.creditManager!,
      info.tokens.vault_token,
    )
    const newAmount = parseInt(newRoverBalance.amount) - parseInt(oldRoverBalance.amount)
    assert(newAmount === state.locked || newAmount === state.unlocked)

    printGreen(
      `Deposited ${this.actions!.vault.depositAmount} ${
        info.tokens.base_token
      } in exchange for ${JSON.stringify(positions.vaults[0].amount)} vault tokens (${
        info.tokens.vault_token
      })`,
    )
  }

  async vaultWithdrawWithTestParams(v: VaultConfigBaseForString, info: VaultInfo) {
    const oldBalance = await this.getAccountBalance(info.tokens.base_token)
    const response = await this.updateCreditAccount([
      {
        exit_vault: {
          amount: this.actions!.vault.withdrawAmount,
          vault: { address: v.addr },
        },
      },
    ])
    printYellow(`Vault withdraw, gas used: ${response.gasUsed}, tx: ${response.transactionHash}`)
    const newBalance = await this.getAccountBalance(info.tokens.base_token)
    assert(newBalance > oldBalance)
    printGreen(
      `Withdrew ${newBalance - oldBalance} ${info.tokens.base_token} in exchange for ${
        this.actions!.vault.withdrawAmount
      } ${info.tokens.vault_token} vault tokens`,
    )
  }

  async vaultRequestUnlockWithTestParams(v: VaultConfigBaseForString, info: VaultInfo) {
    const oldBalance = await this.getVaultBalance(v.addr)
    const response = await this.updateCreditAccount([
      {
        request_vault_unlock: {
          amount: this.actions!.vault.withdrawAmount,
          vault: { address: v.addr },
        },
      },
    ])
    printYellow(
      `Vault request unlock, gas used: ${response.gasUsed}, tx: ${response.transactionHash}`,
    )
    const newBalance = await this.getVaultBalance(v.addr)
    assert(newBalance.locked < oldBalance.locked)
    assert.equal(newBalance.unlocking.length, 1)

    printGreen(
      `Requested unlock: ID #${newBalance.unlocking[0].id}, amount: ${
        newBalance.unlocking[0].coin.amount
      } ${newBalance.unlocking[0].coin.denom} in exchange for: ${
        oldBalance.locked - newBalance.locked
      } ${info.tokens.vault_token}`,
    )
  }

  async refundAllBalances() {
    await this.updateCreditAccount([{ refund_all_coin_balances: {} }])
    const positions = await this.query.positions({ accountId: this.accountId! })
    assert.equal(positions.deposits.length, 0)
    printGreen(`Withdrew all balances back to wallet`)
  }

  async getVaultInfo(v: VaultConfigBaseForString): Promise<VaultInfo> {
    const client = new MarsMockVaultQueryClient(this.cwClient, v.addr)
    return {
      tokens: await client.info(),
      lockup: await this.getLockup(v),
    }
  }

  async depositToPerpVault(denom: string, amount: string) {
    const response = await this.updateCreditAccount(
      [
        {
          deposit_to_perp_vault: {
            denom,
            amount: { exact: amount },
          },
        },
      ],
      [],
    )
    printYellow(
      `Perp vault deposit, gas used: ${response.gasUsed}, tx: ${response.transactionHash}`,
    )
  }

  async unlockFromPerpVault() {
    const positions_before = await this.query.positions({ accountId: this.accountId! })
    const shares = positions_before.perp_vault?.deposit.shares
    const response = await this.updateCreditAccount(
      [
        {
          unlock_from_perp_vault: {
            shares: shares?.toString() || '0',
          },
        },
      ],
      [],
    )
    printYellow(`Perp vault unlock, gas used: ${response.gasUsed}, tx: ${response.transactionHash}`)
  }

  async withdrawFromPerpVault() {
    const response = await this.updateCreditAccount(
      [
        {
          withdraw_from_perp_vault: {},
        },
      ],
      [],
    )
    printYellow(
      `Perp vault withdraw, gas used: ${response.gasUsed}, tx: ${response.transactionHash}`,
    )
  }

  async openPerp(denom: string, size: number) {
    const msg = {
      open_perp: {
        denom,
        size: size.toString() as any,
      },
    }
    const response = await this.updateCreditAccount([msg], [])
    printYellow(`Open perp, gas used: ${response.gasUsed}, tx: ${response.transactionHash}`)
  }

  async closePerp(denom: string) {
    const response = await this.updateCreditAccount(
      [
        {
          close_perp: {
            denom,
          },
        },
      ],
      [],
    )
    printYellow(`Close perp, gas used: ${response.gasUsed}, tx: ${response.transactionHash}`)
  }

  async liquidateDeposit(liqAccId: string, depositDenom: string, debtCoin: Coin) {
    const response = await this.updateCreditAccount(
      [
        {
          liquidate: {
            debt_coin: debtCoin,
            liquidatee_account_id: liqAccId,
            request: {
              deposit: depositDenom,
            },
          },
        },
        { refund_all_coin_balances: {} },
      ],
      [],
    )
    printYellow(`Liquidate deposit, gas used: ${response.gasUsed}, tx: ${response.transactionHash}`)
    const positions = await this.query.positions({ accountId: liqAccId })
    printGreen(`Liquidatee perps positions should be empty: ${JSON.stringify(positions.perps)}`)
  }

  private async getLockup(v: VaultConfigBaseForString): Promise<VaultInfo['lockup']> {
    try {
      return await this.cwClient.queryContractSmart(v.addr, {
        vault_extension: {
          lockup: {
            lockup_duration: {},
          },
        },
      })
    } catch (e) {
      return undefined
    }
  }

  private async getAccountBalance(denom: string) {
    const positions = await this.query.positions({ accountId: this.accountId! })
    const coin = positions.deposits.find((c) => c.denom === denom)
    if (!coin) throw new Error(`No balance of ${denom}`)
    return parseInt(coin.amount)
  }

  private async getVaultBalance(vaultAddr: string) {
    const positions = await this.query.positions({ accountId: this.accountId! })
    const vault = positions.vaults.find((p) => p.vault.address === vaultAddr)
    if (!vault) throw new Error(`No balance for ${vaultAddr}`)

    if ('unlocked' in vault.amount) {
      return {
        unlocked: parseInt(vault.amount.unlocked),
        locked: 0,
        unlocking: [],
      }
    } else {
      return {
        unlocked: 0,
        locked: parseInt(vault.amount.locking.locked),
        unlocking: vault.amount.locking.unlocking.map((lockup) => ({
          id: lockup.id,
          coin: { denom: lockup.coin.denom, amount: parseInt(lockup.coin.amount) },
        })),
      }
    }
  }

  private async updateCreditAccount(actions: Action[], funds?: Coin[]) {
    return await this.exec.updateCreditAccount(
      { actions, accountId: this.accountId! },
      'auto',
      undefined,
      funds,
    )
  }
}
