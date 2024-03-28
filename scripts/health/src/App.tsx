import { useEffect, useState } from 'react'
import './App.css'
import useHealthComputer from './hooks/useHealthComputer.ts'
import init from '../pkg-web/'
import MaxPerpAmount from './components/MaxPerpAmount.tsx'

function App() {
  const [healthComputerJson, setHealthComputerJson] = useState('')
  const [type, setType] = useState<'borrow' | 'swap' | 'perp'>('perp')
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

  return (
    <div className={'h-full w-full flex flex-col gap-4'}>
      <div className='flex gap-4'>
        <button onClick={() => setType('perp')}>Max perp amount</button>
      </div>

      {type === 'perp' && (
        <MaxPerpAmount healthComputer={healthComputerJson && JSON.parse(healthComputerJson)} />
      )}

      <div className={'flex flex-col gap-4 h-full'}>
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
            console.log('updated HC manually')

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
