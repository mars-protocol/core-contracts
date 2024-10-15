import { useCallback, useState } from 'react'
import { HealthComputer, LiquidationPriceKind, liquidation_price_js } from '../../pkg-web'
import { SelectAsset } from './Select/SelectAsset.tsx'
import Select from './Select/index.tsx'
import { SelectPerpsAsset } from './Select/SelectPerpsAsset.tsx'

type Props = {
  healthComputer: HealthComputer
}

export default function LiquidationPrice(props: Props) {
  const [kind, setKind] = useState<LiquidationPriceKind>('asset')
  const [amount, setAmount] = useState('')
  const [denom, setDenom] = useState('')
  const [error, setError] = useState<null | string>(null)

  const onConfirm = useCallback(() => {
    try {
      setError(null)
      const amount = liquidation_price_js(props.healthComputer, denom, kind)
      setAmount(amount)
    } catch (e) {
      setError((e as string).toString())
    }
  }, [denom, kind, props.healthComputer])

  return (
    <div className='flex flex-col items-start gap-4 p-8 bg-black rounded-md'>
      <Select label='Kind' options={['asset', 'debt', 'perp']} value={kind} onSelected={setKind} />
      {kind === 'perp' ? (
        <SelectPerpsAsset value={denom ?? ''} onSelected={setDenom} />
      ) : (
        <SelectAsset value={denom ?? ''} onSelected={setDenom} />
      )}

      <button onClick={onConfirm}>Calculate Liquidation price</button>

      {error ? <p className={'text-red-500'}>{error}</p> : <p>Price: {amount}</p>}
    </div>
  )
}
