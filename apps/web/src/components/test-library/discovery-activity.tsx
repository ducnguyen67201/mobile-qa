/** Activity comes from persisted device receipts, never simulated agent thinking. */
import { Badge, Group, Loader, Stack, Text } from '@mantine/core'
import type { DirectCommand, PhoneControl, PhoneFrame, PhoneTask } from '@/api/generated/types.gen'

function controlFor(command: DirectCommand, frame?: PhoneFrame | null): PhoneControl | undefined {
  if (!('target' in command)) return undefined
  const target = command.target
  const matches = frame?.controls.filter((c) =>
    target.by === 'resource_id' ? c.resource_id === target.value : c.description === target.value,
  )
  return matches?.length === 1 ? matches[0] : undefined
}

export function actionLabel(command: DirectCommand, frame?: PhoneFrame | null): string {
  const control = controlFor(command, frame)
  const target =
    'target' in command
      ? control?.label || control?.description || command.target.value.split(':id/').pop()
      : ''
  switch (command.operation) {
    case 'tap':
      return `Tap ${target}`
    case 'set_text':
      return `Enter “${command.text}” into ${target}`
    case 'swipe':
      return `Swipe ${command.direction}`
    case 'back':
      return 'Go back'
    case 'restart':
      return 'Restart the app'
    case 'wait_for':
      return `Wait for ${target}`
  }
}

export function explorationLabel(task: PhoneTask): string {
  if (task.progress?.state === 'canceled') return 'Exploration stopped'
  if (task.state === 'failed' || task.progress?.state === 'failed')
    return 'Exploration could not finish'
  if (task.state === 'completed')
    return task.progress?.proposals.length ? 'Tests ready to review' : 'No tests generated'
  if (task.progress?.state === 'drafting') return 'Drafting tests from recorded actions'
  return task.state === 'queued' ? 'Waiting to explore' : 'Exploring with AI'
}

export function discoveryGap(gap: string): string {
  const messages: Record<string, string> = {
    discovery_interaction_scope_required:
      'AI could inspect the screen but could not tap or type. Enable “Allow AI to tap, type and change test data” above, then explore again.',
    discovery_time_limit: 'Exploration reached its time limit. Try a shorter journey.',
    discovery_model_budget: 'Exploration reached its AI call limit. Try a more focused journey.',
    discovery_observation_limit: 'Exploration reached its screen limit. Try a shorter journey.',
    discovery_evidence_limit:
      'Exploration reached its action or screen limit. Review the recorded actions.',
    discovery_canceled: 'Exploration was stopped.',
    discovery_previous_action_incomplete:
      'The last action could not be confirmed. Check the phone before starting again.',
    discovery_authority_lost: 'The phone connection was lost. Reconnect the phone and try again.',
    discovery_connection_lost: 'The phone connection was lost. Reconnect the phone and try again.',
  }
  return (
    messages[gap] ??
    (/^[a-z][a-z0-9_]+$/.test(gap)
      ? 'Exploration could not finish this part of the journey. Check the phone and try a more specific task.'
      : gap)
  )
}

export function DiscoveryActivity({ task }: { task: PhoneTask }) {
  const progress = task.progress
  const active = task.state === 'queued' || task.state === 'acting'
  return (
    <Stack gap="xs" aria-label="AI exploration activity">
      <Group gap="xs" role="status">
        {active && <Loader size="xs" />}
        <Text size="sm" fw={600}>
          {explorationLabel(task)}
        </Text>
      </Group>
      <Text size="xs" c="dimmed">
        {progress?.snapshots.length ?? 0} screens observed ·{' '}
        {progress?.journal?.filter((r) => r.outcome === 'completed').length ?? 0} actions recorded
      </Text>
      {!progress?.journal?.length && active && (
        <Text size="sm">
          {task.state === 'queued'
            ? 'Waiting for the phone to start exploration.'
            : 'AI is inspecting the screen and choosing an action. Actions appear here as they are recorded.'}
        </Text>
      )}
      {!!progress?.journal?.length && (
        <ol style={{ margin: 0, paddingLeft: 24 }} aria-label="Explored actions">
          {progress.journal.map((receipt) => (
            <li key={receipt.id}>
              <Group gap="xs" py={3}>
                <Text size="sm" style={{ overflowWrap: 'anywhere' }}>
                  {actionLabel(
                    receipt.command,
                    progress.snapshots.find((s) => s.id === receipt.before_id)?.frame,
                  )}
                </Text>
                <Badge size="xs" variant="light">
                  {receipt.outcome === 'pending'
                    ? active
                      ? 'Acting'
                      : 'Not confirmed'
                    : receipt.outcome === 'completed'
                      ? 'Done'
                      : receipt.outcome === 'uncertain'
                        ? 'Not confirmed'
                        : 'Failed'}
                </Badge>
              </Group>
            </li>
          ))}
        </ol>
      )}
    </Stack>
  )
}

export function DiscoveryTarget({ task, frame }: { task?: PhoneTask; frame: PhoneFrame }) {
  if (task?.state !== 'acting') return null
  const receipt = task.progress?.journal?.at(-1)
  if (
    !receipt ||
    receipt.outcome !== 'pending' ||
    !['tap', 'set_text'].includes(receipt.command.operation)
  )
    return null
  // An old screenshot or ambiguous selector cannot locate the current action honestly.
  const before = task.progress?.snapshots.find((s) => s.id === receipt.before_id)
  if (before?.frame.id !== frame.id) return null
  const control = controlFor(receipt.command, frame)
  if (
    !control ||
    frame.width <= 0 ||
    frame.height <= 0 ||
    control.right <= control.left ||
    control.bottom <= control.top
  )
    return null
  return (
    <div
      role="img"
      aria-label={`AI target: ${actionLabel(receipt.command, frame)}`}
      style={{
        pointerEvents: 'none',
        position: 'absolute',
        boxSizing: 'border-box',
        left: `${(100 * control.left) / frame.width}%`,
        top: `${(100 * control.top) / frame.height}%`,
        width: `${(100 * (control.right - control.left)) / frame.width}%`,
        height: `${(100 * (control.bottom - control.top)) / frame.height}%`,
        border: '3px solid var(--mantine-color-green-8)',
        background: 'rgba(50, 130, 80, 0.18)',
        borderRadius: 4,
      }}
    >
      <span
        aria-hidden="true"
        style={{
          position: 'absolute',
          right: 0,
          top: 0,
          borderRadius: '50%',
          width: 12,
          height: 12,
          background: 'var(--mantine-color-green-8)',
        }}
      />
    </div>
  )
}
