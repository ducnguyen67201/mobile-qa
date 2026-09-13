/** Interactive device work. Captures are server evidence, never a mock emulator. */
import { useEffect, useRef, useState, type ReactNode } from 'react'
import {
  Alert,
  Anchor,
  Badge,
  Button,
  Card,
  Group,
  Loader,
  Select,
  Stack,
  Text,
  Title,
} from '@mantine/core'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Link } from 'react-router'
import { useWorkspace } from '@/hooks/use-workspace'
import { ErrorNotice } from '@/components/app/feedback'
import {
  openPhone,
  phoneOptionsQuery,
  phoneQuery,
  runPhoneTask,
  stopPhone,
} from '@/api/task-sessions'
import type {
  OpenPhoneRequest,
  PhoneSelection,
  PhoneTaskRequest,
  PhoneSession,
  PhoneTask,
} from '@/api/generated/types.gen'

import { createLibraryEntry, saveLibraryDraft } from '@/api/test-library'

import classes from './phone-workspace.module.css'
import { ResizableWorkspace } from './resizable-workspace'
import { newTaskStep, taskGoal, TaskSteps } from './task-steps'

export function PhoneWorkspace({
  appId,
  autoOpen = true,
  children,
}: {
  appId: string
  autoOpen?: boolean
  children?: ReactNode
}) {
  const { href } = useWorkspace()
  const client = useQueryClient()
  const choices = useQuery(phoneOptionsQuery(appId))
  const [id, setId] = useState('')
  const [requestId] = useState(() => crypto.randomUUID())
  const [device, setDevice] = useState<string | null>(null)
  const [steps, setSteps] = useState(() => [newTaskStep()])
  const goal = taskGoal(steps)
  const tooLong = goal.length > 4000
  const [selection, setSelection] = useState<PhoneSelection | null>(null)
  const initiated = useRef(false)
  const mounted = useRef(true)
  useEffect(() => {
    mounted.current = true
    return () => {
      mounted.current = false
    }
  }, [])
  const saved = (s: PhoneSession) => {
    if (mounted.current) {
      client.setQueryData(['phone', s.id], s)
      setId(s.id)
    }
  }
  const create = useMutation({
    mutationFn: (body: OpenPhoneRequest) => openPhone(appId, body),
    onSuccess: saved,
  })
  const submit = useMutation({
    mutationFn: (body: PhoneTaskRequest) => runPhoneTask(id, body),
    onSuccess: (s) => {
      saved(s)
      setSelection(null)
    },
  })
  const stop = useMutation({ mutationFn: () => stopPhone(id), onSuccess: saved })
  const { mutate: createSession } = create
  useEffect(() => {
    if (!choices.data || initiated.current) return
    if (choices.data.active_session) {
      initiated.current = true
      setId(choices.data.active_session)
      return
    }
    if (autoOpen && !choices.data.blockers.length && choices.data.profiles.length === 1) {
      initiated.current = true
      createSession({ id: requestId, build_id: null, profile_id: null })
    }
  }, [choices.data, createSession, requestId, autoOpen])
  const phone = useQuery(phoneQuery(id))
  const s = phone.data
  const frame = s?.frame
  const ready = s?.state === 'ready' && !phone.isError && !submit.isPending
  const done = s && ['closed', 'quarantined'].includes(s.state)
  const selected =
    frame?.id === selection?.frame_id
      ? frame?.controls.find((c) => c.id === selection?.control_id)
      : undefined
  return (
    <Stack gap="lg">
      {!children && (
        <Group justify="space-between">
          <div>
            <Title order={1}>Try your app</Title>
            <Text c="dimmed">Set up a task on the left. Watch your app on the right.</Text>
          </div>
          <Anchor component={Link} to={href(`/apps/${appId}`)}>
            App & builds
          </Anchor>
        </Group>
      )}
      {choices.isError && (
        <ErrorNotice error={choices.error} retry={() => void choices.refetch()} />
      )}
      {!choices.data && !choices.isError && <Loader aria-label="Finding your app" />}
      {choices.data?.blockers.map((message) => (
        <Alert key={message}>{message}</Alert>
      ))}
      {!id && choices.data && !choices.data.blockers.length && choices.data.profiles.length > 1 && (
        <Card withBorder>
          <Stack>
            <Select
              label="Choose a device"
              data={choices.data.profiles.map((p) => ({ value: p.id, label: p.name }))}
              value={device}
              onChange={setDevice}
            />
            <Button
              disabled={!device}
              loading={create.isPending}
              onClick={() => create.mutate({ id: requestId, build_id: null, profile_id: device })}
            >
              Open app
            </Button>
          </Stack>
        </Card>
      )}
      {create.isError && (
        <ErrorNotice
          error={create.error}
          retry={() => create.variables && create.mutate(create.variables)}
        />
      )}
      {phone.isError && <ErrorNotice error={phone.error} retry={() => void phone.refetch()} />}
      <ResizableWorkspace>
        <Stack component="section" aria-label="Task setup" className={classes.editor}>
          <Title order={2} size="h3">
            Set up your task
          </Title>
          <Text size="sm" c="dimmed">
            Add steps, choose actions, or ask AI to handle a task. Run when you’re ready.
          </Text>
          <TaskSteps
            steps={steps}
            disabled={submit.isPending}
            onChange={(next) => {
              setSteps(next)
              submit.reset()
            }}
          />
          {tooLong && (
            <Alert color="orange">Shorten your steps to fit within 4,000 characters.</Alert>
          )}
          {selected && (
            <Group>
              <Text size="sm">Starting control: {selected.label}</Text>
              <Button
                variant="subtle"
                size="xs"
                onClick={() => {
                  setSelection(null)
                  submit.reset()
                }}
              >
                Clear
              </Button>
            </Group>
          )}
          {selection && !selected && <Alert>The screen changed. Select the control again.</Alert>}
          <Group>
            <Button
              size="md"
              disabled={!ready || !goal || tooLong || (!!selection && !selected)}
              loading={submit.isPending}
              onClick={() =>
                submit.mutate(
                  submit.isError && submit.variables
                    ? submit.variables
                    : { id: crypto.randomUUID(), goal, selection },
                )
              }
            >
              Run task
            </Button>
            {s && !done && (
              <Button
                variant="light"
                color="red"
                loading={stop.isPending}
                disabled={s.state === 'stopping'}
                onClick={() => stop.mutate()}
              >
                Stop session
              </Button>
            )}
          </Group>
          {submit.isError && (
            <ErrorNotice
              error={submit.error}
              retry={() => submit.variables && submit.mutate(submit.variables)}
            />
          )}
          {stop.isError && <ErrorNotice error={stop.error} retry={() => stop.mutate()} />}
          {done && (
            <Alert>
              {s.state === 'quarantined'
                ? 'The device needs operator recovery before another session.'
                : 'This session ended.'}
            </Alert>
          )}
          {s?.state === 'closed' && (
            <Button
              variant="light"
              loading={create.isPending}
              onClick={() => {
                setSelection(null)
                submit.reset()
                stop.reset()
                create.mutate({ id: crypto.randomUUID(), build_id: null, profile_id: s.profile.id })
              }}
            >
              Open a new session
            </Button>
          )}
          {s?.tasks
            .slice()
            .reverse()
            .map((t) => (
              <Card key={t.id} withBorder>
                <Stack gap="xs">
                  <Group justify="space-between">
                    <Text fw={600}>{t.goal}</Text>
                    <Badge color={t.state === 'failed' ? 'red' : 'gray'}>{t.state}</Badge>
                  </Group>
                  <Text size="sm" c="dimmed">
                    {t.message}
                  </Text>
                  {t.state === 'completed' && <SaveTask appId={appId} task={t} />}
                </Stack>
              </Card>
            ))}
          {children ?? (
            <Anchor component={Link} to={href('/tests')}>
              Saved tests & advanced editing
            </Anchor>
          )}
        </Stack>
        <aside aria-label="App preview" className={classes.preview}>
          <Stack gap="sm">
            <Group justify="space-between">
              <Title order={2} size="h3">
                App preview
              </Title>
              {s && <Badge>{s.state.replaceAll('_', ' ')}</Badge>}
            </Group>
            {s && (
              <Text size="sm" role="status">
                {s.message}
              </Text>
            )}
            {!id &&
              !autoOpen &&
              choices.data &&
              !choices.data.blockers.length &&
              choices.data.profiles.length === 1 && (
                <Button
                  loading={create.isPending}
                  onClick={() => create.mutate({ id: requestId, build_id: null, profile_id: null })}
                >
                  Open phone preview
                </Button>
              )}
            <Card withBorder radius="xl" p="sm" className={classes.phone}>
              {frame ? (
                <div
                  style={{
                    position: 'relative',
                    lineHeight: 0,
                    width: `min(100%, calc((100dvh - 240px) * ${frame.width / frame.height}))`,
                    marginInline: 'auto',
                  }}
                >
                  <img
                    src={`data:image/png;base64,${frame.png_base64}`}
                    alt="Current screen of your Android app"
                    style={{ width: '100%', borderRadius: 16, display: 'block' }}
                  />
                  {ready &&
                    frame.controls.map((c) => (
                      <button
                        key={c.id}
                        type="button"
                        aria-label={`Select ${c.label}`}
                        aria-pressed={selected?.id === c.id}
                        onClick={() => {
                          setSelection({ frame_id: frame.id, control_id: c.id })
                          submit.reset()
                        }}
                        style={{
                          position: 'absolute',
                          left: `${(100 * c.left) / frame.width}%`,
                          top: `${(100 * c.top) / frame.height}%`,
                          width: `${(100 * (c.right - c.left)) / frame.width}%`,
                          height: `${(100 * (c.bottom - c.top)) / frame.height}%`,
                          cursor: 'crosshair',
                          background: selected?.id === c.id ? '#88bd6260' : 'transparent',
                          border:
                            selected?.id === c.id ? '2px solid #245b46' : '1px solid transparent',
                        }}
                      />
                    ))}
                </div>
              ) : (
                <Stack align="center" justify="center" className={classes.placeholder}>
                  {!done &&
                    !choices.data?.blockers.length &&
                    (id || create.isPending || autoOpen) && <Loader aria-label="Opening phone" />}
                  <Text ta="center">
                    {s?.message ??
                      (choices.data?.blockers.length
                        ? 'Your app screen will appear here once setup is ready.'
                        : !id && !autoOpen && !create.isPending
                          ? 'Open the phone preview to try your task here.'
                          : 'Opening your app…')}
                  </Text>
                </Stack>
              )}
              {frame && (
                <Text size="xs" c="dimmed" mt="sm">
                  {ready
                    ? 'Select a starting control for Minitap. Describe later targets in your steps.'
                    : 'Screen captures update while Minitap works.'}
                </Text>
              )}
            </Card>
            <Text size="xs" c="dimmed">
              Latest device capture. Refreshes during execution; writing a task does not control the
              app.
            </Text>
          </Stack>
        </aside>
      </ResizableWorkspace>
    </Stack>
  )
}

