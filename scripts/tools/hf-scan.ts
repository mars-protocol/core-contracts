/*
  Historical health scan CLI

  Repeatedly queries smart contracts at specified block heights, builds a
  HealthComputer input (bank-only; vaults ignored), and computes health.

  Requirements:
  - An LCD/REST endpoint capable of historical queries
  - Contract addresses for Credit Manager, Oracle, and Params
  - The compiled wasm from scripts/health/pkg-web (already in repo)

  Usage example:
    node build/tools/hf-scan.js \
      --lcd https://lcd.osmosis.zone \
      --credit-manager osmo1... \
      --oracle osmo1... \
      --params osmo1... \
      --account-id <ACCOUNT_ID> \
      --start 1234567 --end 1235567 --step 100

  Notes:
  - Vaults and staked_astro_lps are stripped from positions (ignored).
  - All asset params and all perp params are fetched at each height.
  - Oracle prices are fully paginated via `prices { limit, start_after }`.
*/

import fs from 'node:fs'
import path from 'node:path'
import { pathToFileURL } from 'node:url'

type Decimal = string

type Positions = {
  account_id: string
  account_kind: 'default' | { fund_manager: { owner: string } } | 'usdc_margin' | 'high_levered_strategy'
  debts: Array<{ denom: string; amount: string; shares: string }>
  deposits: Array<{ denom: string; amount: string }>
  lends: Array<{ denom: string; amount: string }>
  perps: Array<{
    denom: string
    base_denom: string
    size: string
    entry_price: Decimal
    current_price: Decimal
    entry_exec_price: Decimal
    current_exec_price: Decimal
    realized_pnl: { accrued_funding: string; closing_fee: string; opening_fee: string; pnl: string; price_pnl: string }
    unrealized_pnl: { accrued_funding: string; closing_fee: string; opening_fee: string; pnl: string; price_pnl: string }
  }>
  staked_astro_lps: Array<{ denom: string; amount: string }>
  vaults: unknown[]
}

type AssetParams = {
  denom: string
  max_loan_to_value: Decimal
  liquidation_threshold: Decimal
  credit_manager: { whitelisted: boolean; withdraw_enabled: boolean; hls?: unknown | null }
  red_bank: { borrow_enabled: boolean; deposit_enabled: boolean; withdraw_enabled: boolean }
  reserve_factor: Decimal
  close_factor: Decimal
  protocol_liquidation_fee: Decimal
  interest_rate_model: unknown
}

type PerpParams = {
  denom: string
  enabled: boolean
  opening_fee_rate: Decimal
  closing_fee_rate: Decimal
  max_loan_to_value: Decimal
  liquidation_threshold: Decimal
  max_long_oi_value: string
  max_short_oi_value: string
  max_net_oi_value: string
  min_position_value: string
  skew_scale: string
  max_loan_to_value_usdc?: Decimal | null
  liquidation_threshold_usdc?: Decimal | null
  max_position_value?: string | null
}

type PriceResponse = { denom: string; price: Decimal }

type PaginationResponse<T> = { data: T[]; metadata: { has_more: boolean } }

type HealthComputerInput = {
  kind: Positions['account_kind'] extends 'default' ? 'default' : Positions['account_kind']
  positions: Positions
  asset_params: Record<string, AssetParams>
  vaults_data: { vault_values: Record<string, unknown>; vault_configs: Record<string, unknown> }
  perps_data: { params: Record<string, PerpParams> }
  oracle_prices: Record<string, Decimal>
}

type Args = {
  lcd: string
  creditManager: string
  oracle: string
  params: string
  accountId: string
  start: number
  end: number
  step: number
  kind?: 'default' | 'liquidation'
  output?: 'json' | 'ndjson'
}

function parseArgs(): Args {
  const get = (name: string) => {
    const idx = process.argv.findIndex((a) => a === `--${name}`)
    if (idx === -1) return undefined
    return process.argv[idx + 1]
  }
  const required = (k: string) => {
    const v = get(k)
    if (!v) throw new Error(`Missing required --${k}`)
    return v
  }
  const lcd = required('lcd')
  const creditManager = required('credit-manager')
  const oracle = required('oracle')
  const params = required('params')
  const accountId = required('account-id')
  const start = Number(required('start'))
  const end = Number(required('end'))
  const step = Number(required('step'))
  const kind = (get('kind') as Args['kind']) ?? 'default'
  const output = (get('output') as Args['output']) ?? 'ndjson'
  return { lcd, creditManager, oracle, params, accountId, start, end, step, kind, output }
}

