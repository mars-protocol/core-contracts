import { ReactNode, useCallback, useEffect, useState } from 'react'
import init, { HealthComputer } from '../pkg-web/'
import './App.css'
import LiquidationPrice from './components/LiquidationPrice.tsx'
import MaxBorrowAmount from './components/MaxBorrowAmount.tsx'
import MaxPerpAmount from './components/MaxPerpAmount.tsx'
import MaxSwapAmount from './components/MaxSwapAmount.tsx'
import MaxWithdrawAmount from './components/MaxWithdrawAmount.tsx'
import useHealthComputer from './hooks/useHealthComputer.ts'

function App() {
  const [healthComputerJson, setHealthComputerJson] = useState('')
  const [type, setType] = useState<FunctionType>('perp')
  const [accountId, setAccountId] = useState('')
  const { data: healthComputer } = useHealthComputer(accountId)

  useEffect(() => {
    const loadHealthComputerWasm = async () => {
      await init()
    }
    loadHealthComputerWasm()
  }, [])

  useEffect(() => {
    if (!accountId) return
    setHealthComputerJson(JSON.stringify(healthComputer, undefined, 4))
  }, [accountId, healthComputer])

  const InteractionInterface = useCallback(() => {
    const func = FUNCTIONS.find(({ functionType }) => functionType === type)
    return func?.component(JSON.parse(healthComputerJson))
  }, [healthComputerJson, type])

  return (
    <div className={'h-full w-full flex flex-col gap-4'}>
      <div className='flex gap-4'>
        {FUNCTIONS.map(({ name, functionType }) => (
          <button
            onClick={() => setType(functionType)}
            className={functionType === type ? 'bg-gray-500' : ''}
          >
            {name}
          </button>
        ))}
      </div>

      <InteractionInterface />

      <div className={'flex flex-col gap-4 h-full w-full'}>
        <h2 className={'text-xl font-bold '}>Health computer object</h2>

        <div className='flex flex-col'>
          Select account Id:
          <input
            type={'text'}
            value={accountId}
            className={'text-center'}
            onChange={(e) => setAccountId(e.target.value)}
          />
        </div>

        <textarea
          className={'w-full h-full'}
          value={healthComputerJson}
          onChange={(event) => {
            if (!event.target.value) return

            setHealthComputerJson(
              JSON.stringify(JSON.parse(event.target.value), undefined, 4) ?? '',
            )
          }}
        ></textarea>
      </div>
    </div>
  )
}

export default App

type FunctionType = 'perp' | 'swap' | 'borrow' | 'liquidation' | 'withdraw'

const FUNCTIONS: {
  name: string
  functionType: FunctionType
  component: (healthComputer: HealthComputer) => ReactNode
}[] = [
  {
    name: 'Max perp amount',
    functionType: 'perp',
    component: (healthComputer: HealthComputer) => (
      <MaxPerpAmount healthComputer={healthComputer} />
    ),
  },
  {
    name: 'Max swap amount',
    functionType: 'swap',
    component: (healthComputer: HealthComputer) => (
      <MaxSwapAmount healthComputer={healthComputer} />
    ),
  },
  {
    name: 'Max borrow amount',
    functionType: 'borrow',
    component: (healthComputer: HealthComputer) => (
      <MaxBorrowAmount healthComputer={healthComputer} />
    ),
  },
  {
    name: 'Max withdraw amount',
    functionType: 'withdraw',
    component: (healthComputer: HealthComputer) => (
      <MaxWithdrawAmount healthComputer={healthComputer} />
    ),
  },
  {
    name: 'Liquidation price',
    functionType: 'liquidation',
    component: (healthComputer: HealthComputer) => (
      <LiquidationPrice healthComputer={healthComputer} />
    ),
  },
]
