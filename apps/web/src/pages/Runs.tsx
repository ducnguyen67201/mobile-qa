import { useState } from 'react'
import { Button, Card, Group, Select, Stack, Text } from '@mantine/core'
import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router'
import { appsQuery } from '@/api/setup'
import { runsQuery } from '@/api/runs'
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
  const runs = useQuery(runsQuery(workspaceId, selected, cursors.at(-1)))
  return (
    <Stack>
      <PageHeading
        eyebrow="Execution"
        title="Runs"
        description="Saved release checks and their evidence."
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
      {!selected && <Text>Add an app and upload a build to configure a release check.</Text>}
      {selected && runs.isPending && <LoadingPanel label="Loading runs…" />}
      {runs.isError && <ErrorNotice error={runs.error} retry={() => void runs.refetch()} />}
      {runs.data?.items.length === 0 && (
        <Text>No runs yet. Start a release check from the app’s build page.</Text>
      )}
      {runs.data?.items.map((r) => (
        <Card key={r.id} component={Link} to={href(`/runs/${r.id}`)} withBorder>
          <Text fw={600}>{r.summary}</Text>
          <Text size="sm">
            {r.state} · {r.manifest.profile.driver === 'fake' ? 'Simulated' : 'Android emulator'} ·{' '}
            {r.created_at}
          </Text>
          <Text size="xs">{r.id}</Text>
        </Card>
      ))}
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
