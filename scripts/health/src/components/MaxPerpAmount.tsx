import { HealthComputer, Direction, max_perp_size_estimate_js } from '../../pkg-web'
import { useCallback, useState } from 'react'
import { SelectPerpsAsset } from './Select/SelectPerpsAsset.tsx'
import Select from './Select/index.tsx'
import { Input } from './Input.tsx'

type Props = {
  healthComputer: HealthComputer
}

export default function MaxPerpAmount(props: Props) {
  const [selectedDenom, setSelectedDenom] = useState<string | null>(null)
  const [direction, setDirection] = useState<Direction>('long')
  const [amount, setAmount] = useState('-')
  const [longOiAmt, setLongOIAmount] = useState('0')
  const [shortOiAmt, setShortOIAmount] = useState('0')
  const [error, setError] = useState<null | string>(null)

  const onConfirm = useCallback(() => {
    try {
      setError(null)
      if (!selectedDenom) return
      const amount = max_perp_size_estimate_js(
        props.healthComputer,
        selectedDenom,
        'factory/neutron1ke0vqqzyymlp5esr8gjwuzh94ysnpvj8er5hm7/USDC',
        longOiAmt,
        shortOiAmt,
        direction,
      )
      setAmount(amount)
    } catch (e) {
      setError((e as string).toString())
    }
  }, [props.healthComputer, selectedDenom, direction, longOiAmt, shortOiAmt])

  return (
    <div className='gap-4 flex flex-col items-start bg-black p-8 rounded-md'>
      <SelectPerpsAsset value={selectedDenom ?? ''} onSelected={setSelectedDenom} />
      <Select
        label='Direction'
        options={['long', 'short']}
        value={direction}
        onSelected={setDirection}
      />

      <Input label='Long OI amount' value={longOiAmt} onChange={setLongOIAmount} />

      <Input label='Short OI amount' value={shortOiAmt} onChange={setShortOIAmount} />

      <button onClick={onConfirm}>Calculate Max Perp Size</button>

      {error ? <p className={'text-red-500'}>{error}</p> : <p>Max amount: {amount}</p>}
    </div>
  )
}
