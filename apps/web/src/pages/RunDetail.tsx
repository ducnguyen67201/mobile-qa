import { actionLabel } from '@/lib/action-label'
import {
  Alert,
  Anchor,
  Badge,
  Button,
  Card,
  Code,
  Group,
  Image,
  List,
  Stack,
  Text,
  Title,
} from '@mantine/core'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useParams } from 'react-router'
import { appQuery } from '@/api/setup'
import { cancelRun, runComparisonQuery, runQuery } from '@/api/runs'
import { useWorkspace } from '@/hooks/use-workspace'
import { ErrorNotice, LoadingPanel, PageHeading } from '@/components/app/feedback'
import { AttemptReadiness } from '@/components/app/attempt-readiness'
export function RunDetail() {
  const { run_id: id = '' } = useParams()
  const { workspaceId = '' } = useWorkspace()
  const client = useQueryClient()
  const run = useQuery(runQuery(workspaceId, id))
  const comparison = useQuery(runComparisonQuery(workspaceId, id, run.data?.state === 'finished'))
  const baseline = useQuery({
    ...runQuery(workspaceId, run.data?.baseline_run_id ?? ''),
    enabled: !!run.data?.baseline_run_id,
  })
  const app = useQuery({ ...appQuery(run.data?.manifest.app_id ?? ''), enabled: !!run.data })
  const cancel = useMutation({
    mutationFn: () => cancelRun(id),
    onSuccess: (value) => client.setQueryData(['run', workspaceId, id], value),
  })
  if (run.isError) return <ErrorNotice error={run.error} retry={() => void run.refetch()} />
  if (!run.data || !app.data)
    return app.isError ? (
      <ErrorNotice error={app.error} retry={() => void app.refetch()} />
    ) : (
      <LoadingPanel label="Loading run…" />
    )
  if (app.data.organization_id !== workspaceId)
    return <Text>This run belongs to a different workspace.</Text>
  const r = run.data
  return (
    <Stack>
      <PageHeading
        eyebrow={r.manifest.source?.kind === 'saved_case' ? 'Saved test run' : 'Release check'}
        title={r.summary}
        description={app.data.name}
      />
      {r.manifest.profile.driver === 'fake' && (
        <Alert color="yellow">Simulated execution. No real phone or model was used.</Alert>
      )}
      <Group>
        <Badge>{r.state}</Badge>
        <Text>
          {r.manifest.profile.name} · {r.manifest.profile.image}
        </Text>
        <Button
          color="red"
          variant="light"
          disabled={r.state === 'finished' || r.state === 'cancel_requested'}
          loading={cancel.isPending}
          onClick={() => cancel.mutate()}
        >
          Cancel run
        </Button>
      </Group>
      {r.state === 'cancel_requested' && (
        <Alert>Cancellation requested. Waiting for your phone to stop and clean up.</Alert>
      )}
      {r.state === 'recovery_required' && (
        <Alert color="orange">
          Your phone needs attention before another run can start. The recorded test result is
          unchanged.
        </Alert>
      )}
      {cancel.isError && <ErrorNotice error={cancel.error} retry={() => cancel.mutate()} />}
      <Text size="sm">Build checksum</Text>
      <Code style={{ overflowWrap: 'anywhere' }}>{r.manifest.build_sha256}</Code>
      {comparison.data && (
        <Card withBorder>
          <Stack gap="sm">
            <Group justify="space-between">
              <Title order={2} size="h4">
                Comparison
              </Title>
              <Badge
                color={
                  comparison.data.label === 'regression' ||
                  comparison.data.label === 'new_failure_same_build'
                    ? 'red'
                    : comparison.data.label === 'recovered'
                      ? 'forest'
                      : 'gray'
                }
              >
                {comparison.data.label.replaceAll('_', ' ')}
              </Badge>
            </Group>
            {comparison.data.reason && <Text size="sm">{comparison.data.reason}</Text>}
            {baseline.data && (
              <Group align="flex-start" grow>
                <Stack gap={2}>
                  <Text size="xs" c="dimmed">
                    Baseline build
                  </Text>
                  <Code style={{ overflowWrap: 'anywhere' }}>
                    {baseline.data.manifest.build_sha256}
                  </Code>
                </Stack>
                <Stack gap={2}>
                  <Text size="xs" c="dimmed">
                    Current build
                  </Text>
                  <Code style={{ overflowWrap: 'anywhere' }}>{r.manifest.build_sha256}</Code>
                </Stack>
              </Group>
            )}
            {comparison.data.checks.map((check) => (
              <Card key={check.check_id} bg="var(--mantine-color-gray-0)">
                <Text fw={600}>{check.check_id}</Text>
                <Text size="sm">Expected: {check.expected}</Text>
                <Text size="sm">
                  Baseline: {check.baseline_observed ?? 'Unknown'} · Current:{' '}
                  {check.current_observed ?? 'Unknown'}
                </Text>
                {check.baseline_observed == null && (
                  <Text size="xs" c="dimmed">
                    Baseline reason: {check.baseline_reason || 'No observation was retained.'}
                  </Text>
                )}
                {check.current_observed == null && (
                  <Text size="xs" c="dimmed">
                    Current reason: {check.current_reason || 'No observation was retained.'}
                  </Text>
                )}
              </Card>
            ))}
          </Stack>
        </Card>
      )}
      {comparison.isError && (
        <ErrorNotice error={comparison.error} retry={() => void comparison.refetch()} />
      )}
      {r.attempts.map((a) => (
        <Card key={a.id} withBorder>
          <Stack gap="sm">
            <Group justify="space-between" align="flex-start" gap="xs">
              <Title order={2} fz="lg">
                {r.manifest.cases.find((c) => c.definition_id === a.case_version_id)?.case.title ??
                  'Test attempt'}
              </Title>
              <Text size="xs" c="dimmed">
                Attempt {a.number}
              </Text>
            </Group>
            <Group gap="xs">
              <Badge
                color={a.outcome === 'failed' ? 'red' : a.outcome === 'passed' ? 'forest' : 'gray'}
              >
                Result: {a.outcome ?? 'Not yet evaluated'}
              </Badge>
              <Text size="xs" c="dimmed">
                {a.state.replaceAll('_', ' ')}
              </Text>
            </Group>
            <AttemptReadiness
              attempt={a}
              runId={r.id}
              requiresCleanStart={!!r.manifest.profile.execution_context}
              simulated={r.manifest.profile.driver === 'fake'}
            />
            {a.reason && <Text size="sm">{a.reason}</Text>}
            <List>
              {r.manifest.cases
                .find((c) => c.definition_id === a.case_version_id)
                ?.case.actions.map((action) => (
                  <List.Item key={action.id}>
                    <Text>
                      {action.id}: {actionLabel(action)}
                    </Text>
                    {a.events.some((e) => e.action_id === action.id) ? (
                      a.events
                        .filter((e) => e.action_id === action.id)
                        .map((e) => (
                          <Text size="sm" key={e.id}>
                            {e.phase}: {e.message}
                          </Text>
                        ))
                    ) : (
                      <Text size="sm">
                        No checkpoint reported{a.reason ? `: ${a.reason}` : '; execution pending'}.
                      </Text>
                    )}
                  </List.Item>
                ))}
            </List>
            {a.checks.length === 0 && <Text>Checks have not been evaluated yet.</Text>}
            <List>
              {a.checks.map((c) => (
                <List.Item key={c.check_id}>
                  <Text fw={600}>
                    {c.check_id}: {c.outcome}
                  </Text>
                  <Text>Expected: {c.expected}</Text>
                  <Text>
                    Observed: {c.observed ?? 'Not established'} · {c.reason}
                  </Text>
                </List.Item>
              ))}
            </List>
            {a.artifacts
              .filter((f) => !a.preflight?.artifact_ids.includes(f.id))
              .map((f) => {
                const url = `/api/runs/${r.id}/artifacts/${f.id}/content`
                return (
                  <div key={f.id}>
                    {f.state === 'sealed' ? (
                      <>
                        {f.mime === 'image/png' && (
                          <Image
                            src={url}
                            alt={`${f.checkpoint_id} evidence`}
                            maw={320}
                            fit="contain"
                          />
                        )}
                        <Anchor href={url} target="_blank" rel="noreferrer">
                          {f.name}
                        </Anchor>
                      </>
                    ) : (
                      <Text>
                        {f.name}: {f.reason ?? f.state}
                      </Text>
                    )}
                  </div>
                )
              })}
            {a.usage.map((u, i) => (
              <Text size="sm" key={`${u.model}:${i}`}>
                {u.model}: {u.calls} calls; {u.input_tokens ?? 'unknown'} input /{' '}
                {u.output_tokens ?? 'unknown'} output tokens; {u.unknown_calls} calls with unknown
                usage
              </Text>
            ))}
          </Stack>
        </Card>
      ))}
    </Stack>
  )
}
