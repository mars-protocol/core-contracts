import { HealthComputer, liquidation_price_js, LiquidationPriceKind } from '../../pkg-web'
import { useCallback, useState } from 'react'
import { SelectAsset } from './Select/SelectAsset.tsx'
import Select from './Select/index.tsx'

type Props = {
  healthComputer: HealthComputer
}

export default function MaxBorrowAmount(props: Props) {
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
    <div className='gap-4 flex flex-col items-start bg-black p-8 rounded-md'>
      <SelectAsset value={denom ?? ''} onSelected={setDenom} />
      <Select label='Kind' options={['asset', 'debt']} value={kind} onSelected={setKind} />

      <button onClick={onConfirm}>Calculate Liquidation price</button>

      {error ? <p className={'text-red-500'}>{error}</p> : <p>Price: {amount}</p>}
    </div>
  )
}
