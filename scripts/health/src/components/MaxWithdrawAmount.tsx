import { HealthComputer, max_withdraw_estimate_js } from '../../pkg-web'
import { useCallback, useState } from 'react'
import { SelectAsset } from './Select/SelectAsset.tsx'

type Props = {
  healthComputer: HealthComputer
}

export default function MaxWithdrawAmount(props: Props) {
  const [denom, setDenom] = useState('')
  const [amount, setAmount] = useState('-')
  const [error, setError] = useState<null | string>(null)

  const onConfirm = useCallback(() => {
    try {
      setError(null)
      const amount = max_withdraw_estimate_js(props.healthComputer, denom)
      setAmount(amount)
    } catch (e) {
      setError((e as string).toString())
    }
  }, [denom, props.healthComputer])

  return (
    <div className='gap-4 flex flex-col items-start bg-black p-8 rounded-md'>
      <SelectAsset value={denom ?? ''} onSelected={setDenom} />

      <button onClick={onConfirm}>Calculate Max Withdraw Amount</button>

      {error ? <p className={'text-red-500'}>{error}</p> : <p>Max amount: {amount}</p>}
    </div>
  )
}
