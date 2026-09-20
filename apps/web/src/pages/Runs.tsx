import { useState } from 'react'
import { Anchor, Button, Card, Group, Select, Stack, Text } from '@mantine/core'
import { Link } from 'react-router'
import { useQuery } from '@tanstack/react-query'
import { appsQuery } from '@/api/setup'
import { historyQuery } from '@/api/regression'
import { RunResult } from '@/components/runs/run-result'
import { SessionResult } from '@/components/runs/session-result'
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
  const [cursors, setCursors] = useState<(string | undefined)[]>([undefined])
  const [source, setSource] = useState('all')
  const runs = useQuery(historyQuery(workspaceId, selected, source, cursors.at(-1)))
  return (
    <Stack>
      <PageHeading
        eyebrow="Execution"
        title="Runs"
        description="Test results, build comparisons and evidence."
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
        label="Show"
        value={source}
        onChange={(v) => {
          setSource(v ?? 'all')
          setCursors([undefined])
        }}
        data={[
          { value: 'all', label: 'Runs and trials' },
          { value: 'runs', label: 'Saved test runs' },
          { value: 'trials', label: 'Authoring trials' },
          { value: 'legacy', label: 'Earlier test activity' },
        ]}
      />
      {!selected && <Text>Add an app and upload a build to configure a release check.</Text>}
      {selected && runs.isPending && <LoadingPanel label="Loading runs…" />}
      {runs.isError && <ErrorNotice error={runs.error} retry={() => void runs.refetch()} />}
      {selected && (source === 'legacy' || source === 'trials') && (
        <Card withBorder padding="md" bg="var(--mantine-color-gray-0)">
          <Stack gap="xs">
            <Text fw={600}>Looking for a regression?</Text>
            <Text size="sm" c="dimmed">
              These results show what happened in the phone preview. They don’t have a verified
              build comparison. Run a saved test on the working build, then run the same version on
              the updated build and select the first run under Compare with. A comparable pass →
              fail shows a Regression badge.
            </Text>
            <Anchor component={Link} to={href(`/tests?app=${selected}&kind=case`)} size="sm">
              Open saved tests →
            </Anchor>
          </Stack>
        </Card>
      )}
      {runs.data?.items.length === 0 && (
        <Stack gap="xs">
          <Text>
            {source === 'legacy'
              ? 'No earlier test activity found.'
              : 'No results in this view. Open a saved test, choose its build and run it.'}
          </Text>
          {source !== 'legacy' && (
            <Button
              variant="subtle"
              onClick={() => {
                setSource('legacy')
                setCursors([undefined])
              }}
              style={{ alignSelf: 'flex-start' }}
            >
              View earlier test activity
            </Button>
          )}
        </Stack>
      )}
      {runs.data?.items.map((item) =>
        item.run ? (
          <Stack key={item.id} gap={4}>
            <Text size="sm">Build: {item.build_label}</Text>
            <RunResult run={item.run} compact />
          </Stack>
        ) : (
          <SessionResult key={item.id} item={item} />
        ),
      )}
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
          disabled={!runs.data?.next_cursor}
          onClick={() => setCursors([...cursors, runs.data?.next_cursor ?? undefined])}
        >
          Next
        </Button>
      </Group>
    </Stack>
  )
}
