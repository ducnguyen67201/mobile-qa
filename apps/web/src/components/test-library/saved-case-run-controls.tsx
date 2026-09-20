import { useEffect, useMemo, useRef, useState } from 'react'
import { Alert, Badge, Button, Card, Group, Select, Stack, Text, Title } from '@mantine/core'
import { type QueryClient, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Link } from 'react-router'
import { baselineCandidatesQuery, createRun, runQuery, savedCasePreviewQuery } from '@/api/runs'
import { buildsQuery } from '@/api/setup'
import { phoneOptionsQuery, phoneQuery, stopPhone } from '@/api/task-sessions'
import { ErrorNotice, LoadingPanel } from '@/components/app/feedback'
import { useSession } from '@/components/app/session'
import { useWorkspace } from '@/hooks/use-workspace'

const NONE = 'none'

function stored(kind: 'localStorage' | 'sessionStorage', key: string): string | null {
  try {
    const storage = globalThis[kind] as Partial<Storage> | undefined
    return typeof storage?.getItem === 'function' ? storage.getItem(key) : null
  } catch {
    return null
  }
}

function store(kind: 'localStorage' | 'sessionStorage', key: string, value?: string): void {
  try {
    const storage = globalThis[kind] as Partial<Storage> | undefined
    if (value === undefined) storage?.removeItem?.(key)
    else storage?.setItem?.(key, value)
  } catch {
    // Private modes can deny storage; the in-memory identity below still makes retries safe.
  }
}

async function releasePreview(sessionId: string, client: QueryClient): Promise<void> {
  let session = await stopPhone(sessionId)
  for (let attempt = 0; attempt < 120 && session.state === 'stopping'; attempt += 1) {
    await new Promise((resolve) => window.setTimeout(resolve, 1_000))
    session = await client.fetchQuery({ ...phoneQuery(sessionId), staleTime: 0 })
  }
  if (session.state === 'quarantined') {
    throw new Error('Preview cleanup was quarantined. Recover the device before running this test.')
  }
  if (session.state !== 'closed') {
    throw new Error('Preview cleanup did not finish. Try again after the phone session closes.')
  }
}

