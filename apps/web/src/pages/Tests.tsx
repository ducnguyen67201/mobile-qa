import { useState } from 'react'
import { Select, Stack, Text } from '@mantine/core'
import { useQuery } from '@tanstack/react-query'
import { appsQuery, buildsQuery } from '@/api/setup'
import { useWorkspace } from '@/hooks/use-workspace'
import { RunPreview } from '@/components/app/run-preview'
import { ErrorNotice, PageHeading } from '@/components/app/feedback'
export function Tests() {
  const { workspaceId = '' } = useWorkspace()
  return <WorkspaceTests key={workspaceId} />
}
function WorkspaceTests() {
  const { workspaceId = '' } = useWorkspace()
  const apps = useQuery(appsQuery(workspaceId))
  const [app, setApp] = useState<string | null>(null)
  const [build, setBuild] = useState<string | null>(null)
  const appId = app ?? apps.data?.items[0]?.id ?? ''
  const builds = useQuery({ ...buildsQuery(appId), enabled: !!appId })
  const buildId = build ?? builds.data?.items[0]?.id ?? ''
  return (
    <Stack>
      <PageHeading
        eyebrow="Approved coverage"
        title="Tests"
        description="Review the actions and expected checks in your operator-managed release plan."
      />
      {apps.isError && <ErrorNotice error={apps.error} retry={() => void apps.refetch()} />}
      <Select
        label="App"
        data={apps.data?.items.map((a) => ({ value: a.id, label: a.name })) ?? []}
        value={appId || null}
        onChange={(v) => {
          setApp(v)
          setBuild(null)
        }}
      />
      {builds.isError && <ErrorNotice error={builds.error} retry={() => void builds.refetch()} />}
      <Select
        label="Build"
        data={builds.data?.items.map((b) => ({ value: b.id, label: b.original_filename })) ?? []}
        value={buildId || null}
        onChange={setBuild}
      />
      {buildId ? (
        <RunPreview key={`${appId}:${buildId}`} appId={appId} buildId={buildId} readOnly />
      ) : (
        <Text>Upload a build to preview compatible approved tests.</Text>
      )}
    </Stack>
  )
}
