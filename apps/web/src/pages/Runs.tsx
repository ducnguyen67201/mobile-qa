import { useState } from 'react'
import { Button, Card, Group, Select, Stack, Text } from '@mantine/core'
import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router'
import { appsQuery } from '@/api/setup'
import { runHistoryQuery } from '@/api/runs'
import type { RunHistoryFilter } from '@/api/generated/types.gen'
import { useWorkspace } from '@/hooks/use-workspace'
import { ErrorNotice, LoadingPanel, PageHeading } from '@/components/app/feedback'
export function Runs() {
  const { workspaceId = '' } = useWorkspace()
  return <WorkspaceRuns key={workspaceId} />
}
function WorkspaceRuns() {
  const { workspaceId = '', href } = useWorkspace()
  const apps = useQuery(appsQuery(workspaceId))
  const [appId, setAppId] = useState<string | null>(null)
  const selected = appId ?? apps.data?.items[0]?.id ?? ''
  const [filter, setFilter] = useState<RunHistoryFilter>('all')
  const [cursors, setCursors] = useState<(string | undefined)[]>([undefined])
  const history = useQuery(runHistoryQuery(workspaceId, selected, filter, cursors.at(-1)))
  return (
    <Stack>
      <PageHeading
        eyebrow="Execution"
        title="Runs"
        description="Durable test and release runs, plus clearly labeled editor trials."
      />
      {apps.isError && <ErrorNotice error={apps.error} retry={() => void apps.refetch()} />}
      <Select
        label="App"
        placeholder="Choose an app"
        data={apps.data?.items.map((a) => ({ value: a.id, label: a.name })) ?? []}
        value={selected || null}
        onChange={(v) => {
          setAppId(v)
          setCursors([undefined])
        }}
      />
      <Select
        label="History"
        value={filter}
        data={[
          { value: 'all', label: 'Test runs, release runs and trials' },
          { value: 'test_runs', label: 'Saved test runs' },
          { value: 'release_runs', label: 'Release runs' },
          { value: 'trials', label: 'Editor trials' },
          { value: 'legacy', label: 'Legacy session activity' },
        ]}
        onChange={(value) => {
          if (!value) return
          setFilter(value as RunHistoryFilter)
          setCursors([undefined])
        }}
      />
      {!selected && <Text>Add an app and upload a build to configure a release check.</Text>}
      {selected && history.isPending && <LoadingPanel label="Loading history…" />}
      {history.isError && (
        <ErrorNotice error={history.error} retry={() => void history.refetch()} />
      )}
      {history.data?.items.length === 0 && <Text>No activity matches this history filter.</Text>}
      {history.data?.items.map((item) => {
        if (item.kind === 'execution') {
          const run = item.run
          const savedCase = run.manifest.source?.kind === 'saved_case'
          return (
            <Card key={item.stable_id} component={Link} to={href(`/runs/${run.id}`)} withBorder>
              <Group justify="space-between">
                <Text fw={600}>{run.summary}</Text>
                <Text size="xs">{savedCase ? 'Saved test' : 'Release run'}</Text>
              </Group>
              <Text size="sm">
                {run.state} ·{' '}
                {run.manifest.profile.driver === 'fake' ? 'Simulated' : 'Android emulator'} ·{' '}
                {item.created_at}
              </Text>
            </Card>
          )
        }
        return (
          <Card key={item.stable_id} withBorder>
            <Group justify="space-between">
              <Text fw={600}>{item.title}</Text>
              <Text size="xs">{item.kind === 'trial' ? 'Trial' : 'Legacy session activity'}</Text>
            </Group>
            <Text size="sm">
              {item.state.replaceAll('_', ' ')} · {item.created_at}
            </Text>
            <Text size="sm" c="dimmed">
              {item.kind === 'legacy_session_activity'
                ? 'No comparable baseline — saved-version and clean-start proof were not recorded.'
                : item.message}
            </Text>
          </Card>
        )
      })}
      <Group>
        <Button
          variant="default"
          disabled={cursors.length < 2}
          onClick={() => setCursors(cursors.slice(0, -1))}
        >
          Previous
        </Button>
        <Button
          variant="default"
          disabled={!history.data?.next_cursor}
          onClick={() => setCursors([...cursors, history.data?.next_cursor ?? undefined])}
        >
          Next
        </Button>
      </Group>
    </Stack>
  )
}