function SaveTask({ appId, task }: { appId: string; task: PhoneTask }) {
  const { href } = useWorkspace()
  const [mutationId] = useState(() => crypto.randomUUID())
  const save = useMutation({
    mutationFn: async () => {
      const draft = await createLibraryEntry(appId, {
        entry_id: task.id,
        mutation_id: task.id,
        kind: 'case',
        key: `task-${task.id}`,
        template_profile_id: null,
      })
      if (draft.definition.kind !== 'case') throw new Error('Expected a case draft')
      return saveLibraryDraft(appId, task.id, {
        mutation_id: mutationId,
        expected_revision: draft.entry.revision,
        definition: {
          kind: 'case',
          content: {
            ...draft.definition.content,
            title: task.goal.slice(0, 200),
            requirement: task.goal,
            actions: [
              {
                id: 'task',
                kind: 'navigate',
                instruction: task.goal,
                checkpoint_id: 'task-result',
              },
            ],
            checks: [],
          },
        },
      })
    },
  })
  return (
    <Stack gap="xs">
      {save.data ? (
        <Anchor component={Link} to={href(`/tests/${appId}/${task.id}`)}>
          Open saved test draft
        </Anchor>
      ) : (
        <Button variant="light" loading={save.isPending} onClick={() => save.mutate()}>
          Save as test
        </Button>
      )}
      {save.isError && <ErrorNotice error={save.error} retry={() => save.mutate()} />}
      {save.data && (
        <Text size="xs" c="dimmed">
          Draft saved. Add an expected result before reviewing it as a reusable test.
        </Text>
      )}
    </Stack>
  )
}