function lcdUrl(base: string) {
  return base.replace(/\/?$/, '')
}

function base64Query(msg: unknown) {
  return Buffer.from(JSON.stringify(msg)).toString('base64')
}

async function querySmartAtHeight<T>(lcd: string, contract: string, msg: unknown, height: number): Promise<T> {
  const q = base64Query(msg)
  // Prefer contracts path; some LCDs have singular 'contract' – try both if needed
  const paths = [
    `/cosmwasm/wasm/v1/contracts/${contract}/smart/${q}`,
    `/cosmwasm/wasm/v1/contract/${contract}/smart/${q}`,
  ]
  const headers = { 'x-cosmos-block-height': String(height) }
  const errors: string[] = []
  for (const p of paths) {
    const url = `${lcdUrl(lcd)}${p}?height=${height}`
    try {
      const res = await fetch(url, { headers })
      if (!res.ok) {
        const body = await res.text().catch(() => '')
        throw new Error(`${res.status} ${res.statusText} url=${url} body=${body.slice(0, 300)}`)
      }
      const json: unknown = await res.json()
      // gRPC-gateway typically returns { data: base64 } for smart queries.
      // Decode if present; otherwise assume JSON is the decoded result.
      const anyJson = json as any
      if (anyJson && typeof anyJson.data === 'string') {
        const decoded = Buffer.from(anyJson.data, 'base64').toString('utf8')
        return JSON.parse(decoded) as T
      }
      return json as T
    } catch (err) {
      console.error(`[hf-scan] query error height=${height} url=${url} error=${String(err)}`)
      errors.push(`${url} -> ${String(err)}`)
      continue
    }
  }
  throw new Error(`Query failed at height ${height}. Attempts: ${errors.join(' | ')}`)
}

async function fetchPositions(lcd: string, creditManager: string, accountId: string, height: number, kind: 'default' | 'liquidation'): Promise<Positions> {
  const msg = { positions: { account_id: accountId, action: kind } }
  const res = await querySmartAtHeight<Positions>(lcd, creditManager, msg, height)
  // Strip vaults and staked LPs as requested
  return { ...res, vaults: [], staked_astro_lps: [] }
}

async function fetchAllOraclePrices(lcd: string, oracle: string, height: number, actionKind: 'default' | 'liquidation' = 'liquidation'): Promise<Record<string, Decimal>> {
  // Reduce page size to mitigate LCD OOG
  const limit = 3
  let startAfter: string | undefined
  const prices: Record<string, Decimal> = {}

  // Keep paginating until response size < limit
  while (true) {
    const msg = { prices: { limit, start_after: startAfter, kind: actionKind } }
    const page = await querySmartAtHeight<PriceResponse[]>(lcd, oracle, msg, height)
    page.forEach((p) => (prices[p.denom] = p.price))
    if (page.length < limit) break
    startAfter = page[page.length - 1].denom
  }
  return prices
}

async function fetchAllAssetParams(lcd: string, paramsAddr: string, height: number): Promise<Record<string, AssetParams>> {
  const limit = 50
  const out: Record<string, AssetParams> = {}
  // Try v2 first (paginated response with metadata). Fallback to v1.
  try {
    let startAfter: string | undefined
    while (true) {
      const msg = { all_asset_params_v2: { limit, start_after: startAfter } }
      const page = await querySmartAtHeight<PaginationResponse<AssetParams>>(lcd, paramsAddr, msg, height)
      page.data.forEach((p) => {
        const withDefaults = Object.assign({}, p, {
          close_factor: (p as any).close_factor ?? '0',
        }) as AssetParams
        out[p.denom] = withDefaults
      })
      if (!page.metadata?.has_more || page.data.length === 0) break
      startAfter = page.data[page.data.length - 1].denom
    }
    return out
  } catch (_e) {
    // Fall through to v1 shape
  }

  let startAfter: string | undefined
  while (true) {
    const msg = { all_asset_params: { limit, start_after: startAfter } }
    const page = await querySmartAtHeight<AssetParams[]>(lcd, paramsAddr, msg, height)
    page.forEach((p) => {
      const withDefaults = Object.assign({}, p, {
        close_factor: (p as any).close_factor ?? '0',
      }) as AssetParams
      out[p.denom] = withDefaults
    })
    if (page.length < limit) break
    startAfter = page[page.length - 1].denom
  }
  return out
}

