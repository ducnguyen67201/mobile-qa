/** Real device capture and structured commands share the case draft's state. */
import { type ReactNode, useEffect, useRef, useState } from 'react'
import {
  Accordion,
  Alert,
  Badge,
  Button,
  Card,
  Checkbox,
  Group,
  Loader,
  Modal,
  Select,
  Stack,
  Text,
  Textarea,
  TextInput,
  Title,
} from '@mantine/core'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Link, useSearchParams } from 'react-router'
import type {
  AutomationSequence,
  CaseDefinition,
  LibraryIssue,
  DirectCommand,
  DirectTarget,
  PhoneCommandRequest,
  PhoneControl,
  PhoneSession,
  SaveAuthoredTestsRequest,
} from '@/api/generated/types.gen'
import { useWorkspace } from '@/hooks/use-workspace'
import { ErrorNotice } from '@/components/app/feedback'
import { openPhone, phoneOptionsQuery, phoneQuery, stopPhone } from '@/api/task-sessions'
import { runPhoneCommand, saveAuthoredTests } from '@/api/test-authoring'
import {
  DiscoveryActivity,
  DiscoveryTarget,
  explorationLabel,
} from '@/components/test-library/discovery-activity'
import { GenerationPanel, ProposalReview } from '@/components/test-library/ai-authoring'
import { BudgetFields } from '@/components/test-library/definition-fields'
import { ResizableWorkspace } from './resizable-workspace'
import { newTaskStep, sequenceReady, TaskSteps } from './task-steps'
import classes from './phone-workspace.module.css'
import {
  caseIssueMessage,
  focusIssueField,
  issueFieldId,
  type IssueFocus,
} from '@/components/test-library/case-issues'

