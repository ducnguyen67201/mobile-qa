import { useState } from 'react'
import { Alert, Badge, Button, Card, Group, Modal, Select, Stack, Text, Title } from '@mantine/core'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Link, useParams } from 'react-router'
import { ArrowLeft } from 'lucide-react'
import {
  archiveLibraryEntry,
  defaultPlanQuery,
  libraryDraftQuery,
  libraryEntryQuery,
  libraryErrorDetails,
  libraryHistoryQuery,
  libraryKey,
  libraryOptionsQuery,
  libraryVersionQuery,
  setDefaultPlan,
} from '@/api/test-library'
import type {
  ArchiveLibraryEntryRequest,
  LibraryEntryResponse,
  LibraryVersionResponse,
  SetDefaultPlanRequest,
} from '@/api/generated/types.gen'
import { buildsQuery } from '@/api/setup'
import { useWorkspace } from '@/hooks/use-workspace'
import { useMounted } from '@/hooks/use-mounted'
import { ErrorNotice, LoadingPanel, PageHeading } from '@/components/app/feedback'
import { RunPreview } from '@/components/app/run-preview'
import { SavedSuiteRun } from '@/components/runs/saved-suite-run'
import { DraftEditor } from '@/components/test-library/draft-editor'
import {
  Coverage,
  DefinitionSummary,
  LibraryIssues,
} from '@/components/test-library/library-presentation'
import { LibraryCoverageFlow } from '@/components/test-library/coverage-flow'

