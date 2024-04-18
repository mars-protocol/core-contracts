import { useEffect, useState } from 'react'

export default function useChainConfig() {
  const [addresses, setAddresses] = useState<{ [key: string]: string }>()

  const chain = import.meta.env.VITE_CHAIN_ID

  useEffect(() => {
    switch (chain) {
      case 'osmosis-1':
        import(`../../../deploy/addresses/osmosis-1-multisig-owner.json`).then((json) =>
          setAddresses(json.default),
        )
        break

      default:
        import(`../../../deploy/addresses/pion-1-deployer-owner.json`).then((json) =>
          setAddresses(json.default),
        )
    }
  }, [chain])

  switch (chain) {
    case 'osmosis-1':
      return {
        chain,
        addresses,
        rpc: 'https://osmosis-rpc.cosmos-apis.com',
        pythAssets: [
          {
            denom: 'uosmo',
            priceFeedId: '5867f5683c757393a0670ef0f701490950fe93fdb006d181c8265a831ac0c5c6',
            decimals: 6,
          },
          {
            denom: 'ibc/498A0751C798A0D9A389AA3691123DADA57DAA4FE165D5C75894505B876BA6E4',
            priceFeedId: 'eaa020c61cc479712813461ce153894a96a6c00b21ed0cfc2798d1f9a9e9c94a',
            decimals: 6,
          },
          {
            denom: 'ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2',
            priceFeedId: 'b00b60f88b03a6a625a8d1c048c3f66653edf217439983d037e7222c4e612819',
            decimals: 6,
          },
          {
            denom: 'ibc/D1542AA8762DB13087D8364F3EA6509FD6F009A34F00426AF9E4F9FA85CBBF1F',
            priceFeedId: 'c9d8b075a5c69303365ae23633d4e085199bf5c520a3b90fed1322a0342ffc33',
            decimals: 8,
          },
          {
            denom: 'ibc/D189335C6E4A68B513C10AB227BF1C1D38C746766278BA3EEB4FB14124F1D858',
            priceFeedId: 'eaa020c61cc479712813461ce153894a96a6c00b21ed0cfc2798d1f9a9e9c94a',
            decimals: 6,
          },
          {
            denom: 'ibc/EA1D43981D5C9A1C4AAEA9C23BB1D4FA126BA9BC7020A25E0AE4AA841EA25DC5',
            priceFeedId: 'ff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace',
            decimals: 18,
          },
          {
            denom: 'ibc/1480B8FD20AD5FCAE81EA87584D269547DD4D436843C1D20F15E00EB64743EF4',
            priceFeedId: '4ea5bb4d2f5900cc2e97ba534240950740b4d3b89fe712a94a7304fd2fd92702',
            decimals: 6,
          },
          {
            denom: 'ibc/4ABBEF4C8926DDDB320AE5188CFD63267ABBCEFC0583E4AE05D6E5AA2401DDAB',
            priceFeedId: '2b89b9dc8fdf9f34709a5b106b472f0f39bb6ca9ce04b0fd7f2e971688e2e53b',
            decimals: 6,
          },
          {
            denom: 'ibc/903A61A498756EA560B85A85132D3AEE21B5DEDD41213725D22ABF276EA6945E',
            priceFeedId: '60144b1d5c9e9851732ad1d9760e3485ef80be39b984f6bf60f82b28a2b7f126',
            decimals: 6,
          },
          {
            denom: 'ibc/64BA6E31FE887D66C6F8F31C7B1A80C7CA179239677B4088BB55F5EA07DBE273',
            priceFeedId: '7a5bc1d2b56ad029048cd63964b3ad2776eadf812edc1a43a31406cb54bff592',
            decimals: 18,
          },
          {
            denom: 'ibc/D79E7D83AB399BFFF93433E54FAA480C191248FC556924A2A8351AE2638B3877',
            priceFeedId: '09f7c1d7dfbb7df2b8fe3d3d87ee94a2259d212da4f30c1f0540d066dfa44723',
            decimals: 6,
          },
          {
            denom: 'ibc/831F0B1BBB1D08A2B75311892876D71565478C532967545476DF4C2D7492E48C',
            priceFeedId: '6489800bb8974169adfe35937bf6736507097d13c190d760c557108c7e93a81b',
            decimals: 18,
          },
        ],
      }
    default:
      return {
        chain,
        addresses,
        rpc: 'https://rpc-palvus.pion-1.ntrn.tech',
        pythAssets: [
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
        ],
      }
  }
}