export function PhoneWorkspace({
  appId,
  autoOpen = true,
  value,
  onChange,
  disabled = false,
  issues = [],
  issuePrefix = 'phone',
  issueFocus,
  runControl,
}: {
  appId: string
  runControl?: ReactNode
  autoOpen?: boolean
  value?: CaseDefinition
  onChange?: (v: CaseDefinition) => void
  disabled?: boolean
  issues?: LibraryIssue[]
  issuePrefix?: string
  issueFocus?: IssueFocus | null
}) {
  const [detailsOpen, setDetailsOpen] = useState<string | null>(null)
  const fieldError = (field: string) => {
    const issue = issues.find((i) => i.field === field && !i.item_id)
    return issue ? caseIssueMessage(issue) : undefined
  }
  useEffect(() => {
    if (!issueFocus || !['title', 'requirement'].includes(issueFocus.issue.field)) return
    if (issueFocus.issue.field === 'requirement') setDetailsOpen('details')
    return focusIssueField(issueFieldId(issuePrefix, issueFocus.issue.field))
  }, [issueFocus, issuePrefix])
  const { href } = useWorkspace()
  const [params] = useSearchParams()
  const client = useQueryClient()
  const choices = useQuery(phoneOptionsQuery(appId))
  const [id, setId] = useState('')
  const [local, setLocal] = useState<AutomationSequence>({ actions: [newTaskStep()], checks: [] })
  const [title, setTitle] = useState('New test')
  const [requirement, setRequirement] = useState('')
  const [device, setDevice] = useState<string | null>(null)
  const [picking, setPicking] = useState<string | null>(null)
  const [mode, setMode] = useState('pick')
  const [record, setRecord] = useState(false)
  const [typing, setTyping] = useState<PhoneControl | null>(null)
  const [text, setText] = useState('')
  const [reconnectRequested, setReconnectRequested] = useState(false)
  const [ai, setAi] = useState(params.get('generate') === '1')
  const initiated = useRef(false)
  const [openId] = useState(() => crypto.randomUUID())
  const recorded = useRef(new Set<string>())
  const awaitingRecord = useRef(new Set<string>())
  const sequence: AutomationSequence = value
    ? { actions: value.actions, checks: value.checks }
    : local
  const current = useRef(sequence)
  current.current = sequence
  const setSequence = (next: AutomationSequence) => {
    if (value && onChange) onChange({ ...value, ...next })
    else setLocal(next)
  }
  const sessionSaved = (s: PhoneSession) => {
    client.setQueryData(['phone', s.id], s)
    setId(s.id)
  }
  const reopen = useMutation({
    mutationFn: (profile: string) =>
      openPhone(appId, { id: crypto.randomUUID(), build_id: null, profile_id: profile }),
    onSuccess: sessionSaved,
  })
  const create = useMutation({
    mutationFn: (profile: string | null) =>
      openPhone(appId, { id: openId, build_id: null, profile_id: profile }),
    onSuccess: sessionSaved,
  })
  const { mutate: open } = create
  useEffect(() => {
    if (!choices.data || initiated.current) return
    if (choices.data.active_session) {
      initiated.current = true
      setId(choices.data.active_session)
      return
    }
    if (autoOpen && !choices.data.blockers.length && choices.data.profiles.length === 1) {
      initiated.current = true
      open(null)
    }
  }, [choices.data, autoOpen, open])
  const phone = useQuery(phoneQuery(id))
  const s = phone.data
  const frame = s?.frame
  const disconnected = s?.state === 'closed'
  const previewMessage = disconnected
    ? reopen.isPending
      ? 'Reconnecting…'
      : 'Phone disconnected. Reconnect to continue testing.'
    : (s?.message ?? 'Connect the phone to pick controls or try your test.')
  const activeExploration = s?.tasks.find(
    (task) => task.generation && ['queued', 'acting'].includes(task.state),
  )
  const submit = useMutation({
    mutationFn: (body: PhoneCommandRequest) => runPhoneCommand(id, body),
    onSuccess: sessionSaved,
  })
  const stop = useMutation({ mutationFn: () => stopPhone(id), onSuccess: sessionSaved })
  const { mutate: reopenSession } = reopen
  useEffect(() => {
    if (!reconnectRequested || !s) return
    // Wait for device cleanup before requesting another exclusive session.
    if (s.state === 'closed') {
      setReconnectRequested(false)
      reopenSession(s.profile.id)
    } else if (s.state === 'quarantined') {
      setReconnectRequested(false)
    }
  }, [reconnectRequested, s, reopenSession])
  const save = useMutation({
    mutationFn: (body: SaveAuthoredTestsRequest) => saveAuthoredTests(appId, body),
    onSuccess: () => void client.invalidateQueries({ queryKey: ['test-library'] }),
  })
  const supportsDirect = [2, 3, 4].includes(s?.protocol_version ?? 0)
  const ready = s?.state === 'ready' && supportsDirect && !phone.isError && !submit.isPending
  const usesAi = sequence.actions.some((a) => a.kind === 'navigate')
  useEffect(() => {
    for (const task of s?.tasks ?? []) {
      if (
        !awaitingRecord.current.has(task.id) ||
        recorded.current.has(task.id) ||
        task.state !== 'completed' ||
        !task.sequence
      )
        continue
      recorded.current.add(task.id)
      const draft = current.current
      const placeholder =
        draft.actions.length === 1 &&
        !draft.checks.length &&
        draft.actions[0]?.command?.operation === 'tap' &&
        !draft.actions[0].command.target.value
      setSequence({
        ...draft,
        actions: [...(placeholder ? [] : draft.actions), ...task.sequence.actions].slice(0, 20),
      })
    }
    // Receipts append once to the active draft; mutable editor values must not trigger replay.
  }, [s?.tasks])
  const run = (next: AutomationSequence, manual = false) => {
    if (!s || (manual && record && sequence.actions.length >= 20)) return
    const commandId = crypto.randomUUID()
    if (manual && record) awaitingRecord.current.add(commandId)
    submit.mutate({
      id: commandId,
      expected_revision: s.revision ?? 0,
      frame_id: manual ? (frame?.id ?? null) : null,
      title: manual ? 'Phone interaction' : value?.title || title,
      sequence: next,
      purpose: manual ? 'manual' : 'trial',
    })
  }
  const targetFor = (control: PhoneControl): DirectTarget | undefined =>
    control.resource_id
      ? { by: 'resource_id', value: control.resource_id }
      : control.description
        ? { by: 'description', value: control.description }
        : undefined
  const control = (command: DirectCommand) =>
    run({ actions: [newTaskStep(command)], checks: [] }, true)
  const pick = (c: PhoneControl) => {
    const target = targetFor(c)
    if (!target) return
    if (mode === 'control') {
      if (c.editable) {
        setTyping(c)
        setText('')
      } else control({ operation: 'tap', target })
      return
    }
    if (!picking) return
    const action = sequence.actions.find((a) => a.id === picking)
    if (action?.command && 'target' in action.command)
      setSequence({
        ...sequence,
        actions: sequence.actions.map((a) =>
          a.id === picking && a.command && 'target' in a.command
            ? { ...a, command: { ...a.command, target } }
            : a,
        ),
      })
    else if (c.resource_id)
      setSequence({
        ...sequence,
        checks: sequence.checks.map((check) =>
          `ready:${check.id}` === picking
            ? { ...check, ready_resource_id: c.resource_id }
            : check.id === picking
              ? {
                  ...check,
                  resource_id: c.resource_id,
                  ready_resource_id: check.ready_resource_id || c.resource_id,
                }
              : check,
        ),
      })
    setPicking(null)
  }
  return (
    <Stack
      gap="md"
      onKeyDown={(e) => {
        if (e.key === 'Escape') setPicking(null)
      }}
    >
      {choices.isError && (
        <ErrorNotice error={choices.error} retry={() => void choices.refetch()} />
      )}
      {choices.data?.blockers.map((message) => (
        <Alert key={message}>{message}</Alert>
      ))}
      <ResizableWorkspace>
        <Stack component="section" aria-label="Task setup" className={classes.editor}>
          <Group justify="space-between">
            <Title order={2} size="h3">
              Set up your test
            </Title>
            <Button
              variant="light"
              onClick={() => {
                setAi(!ai)
                if (
                  !ai &&
                  !s &&
                  !create.isPending &&
                  choices.data?.profiles.length === 1 &&
                  !choices.data.blockers.length
                )
                  create.mutate(null)
              }}
            >
              {ai ? 'Hide AI generation' : 'Generate with AI'}
            </Button>
          </Group>
          {ai && (
            <GenerationPanel
              session={s}
              onSession={sessionSaved}
              reconnecting={stop.isPending || reconnectRequested || reopen.isPending}
              onReconnect={() =>
                stop.mutate(undefined, { onSuccess: () => setReconnectRequested(true) })
              }
            />
          )}
          <TextInput
            label="Test name"
            id={issueFieldId(issuePrefix, 'title')}
            error={fieldError('title')}
            value={value?.title ?? title}
            disabled={disabled}
            maxLength={200}
            onChange={(e) =>
              value && onChange
                ? onChange({ ...value, title: e.currentTarget.value })
                : setTitle(e.currentTarget.value)
            }
          />
          <TaskSteps
            value={sequence}
            issues={issues}
            issuePrefix={issuePrefix}
            issueFocus={issueFocus}
            disabled={disabled}
            canPick={!!ready && !!frame?.controls.length}
            pickingId={picking}
            onPick={(key) => {
              setMode('pick')
              setPicking(picking === key ? null : key)
            }}
            onChange={(next) => {
              setSequence(next)
              setPicking(null)
              submit.reset()
            }}
          />
          <Accordion variant="contained" value={detailsOpen} onChange={setDetailsOpen}>
            <Accordion.Item value="details">
              <Accordion.Control>
                Requirement and setup
                {fieldError('requirement') && (
                  <Text span c="red" size="sm">
                    {' '}
                    · Needs setup
                  </Text>
                )}
              </Accordion.Control>
              <Accordion.Panel>
                <Stack>
                  <Textarea
                    label="Expected behavior / requirement"
                    id={issueFieldId(issuePrefix, 'requirement')}
                    error={fieldError('requirement')}
                    value={value?.requirement ?? requirement}
                    onChange={(e) =>
                      value && onChange
                        ? onChange({ ...value, requirement: e.currentTarget.value })
                        : setRequirement(e.currentTarget.value)
                    }
                  />
                  {value && onChange && (
                    <>
                      <Textarea
                        label="Before the test"
                        value={value.preconditions.join('\n')}
                        onChange={(e) =>
                          onChange({
                            ...value,
                            preconditions: e.currentTarget.value.split('\n').filter(Boolean),
                          })
                        }
                      />
                      <BudgetFields
                        value={value.budget}
                        onChange={(budget) => onChange({ ...value, budget })}
                      />
                    </>
                  )}
                </Stack>
              </Accordion.Panel>
            </Accordion.Item>
          </Accordion>
          <Text size="xs" c="dimmed">
            {usesAi
              ? 'Uses AI for the explicitly selected Ask AI steps.'
              : 'Direct execution · no AI calls'}
          </Text>
          {runControl}
          <Group>
            <Button
              variant="subtle"
              disabled={
                !ready ||
                !sequenceReady(sequence) ||
                disabled ||
                (usesAi && !s?.profile.model) ||
                !!picking
              }
              loading={submit.isPending}
              onClick={() => run(sequence)}
            >
              {runControl ? 'Try actions (trial)' : 'Run test'}
            </Button>
            {!value && (
              <Button
                variant="light"
                loading={save.isPending}
                disabled={!sequence.actions.length}
                onClick={() =>
                  save.mutate({
                    mutation_id: crypto.randomUUID(),
                    source_task_id: null,
                    tests: [{ template_id: null, proposal_id: null, title, requirement, sequence }],
                  })
                }
              >
                Save as test
              </Button>
            )}
          </Group>
          {submit.isError && (
            <ErrorNotice
              error={submit.error}
              retry={() => submit.variables && submit.mutate(submit.variables)}
            />
          )}
          {save.isError && (
            <ErrorNotice
              error={save.error}
              retry={() => save.variables && save.mutate(save.variables)}
            />
          )}
          {save.data?.entry_ids.map((key) => (
            <Button variant="light" component={Link} to={href(`/tests/${appId}/${key}`)} key={key}>
              Open saved test
            </Button>
          ))}
          {s?.tasks
            .slice()
            .reverse()
            .map((task) => (
              <Card withBorder key={task.id} style={{ overflow: 'visible' }}>
                <Stack gap="sm">
                  <Group justify="space-between">
                    <Text fw={600}>{task.goal}</Text>
                    <Badge>
                      {task.generation
                        ? explorationLabel(task)
                        : `Session activity · ${task.state}`}
                    </Badge>
                  </Group>
                  {!task.generation && <Text size="sm">{task.message}</Text>}
                  {task.steps?.map((step, i) => (
                    <Text key={step.action_id} size="sm">
                      Step {i + 1}: {step.state} — {step.message}
                    </Text>
                  ))}
                  {task.progress && (
                    <ProposalReview key={`${task.id}:${task.state}`} task={task} appId={appId} />
                  )}
                </Stack>
              </Card>
            ))}
        </Stack>
        <aside aria-label="App preview" className={classes.preview}>
          <Stack>
            <Group justify="space-between">
              <Title order={2} size="h3">
                App preview
              </Title>
              {s && (
                <Badge color={disconnected ? 'gray' : undefined}>
                  {disconnected ? 'Disconnected' : s.state}
                </Badge>
              )}
            </Group>
            <Text size="sm" role="status">
              {previewMessage}
            </Text>
            {disconnected && s && (
              <Button loading={reopen.isPending} onClick={() => reopen.mutate(s.profile.id)}>
                {reopen.isPending ? 'Reconnecting…' : 'Reconnect'}
              </Button>
            )}
            {activeExploration && <DiscoveryActivity task={activeExploration} />}
            {!id && choices.data && !choices.data.blockers.length && (
              <>
                {choices.data.profiles.length > 1 && (
                  <Select
                    label="Device"
                    value={device}
                    data={choices.data.profiles.map((p) => ({ value: p.id, label: p.name }))}
                    onChange={setDevice}
                  />
                )}
                <Button
                  loading={create.isPending}
                  disabled={choices.data.profiles.length > 1 && !device}
                  onClick={() => create.mutate(device)}
                >
                  Open phone preview
                </Button>
              </>
            )}
            {s && !supportsDirect && s.state === 'ready' && (
              <Alert>
                This connection cannot run direct actions. Disconnect and reconnect to try again.
              </Alert>
            )}
            {create.isError && (
              <ErrorNotice error={create.error} retry={() => create.mutate(device)} />
            )}
            {phone.isError && (
              <ErrorNotice error={phone.error} retry={() => void phone.refetch()} />
            )}
            <Group>
              <Button
                size="xs"
                variant={mode === 'pick' ? 'filled' : 'light'}
                onClick={() => {
                  setMode('pick')
                  setTyping(null)
                }}
              >
                Pick target
              </Button>
              <Button
                size="xs"
                variant={mode === 'control' ? 'filled' : 'light'}
                disabled={!ready}
                onClick={() => {
                  setMode('control')
                  setPicking(null)
                }}
              >
                Control phone
              </Button>
            </Group>
            {mode === 'control' && (
              <>
                <Checkbox
                  label="Record interactions into this test"
                  checked={record}
                  onChange={(e) => setRecord(e.currentTarget.checked)}
                  disabled={disabled || sequence.actions.length >= 20}
                />
                <Group>
                  <Button
                    variant="light"
                    size="xs"
                    disabled={!ready}
                    onClick={() => control({ operation: 'back' })}
                  >
                    Back
                  </Button>
                  {(['up', 'down', 'left', 'right'] as const).map((direction) => (
                    <Button
                      size="xs"
                      variant="light"
                      key={direction}
                      disabled={!ready}
                      onClick={() => control({ operation: 'swipe', direction })}
                    >
                      Swipe {direction}
                    </Button>
                  ))}
                </Group>
              </>
            )}
            {picking && (
              <Alert>Pick the control for this step. Selecting it does not tap the app.</Alert>
            )}
            <Card withBorder radius="xl" className={classes.phone} p="sm">
              {frame ? (
                <div
                  style={{
                    position: 'relative',
                    width: `min(100%, calc((100dvh - 270px) * ${frame.width / frame.height}))`,
                    marginInline: 'auto',
                    lineHeight: 0,
                  }}
                >
                  <img
                    src={`data:image/png;base64,${frame.png_base64}`}
                    alt={
                      disconnected
                        ? 'Last captured screen of your Android app'
                        : 'Current screen of your Android app'
                    }
                    style={{ width: '100%', display: 'block', borderRadius: 16 }}
                  />
                  <DiscoveryTarget task={activeExploration} frame={frame} />
                  {ready &&
                    (mode === 'control' || picking) &&
                    frame.controls
                      .filter((c) => !!targetFor(c))
                      .map((c) => (
                        <button
                          key={c.id}
                          type="button"
                          aria-label={`${mode === 'control' ? 'Interact with' : 'Select'} ${c.label}`}
                          onClick={() => pick(c)}
                          style={{
                            position: 'absolute',
                            left: `${(100 * c.left) / frame.width}%`,
                            top: `${(100 * c.top) / frame.height}%`,
                            width: `${(100 * (c.right - c.left)) / frame.width}%`,
                            height: `${(100 * (c.bottom - c.top)) / frame.height}%`,
                            background: 'transparent',
                            border: '1px dashed #245b46',
                            cursor: mode === 'control' ? 'pointer' : 'crosshair',
                          }}
                        />
                      ))}
                </div>
              ) : (
                <Stack align="center" justify="center" className={classes.placeholder}>
                  {id && !disconnected && <Loader />}
                  <Text ta="center">
                    {disconnected
                      ? previewMessage
                      : (s?.message ?? 'Your app screen appears here after connecting.')}
                  </Text>
                </Stack>
              )}
            </Card>
            <Text size="xs" c="dimmed">
              {disconnected
                ? 'Last captured screen. Reconnect to see and control the app again.'
                : 'Latest device capture. Pick target binds a step; Control phone performs the action.'}
            </Text>
            {s && !['closed', 'quarantined'].includes(s.state) && (
              <Button
                color="red"
                variant="light"
                disabled={s.state === 'stopping'}
                loading={stop.isPending}
                onClick={() => stop.mutate()}
              >
                Stop session
              </Button>
            )}
            {stop.isError && <ErrorNotice error={stop.error} retry={() => stop.mutate()} />}
            {reopen.isError && (
              <ErrorNotice error={reopen.error} retry={() => s && reopen.mutate(s.profile.id)} />
            )}
          </Stack>
        </aside>
      </ResizableWorkspace>
      <Modal opened={!!typing} onClose={() => setTyping(null)} title="Enter text on the phone">
        <Stack>
          <Textarea
            label="Text to enter"
            value={text}
            maxLength={4000}
            onChange={(e) => setText(e.currentTarget.value)}
          />
          <Button
            disabled={!ready}
            onClick={() => {
              const target = typing && targetFor(typing)
              if (target) control({ operation: 'set_text', target, text })
              setTyping(null)
            }}
          >
            Enter text
          </Button>
        </Stack>
      </Modal>
    </Stack>
  )
}
