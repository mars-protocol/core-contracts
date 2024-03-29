type Props = {
  label: string
  options: string[]
  value: string
  onSelected: (value: never) => void
}

export default function Select(props: Props) {
  return (
    <label>
      {`${props.label}: `}
      <select
        name={props.label}
        value={props.value}
        defaultValue={''}
        onChange={(e) => props.onSelected(e.target.value as never)}
      >
        <option>-</option>
        {props.options.map((option) => (
          <option key={option} value={option}>
            {option}
          </option>
        ))}
      </select>
    </label>
  )
}
