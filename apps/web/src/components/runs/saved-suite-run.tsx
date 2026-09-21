import { useRef, useState } from 'react'
import { Button, Stack, Text } from '@mantine/core'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Play } from 'lucide-react'
import { useNavigate } from 'react-router'
import type {
  LibraryDraftResponse,
  LibraryProfileChoice,
  SuiteRunRequest,
} from '@/api/generated/types.gen'
import { createSuiteRun, suiteRunPreviewQuery } from '@/api/regression'
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
  const pending = useRef<{ body: SuiteRunRequest; key: string } | null>(null)
  const build = choices.data?.builds[0]
  const profile = profiles.find((value) => value.qualified)

  const submit = useMutation({
    mutationFn: async () => {
      if (!pending.current) {
        let selectedVersion = versionId
        if (dirty || !selectedVersion) {
          if (!save) throw new SuiteRunSetupError('Save this sequence before running it.')
          setStage('Saving sequence…')
          const saved = await save()
          selectedVersion = saved.saved_version_id
          if (!selectedVersion || saved.issues.length)
            throw new SuiteRunSetupError('Complete the highlighted sequence setup before running.')
        }
        if (!build || !profile)
          throw new SuiteRunSetupError('Upload a build and add a qualified device before running.')
        const request: SuiteRunRequest = {
          suite_version_id: selectedVersion,
          build_id: build.id,
          profile_id: profile.id,
          environment_revision: 0,
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
            'Wait for the phone action to finish, or stop it explicitly, before running this sequence.',
          )
        if (phone.state === 'quarantined')
          throw new SuiteRunSetupError('The phone needs recovery before it can run this sequence.')
        if (!['closed', 'stopping'].includes(phone.state)) await stopPhone(phone.id)
        for (let count = 0; count < 30 && phone.state !== 'closed'; count++) {
          await new Promise((resolve) => setTimeout(resolve, 1000))
          phone = await client.fetchQuery({ ...phoneQuery(phone.id), staleTime: 0 })
          if (phone.state === 'quarantined')
            throw new SuiteRunSetupError('Phone cleanup needs recovery. No sequence was queued.')
        }
        if (phone.state !== 'closed')
          throw new SuiteRunSetupError(
            'Phone cleanup is still running. Retry when it has disconnected.',
          )
      }

      setStage('Queuing sequence…')
      return createSuiteRun(appId, pending.current.body, pending.current.key)
    },
    onSuccess: (run) => {
      pending.current = null
      client.setQueryData(['run', workspaceId, run.id], run)
      void navigate(href(`/runs/${run.id}`))
    },
    onSettled: () => setStage(''),
  })

  const unavailable = !build || !profile || (!versionId && !dirty)
  return (
    <Stack gap={2} align="flex-end">
      <Button
        size="sm"
        leftSection={<Play size={15} aria-hidden />}
        disabled={unavailable || choices.isError}
        loading={submit.isPending}
        onClick={() => submit.mutate()}
      >
        {pending.current ? 'Retry run' : dirty ? 'Save & run' : 'Run sequence'}
      </Button>
      <Text size="xs" c="dimmed" ta="right">
        {build && profile
          ? `${build.name} · ${profile.name}`
          : choices.isPending
            ? 'Loading run setup…'
            : 'Build and device required'}
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
            : 'The sequence could not be queued. Retry with the same run request.'}
        </Text>
      )}
    </Stack>
  )
}
