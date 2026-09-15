import { useEffect, useState } from 'react'
import {
  Alert,
  Badge,
  Button,
  Card,
  Group,
  Modal,
  Select,
  Stack,
  Text,
  Textarea,
  Title,
} from '@mantine/core'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Link, useNavigate, useParams } from 'react-router'
import { ArrowLeft } from 'lucide-react'
import {
  archiveLibraryEntry,
  defaultPlanQuery,
  forkLibraryDraft,
  libraryDraftQuery,
  libraryEntryQuery,
  libraryErrorDetails,
  libraryHistoryQuery,
  libraryKey,
  libraryOptionsQuery,
  libraryVersionQuery,
  reviewLibraryVersion,
  setDefaultPlan,
} from '@/api/test-library'
import type {
  ApprovalPurpose,
  ArchiveLibraryEntryRequest,
  ForkLibraryDraftRequest,
  LibraryReviewDecision,
  LibraryEntryResponse,
  LibraryVersionResponse,
  ReviewLibraryVersionRequest,
  SetDefaultPlanRequest,
} from '@/api/generated/types.gen'
import { buildsQuery } from '@/api/setup'
import { useWorkspace } from '@/hooks/use-workspace'
import { useMounted } from '@/hooks/use-mounted'
import { ErrorNotice, formatDate, LoadingPanel, PageHeading } from '@/components/app/feedback'
import { RunPreview } from '@/components/app/run-preview'
import { DraftEditor } from '@/components/test-library/draft-editor'
import {
  Coverage,
  DefinitionSummary,
  LibraryIssues,
  ReviewBadge,
} from '@/components/test-library/library-presentation'

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
/** Once a draft editor mounts, background lifecycle changes must not discard its local work. */
function EntryContent({ entry, versionId }: { entry: LibraryEntryResponse; versionId: string }) {
  const { href } = useWorkspace()
  const [startedWithDraft, keepDraftSession] = useState(!versionId && entry.draft_version != null)
  useEffect(() => {
    if (!versionId && entry.draft_version != null) keepDraftSession(true)
  }, [versionId, entry.draft_version])
  if (!versionId && (startedWithDraft || entry.draft_version != null))
    return (
      <Stack>
        {entry.draft_version == null && (
          <Alert color="orange" title="This draft was submitted elsewhere">
            <Stack gap="sm">
              <Text size="sm">
                Your local editor is preserved. This version can no longer be saved. Review the
                submitted version before starting a new draft.
              </Text>
              {entry.latest_version_id && (
                <Button
                  component={Link}
                  to={href(
                    `/tests/${entry.app_id}/${entry.id}/versions/${entry.latest_version_id}`,
                  )}
                  variant="outline"
                  w="fit-content"
                >
                  Open submitted version
                </Button>
              )}
            </Stack>
          </Alert>
        )}
        <DraftWorkspace
          appId={entry.app_id}
          entryId={entry.id}
          available={entry.draft_version != null}
        />
      </Stack>
    )
  const selectedVersion = versionId || entry.latest_version_id
  return selectedVersion ? (
    <VersionWorkspace appId={entry.app_id} entryId={entry.id} versionId={selectedVersion} />
  ) : (
    <Text>No draft or submitted version is available.</Text>
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
              ? 'The entry becomes available again. Frozen reviews and dependencies are still checked before new runs.'
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
      {draft.data && <DraftEditor initial={draft.data} available={available} />}
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
          Submitted versions are frozen. Each new draft gets a new version and fresh reviews.
        </Text>
        {history.isPending && <LoadingPanel label="Loading version history…" />}
        {history.isError && (
          <ErrorNotice error={history.error} retry={() => void history.refetch()} />
        )}
        {history.data && (
          <>
            {!history.data.items.length && (
              <Text size="sm" c="dimmed">
                No versions have been submitted for review yet.
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
                <ReviewBadge state={item.review_state} />
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
    <FrozenVersion value={version.data} refresh={() => void version.refetch()} />
  )
}
function FrozenVersion({ value, refresh }: { value: LibraryVersionResponse; refresh: () => void }) {
  const { workspaceId = '', href } = useWorkspace()
  const appId = value.entry.app_id
  const entryId = value.entry.id
  const navigate = useNavigate()
  const client = useQueryClient()
  const mounted = useMounted()
  const invalidate = () => {
    void client.invalidateQueries({ queryKey: libraryKey(workspaceId, appId) })
    void client.invalidateQueries({ queryKey: ['execution-plan', workspaceId, appId] })
  }
  const fork = useMutation({
    mutationFn: (body: ForkLibraryDraftRequest) => forkLibraryDraft(appId, entryId, body),
    onSuccess: () => {
      if (!mounted.current) return
      invalidate()
      void navigate(href(`/tests/${appId}/${entryId}`))
    },
  })

  const options = useQuery(libraryOptionsQuery(workspaceId, appId))
  const frozen = value.version
  return (
    <Stack gap="lg">
      <Card bg="var(--mantine-color-forest-0)">
        <Stack>
          <Group justify="space-between">
            <Group>
              <Badge variant="outline">Version {frozen.definition.content.version}</Badge>
              <ReviewBadge state={value.review_state} />
            </Group>
            <Group>
              {value.entry.draft_version != null ? (
                <Button component={Link} to={href(`/tests/${appId}/${entryId}`)} variant="default">
                  Open draft v{value.entry.draft_version}
                </Button>
              ) : (
                <Button
                  variant="default"
                  disabled={!value.entry.capabilities.can_edit || !!value.entry.archived_at}
                  loading={fork.isPending}
                  onClick={() =>
                    fork.mutate({
                      mutation_id: crypto.randomUUID(),
                      expected_revision: value.entry.revision,
                      source_version_id: frozen.id,
                    })
                  }
                >
                  New draft version
                </Button>
              )}
            </Group>
          </Group>
          <Title order={2}>{frozen.definition.content.title}</Title>
          <Text size="sm">
            This exact content is frozen. Changes belong in a new draft; existing plans and reports
            keep their pinned version.
          </Text>
          <Text size="xs" c="dimmed" className="identifier">
            Content hash · {frozen.content_hash}
          </Text>
        </Stack>
      </Card>
      {fork.isError && (
        <ErrorNotice
          error={fork.error}
          retry={() => {
            if (fork.variables) fork.mutate(fork.variables)
          }}
        />
      )}
      {libraryErrorDetails(fork.error)?.kind === 'stale_revision' && (
        <Button variant="light" w="fit-content" onClick={refresh}>
          Reload current revision
        </Button>
      )}
      <DefinitionSummary definition={frozen.definition} options={options.data} />
      {frozen.definition.kind !== 'case' && <Coverage value={value.coverage} />}
      <LibraryIssues issues={value.issues} title="Readiness needs attention" />
      <Card>
        <Stack>
          <Title order={2} size="h3">
            Two reviews, one exact version
          </Title>
          <Text size="sm" c="dimmed">
            Business review confirms the intended behavior. Executability review confirms the runner
            can verify it. Both must approve this content hash.
          </Text>
          {(['business', 'executability'] as const).map((purpose) => (
            <ReviewPurpose
              key={`${purpose}:${value.entry.revision}`}
              value={value}
              purpose={purpose}
              changed={invalidate}
              refresh={refresh}
            />
          ))}
          {value.review_state === 'needs_input' || value.review_state === 'rejected' ? (
            <Alert color="orange">
              This candidate is closed for review. Create a new draft version to address the
              feedback; its approvals start fresh.
            </Alert>
          ) : null}
          {!!value.review_events.length && (
            <Stack gap="xs">
              <Text fw={600} size="sm">
                Review activity
              </Text>
              {value.review_events.map((event) => (
                <Stack key={event.id} gap={2}>
                  <Text size="sm">
                    {event.purpose === 'business' ? 'Business' : 'Executability'} ·{' '}
                    {event.decision === 'approve'
                      ? 'Approved'
                      : event.decision === 'reject'
                        ? 'Rejected'
                        : 'Changes requested'}{' '}
                    · {formatDate(event.created_at)}
                  </Text>
                  {event.reason && <Text size="sm">{event.reason}</Text>}
                  <Text size="xs" c="dimmed">
                    Reviewer {event.actor_name} · {event.actor_id}
                  </Text>
                </Stack>
              ))}
            </Stack>
          )}
        </Stack>
      </Card>
      {frozen.definition.kind === 'plan' && <PlanRun value={value} />}
    </Stack>
  )
}
function ReviewPurpose({
  value,
  purpose,
  changed,
  refresh,
}: {
  value: LibraryVersionResponse
  purpose: ApprovalPurpose
  changed: () => void
  refresh: () => void
}) {
  const mounted = useMounted()
  const [decision, setDecision] = useState<LibraryReviewDecision>('approve')
  const [reason, setReason] = useState('')
  const approval = value.version.approvals.find(
    (a) => a.purpose === purpose && a.content_hash === value.version.content_hash,
  )
  const allowed =
    purpose === 'business'
      ? value.entry.capabilities.can_review_business
      : value.entry.capabilities.can_review_executability
  const active = value.review_state === 'in_review' && !value.entry.archived_at && !approval
  const review = useMutation({
    mutationFn: (body: ReviewLibraryVersionRequest) =>
      reviewLibraryVersion(value.entry.app_id, value.entry.id, value.version.id, body),
    onSuccess: () => {
      if (mounted.current) changed()
    },
  })
  const details = libraryErrorDetails(review.error)
  return (
    <Card padding="md">
      <Stack gap="sm">
        <Group justify="space-between">
          <Text fw={600}>
            {purpose === 'business' ? 'Business review' : 'Executability review'}
          </Text>
          <Badge color={approval ? 'green' : 'gray'}>
            {approval
              ? 'Approved'
              : value.review_state === 'needs_input' || value.review_state === 'rejected'
                ? 'Closed'
                : 'Awaiting review'}
          </Badge>
        </Group>
        {approval && (
          <Text size="xs" c="dimmed">
            {value.review_state === 'needs_input' || value.review_state === 'rejected'
              ? 'Historical approval'
              : 'Approved'}{' '}
            by{' '}
            {value.review_events.find(
              (event) =>
                event.actor_id === approval.actor_id &&
                event.purpose === purpose &&
                event.decision === 'approve',
            )?.actor_name ?? approval.actor_id}{' '}
            · {formatDate(approval.approved_at)}
          </Text>
        )}
        {!allowed && !approval && (
          <Text size="sm" c="dimmed">
            An explicitly authorized {purpose === 'business' ? 'business' : 'executability'}{' '}
            reviewer must complete this step. App access alone does not grant review authority.
          </Text>
        )}
        {allowed && active && (
          <>
            <Select
              label={`${purpose === 'business' ? 'Business' : 'Executability'} decision`}
              allowDeselect={false}
              data={[
                { value: 'approve', label: 'Approve this exact version' },
                { value: 'needs_input', label: 'Request changes' },
                { value: 'reject', label: 'Reject this version' },
              ]}
              value={decision}
              disabled={review.isPending}
              onChange={(v) => {
                if (v === 'approve' || v === 'needs_input' || v === 'reject') {
                  setDecision(v)
                  review.reset()
                }
              }}
            />
            <Textarea
              label={`${purpose === 'business' ? 'Business' : 'Executability'} review note`}
              description={
                decision === 'approve'
                  ? 'Optional context for the author.'
                  : 'Explain what needs to change. A reason is required.'
              }
              required={decision !== 'approve'}
              minRows={2}
              maxLength={2000}
              disabled={review.isPending}
              value={reason}
              onChange={(e) => {
                setReason(e.currentTarget.value)
                review.reset()
              }}
            />
            <Button
              w="fit-content"
              loading={review.isPending}
              disabled={decision !== 'approve' && !reason.trim()}
              onClick={() =>
                review.mutate({
                  mutation_id: crypto.randomUUID(),
                  expected_revision: value.entry.revision,
                  content_hash: value.version.content_hash,
                  purpose,
                  decision,
                  reason: reason.trim() || null,
                })
              }
            >
              {decision === 'approve'
                ? `Approve ${purpose === 'business' ? 'business' : 'executability'}`
                : decision === 'needs_input'
                  ? `Request ${purpose} changes`
                  : `Reject ${purpose} review`}
            </Button>
          </>
        )}
        {review.isError && (
          <ErrorNotice
            error={review.error}
            retry={
              details?.kind === 'stale_revision'
                ? undefined
                : () => {
                    if (review.variables) review.mutate(review.variables)
                  }
            }
          />
        )}
        {details?.kind === 'stale_revision' && (
          <Button variant="light" w="fit-content" onClick={refresh}>
            Reload reviews before deciding
          </Button>
        )}
        {details?.kind === 'validation' && (
          <LibraryIssues issues={details.issues} title="This review needs attention" />
        )}
      </Stack>
    </Card>
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
            The build is selected at run time. This plan’s reviewed case versions stay pinned.
          </Text>
          {value.review_state !== 'approved' && (
            <Alert color="gray">
              Complete both reviews before setting this plan as default or running it.
            </Alert>
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
                value.review_state !== 'approved' ||
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
