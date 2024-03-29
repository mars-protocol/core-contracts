import { useMemo } from 'react'
import Select from './index.tsx'
import usePerpsParams from '../../hooks/usePerpsParams.ts'

type Props = {
  value: string
  onSelected: (value: string) => void
}

export function SelectPerpsAsset(props: Props) {
  const { data: perpsParams } = usePerpsParams()

  const options = useMemo(() => (perpsParams ? Object.keys(perpsParams) : []), [perpsParams])

  return <Select label='Asset' options={options} {...props} />
}
