type Props = {
  label: string
  value: string
  onChange: (value: string) => void
}

export function Input(props: Props) {
  return (
    <label>
      {`${props.label}: `}
      <input placeholder='' value={props.value} onChange={(e) => props.onChange(e.target.value)} />
    </label>
  )
}