export function TestLibraryDetail() {
  const { workspaceId = '' } = useWorkspace()
  const { app_id = '', entry_id = '', version_id = '' } = useParams()
  return (
    <EntryWorkspace
      key={`${workspaceId}:${app_id}:${entry_id}:${version_id}`}
      appId={app_id}
      entryId={entry_id}
      versionId={version_id}
    />
  )
}
function EntryWorkspace({
  appId,
  entryId,
  versionId,
}: {
  appId: string
  entryId: string
  versionId: string
}) {
  const { workspaceId = '', href } = useWorkspace()
  const entry = useQuery(libraryEntryQuery(workspaceId, appId, entryId))
  return (
    <Stack gap="lg">
      <Button
        component={Link}
        to={href(`/tests?app=${appId}&kind=${entry.data?.kind ?? 'case'}`)}
        variant="subtle"
        leftSection={<ArrowLeft size={16} />}
        w="fit-content"
      >
        Back to test library
      </Button>
      {entry.isPending && <LoadingPanel label="Loading test entry…" />}
      {entry.isError && <ErrorNotice error={entry.error} retry={() => void entry.refetch()} />}
      {entry.data && (
        <>
          {entry.data.kind === 'case' ? (
            <Group justify="space-between" gap="sm">
              <Group gap="xs">
                <Title order={1} size="h3">
                  {entry.data.title || 'Untitled task'}
                </Title>
                {entry.data.ai_generated && (
                  <Badge color="grape" variant="light">
                    AI generated
                  </Badge>
                )}
              </Group>
              <EntryLifecycle entry={entry.data} />
            </Group>
          ) : (
            <PageHeading
              eyebrow={entry.data.kind === 'plan' ? 'Release plan' : 'Reusable coverage'}
              title={entry.data.title || 'Untitled draft'}
              description={entry.data.key}
            />
          )}
          {entry.data.kind !== 'case' && <EntryLifecycle entry={entry.data} />}
          {entry.data.archived_at && (
            <Alert color="orange" title="Archived entry">
              History is preserved. This entry and plans that depend on it cannot enter new runs
              until an operator restores it.
            </Alert>
          )}
          <EntryContent entry={entry.data} versionId={versionId} />
          <VersionHistory appId={appId} entryId={entryId} />
        </>
      )}
    </Stack>
  )
}
/** Current entries remain editable; explicit historical links keep their immutable content. */
function EntryContent({ entry, versionId }: { entry: LibraryEntryResponse; versionId: string }) {
  return versionId ? (
    <VersionWorkspace appId={entry.app_id} entryId={entry.id} versionId={versionId} />
  ) : (
    <DraftWorkspace appId={entry.app_id} entryId={entry.id} available={!entry.archived_at} />
  )
}
function EntryLifecycle({ entry }: { entry: LibraryEntryResponse }) {
  const { workspaceId = '' } = useWorkspace()
  const client = useQueryClient()
  const mounted = useMounted()
  const [opened, setOpened] = useState(false)
  const archive = useMutation({
    mutationFn: (body: ArchiveLibraryEntryRequest) =>
      archiveLibraryEntry(entry.app_id, entry.id, body),
    onSuccess: () => {
      if (!mounted.current) return
      setOpened(false)
      void client.invalidateQueries({ queryKey: libraryKey(workspaceId, entry.app_id) })
      void client.invalidateQueries({ queryKey: ['execution-plan', workspaceId, entry.app_id] })
    },
  })
  if (!entry.capabilities.can_archive) return null
  return (
    <>
      <Button variant="subtle" color="orange" w="fit-content" onClick={() => setOpened(true)}>
        {entry.archived_at ? 'Restore entry' : 'Archive entry'}
      </Button>
      <Modal
        opened={opened}
        onClose={() => setOpened(false)}
        title={entry.archived_at ? 'Restore this entry?' : 'Archive this entry?'}
        centered
      >
        <Stack>
          <Text size="sm">
            {entry.archived_at
              ? 'The entry becomes available again. Saved dependencies are still checked before new runs.'
              : 'This blocks editing and new runs that depend on this entry. Existing queued runs and saved reports are unchanged. An operator can restore it later.'}
          </Text>
          {archive.isError && (
            <ErrorNotice
              error={archive.error}
              retry={
                libraryErrorDetails(archive.error)?.kind === 'stale_revision'
                  ? () => {
                      void client.invalidateQueries({
                        queryKey: libraryKey(workspaceId, entry.app_id),
                      })
                      archive.reset()
                    }
                  : () => {
                      if (archive.variables) archive.mutate(archive.variables)
                    }
              }
            />
          )}
          <Group justify="flex-end">
            <Button variant="default" onClick={() => setOpened(false)}>
              Keep current state
            </Button>
            <Button
              color="orange"
              loading={archive.isPending}
              onClick={() =>
                archive.mutate({
                  mutation_id: crypto.randomUUID(),
                  expected_revision: entry.revision,
                  archived: !entry.archived_at,
                })
              }
            >
              {entry.archived_at ? 'Restore entry' : 'Archive entry'}
            </Button>
          </Group>
        </Stack>
      </Modal>
    </>
  )
}
function DraftWorkspace({
  appId,
  entryId,
  available,
}: {
  appId: string
  entryId: string
  available: boolean
}) {
  const { workspaceId = '' } = useWorkspace()
  const draft = useQuery(libraryDraftQuery(workspaceId, appId, entryId))
  return (
    <Stack>
      {draft.isPending && <LoadingPanel label="Loading saved draft…" />}
      {draft.isError && (
        <ErrorNotice
          title={
            draft.data
              ? 'Saved status could not be refreshed. Your editor is preserved.'
              : undefined
          }
          error={draft.error}
          retry={() => void draft.refetch()}
        />
      )}
      {draft.data && (
        <DraftEditor
          initial={draft.data}
          available={available}
          renderSaved={(saved) =>
            saved.definition.kind === 'plan' && saved.saved_version_id ? (
              <SavedPlan appId={appId} entryId={entryId} versionId={saved.saved_version_id} />
            ) : null
          }
        />
      )}
    </Stack>
  )
}
function VersionHistory({ appId, entryId }: { appId: string; entryId: string }) {
  const { workspaceId = '', href } = useWorkspace()
  const [cursor, setCursor] = useState<string | undefined>()
  const history = useQuery(libraryHistoryQuery(workspaceId, appId, entryId, cursor))
  return (
    <Card>
      <Stack>
        <Title order={2} size="h3">
          Version history
        </Title>
        <Text size="sm" c="dimmed">
          Saved versions are preserved. Editing and saving creates a new version automatically.
        </Text>
        {history.isPending && <LoadingPanel label="Loading version history…" />}
        {history.isError && (
          <ErrorNotice error={history.error} retry={() => void history.refetch()} />
        )}
        {history.data && (
          <>
            {!history.data.items.length && (
              <Text size="sm" c="dimmed">
                Complete the test setup and save to create its first reusable version.
              </Text>
            )}
            {history.data.items.map((item) => (
              <Group key={item.version.id} justify="space-between">
                <Button
                  component={Link}
                  to={href(`/tests/${appId}/${entryId}/versions/${item.version.id}`)}
                  variant="subtle"
                >
                  v{item.version.definition.content.version} ·{' '}
                  {item.version.definition.content.title}
                </Button>
                <Badge color="gray">Saved</Badge>
              </Group>
            ))}
            <Group justify="space-between">
              {cursor ? (
                <Button variant="subtle" onClick={() => setCursor(undefined)}>
                  Newest versions
                </Button>
              ) : (
                <span />
              )}
              {history.data.next_cursor && (
                <Button
                  variant="subtle"
                  onClick={() => setCursor(history.data.next_cursor ?? undefined)}
                >
                  Older versions
                </Button>
              )}
            </Group>
          </>
        )}
      </Stack>
    </Card>
  )
}
function VersionWorkspace({
  appId,
  entryId,
  versionId,
}: {
  appId: string
  entryId: string
  versionId: string
}) {
  const { workspaceId = '' } = useWorkspace()
  const version = useQuery(libraryVersionQuery(workspaceId, appId, entryId, versionId))
  return version.isPending ? (
    <LoadingPanel label="Loading frozen version…" />
  ) : version.isError ? (
    <ErrorNotice error={version.error} retry={() => void version.refetch()} />
  ) : (
    <FrozenVersion value={version.data} />
  )
}
function SavedPlan({
  appId,
  entryId,
  versionId,
}: {
  appId: string
  entryId: string
  versionId: string
}) {
  const { workspaceId = '' } = useWorkspace()
  const version = useQuery(libraryVersionQuery(workspaceId, appId, entryId, versionId))
  return version.data ? (
    <PlanRun value={version.data} />
  ) : version.isError ? (
    <ErrorNotice error={version.error} retry={() => void version.refetch()} />
  ) : (
    <LoadingPanel label="Loading saved plan…" />
  )
}
function FrozenVersion({ value }: { value: LibraryVersionResponse }) {
  const { workspaceId = '', href } = useWorkspace()
  const options = useQuery(libraryOptionsQuery(workspaceId, value.entry.app_id))
  const frozen = value.version
  return (
    <Stack gap="lg">
      <Card bg="var(--mantine-color-forest-0)">
        <Stack>
          <Group justify="space-between">
            <Badge variant="outline">Saved version {frozen.definition.content.version}</Badge>
            <Button
              component={Link}
              to={href(`/tests/${value.entry.app_id}/${value.entry.id}`)}
              variant="default"
            >
              Return to current test
            </Button>
          </Group>
          <Title order={2}>{frozen.definition.content.title}</Title>
          <Text size="sm">
            This version is preserved. Edit the current test to save changes; existing plans and
            reports keep their pinned version.
          </Text>
          <Text size="xs" c="dimmed" className="identifier">
            Content hash · {frozen.content_hash}
          </Text>
        </Stack>
      </Card>
      {frozen.definition.kind !== 'case' && options.data && (
        <LibraryCoverageFlow
          definition={frozen.definition}
          options={options.data}
          frozen
          runControl={
            frozen.definition.kind === 'suite' ? (
              <SavedSuiteRun
                appId={value.entry.app_id}
                versionId={frozen.id}
                dirty={false}
                profiles={options.data.profiles}
              />
            ) : undefined
          }
        />
      )}
      <DefinitionSummary definition={frozen.definition} options={options.data} />
      {frozen.definition.kind !== 'case' && <Coverage value={value.coverage} />}
      <LibraryIssues issues={value.issues} title="Setup needs attention" />
      {frozen.definition.kind === 'plan' && <PlanRun value={value} />}
    </Stack>
  )
}
function PlanRun({ value }: { value: LibraryVersionResponse }) {
  const { workspaceId = '', href } = useWorkspace()
  const appId = value.entry.app_id
  const mounted = useMounted()
  const client = useQueryClient()
  const defaultPlan = useQuery(defaultPlanQuery(workspaceId, appId))
  const builds = useQuery(buildsQuery(appId))
  const [buildId, setBuildId] = useState<string | null>(null)
  const setDefault = useMutation({
    mutationFn: (body: SetDefaultPlanRequest) => setDefaultPlan(appId, body),
    onSuccess: () => {
      if (!mounted.current) return
      void client.invalidateQueries({ queryKey: libraryKey(workspaceId, appId) })
      void client.invalidateQueries({ queryKey: ['execution-plan', workspaceId, appId] })
    },
  })
  const selectedBuild = buildId ?? builds.data?.items[0]?.id ?? ''
  const isDefault = defaultPlan.data?.plan_version_id === value.version.id
  return (
    <Stack>
      <Card>
        <Stack>
          <Group justify="space-between">
            <Title order={2} size="h3">
              Ready for the next build
            </Title>
            {isDefault && <Badge>Default release plan</Badge>}
          </Group>
          <Text size="sm" c="dimmed">
            The build is selected at run time. This plan’s saved case versions stay pinned.
          </Text>
          {!isDefault && defaultPlan.data?.plan_version_id && (
            <Text size="sm" c="dimmed">
              The default still uses another saved version. Saving does not change it.
            </Text>
          )}
          {defaultPlan.isError && (
            <ErrorNotice error={defaultPlan.error} retry={() => void defaultPlan.refetch()} />
          )}
          {value.entry.capabilities.can_set_default && !isDefault && (
            <Button
              w="fit-content"
              variant="light"
              disabled={
                !defaultPlan.data ||
                !!value.entry.archived_at ||
                !!value.issues.length ||
                !!value.coverage.issues.length
              }
              loading={setDefault.isPending}
              onClick={() => {
                if (defaultPlan.data)
                  setDefault.mutate({
                    mutation_id: crypto.randomUUID(),
                    expected_revision: defaultPlan.data.revision,
                    plan_version_id: value.version.id,
                  })
              }}
            >
              Set as default release plan
            </Button>
          )}
          {setDefault.isError && (
            <ErrorNotice
              error={setDefault.error}
              retry={
                libraryErrorDetails(setDefault.error)?.kind === 'stale_revision'
                  ? () => void defaultPlan.refetch()
                  : () => {
                      if (setDefault.variables) setDefault.mutate(setDefault.variables)
                    }
              }
            />
          )}
          {builds.isPending && <LoadingPanel label="Loading builds…" />}
          {builds.isError && (
            <ErrorNotice error={builds.error} retry={() => void builds.refetch()} />
          )}
          {!!builds.data?.items.length && (
            <Select
              label="Build to test"
              allowDeselect={false}
              data={builds.data.items.map((b) => ({ value: b.id, label: b.original_filename }))}
              value={selectedBuild}
              onChange={setBuildId}
            />
          )}
          {builds.data && !builds.data.items.length && (
            <Text size="sm">
              Your plan is saved.{' '}
              <Button component={Link} to={href(`/apps/${appId}`)} variant="subtle" size="xs">
                Upload a build
              </Button>{' '}
              when you are ready to run it.
            </Text>
          )}
        </Stack>
      </Card>
      {selectedBuild && (
        <RunPreview
          key={`${value.version.id}:${selectedBuild}`}
          appId={appId}
          buildId={selectedBuild}
          planVersionId={value.version.id}
        />
      )}
    </Stack>
  )
}