export function SavedCaseRunControls({
  appId,
  savedVersionId,
  dirty,
  disabled,
  saveAndGetVersion,
}: {
  appId: string
  savedVersionId?: string | null
  dirty: boolean
  disabled: boolean
  saveAndGetVersion: () => Promise<string>
}) {
  const { workspaceId = '', href } = useWorkspace()
  const session = useSession()
  const client = useQueryClient()
  const builds = useQuery(buildsQuery(appId))
  const phoneOptions = useQuery(phoneOptionsQuery(appId))
  const [buildId, setBuildId] = useState<string | null>(null)
  const [profileId, setProfileId] = useState<string | null>(null)
  const [baselineId, setBaselineId] = useState<string>(NONE)
  const [baselineReady, setBaselineReady] = useState(false)
  const [phase, setPhase] = useState<'idle' | 'saving' | 'preparing' | 'queueing'>('idle')
  const [activeRunId, setActiveRunId] = useState<string | null>(null)
  const runIdentity = useRef<{ storageKey: string; value: string } | null>(null)
  const environmentRevision = phoneOptions.data?.environment_revision
  const versionForPreview = dirty ? '' : (savedVersionId ?? '')
  const preview = useQuery(
    savedCasePreviewQuery(workspaceId, appId, buildId ?? '', versionForPreview, profileId ?? ''),
  )
  const candidates = useQuery(
    baselineCandidatesQuery(
      workspaceId,
      appId,
      buildId ?? '',
      versionForPreview,
      profileId ?? '',
      environmentRevision,
    ),
  )
  const baselineStorageKey = useMemo(
    () =>
      versionForPreview && profileId && environmentRevision !== undefined
        ? `mobile-qa:baseline:${session.user.id}:${workspaceId}:${appId}:${versionForPreview}:${profileId}:${environmentRevision}`
        : '',
    [appId, environmentRevision, profileId, session.user.id, versionForPreview, workspaceId],
  )
  useEffect(() => {
    setBaselineReady(false)
  }, [baselineStorageKey])
  useEffect(() => {
    if (!baselineStorageKey || !candidates.data || baselineReady) return
    const retained = stored('localStorage', baselineStorageKey)
    const available = candidates.data.items.some((item) => item.run_id === retained)
    setBaselineId(
      retained === NONE || available ? retained! : (candidates.data.items[0]?.run_id ?? NONE),
    )
    setBaselineReady(true)
  }, [baselineReady, baselineStorageKey, candidates.data])
  const activeRun = useQuery({
    ...runQuery(workspaceId, activeRunId ?? ''),
    enabled: !!activeRunId,
  })
  const start = useMutation({
    mutationFn: async () => {
      if (!buildId || !profileId || environmentRevision === undefined) {
        throw new Error('Choose a build and qualified device before running')
      }
      setPhase(dirty ? 'saving' : 'preparing')
      const caseVersionId = dirty ? await saveAndGetVersion() : savedVersionId
      if (!caseVersionId) throw new Error('Save a complete test before running it')
      setPhase('preparing')
      if (phoneOptions.data?.active_session) {
        await releasePreview(phoneOptions.data.active_session, client)
        await client.invalidateQueries({ queryKey: ['phone-options', appId] })
      }
      setPhase('queueing')
      const selectedBaseline = dirty || baselineId === NONE ? null : baselineId
      const keyParts = [
        session.user.id,
        workspaceId,
        appId,
        buildId,
        caseVersionId,
        profileId,
        environmentRevision,
        selectedBaseline ?? NONE,
      ]
      const storageKey = `mobile-qa:test-run:${keyParts.join(':')}`
      let idempotencyKey = stored('sessionStorage', storageKey)
      if (!idempotencyKey && runIdentity.current?.storageKey === storageKey) {
        idempotencyKey = runIdentity.current.value
      }
      if (!idempotencyKey) {
        idempotencyKey = crypto.randomUUID()
        runIdentity.current = { storageKey, value: idempotencyKey }
        store('sessionStorage', storageKey, idempotencyKey)
      }
      const run = await createRun(
        appId,
        {
          build_id: buildId,
          source: {
            kind: 'saved_case',
            case_version_id: caseVersionId,
            profile_id: profileId,
          },
          environment_revision: environmentRevision,
          baseline_run_id: selectedBaseline,
        },
        idempotencyKey,
      )
      store('sessionStorage', storageKey)
      runIdentity.current = null
      return run
    },
    onSuccess: (run) => {
      setActiveRunId(run.id)
      setPhase('idle')
      void client.invalidateQueries({ queryKey: ['run-history', workspaceId, appId] })
    },
    onError: () => setPhase('idle'),
  })
  const baselineOptions = [
    { value: NONE, label: 'None — no comparison' },
    ...(candidates.data?.items.map((candidate) => ({
      value: candidate.run_id,
      label: `${candidate.build_name} · ${candidate.outcome} · ${new Date(candidate.created_at).toLocaleString()}`,
    })) ?? []),
  ]
  const blockers = preview.data?.blockers ?? []
  return (
    <Card withBorder>
      <Stack gap="sm">
        <Group justify="space-between">
          <Title order={2} size="h4">
            Run saved test
          </Title>
          <Badge variant="light">Durable history</Badge>
        </Group>
        <Group grow align="flex-start">
          <Select
            label="Build"
            placeholder="Choose a build"
            value={buildId}
            data={
              builds.data?.items.map((build) => ({
                value: build.id,
                label: build.original_filename,
              })) ?? []
            }
            onChange={setBuildId}
          />
          <Select
            label="Qualified device"
            placeholder="Choose a device"
            value={profileId}
            data={
              phoneOptions.data?.profiles.map((profile) => ({
                value: profile.id,
                label: profile.name,
              })) ?? []
            }
            onChange={setProfileId}
          />
        </Group>
        <Select
          label="Compare with"
          description={
            dirty
              ? 'Saving creates a new immutable version, so this run starts without a baseline.'
              : 'The selected run is frozen when this run is created.'
          }
          disabled={dirty || !versionForPreview || candidates.isPending}
          value={dirty ? NONE : baselineId}
          data={baselineOptions}
          onChange={(value) => {
            const next = value ?? NONE
            setBaselineId(next)
            if (baselineStorageKey) store('localStorage', baselineStorageKey, next)
          }}
        />
        {builds.isPending || phoneOptions.isPending ? (
          <LoadingPanel label="Loading run choices…" />
        ) : null}
        {builds.isError && <ErrorNotice error={builds.error} retry={() => void builds.refetch()} />}
        {phoneOptions.isError && (
          <ErrorNotice error={phoneOptions.error} retry={() => void phoneOptions.refetch()} />
        )}
        {preview.isError && (
          <ErrorNotice error={preview.error} retry={() => void preview.refetch()} />
        )}
        {!!blockers.length && (
          <Alert title="Before this test can run">{blockers.join(' · ')}</Alert>
        )}
        {preview.data?.manifest && (
          <Text size="xs" c="dimmed">
            Pinned build checksum: {preview.data.manifest.build_sha256}
          </Text>
        )}
        {phase === 'preparing' && <Text role="status">Preparing a clean test session…</Text>}
        <Button
          w="fit-content"
          disabled={
            disabled ||
            !buildId ||
            !profileId ||
            environmentRevision === undefined ||
            (!dirty && (!savedVersionId || !!blockers.length || preview.isError))
          }
          loading={start.isPending}
          onClick={() => start.mutate()}
        >
          {dirty ? 'Save & run' : 'Run test'}
        </Button>
        {start.isError && <ErrorNotice error={start.error} retry={() => start.mutate()} />}
        {activeRun.data && (
          <Alert
            title="Run started"
            color={activeRun.data.state === 'finished' ? 'forest' : 'blue'}
          >
            <Stack gap="xs">
              <Group justify="space-between">
                <Text>
                  {activeRun.data.state.replaceAll('_', ' ')} · {activeRun.data.summary}
                </Text>
                <Button component={Link} to={href(`/runs/${activeRun.data.id}`)} variant="light">
                  Open run
                </Button>
              </Group>
              {activeRun.data.attempts.at(-1)?.events.at(-1) && (
                <Text size="sm">
                  Latest execution event: {activeRun.data.attempts.at(-1)!.events.at(-1)!.message}
                </Text>
              )}
              {activeRun.data.attempts.at(-1)?.checks.map((check) => (
                <Text size="sm" key={check.check_id}>
                  {check.check_id}: {check.outcome} · observed {check.observed ?? 'Unknown'}
                </Text>
              ))}
            </Stack>
          </Alert>
        )}
      </Stack>
    </Card>
  )
}
