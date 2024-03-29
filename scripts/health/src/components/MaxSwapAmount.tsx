import { HealthComputer, SwapKind, max_swap_estimate_js } from '../../pkg-web'
import { useCallback, useState } from 'react'
import { SelectAsset } from './Select/SelectAsset.tsx'
import { Input } from './Input.tsx'
import Select from './Select/index.tsx'

type Props = {
  healthComputer: HealthComputer
}

export default function MaxSwapAmount(props: Props) {
  const [amount, setAmount] = useState('-')
  const [error, setError] = useState<null | string>(null)

  const [fromDenom, setFromDenom] = useState('')
  const [toDenom, setToDenom] = useState('')
  const [swapKind, setSwapKind] = useState<SwapKind>('default')
  const [slippage, setSlippage] = useState('0.05')

  const onConfirm = useCallback(() => {
    try {
      setError(null)
      const amount = max_swap_estimate_js(
        props.healthComputer,
        fromDenom,
        toDenom,
        swapKind,
        slippage,
      )
      setAmount(amount)
    } catch (e) {
      setError((e as string).toString())
    }
  }, [fromDenom, props.healthComputer, slippage, swapKind, toDenom])

  return (
    <div className='gap-4 flex flex-col items-start bg-black p-8 rounded-md'>
      <SelectAsset value={fromDenom} onSelected={setFromDenom} />
      <SelectAsset value={toDenom} onSelected={setToDenom} />
      <Select
        label='Swap kind'
        options={['default', 'margin']}
        value={swapKind}
        onSelected={setSwapKind}
      />
      <Input label='Slippage' value={slippage} onChange={setSlippage} />

      <button onClick={onConfirm}>Calculate Max Swap amount </button>

      {error ? <p className={'text-red-500'}>{error}</p> : <p>Max amount: {amount}</p>}
    </div>
  )
}
