import { useEffect, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Link, useParams, useSearchParams } from 'react-router'
import { ArrowLeft, FileArchive, RotateCw } from 'lucide-react'
import { useWorkspace } from '@/hooks/use-workspace'
import { appQuery, buildQuery, buildsQuery, completeUpload, settingsQuery } from '@/api/setup'
import {
  PageHeading,
  ErrorNotice,
  LoadingPanel,
  formatBytes,
  formatDate,
} from '@/components/app/feedback'
import { RunPreview } from '@/components/app/run-preview'
import { ApkUpload } from '@/components/app/apk-upload'
import { EnvironmentCard } from '@/components/app/environment'
import { BuildDetail, ReadinessCard, ValidationBadge } from '@/components/app/build-status'
import {
  Anchor,
  Box,
  Button,
  Card,
  Group,
  Stack,
  Table,
  Text,
  Title,
  UnstyledButton,
} from '@mantine/core'
export function AppDetail() {
  const { app_id: appId = '' } = useParams()
  const { workspace } = useWorkspace()
  const app = useQuery(appQuery(appId))
  if (app.isError)
    return (
      <ErrorNotice
        error={app.error}
        retry={() => void app.refetch()}
        title="App could not be loaded"
      />
    )
  if (!app.data) return <LoadingPanel label="Loading app setup…" />
  if (app.data.organization_id !== workspace!.organization_id)
    return (
      <Card>
        <Text>This app belongs to a different workspace.</Text>
        <Button component={Link} to={`/apps?workspace=${workspace!.organization_id}`} mt="md">
          Back to workspace
        </Button>
      </Card>
    )
  return <AppDetailContent key={`${workspace!.organization_id}:${appId}`} appId={appId} />
}
function AppDetailContent({ appId }: { appId: string }) {
  const { href } = useWorkspace()
  const [params, setParams] = useSearchParams()
  const [cursors, setCursors] = useState<(string | undefined)[]>([undefined])
  const client = useQueryClient()
  const app = useQuery(appQuery(appId))
  const settings = useQuery(settingsQuery)
  const builds = useQuery(buildsQuery(appId, cursors.at(-1)))
  const selectedId = params.get('build') ?? builds.data?.items[0]?.id ?? ''
  const selected = useQuery({
    ...buildQuery(appId, selectedId),
    enabled: !!selectedId,
  })
  const validationState = selected.data?.validation.state
  useEffect(() => {
    if (validationState && validationState !== 'validating')
      void client.invalidateQueries({ queryKey: ['builds', appId] })
  }, [validationState, appId, client])
  const retry = useMutation({
    mutationFn: async () => {
      const latest = await client.fetchQuery({
        ...buildQuery(appId, selectedId),
        staleTime: 0,
      })
      return latest.can_retry_validation ? completeUpload(appId, latest.upload_id) : latest
    },
    onSuccess: (build) => {
      client.setQueryData(['build', appId, build.id], build)
      void client.invalidateQueries({ queryKey: ['builds', appId] })
    },
  })
  if (!app.data && app.isError)
    return (
      <ErrorNotice
        error={app.error}
        retry={() => void app.refetch()}
        title="App could not be loaded"
      />
    )
  if (!app.data) return <LoadingPanel label="Loading app setup…" />
  return (
    <>
      <Anchor
        component={Link}
        to={href('/apps')}
        size="xs"
        c="dimmed"
        mb="lg"
        display="inline-block"
      >
        <Group gap="xs">
          <ArrowLeft size={14} />
          All apps
        </Group>
      </Anchor>
      <PageHeading
        eyebrow="App setup / Android"
        title={app.data.name}
        description={app.data.android_package}
        identifier
      />
      {app.isError && (
        <Box mb="lg">
          <ErrorNotice
            error={app.error}
            retry={() => void app.refetch()}
            title="Showing the last saved app details"
          />
        </Box>
      )}
      <Button component={Link} to={href(`/apps/${appId}/try`)} mb="lg" size="md">
        Open app & try a task
      </Button>
      {selected.data && <RunPreview key={selectedId} appId={appId} buildId={selectedId} />}
      <div className="detail-layout">
        <Stack gap="xl" miw={0}>
          {settings.data ? (
            <ApkUpload
              key={appId}
              appId={appId}
              maxBytes={settings.data.max_apk_bytes}
              multipartEnabled={!!settings.data.multipart}
            />
          ) : settings.isError ? (
            <ErrorNotice
              error={settings.error}
              retry={() => void settings.refetch()}
              title="Upload limits could not be loaded"
            />
          ) : (
            <LoadingPanel label="Loading upload settings…" />
          )}
          <Box component="section" aria-labelledby="build-history-title">
            <Group justify="space-between" mb="md">
              <Title id="build-history-title" order={2} size="h5">
                Build history
              </Title>
              <Button
                variant="subtle"
                leftSection={<RotateCw size={14} />}
                disabled={builds.isFetching}
                onClick={() => {
                  void builds.refetch()
                  if (selectedId) void selected.refetch()
                }}
              >
                Refresh
              </Button>
            </Group>
            {builds.isError && (
              <ErrorNotice
                error={builds.error}
                retry={() => void builds.refetch()}
                title="Build history could not be refreshed"
              />
            )}
            {builds.isPending ? (
              <LoadingPanel label="Loading builds…" />
            ) : builds.data?.items.length === 0 ? (
              <Card py={40}>
                <Stack align="center" gap="sm" ta="center">
                  <FileArchive size={28} />
                  <Title order={3} size="h6">
                    No builds yet.
                  </Title>
                  <Text size="xs" c="dimmed">
                    Upload your first APK to see its identity and validation results.
                  </Text>
                </Stack>
              </Card>
            ) : (
              builds.data && (
                <Card p={0}>
                  <Table.ScrollContainer minWidth={460}>
                    <Table verticalSpacing="sm" horizontalSpacing="lg" highlightOnHover>
                      <Table.Thead>
                        <Table.Tr>
                          <Table.Th>Build</Table.Th>
                          <Table.Th>Status</Table.Th>
                          <Table.Th ta="right">Size</Table.Th>
                        </Table.Tr>
                      </Table.Thead>
                      <Table.Tbody>
                        {builds.data.items.map((build) => (
                          <Table.Tr
                            key={build.id}
                            className="build-row"
                            data-selected={selectedId === build.id || undefined}
                          >
                            <Table.Td maw={240}>
                              <UnstyledButton
                                w="100%"
                                py={4}
                                aria-pressed={selectedId === build.id}
                                onClick={() => {
                                  const next = new URLSearchParams(params)
                                  next.set('build', build.id)
                                  setParams(next)
                                }}
                              >
                                <Text size="sm" fw={500} c="forest.7" truncate>
                                  {build.metadata?.version_name
                                    ? `v${build.metadata.version_name}`
                                    : build.original_filename}
                                </Text>
                                <Text size="xs" c="dimmed" mt={4}>
                                  {formatDate(build.created_at)}
                                </Text>
                              </UnstyledButton>
                            </Table.Td>
                            <Table.Td>
                              <ValidationBadge state={build.validation.state} />
                            </Table.Td>
                            <Table.Td ta="right">
                              <Text size="xs" c="dimmed">
                                {formatBytes(build.byte_size)}
                              </Text>
                            </Table.Td>
                          </Table.Tr>
                        ))}
                      </Table.Tbody>
                    </Table>
                  </Table.ScrollContainer>
                </Card>
              )
            )}
            <Group justify="flex-end" mt="sm" gap="xs">
              {cursors.length > 1 && (
                <Button variant="outline" onClick={() => setCursors((c) => c.slice(0, -1))}>
                  Previous builds
                </Button>
              )}
              {builds.data?.next_cursor && (
                <Button
                  variant="outline"
                  onClick={() => setCursors((c) => [...c, builds.data?.next_cursor ?? undefined])}
                >
                  Older builds
                </Button>
              )}
            </Group>
          </Box>
          {selectedId && selected.isPending && <LoadingPanel label="Loading build validation…" />}
          {selected.isError && (
            <ErrorNotice
              error={selected.error}
              retry={() => void selected.refetch()}
              title="Build status could not be refreshed"
            />
          )}
          {selected.data && (
            <>
              <BuildDetail build={selected.data} />
              {selected.data.can_retry_validation && (
                <Stack gap="sm" align="flex-start">
                  <Button
                    variant="outline"
                    disabled={retry.isPending}
                    onClick={() => retry.mutate()}
                    leftSection={<RotateCw size={16} />}
                  >
                    {retry.isPending ? 'Retrying validation…' : 'Retry validation'}
                  </Button>
                  {retry.isError && (
                    <ErrorNotice
                      error={retry.error}
                      retry={() => void selected.refetch()}
                      title="Check saved status before retrying"
                    />
                  )}
                </Stack>
              )}
            </>
          )}
        </Stack>
        <Stack component="aside" gap="lg">
          <EnvironmentCard app={app.data} />
          <ReadinessCard readiness={selected.data?.readiness ?? app.data.readiness} />
        </Stack>
      </div>
    </>
  )
}
