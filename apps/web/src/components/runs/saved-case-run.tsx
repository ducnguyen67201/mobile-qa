import { useRef, useState } from 'react'
import { Alert, Button, Card, Group, Select, Stack, Text } from '@mantine/core'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import type { CaseRunRequest, LibraryDraftResponse } from '@/api/generated/types.gen'
import { caseRunPreviewQuery, createCaseRun, historyQuery } from '@/api/regression'
import { runQuery } from '@/api/runs'
import { phoneOptionsQuery, phoneQuery, stopPhone } from '@/api/task-sessions'
import { useWorkspace } from '@/hooks/use-workspace'
import { ErrorNotice } from '@/components/app/feedback'
import { RunResult } from './run-result'
// Only deliberate authoring guidance is displayed verbatim; network errors keep
// the shared sanitized error presentation.
class RunSetupError extends Error {}
export function SavedCaseRun({
  appId,
  versionId,
  dirty,
  save,
}: {
  appId: string
  versionId?: string | null
  dirty: boolean
  save: () => Promise<LibraryDraftResponse>
}) {
  const { workspaceId = '' } = useWorkspace(),
    client = useQueryClient()
  const choices = useQuery(phoneOptionsQuery(appId))
  const [build, setBuild] = useState<string | null>(null),
    [profile, setProfile] = useState<string | null>(null)
  const [baseline, setBaseline] = useState<string | undefined>(),
    [runId, setRunId] = useState(''),
    [stage, setStage] = useState('')
  // A lost response retries the identical immutable request and idempotency identity.
  const pending = useRef<{ body: CaseRunRequest; key: string } | null>(null)
  const body: CaseRunRequest = {
    case_version_id: versionId ?? '',
    build_id: build ?? '',
    profile_id: profile ?? choices.data?.profiles[0]?.id ?? '',
    environment_revision: 0,
    baseline_run_id: null,
  }
  const preview = useQuery(caseRunPreviewQuery(appId, body))
  const selectedBaseline = baseline ?? preview.data?.suggested_baseline_id ?? ''
  const history = useQuery(historyQuery(workspaceId, appId, 'runs'))
  const latest = history.data?.items.find((i) =>
    i.run?.manifest.cases.some((c) => c.definition_id === versionId),
  )?.run
  const result = useQuery(runQuery(workspaceId, runId || latest?.id || ''))
  const submit = useMutation({
    mutationFn: async (displayedBaseline: string) => {
      if (!pending.current) {
        setStage('Saving test…')
        const saved = dirty || !versionId ? await save() : null
        const id = saved?.saved_version_id ?? versionId
        if (!id || saved?.issues.length)
          throw new RunSetupError('Complete the highlighted test fields before running.')
        const request = { ...body, case_version_id: id }
        const p = await client.fetchQuery({ ...caseRunPreviewQuery(appId, request), staleTime: 0 })
        if (p.blockers.length) throw new RunSetupError(p.blockers.join('; '))
        pending.current = {
          body: {
            ...request,
            environment_revision: p.environment_revision,
            // Refresh readiness, but never replace the selection shown at click time.
            baseline_run_id: displayedBaseline || null,
          },
          key: crypto.randomUUID(),
        }
      }
      setStage('Preparing a clean test session…')
      const options = await client.fetchQuery({ ...phoneOptionsQuery(appId), staleTime: 0 })
      if (options.active_session) {
        let phone = await client.fetchQuery({ ...phoneQuery(options.active_session), staleTime: 0 })
        if (phone.state === 'acting')
          throw new RunSetupError(
            'Wait for the phone action to finish, or stop it explicitly, before running this test.',
          )
        if (phone.state === 'quarantined')
          throw new RunSetupError('The phone needs recovery before it can run a clean test.')
        if (!['closed', 'stopping'].includes(phone.state)) await stopPhone(phone.id)
        for (let n = 0; n < 30 && phone.state !== 'closed'; n++) {
          await new Promise((resolve) => setTimeout(resolve, 1000))
          phone = await client.fetchQuery({ ...phoneQuery(phone.id), staleTime: 0 })
          if (phone.state === 'quarantined')
            throw new RunSetupError('Phone cleanup needs recovery. No test was queued.')
        }
        if (phone.state !== 'closed')
          throw new RunSetupError('Phone cleanup is still running. Retry when it has disconnected.')
      }
      setStage('Queuing saved test…')
      return createCaseRun(appId, pending.current.body, pending.current.key)
    },
    onSuccess: (r) => {
      pending.current = null
      setRunId(r.id)
      client.setQueryData(['run', workspaceId, r.id], r)
      void client.invalidateQueries({ queryKey: ['run-history'] })
    },
    onSettled: () => setStage(''),
  })
  const change = () => {
    pending.current = null
    submit.reset()
  }
  return (
    <Stack gap="sm">
      <Card withBorder padding="sm">
        <Stack gap="xs">
          <Text fw={600}>Run saved test</Text>
          <Group grow align="flex-start">
            <Select
              label="Build to test"
              placeholder="Choose a build"
              value={build}
              disabled={submit.isPending}
              data={choices.data?.builds.map((b) => ({ value: b.id, label: b.name })) ?? []}
              onChange={(v) => {
                setBuild(v)
                setBaseline(undefined)
                change()
              }}
            />
            <Select
              label="Test device"
              value={body.profile_id || null}
              disabled={submit.isPending}
              data={choices.data?.profiles.map((p) => ({ value: p.id, label: p.name })) ?? []}
              onChange={(v) => {
                setProfile(v)
                setBaseline(undefined)
                change()
              }}
            />
          </Group>
          <Select
            label="Compare with"
            placeholder="No baseline"
            value={selectedBaseline}
            disabled={submit.isPending}
            data={[
              { value: '', label: 'None — establish a first result' },
              ...(preview.data?.baselines.map((b) => ({
                value: b.id,
                label: `${b.build_label} · ${new Date(b.created_at).toLocaleString()}${b.compatible ? '' : ' · Not comparable'}`,
              })) ?? []),
            ]}
            onChange={(v) => {
              setBaseline(v ?? '')
              change()
            }}
          />
          <Text size="xs" c="dimmed">
            Uses the selected build in a clean session. The phone preview is a separate last
            capture. A baseline is pinned when you run.
          </Text>
          {preview.data?.blockers.map((b) => (
            <Alert key={b} color="orange">
              {b}
            </Alert>
          ))}
          {choices.isError && (
            <ErrorNotice error={choices.error} retry={() => void choices.refetch()} />
          )}
          {preview.isError && (
            <ErrorNotice error={preview.error} retry={() => void preview.refetch()} />
          )}
          <Group>
            <Button
              disabled={!build || !body.profile_id || (!dirty && !!preview.data?.blockers.length)}
              loading={submit.isPending}
              onClick={() => submit.mutate(selectedBaseline)}
            >
              {pending.current ? 'Retry run' : dirty ? 'Save & run' : 'Run test'}
            </Button>
            {stage && (
              <Text size="sm" role="status">
                {stage}
              </Text>
            )}
          </Group>
          {submit.isError &&
            (submit.error instanceof RunSetupError ? (
              <Alert color="orange" role="alert">
                {submit.error.message}
              </Alert>
            ) : (
              <ErrorNotice error={submit.error} retry={() => submit.mutate(selectedBaseline)} />
            ))}
        </Stack>
      </Card>
      {result.data && <RunResult run={result.data} />}
    </Stack>
  )
}
