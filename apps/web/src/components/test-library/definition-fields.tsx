/** Form state uses generated domain types; the server owns publishability and readiness. */
import { ActionIcon, Group, NumberInput, SimpleGrid } from '@mantine/core'
import { ArrowDown, ArrowUp, Trash2 } from 'lucide-react'
import type { ExecutionBudget } from '@/api/generated/types.gen'

export function moveItem<T>(items: readonly T[], index: number, direction: -1 | 1): T[] {
  const next = index + direction
  if (index < 0 || next < 0 || index >= items.length || next >= items.length) return [...items]
  const result = [...items]
  ;[result[index], result[next]] = [result[next]!, result[index]!]
  return result
}
export function Ordering({
  index,
  count,
  name,
  move,
  remove,
}: {
  index: number
  count: number
  name: string
  move: (direction: -1 | 1) => void
  remove: () => void
}) {
  return (
    <Group gap={4} wrap="nowrap">
      <ActionIcon
        variant="subtle"
        aria-label={`Move ${name} up`}
        disabled={index === 0}
        onClick={() => move(-1)}
      >
        <ArrowUp size={16} />
      </ActionIcon>
      <ActionIcon
        variant="subtle"
        aria-label={`Move ${name} down`}
        disabled={index === count - 1}
        onClick={() => move(1)}
      >
        <ArrowDown size={16} />
      </ActionIcon>
      <ActionIcon variant="subtle" color="red" aria-label={`Remove ${name}`} onClick={remove}>
        <Trash2 size={16} />
      </ActionIcon>
    </Group>
  )
}
export function BudgetFields({
  value,
  onChange,
}: {
  value: ExecutionBudget
  onChange: (value: ExecutionBudget) => void
}) {
  return (
    <SimpleGrid cols={{ base: 1, sm: 3 }}>
      <NumberInput
        label="Time limit (seconds)"
        min={0}
        max={1800}
        allowDecimal={false}
        value={value.duration_seconds}
        onChange={(n) => onChange({ ...value, duration_seconds: Number(n) })}
      />
      <NumberInput
        label="Maximum actions"
        min={0}
        max={400}
        allowDecimal={false}
        value={value.max_steps}
        onChange={(n) => onChange({ ...value, max_steps: Number(n) })}
      />
      <NumberInput
        label="Evidence limit (bytes)"
        min={0}
        max={262144000}
        allowDecimal={false}
        value={value.artifact_bytes}
        onChange={(n) => onChange({ ...value, artifact_bytes: Number(n) })}
      />
    </SimpleGrid>
  )
}
