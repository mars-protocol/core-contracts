import { HealthComputer, max_perp_size_estimate_js } from '../../pkg-web'
import usePerpsParams from '../hooks/usePerpsParams.ts'
import { useState } from 'react'

type Props = {
  healthComputer: HealthComputer
}

export default function MaxPerpAmount(props: Props) {
  const { data: perpsParams } = usePerpsParams()
  const [selectedDenom, setSelectedDenom] = useState<string | null>(null)
  const [direction, setDirection] = useState('long')
  const [amount, setAmount] = useState('-')
  const [error, setError] = useState<null | string>(null)

  if (!perpsParams) return

  return (
    <div className='gap-4 flex flex-col items-start bg-black p-8 rounded-md'>
      <label>
        Asset
        <select
          name='asset'
          value={selectedDenom ?? ''}
          defaultValue={''}
          onChange={(e) => setSelectedDenom(e.target.value)}
        >
          <option>-</option>
          {Object.keys(perpsParams)?.map((perpsParam) => (
            <option key={perpsParam} value={perpsParam}>
              {perpsParam}
            </option>
          ))}
        </select>
      </label>
      <label>
        Direction
        <select
          name='direction'
          value={direction}
          onChange={(e) => {
            setDirection(e.target.value)
          }}
        >
          <option key={'long'} value={'long'}>
            LONG
          </option>
          <option key={'short'} value={'short'}>
            SHORT
          </option>
        </select>
      </label>

      <button
        onClick={() => {
          try {
            setError(null)
            if (!selectedDenom) return
            console.log(props.healthComputer)
            const amount = max_perp_size_estimate_js(
              props.healthComputer,
              selectedDenom,
              'ibc/4C19E7EC06C1AB2EC2D70C6855FEB6D48E9CE174913991DA0A517D21978E7E42',
              props.healthComputer.perps_data.denom_states[selectedDenom].long_oi,
              props.healthComputer.perps_data.denom_states[selectedDenom].short_oi,
              'long',
            )
            setAmount(amount)
          } catch (e) {
            setError((e as string).toString())
          }
        }}
      >
        Calculate Max Perp Size
      </button>

      {error ? <p className={'text-red-500'}>{error}</p> : <p>Max amount: {amount}</p>}
    </div>
  )
}
