import useSWR from 'swr'
import BigNumber from 'bignumber.js'

export default function usePrices() {
  const assets: Asset[] = [
    {
      denom: 'untrn',
      priceFeedId: 'a8e6517966a52cb1df864b2764f3629fde3f21d2b640b5c572fcd654cbccd65e',
      decimals: 6,
    },
    {
      denom: 'ibc/4C19E7EC06C1AB2EC2D70C6855FEB6D48E9CE174913991DA0A517D21978E7E42',
      priceFeedId: 'eaa020c61cc479712813461ce153894a96a6c00b21ed0cfc2798d1f9a9e9c94a',
      decimals: 6,
    },
    {
      denom: 'ibc/C4CFF46FD6DE35CA4CF4CE031E643C8FDC9BA4B99AE598E9B0ED98FE3A2319F9',
      priceFeedId: 'b00b60f88b03a6a625a8d1c048c3f66653edf217439983d037e7222c4e612819',
      decimals: 6,
    },
    {
      denom: 'factory/neutron166t9ww3p6flv7c86376fy0r92r88t3492xxj2h/ubtc',
      priceFeedId: 'c9d8b075a5c69303365ae23633d4e085199bf5c520a3b90fed1322a0342ffc33',
      decimals: 8,
    },
    {
      denom: 'factory/neutron166t9ww3p6flv7c86376fy0r92r88t3492xxj2h/ueth',
      priceFeedId: 'ff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace',
      decimals: 18,
    },
    {
      denom: 'factory/neutron166t9ww3p6flv7c86376fy0r92r88t3492xxj2h/uinj',
      priceFeedId: '7a5bc1d2b56ad029048cd63964b3ad2776eadf812edc1a43a31406cb54bff592',
      decimals: 18,
    },
    {
      denom: 'factory/neutron166t9ww3p6flv7c86376fy0r92r88t3492xxj2h/udydx',
      priceFeedId: '6489800bb8974169adfe35937bf6736507097d13c190d760c557108c7e93a81b',
      decimals: 18,
    },
    {
      denom: 'factory/neutron166t9ww3p6flv7c86376fy0r92r88t3492xxj2h/utia',
      priceFeedId: '09f7c1d7dfbb7df2b8fe3d3d87ee94a2259d212da4f30c1f0540d066dfa44723',
      decimals: 6,
    },
    {
      denom: 'factory/neutron166t9ww3p6flv7c86376fy0r92r88t3492xxj2h/usol',
      priceFeedId: 'ef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d',
      decimals: 9,
    },
  ]

  return useSWR(`chains/pion01/prices`, () => fetchPythPrices(assets))
}

BigNumber.config({ EXPONENTIAL_AT: 1e9 })

async function fetchPythPrices(assets: Asset[]) {
  const pricesUrl = new URL(`https://hermes.pyth.network/api/latest_price_feeds`)
  assets.forEach((asset) => pricesUrl.searchParams.append('ids[]', asset.priceFeedId))

  const pythResponse: PythPriceData[] = await fetch(pricesUrl).then((res) => res.json())

  const prices: { [key: string]: string } = {}

  pythResponse.forEach((price, index) => {
    prices[assets[index].denom] = BigNumber(price.price.price)
      .shiftedBy(price.price.expo - assets[index].decimals + 6)
      .decimalPlaces(18)
      .toString()
  })

  return prices
}

interface PythPriceData {
  price: PythConfidenceData
  ema_price: PythConfidenceData
  id: string
}

interface PythConfidenceData {
  conf: string
  expo: number
  price: string
  publish_time: number
}

type Asset = {
  denom: string
  priceFeedId: string
  decimals: number
}
