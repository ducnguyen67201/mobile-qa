import { Anchor, Badge, Group, List, Stack, Text } from '@mantine/core'
import type { AttemptResponse } from '@/api/generated/types.gen'
import { formatDate } from '@/lib/format'

export function AttemptReadiness({
  attempt,
  runId,
  requiresCleanStart,
  simulated,
}: {
  attempt: AttemptResponse
  runId: string
  requiresCleanStart: boolean
  simulated: boolean
}) {
  const preflight = attempt.preflight
  const recovery = attempt.recovery_events ?? []
  const preparing = ['queued', 'leased', 'running', 'cancel_requested'].includes(attempt.state)
  const canceledWithoutReceipt = attempt.outcome === 'canceled' && !preflight
  const startLabel = preflight
    ? simulated
      ? 'Simulated clean start'
      : 'Clean start verified'
    : !requiresCleanStart
      ? 'Clean start not recorded'
      : canceledWithoutReceipt
        ? 'Clean start not recorded'
        : preparing
          ? 'Clean start pending'
          : 'Clean start missing'
  // Recovery releases the phone; it does not replace the attempt's original cleanup proof.
  const original = attempt.original_cleanup
  const cleanup = original
    ? original.stopped && original.reset === 'verified_clean'
      ? 'verified_clean'
      : 'quarantined'
    : attempt.cleanup
  const cleanupLabel =
    cleanup === 'verified_clean'
      ? simulated
        ? 'Simulated cleanup'
        : 'Cleanup verified'
      : cleanup === 'quarantined'
        ? 'Cleanup needs attention'
        : 'Cleanup pending'

  return (
    <Stack gap={6}>
      <Group gap={6} role="group" aria-label="Attempt readiness">
        <Badge
          color={
            preflight && !simulated
              ? 'forest'
              : !preflight && requiresCleanStart && !preparing && !canceledWithoutReceipt
                ? 'orange'
                : 'gray'
          }
        >
          {startLabel}
        </Badge>
        <Badge
          color={
            cleanup === 'quarantined' ? 'orange' : cleanup === 'verified_clean' ? 'forest' : 'gray'
          }
        >
          {cleanupLabel}
        </Badge>
        {recovery.length > 0 && (
          <Badge color="gray">
            {recovery.length} recovery {recovery.length === 1 ? 'record' : 'records'}
          </Badge>
        )}
      </Group>
      <details>
        <summary style={{ cursor: 'pointer', fontSize: 'var(--mantine-font-size-xs)' }}>
          Start and cleanup details
        </summary>
        <Stack gap="xs" pt="xs">
          {preflight ? (
            <>
              <Text size="xs">
                {simulated ? 'Simulated starting state recorded' : 'Starting state verified'}{' '}
                {formatDate(preflight.ready_at)} · {(preflight.duration_ms / 1000).toFixed(1)}{' '}
                seconds
              </Text>
              <Group gap="xs">
                {preflight.artifact_ids.map((id) => (
                  <Anchor
                    key={id}
                    href={`/api/runs/${runId}/artifacts/${id}/content`}
                    target="_blank"
                    rel="noreferrer"
                    size="xs"
                  >
                    {attempt.artifacts.find((artifact) => artifact.id === id)?.name ??
                      'Starting state evidence'}
                  </Anchor>
                ))}
              </Group>
            </>
          ) : (
            <Text size="xs">
              {requiresCleanStart
                ? 'No verified starting state has been recorded for this attempt.'
                : 'This run has no clean-start record. Its starting state cannot be verified here.'}
            </Text>
          )}
          {original && (
            <Text size="xs">
              Original cleanup: {cleanupLabel.toLowerCase()}. {original.evidence_reference}
            </Text>
          )}
          {recovery.length > 0 && (
            <>
              <Text size="xs">
                {attempt.cleanup === 'verified_clean'
                  ? 'The phone was released after recovery.'
                  : 'The phone is still awaiting cleanup.'}{' '}
                The recorded test result and original cleanup remain unchanged.
              </Text>
              <List size="xs" spacing={4}>
                {recovery.map((event, index) => (
                  <List.Item key={`${event.created_at}:${index}`}>
                    {formatDate(event.created_at)} · {event.evidence_reference}
                  </List.Item>
                ))}
              </List>
            </>
          )}
        </Stack>
      </details>
    </Stack>
  )
}
