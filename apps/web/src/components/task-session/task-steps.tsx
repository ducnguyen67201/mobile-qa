import {
  ActionIcon,
  Button,
  Card,
  Group,
  NativeSelect,
  Stack,
  Text,
  Textarea,
  TextInput,
} from '@mantine/core'
import { ArrowDown, ArrowUp, Trash2 } from 'lucide-react'

/** Editor-only state. The existing generated task contract receives the ordered goal. */
export type TaskStep = {
  id: string
  kind: 'ai' | 'tap' | 'type' | 'swipe' | 'back' | 'restart'
  text: string
  target: string
}
export const newTaskStep = (): TaskStep => ({
  id: crypto.randomUUID(),
  kind: 'ai',
  text: '',
  target: '',
})
export function stepInstruction(step: TaskStep): string {
  switch (step.kind) {
    case 'ai':
      return step.text.trim()
    case 'tap':
      return step.target.trim() ? `Tap ${step.target.trim()}.` : ''
    case 'type':
      return step.target.trim() && step.text.trim()
        ? `Enter ${JSON.stringify(step.text)} into ${step.target.trim()}.`
        : ''
    case 'swipe':
      return step.text.trim() ? `Swipe ${step.text.trim()}.` : ''
    case 'back':
      return 'Press the Android Back button.'
    case 'restart':
      return 'Restart the app without clearing its saved data.'
  }
}
export function taskGoal(steps: TaskStep[]): string {
  const instructions = steps.map(stepInstruction)
  if (!instructions.length || instructions.some((instruction) => !instruction)) return ''
  return instructions.length === 1
    ? (instructions[0] ?? '')
    : `Perform these steps in order. Stop if a step cannot be completed.\n${instructions.map((instruction, i) => `${i + 1}. ${instruction}`).join('\n')}`
}
export function TaskSteps({
  steps,
  onChange,
  disabled,
}: {
  steps: TaskStep[]
  onChange: (steps: TaskStep[]) => void
  disabled: boolean
}) {
  const update = (id: string, value: Partial<TaskStep>) =>
    onChange(steps.map((step) => (step.id === id ? { ...step, ...value } : step)))
  const move = (index: number, offset: number) => {
    const next = [...steps]
    const [step] = next.splice(index, 1)
    if (step) next.splice(index + offset, 0, step)
    onChange(next)
  }
  return (
    <Stack gap="sm">
      {steps.map((step, index) => (
        <Card key={step.id} withBorder padding="md">
          <Stack gap="sm">
            <Group justify="space-between">
              <Text fw={600}>Step {index + 1}</Text>
              <Group gap={4}>
                <ActionIcon
                  variant="subtle"
                  aria-label={`Move step ${index + 1} up`}
                  disabled={disabled || index === 0}
                  onClick={() => move(index, -1)}
                >
                  <ArrowUp size={16} />
                </ActionIcon>
                <ActionIcon
                  variant="subtle"
                  aria-label={`Move step ${index + 1} down`}
                  disabled={disabled || index === steps.length - 1}
                  onClick={() => move(index, 1)}
                >
                  <ArrowDown size={16} />
                </ActionIcon>
                <ActionIcon
                  variant="subtle"
                  color="red"
                  aria-label={`Remove step ${index + 1}`}
                  disabled={disabled || steps.length === 1}
                  onClick={() => onChange(steps.filter((item) => item.id !== step.id))}
                >
                  <Trash2 size={16} />
                </ActionIcon>
              </Group>
            </Group>
            <NativeSelect
              label={`Step ${index + 1} action`}
              value={step.kind}
              disabled={disabled}
              data={[
                { value: 'ai', label: 'Ask AI — describe what to do' },
                { value: 'tap', label: 'Tap a control' },
                { value: 'type', label: 'Enter text' },
                { value: 'swipe', label: 'Swipe' },
                { value: 'back', label: 'Go back' },
                { value: 'restart', label: 'Restart app' },
              ]}
              onChange={(event) => {
                const kind = event.currentTarget.value
                if (
                  kind === 'ai' ||
                  kind === 'tap' ||
                  kind === 'type' ||
                  kind === 'swipe' ||
                  kind === 'back' ||
                  kind === 'restart'
                )
                  update(step.id, { kind })
              }}
            />
            {(step.kind === 'tap' || step.kind === 'type') && (
              <TextInput
                label={`Step ${index + 1} target`}
                placeholder="e.g. Save button or task input"
                value={step.target}
                disabled={disabled}
                maxLength={500}
                onChange={(event) => update(step.id, { target: event.currentTarget.value })}
              />
            )}
            {(step.kind === 'ai' || step.kind === 'type' || step.kind === 'swipe') && (
              <Textarea
                label={
                  step.kind === 'ai'
                    ? `Step ${index + 1} instruction`
                    : step.kind === 'type'
                      ? `Step ${index + 1} text`
                      : `Step ${index + 1} swipe`
                }
                placeholder={
                  step.kind === 'ai'
                    ? 'Describe a task, such as create and save a task called Buy milk.'
                    : step.kind === 'type'
                      ? 'Buy milk'
                      : 'up to find the Settings button'
                }
                autosize
                minRows={2}
                maxLength={4000}
                value={step.text}
                disabled={disabled}
                onChange={(event) => update(step.id, { text: event.currentTarget.value })}
              />
            )}
          </Stack>
        </Card>
      ))}
      <Button
        variant="light"
        w="fit-content"
        disabled={disabled || steps.length >= 20}
        onClick={() => onChange([...steps, newTaskStep()])}
      >
        Add step
      </Button>
      <Text size="xs" c="dimmed">
        Minitap uses AI to carry out these steps in order. You can also give it a whole task in one
        Ask AI step.
      </Text>
    </Stack>
  )
}
