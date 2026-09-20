import { Badge, Card, Group, Stack, Text } from '@mantine/core'
import type { RunHistoryItem } from '@/api/generated/types.gen'
import { actionLabel } from '@/lib/action-label'

/** Historical receipts are execution facts, never an inferred regression verdict. */
export function SessionResult({ item }: { item: RunHistoryItem }) {
  const task = item.trial
  if (!task) return null
  const problem = task.steps?.find((step) =>
    ['failed', 'blocked', 'inconclusive'].includes(step.state),
  )
  const outcome = problem?.state ?? task.state
  const label =
    {
      completed: 'Finished',
      failed: 'Failed',
      blocked: 'Blocked',
      started: 'Running',
      inconclusive: 'Inconclusive',
      queued: 'Queued',
      acting: 'Running',
      stopped: 'Stopped',
    }[outcome] ?? outcome
  const actionIndex = task.sequence?.actions.findIndex((a) => a.id === problem?.action_id) ?? -1
  const problemText =
    problem?.message === 'assertion failed' ? 'An expected result did not match.' : problem?.message
  return (
    <Card withBorder padding="md">
      <Stack gap={6}>
        <Group justify="space-between" wrap="nowrap" align="flex-start">
          <Text fw={600}>{task.goal}</Text>
          <Badge
            color={outcome === 'failed' ? 'red' : outcome === 'completed' ? 'green' : 'gray'}
            style={{ flexShrink: 0 }}
          >
            {label}
          </Badge>
        </Group>
        <Text size="xs" c="dimmed">
          Build {item.build_label} · {new Date(item.created_at).toLocaleString()} ·{' '}
          {item.source === 'legacy' ? 'Earlier test' : 'Trial'}
        </Text>
        {problem && (
          <Text size="sm">
            {actionIndex >= 0 ? `Action ${actionIndex + 1}: ` : ''}
            {problemText}
          </Text>
        )}
        <details>
          <summary style={{ cursor: 'pointer', fontSize: 'var(--mantine-font-size-sm)' }}>
            View steps
          </summary>
          <Stack gap="xs" mt="xs">
            {task.steps?.map((step, index) => {
              const action = task.sequence?.actions.find((a) => a.id === step.action_id)
              const position = action ? task.sequence!.actions.indexOf(action) + 1 : index + 1
              return (
                <div key={step.action_id}>
                  <Text size="sm">
                    {position}. {action ? actionLabel(action) : `Action ${position}`}
                  </Text>
                  <Text size="xs" c="dimmed">
                    {step.state === 'completed' ? 'Completed' : step.message}
                  </Text>
                </div>
              )
            })}
            {!task.steps?.length && <Text size="sm">{task.message}</Text>}
          </Stack>
        </details>
      </Stack>
    </Card>
  )
}
