import BigNumber from 'bignumber.js'
import useSWR from 'swr'
import useChainConfig from './useChainConfig'

export default function usePrices() {
  const chainConfig = useChainConfig()

  return useSWR(`chains/${chainConfig.chain}/prices`, () => fetchPythPrices(chainConfig.pythAssets))
}

BigNumber.config({ EXPONENTIAL_AT: 1e9 })

async function fetchPythPrices(assets: Asset[]) {
  const pricesUrl = new URL(`https://hermes.pyth.network/api/latest_price_feeds`)
  assets.forEach((asset) => pricesUrl.searchParams.append('ids[]', asset.priceFeedId))

  const pythResponse: PythPriceData[] = await fetch(pricesUrl).then((res) => res.json())

  const prices: { [key: string]: string } = {}

  const VALUE_SCALE_FACTOR = 12

  pythResponse.forEach((price, index) => {
    prices[assets[index].denom] = BigNumber(price.price.price)
      .shiftedBy(VALUE_SCALE_FACTOR)
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
