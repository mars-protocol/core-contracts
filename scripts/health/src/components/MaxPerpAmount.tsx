import { HealthComputer, max_perp_size_estimate_js } from '../../pkg-web'
import { useCallback, useState } from 'react'
import { SelectPerpsAsset } from './Select/SelectPerpsAsset.tsx'
import Select from './Select/index.tsx'

type Props = {
  healthComputer: HealthComputer
}

export default function MaxPerpAmount(props: Props) {
  const [selectedDenom, setSelectedDenom] = useState<string | null>(null)
  const [direction, setDirection] = useState('long')
  const [amount, setAmount] = useState('-')
  const [error, setError] = useState<null | string>(null)

  const onConfirm = useCallback(() => {
    try {
      setError(null)
      if (!selectedDenom) return
      const amount = max_perp_size_estimate_js(
        props.healthComputer,
        selectedDenom,
        'ibc/4C19E7EC06C1AB2EC2D70C6855FEB6D48E9CE174913991DA0A517D21978E7E42',
        props.healthComputer.perps_data.market_states[selectedDenom].long_oi,
        props.healthComputer.perps_data.market_states[selectedDenom].short_oi,
        'long',
      )
      setAmount(amount)
    } catch (e) {
      setError((e as string).toString())
    }
  }, [props.healthComputer, selectedDenom])

  return (
    <div className='gap-4 flex flex-col items-start bg-black p-8 rounded-md'>
      <SelectPerpsAsset value={selectedDenom ?? ''} onSelected={setSelectedDenom} />
      <Select
        label='Direction'
        options={['long', 'short']}
        value={direction}
        onSelected={setDirection}
      />

      <button onClick={onConfirm}>Calculate Max Perp Size</button>

      {error ? <p className={'text-red-500'}>{error}</p> : <p>Max amount: {amount}</p>}
    </div>
  )
}
