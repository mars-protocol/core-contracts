import { useMemo } from 'react'
import useAssetParams from '../../hooks/useAssetParams.ts'
import Select from './index.tsx'

type Props = {
  value: string
  onSelected: (value: string) => void
}

export function SelectAsset(props: Props) {
  const { data: assetParams } = useAssetParams()

  const options = useMemo(() => (assetParams ? Object.keys(assetParams) : []), [assetParams])

  return <Select label='Asset' options={options} {...props} />
}
