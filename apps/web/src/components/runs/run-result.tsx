import { Anchor, Badge, Card, Group, Image, Stack, Text } from '@mantine/core'
import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router'
import type { CheckResult, ComparisonKind, RunResponse } from '@/api/generated/types.gen'
import { runQuery } from '@/api/runs'
import { useWorkspace } from '@/hooks/use-workspace'
const labels: Record<ComparisonKind, string> = {
  regression: 'Regression',
  still_failing: 'Still failing',
  recovered: 'Recovered',
  unchanged: 'Unchanged',
  new_failure: 'New failure on same build',
  no_baseline: 'No baseline',
  not_comparable: 'Not comparable',
  added: 'Added test',
  removed: 'Removed test',
}
const observed = (c: CheckResult) =>
  c.observation_kind === 'absent' ? 'Control absent' : (c.observed ?? 'Not observed')
/** Display backend facts. The browser does not choose or calculate a verdict. */
export function RunResult({ run, compact = false }: { run: RunResponse; compact?: boolean }) {
  const { workspaceId = '', href } = useWorkspace()
  const baseline = useQuery({
    ...runQuery(workspaceId, run.baseline_run_id ?? ''),
    enabled: !compact && !!run.baseline_run_id,
  })
  const screenshot = (r: RunResponse | undefined, check: CheckResult | undefined) =>
    r?.attempts
      .flatMap((a) => a.artifacts)
      .find(
        (f) => check?.artifact_ids.includes(f.id) && f.mime === 'image/png' && f.state === 'sealed',
      )
  return (
    <Card withBorder padding="sm">
      <Stack gap="xs">
        <Group justify="space-between">
          <Text fw={600}>{run.manifest.cases[0]?.case.title ?? run.summary}</Text>
          <Badge color={run.attempts.some((a) => a.outcome === 'failed') ? 'red' : 'gray'}>
            {run.state === 'finished' ? run.summary : run.state.replaceAll('_', ' ')}
          </Badge>
        </Group>
        <Text size="xs" c="dimmed">
          {new Date(run.created_at).toLocaleString()} ·{' '}
          {run.manifest.source?.kind === 'saved_case_v1'
            ? 'Saved test'
            : run.manifest.source?.kind === 'saved_suite_v1'
              ? 'Saved suite'
              : 'Release plan'}
          {run.manifest.profile.driver === 'fake' ? ' · Simulated' : ''}
        </Text>
        {!run.comparison && (
          <Text size="sm">
            {run.state === 'finished' ? 'Finalizing comparison…' : 'Comparison pending'}
          </Text>
        )}
        {run.comparison?.cases.map((c) => {
          const failed = c.current_checks.find((v) => v.outcome === 'failed')
          const definition = run.manifest.cases.find(
            (v) => v.definition_id === c.current_case_id,
          )?.case
          const check = definition?.checks.find((v) => v.id === failed?.check_id)
          const action = definition?.actions.findIndex(
            (v) => v.checkpoint_id === check?.checkpoint_id,
          )
          const before = c.baseline_checks.find((v) => v.check_id === failed?.check_id)
          const currentImage = screenshot(run, failed),
            previousImage = screenshot(baseline.data, before)
          return (
            <Stack gap={4} key={`${c.case_key}:${c.data_variant}`}>
              <Group gap="xs">
                <Badge
                  color={
                    c.kind === 'regression' ? 'red' : c.kind === 'recovered' ? 'green' : 'gray'
                  }
                >
                  {labels[c.kind]}
                </Badge>
                {run.comparison!.cases.length > 1 && <Text size="sm">{c.title}</Text>}
              </Group>
              <Text size="sm" c="dimmed">
                {c.reason}
              </Text>
              {!compact && failed && (
                <>
                  <Text size="sm">
                    {action !== undefined && action >= 0 ? `Action ${action + 1} · ` : ''}
                    {check?.description ?? failed.check_id}
                  </Text>
                  <Text size="sm">
                    Expected “{failed.expected}”; observed {observed(failed)}.
                  </Text>
                  {before && (
                    <Text size="sm">
                      Baseline: {before.outcome} · observed {observed(before)}
                    </Text>
                  )}
                  <details>
                    <summary>Compare evidence</summary>
                    <Group align="flex-start">
                      {[
                        { r: baseline.data, f: previousImage, label: 'Baseline evidence' },
                        { r: run, f: currentImage, label: 'Current evidence' },
                      ].map(({ r, f, label }) => (
                        <Stack gap={4} key={label}>
                          <Text size="xs">{label}</Text>
                          {r && f ? (
                            <Anchor
                              href={`/api/runs/${r.id}/artifacts/${f.id}/content`}
                              target="_blank"
                            >
                              <Image
                                src={`/api/runs/${r.id}/artifacts/${f.id}/content`}
                                h={180}
                                w={102}
                                fit="contain"
                                alt={label}
                              />
                            </Anchor>
                          ) : (
                            <Text size="xs">Matching capture unavailable</Text>
                          )}
                        </Stack>
                      ))}
                    </Group>
                  </details>
                </>
              )}
            </Stack>
          )
        })}
        <Group gap="sm">
          <Anchor component={Link} to={href(`/runs/${run.id}`)} size="sm">
            View run & evidence
          </Anchor>
          {run.baseline_run_id && (
            <Anchor component={Link} to={href(`/runs/${run.baseline_run_id}`)} size="sm">
              Baseline run
            </Anchor>
          )}
        </Group>
        {!compact && (
          <Text size="xs" c="dimmed">
            Current build: {run.build_label ?? run.manifest.build_id} ·{' '}
            {run.manifest.build_sha256.slice(0, 12)}
            {baseline.data &&
              ` · Baseline build: ${baseline.data.build_label ?? baseline.data.manifest.build_id} · ${baseline.data.manifest.build_sha256.slice(0, 12)}`}
          </Text>
        )}
      </Stack>
    </Card>
  )
}
