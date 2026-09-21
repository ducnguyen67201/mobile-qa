import { useRef, useState } from 'react'
import { Alert, Button, Group, Select, Stack, Text } from '@mantine/core'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Play } from 'lucide-react'
import { useNavigate } from 'react-router'
import type {
  LibraryDraftResponse,
  LibraryProfileChoice,
  SuiteRunRequest,
} from '@/api/generated/types.gen'
import { createSuiteRun, suiteRunPreviewQuery } from '@/api/regression'
import { buildQuery } from '@/api/setup'
import { phoneOptionsQuery, phoneQuery, stopPhone } from '@/api/task-sessions'
import { useWorkspace } from '@/hooks/use-workspace'

class SuiteRunSetupError extends Error {}

export function SavedSuiteRun({
  appId,
  versionId,
  dirty,
  save,
  profiles,
}: {
  appId: string
  versionId?: string | null
  dirty: boolean
  save?: () => Promise<LibraryDraftResponse>
  profiles: LibraryProfileChoice[]
}) {
  const { workspaceId = '', href } = useWorkspace()
  const navigate = useNavigate()
  const client = useQueryClient()
  const choices = useQuery(phoneOptionsQuery(appId))
  const [stage, setStage] = useState('')
  const [buildId, setBuildId] = useState<string | null>(null)
  const [profileId, setProfileId] = useState<string | null>(null)
  const [baselineId, setBaselineId] = useState<string | undefined>()
  const pending = useRef<{ body: SuiteRunRequest; key: string } | null>(null)
  const build = choices.data?.builds.find((value) => value.id === buildId)
  const profile = profiles.find((value) => value.id === profileId && value.qualified)
  const selectedBuild = useQuery({ ...buildQuery(appId, buildId ?? ''), enabled: !!buildId })
  const setup: SuiteRunRequest = {
    suite_version_id: versionId ?? '',
    build_id: buildId ?? '',
    profile_id: profileId ?? '',
    environment_revision: 0,
    baseline_run_id: null,
  }
  const readiness = useQuery({
    ...suiteRunPreviewQuery(appId, setup),
    enabled: !!versionId && !dirty && !!buildId && !!profile,
  })
  const selectedBaseline = baselineId ?? ''
  const suggestedBaseline = readiness.data?.baselines.find(
    (value) => value.id === readiness.data?.suggested_baseline_id,
  )

  const submit = useMutation({
    mutationFn: async (displayedBaseline: string) => {
      if (!pending.current) {
        let selectedVersion = versionId
        if (dirty || !selectedVersion) {
          if (!save) throw new SuiteRunSetupError('Save this suite before running it.')
          setStage('Saving suite…')
          const saved = await save()
          selectedVersion = saved.saved_version_id
          if (!selectedVersion || saved.issues.length)
            throw new SuiteRunSetupError('Complete the highlighted suite setup before running.')
        }
        if (!build || !profile)
          throw new SuiteRunSetupError('Upload a build and add a qualified device before running.')
        const request: SuiteRunRequest = {
          suite_version_id: selectedVersion,
          build_id: build.id,
          profile_id: profile.id,
          environment_revision: 0,
          baseline_run_id: displayedBaseline || null,
        }
        setStage('Checking run readiness…')
        const preview = await client.fetchQuery({
          ...suiteRunPreviewQuery(appId, request),
          staleTime: 0,
        })
        if (preview.blockers.length) throw new SuiteRunSetupError(preview.blockers.join('; '))
        pending.current = {
          body: { ...request, environment_revision: preview.environment_revision },
          key: crypto.randomUUID(),
        }
      }

      setStage('Preparing a clean test session…')
      const latest = await client.fetchQuery({ ...phoneOptionsQuery(appId), staleTime: 0 })
      if (latest.active_session) {
        let phone = await client.fetchQuery({
          ...phoneQuery(latest.active_session),
          staleTime: 0,
        })
        if (phone.state === 'acting')
          throw new SuiteRunSetupError(
            'Wait for the phone action to finish, or stop it explicitly, before running this suite.',
          )
        if (phone.state === 'quarantined')
          throw new SuiteRunSetupError('The phone needs recovery before it can run this suite.')
        if (!['closed', 'stopping'].includes(phone.state)) await stopPhone(phone.id)
        for (let count = 0; count < 30 && phone.state !== 'closed'; count++) {
          await new Promise((resolve) => setTimeout(resolve, 1000))
          phone = await client.fetchQuery({ ...phoneQuery(phone.id), staleTime: 0 })
          if (phone.state === 'quarantined')
            throw new SuiteRunSetupError('Phone cleanup needs recovery. No suite was queued.')
        }
        if (phone.state !== 'closed')
          throw new SuiteRunSetupError(
            'Phone cleanup is still running. Retry when it has disconnected.',
          )
      }

      setStage('Queuing suite…')
      return createSuiteRun(appId, pending.current.body, pending.current.key)
    },
    onSuccess: (run) => {
      pending.current = null
      client.setQueryData(['run', workspaceId, run.id], run)
      void navigate(href(`/runs/${run.id}`))
    },
    onSettled: () => setStage(''),
  })
  const change = () => {
    pending.current = null
    submit.reset()
  }

  const unavailable = !build || !profile || (!versionId && !dirty)
  return (
    <Stack gap="xs" align="stretch">
      <Text size="sm" fw={600}>
        Run saved suite{versionId ? ` · version ${versionId.slice(0, 8)}` : ''}
      </Text>
      <Group grow align="flex-start">
        <Select
          label="Build to test"
          placeholder="Choose a build"
          value={buildId}
          disabled={submit.isPending}
          data={choices.data?.builds.map((value) => ({ value: value.id, label: value.name })) ?? []}
          onChange={(value) => {
            setBuildId(value)
            setBaselineId(undefined)
            change()
          }}
        />
        <Select
          label="Test device"
          placeholder="Choose a qualified device"
          value={profileId}
          disabled={submit.isPending}
          data={profiles
            .filter((value) => value.qualified)
            .map((value) => ({ value: value.id, label: value.name }))}
          onChange={(value) => {
            setProfileId(value)
            setBaselineId(undefined)
            change()
          }}
        />
      </Group>
      {selectedBuild.data && <Text size="xs">Build checksum: {selectedBuild.data.sha256}</Text>}
      {selectedBuild.isError && (
        <Text size="xs" role="alert">
          Unable to load the selected build checksum.
        </Text>
      )}
      <Select
        label="Compare with"
        value={selectedBaseline}
        disabled={submit.isPending}
        data={[
          { value: '', label: 'None — establish a first result' },
          ...(readiness.data?.baselines.map((value) => ({
            value: value.id,
            label: `${value.build_label} · ${new Date(value.created_at).toLocaleString()}${value.compatible ? '' : ' · Not comparable'}`,
          })) ?? []),
        ]}
        onChange={(value) => {
          setBaselineId(value ?? '')
          change()
        }}
      />
      {suggestedBaseline && (
        <Text size="xs" c="dimmed">
          Suggested baseline: {suggestedBaseline.build_label} ·{' '}
          {new Date(suggestedBaseline.created_at).toLocaleString()}
        </Text>
      )}
      {dirty && (
        <Text size="xs">
          Save will create a new suite version; readiness is checked after saving.
        </Text>
      )}
      {readiness.data?.blockers.map((blocker) => (
        <Alert key={blocker} color="orange">
          {blocker}
        </Alert>
      ))}
      {readiness.isError && (
        <Text size="xs" role="alert">
          Unable to load run readiness. Retry the run setup.
        </Text>
      )}
      <Button
        size="sm"
        leftSection={<Play size={15} aria-hidden />}
        disabled={
          unavailable ||
          choices.isError ||
          (!pending.current &&
            (!selectedBuild.data || !!(!dirty && readiness.data?.blockers.length)))
        }
        loading={submit.isPending}
        onClick={() => submit.mutate(selectedBaseline)}
      >
        {pending.current ? 'Retry run' : dirty ? 'Save & run suite' : 'Run suite'}
      </Button>
      <Text size="xs" c="dimmed" ta="right">
        {choices.isPending
          ? 'Loading run setup…'
          : build && profile
            ? `${build.name} · ${profile.name}`
            : 'Choose a build and qualified device'}
      </Text>
      {stage && (
        <Text size="xs" role="status">
          {stage}
        </Text>
      )}
      {submit.isError && (
        <Text size="xs" c="red" role="alert" maw={280} ta="right">
          {submit.error instanceof SuiteRunSetupError
            ? submit.error.message
            : 'The suite could not be queued. Retry with the same run request.'}
        </Text>
      )}
    </Stack>
  )
}