async function fetchAllPerpParams(lcd: string, paramsAddr: string, height: number): Promise<Record<string, PerpParams>> {
  const limit = 50
  const out: Record<string, PerpParams> = {}
  // Try v2 first (metadata.has_more). Fallback to v1 if not implemented.
  try {
    let startAfter: string | undefined
    while (true) {
      const msg = { all_perp_params_v2: { limit, start_after: startAfter } }
      const page = await querySmartAtHeight<PaginationResponse<PerpParams>>(lcd, paramsAddr, msg, height)
      page.data.forEach((p) => (out[p.denom] = p))
      if (!page.metadata?.has_more || page.data.length === 0) break
      startAfter = page.data[page.data.length - 1].denom
    }
    return out
  } catch (_e) {
    // Fall through to v1 shape
  }

  let startAfter: string | undefined
  while (true) {
    const msg = { all_perp_params: { limit, start_after: startAfter } }
    const page = await querySmartAtHeight<PerpParams[]>(lcd, paramsAddr, msg, height)
    page.forEach((p) => (out[p.denom] = p))
    if (page.length < limit) break
    startAfter = page[page.length - 1].denom
  }
  return out
}

async function loadHealthWasm() {
  // Resolve the wasm JS glue relative to the compiled script location
  // Resolve relative to the compiled file location under build/tools
  const here = __dirname
  // build/tools -> ../../health/pkg-web
  const glueUrl = pathToFileURL(path.resolve(here, '../../health/pkg-web/index.js')).href
  const wasmPath = path.resolve(here, '../../health/pkg-web/index_bg.wasm')

  const mod: any = await import(glueUrl)
  const init: any = mod.default
  const wasmBytes = fs.readFileSync(wasmPath)
  const wasm = await init(wasmBytes)
  return {
    compute_health_js: mod.compute_health_js as (c: HealthComputerInput) => {
      total_debt_value: string
      total_collateral_value: string
      max_ltv_adjusted_collateral: string
      liquidation_threshold_adjusted_collateral: string
      max_ltv_health_factor: Decimal | null
      liquidation_health_factor: Decimal | null
      perps_pnl_profit: string
      perps_pnl_loss: string
      liquidatable: boolean
      above_max_ltv: boolean
      has_perps: boolean
    },
    wasm,
  }
}

async function main() {
  const args = parseArgs()
  const { compute_health_js } = await loadHealthWasm()

  const heights: number[] = []
  for (let h = args.start; h <= args.end; h += args.step) heights.push(h)

  for (const height of heights) {
    try {
      const [positions, prices, assetParams, perpParams] = await Promise.all([
        fetchPositions(args.lcd, args.creditManager, args.accountId, height, args.kind!),
        fetchAllOraclePrices(args.lcd, args.oracle, height, args.kind!),
        fetchAllAssetParams(args.lcd, args.params, height),
        fetchAllPerpParams(args.lcd, args.params, height),
      ])

      const healthComputer: HealthComputerInput = {
        kind: (positions as any).account_kind ?? 'default',
        positions,
        oracle_prices: prices,
        asset_params: assetParams,
        vaults_data: { vault_configs: {}, vault_values: {} },
        perps_data: { params: perpParams },
      }

      const health = compute_health_js(healthComputer)
      const row = {
        height,
        total_debt_value: health.total_debt_value,
        total_collateral_value: health.total_collateral_value,
        max_ltv_health_factor: health.max_ltv_health_factor,
        liquidation_health_factor: health.liquidation_health_factor,
        above_max_ltv: health.above_max_ltv,
        liquidatable: health.liquidatable,
      }
      if (args.output === 'json') {
        console.log(JSON.stringify({ height, health, inputs: healthComputer }, null, 2))
      } else {
        console.log(JSON.stringify(row))
      }
    } catch (err) {
      console.error(JSON.stringify({ height, error: String(err) }))
    }
  }
}

main().catch((e) => {
  console.error(e)
  process.exit(1)
})
