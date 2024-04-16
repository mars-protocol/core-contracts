import { BorrowTarget, HealthComputer, max_borrow_estimate_js } from '../../pkg-web'
import { useCallback, useState } from 'react'
import { SelectAsset } from './Select/SelectAsset.tsx'
import Select from './Select/index.tsx'
import { Input } from './Input.tsx'

type Props = {
  healthComputer: HealthComputer
}

export default function MaxBorrowAmount(props: Props) {
  const [selectedDenom, setSelectedDenom] = useState<string | null>(null)
  const [borrowTarget, setBorrowTarget] = useState<Target>('deposit')
  const [amount, setAmount] = useState('-')
  const [vaultAddress, setVaultAddress] = useState('')
  const [swapIntoDenom, setSwapIntoDenom] = useState('')
  const [error, setError] = useState<null | string>(null)
  const [slippage, setSlippage] = useState('0.05')

  const onConfirm = useCallback(() => {
    try {
      setError(null)
      if (!selectedDenom) return
      let target: BorrowTarget = 'deposit'

      if (borrowTarget === 'wallet') target = 'wallet'
      if (borrowTarget === 'swap')
        target = { swap: { denom_out: swapIntoDenom, slippage: slippage } }
      if (borrowTarget === 'vault') target = { vault: { address: vaultAddress } }

      const amount = max_borrow_estimate_js(props.healthComputer, selectedDenom, target)
      setAmount(amount)
    } catch (e) {
      setError((e as string).toString())
    }
  }, [borrowTarget, props.healthComputer, selectedDenom, slippage, swapIntoDenom, vaultAddress])

  return (
    <div className='gap-4 flex flex-col items-start bg-black p-8 rounded-md'>
      <SelectAsset value={selectedDenom ?? ''} onSelected={setSelectedDenom} />
      <Select
        label='Borrow target'
        options={TARGETS}
        value={borrowTarget}
        onSelected={setBorrowTarget}
      />
      {borrowTarget === 'vault' && (
        <Input label='Vault address' value={vaultAddress} onChange={setVaultAddress} />
      )}
      {borrowTarget === 'swap' && (
        <>
          <SelectAsset value={swapIntoDenom} onSelected={setSwapIntoDenom} />
          <Input label='Slippage' value={slippage} onChange={setSlippage} />
        </>
      )}

      <button onClick={onConfirm}>Calculate Max Borrow Amount</button>

      {error ? <p className={'text-red-500'}>{error}</p> : <p>Max amount: {amount}</p>}
    </div>
  )
}

type Target = 'deposit' | 'wallet' | 'swap' | 'vault'
const TARGETS: Target[] = ['deposit', 'wallet', 'swap', 'vault']
