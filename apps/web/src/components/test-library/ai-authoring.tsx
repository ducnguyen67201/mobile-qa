/** AI proposals are editable drafts. Saving them never runs a regression plan. */
import { useState } from 'react'
import {
  Alert,
  Button,
  Card,
  Checkbox,
  Group,
  Select,
  Stack,
  Text,
  Textarea,
  TextInput,
  Title,
} from '@mantine/core'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Link } from 'react-router'
import type {
  CoverageKind,
  GenerateTestsRequest,
  GenerationProposal,
  PhoneSession,
  PhoneTask,
  SaveAuthoredTestsRequest,
} from '@/api/generated/types.gen'
import { libraryQuery } from '@/api/test-library'
import { cancelTestGeneration, generateTests, saveAuthoredTests } from '@/api/test-authoring'
import { useWorkspace } from '@/hooks/use-workspace'
import { ErrorNotice } from '@/components/app/feedback'
import { DiscoveryActivity, actionLabel, discoveryGap } from './discovery-activity'
import { TaskSteps } from '@/components/task-session/task-steps'
import classes from './proposal-review.module.css'
import { hasModelCapability } from '@/lib/model-capabilities'

const defaultJourneys: Record<CoverageKind, string> = {
  smoke:
    'Explore one basic flow on this screen: enter sample test data in a visible input, use its primary button, and check the visible result. Use only test data; stop if sign-in or other setup is needed.',
  happy_path:
    'Complete one main user flow with sample test data and check its visible result. Stop if sign-in or other setup is needed.',
  validation:
    'Try one visible input with an empty or invalid test value and check the validation message. Stop if sign-in or other setup is needed.',
  persistence:
    'Create one item with sample test data, restart the app, and check whether the saved item remains. Stop if sign-in or other setup is needed.',
}

export function GenerationPanel({
  session,
  onSession,
  onReconnect,
  reconnecting = false,
}: {
  session?: PhoneSession
  onSession: (s: PhoneSession) => void
  onReconnect?: () => void
  reconnecting?: boolean
}) {
  const [category, setCategory] = useState<CoverageKind>('smoke')
  const [journey, setJourney] = useState('')
  const [writes, setWrites] = useState(false)
  const [reuse, setReuse] = useState(false)
  const effectiveJourney = journey.trim() || defaultJourneys[category]
  const previous = session?.tasks
    .slice()
    .reverse()
    .find(
      (t) =>
        t.state === 'completed' &&
        t.generation?.engine === 'minitap_v1' &&
        t.generation?.journey === effectiveJourney &&
        t.generation?.allow_writes === writes &&
        !!t.progress?.snapshots.length,
    )
  const start = useMutation({
    mutationFn: (r: GenerateTestsRequest) => generateTests(session!.app_id, r),
    onSuccess: onSession,
  })
  const ready = session?.state === 'ready' && (session.protocol_version ?? 0) >= 3
  const active = session?.tasks.find(
    (t) => t.generation && (t.state === 'queued' || t.state === 'acting'),
  )
  const stop = useMutation({
    mutationFn: () => cancelTestGeneration(session!.app_id, active!.id),
    onSuccess: onSession,
  })
  const ai =
    session?.profile.driver === 'minitap' &&
    hasModelCapability(session.resolved_model, 'minitap_navigation') &&
    hasModelCapability(session.resolved_model, 'structured_authoring')
  return (
    <Card withBorder>
      <Stack>
        <Title order={2} size="h3">
          Generate tests with AI
        </Title>
        <Text size="sm" c="dimmed">
          AI explores your app and suggests named tests. Saved actions replay without AI.
        </Text>
        <Select
          label="Coverage"
          value={category}
          data={[
            { value: 'smoke', label: 'Smoke tests' },
            { value: 'happy_path', label: 'Happy paths' },
            { value: 'validation', label: 'Input validation' },
            { value: 'persistence', label: 'Saved data and restart' },
          ]}
          onChange={(v) => {
            if (v === 'smoke' || v === 'happy_path' || v === 'validation' || v === 'persistence')
              setCategory(v)
          }}
        />
        <Textarea
          label="What should we explore? (optional)"
          placeholder={defaultJourneys[category]}
          value={journey}
          maxLength={4000}
          onChange={(e) => setJourney(e.currentTarget.value)}
        />
        {!journey.trim() && <Text size="sm">Default task: {effectiveJourney}</Text>}
        <Checkbox
          label="Allow AI to tap, type and change test data"
          checked={writes}
          onChange={(e) => setWrites(e.currentTarget.checked)}
        />
        {!writes && (
          <Text size="sm">
            Enable interactions to record test actions. No AI calls start until you allow this and
            click Explore.
          </Text>
        )}
        <Text size="xs" c="dimmed">
          Up to 8 screen states, 12 actions, 3 minutes and 5 proposed tests. AI calls are limited;
          this does not discover every screen.
        </Text>
        {session && !ai && (
          <Alert>AI needs a configured model. You can still use templates and direct steps.</Alert>
        )}
        {(session?.state === 'queued' || session?.state === 'preparing') && (
          <Text size="sm" c="dimmed" role="status">
            Waiting for the phone to be ready. You can fill in your exploration instructions now.
          </Text>
        )}
        {previous && (
          <Checkbox
            label="Draft again from the recorded flow (no new exploration)"
            checked={reuse}
            onChange={(e) => setReuse(e.currentTarget.checked)}
          />
        )}
        <Button
          loading={start.isPending}
          disabled={!ready || !ai || !writes || !!active}
          onClick={() =>
            session &&
            start.mutate({
              engine: 'minitap_v1',
              id: crypto.randomUUID(),
              session_id: session.id,
              expected_revision: session.revision ?? 0,
              category,
              journey: effectiveJourney,
              allow_writes: writes,
              reuse_job_id: reuse && previous ? previous.id : null,
            })
          }
        >
          {reuse && previous ? 'Draft from recorded flow' : 'Explore with AI'}
        </Button>
        {active && (
          <Button
            variant="light"
            color="red"
            loading={stop.isPending}
            onClick={() => stop.mutate()}
          >
            Stop exploration
          </Button>
        )}
        {stop.isError && <ErrorNotice error={stop.error} retry={() => stop.mutate()} />}
        {session && session.state === 'ready' && (session.protocol_version ?? 0) < 3 && (
          <Alert>
            <Stack gap="xs">
              <Text size="sm">
                Your phone is connected. AI exploration isn’t available in this session. You can
                keep using direct actions or try a new connection.
              </Text>
              {onReconnect && (
                <Button variant="light" loading={reconnecting} onClick={onReconnect}>
                  Reconnect phone
                </Button>
              )}
            </Stack>
          </Alert>
        )}
        {start.isError && (
          <ErrorNotice
            error={start.error}
            retry={() => start.variables && start.mutate(start.variables)}
          />
        )}
      </Stack>
    </Card>
  )
}
export function ProposalReview({ task, appId }: { task: PhoneTask; appId: string }) {
  const progress = task.progress
  const [proposals, setProposals] = useState<GenerationProposal[]>(progress?.proposals ?? [])
  const [selected, setSelected] = useState<string[]>(() =>
    (progress?.proposals ?? []).map((p) => p.id),
  )
  const { href, workspaceId } = useWorkspace()
  const existing = useQuery({
    ...libraryQuery(workspaceId ?? '', appId, { kind: 'case' }),
    enabled: !!progress?.proposals.length,
  })
  const client = useQueryClient()
  const save = useMutation({
    mutationFn: (body: SaveAuthoredTestsRequest) => saveAuthoredTests(appId, body),
    onSuccess: () => void client.invalidateQueries({ queryKey: ['test-library'] }),
  })
  if (!progress) return null
  const locked = save.isPending || save.isSuccess
  return (
    <Stack gap="sm">
      {!!proposals.length && (
        <div className={classes.saveBar}>
          <Group justify="space-between" gap="xs">
            <Text size="sm" fw={600}>
              {save.isSuccess
                ? 'Saved'
                : `${selected.length} ${selected.length === 1 ? 'test' : 'tests'} selected`}
            </Text>
            <Button
              size="sm"
              disabled={!selected.length || save.isSuccess}
              loading={save.isPending}
              onClick={() =>
                save.mutate({
                  mutation_id: crypto.randomUUID(),
                  source_task_id: task.id,
                  tests: proposals
                    .filter((p) => selected.includes(p.id))
                    .map((p) => ({
                      template_id: null,
                      proposal_id: p.id,
                      title: p.title,
                      requirement: p.requirement,
                      sequence: p.sequence,
                    })),
                })
              }
            >
              {save.isSuccess
                ? 'Saved'
                : `Save ${selected.length === 1 ? 'test' : `${selected.length} tests`}`}
            </Button>
          </Group>
          <Text size="xs" c="dimmed" mt={4}>
            Save the selected tests. You can edit them later.
          </Text>
        </div>
      )}
      {save.isError && (
        <ErrorNotice
          error={save.error}
          retry={() => save.variables && save.mutate(save.variables)}
        />
      )}
      {save.data && (
        <Group gap="sm">
          {save.data.entry_ids.map((id, i) => (
            <Text
              size="sm"
              key={id}
              component={Link}
              to={href(`/tests/${appId}/${id}`)}
              td="underline"
            >
              Open saved test{save.data!.entry_ids.length > 1 ? ` ${i + 1}` : ''}
            </Text>
          ))}
        </Group>
      )}
      {!proposals.length && <DiscoveryActivity task={task} />}
      {progress.gaps.map((gap, i) => (
        <Alert key={i}>{discoveryGap(gap)}</Alert>
      ))}
      {proposals.map((p, index) => (
        <div key={p.id} className={classes.proposal}>
          <Stack gap="xs">
            <Group gap="xs" wrap="nowrap" align="flex-start">
              {proposals.length > 1 && (
                <Checkbox
                  aria-label={`Include ${p.title}`}
                  checked={selected.includes(p.id)}
                  disabled={locked}
                  onChange={(e) =>
                    setSelected(
                      e.currentTarget.checked
                        ? [...selected, p.id]
                        : selected.filter((id) => id !== p.id),
                    )
                  }
                />
              )}
              <div className={classes.heading}>
                <Text fw={600} size="sm">
                  {p.title}
                </Text>
                <Text size="xs" c="dimmed">
                  {p.sequence.actions.length} actions · {p.sequence.checks.length} checks
                </Text>
              </div>
              {existing.data?.items.some(
                (item) => item.title.trim().toLowerCase() === p.title.trim().toLowerCase(),
              ) && (
                <Text size="xs" className={classes.duplicate}>
                  Name already exists
                </Text>
              )}
            </Group>
            <Text size="sm" c="dimmed">
              {p.requirement}
            </Text>
            <ol aria-label="Suggested test actions" className={classes.actions}>
              {p.sequence.actions.map((action) => (
                <li key={action.id}>
                  <Text size="sm">
                    {action.command
                      ? actionLabel(action.command)
                      : action.instruction || 'Capture a checkpoint'}
                  </Text>
                  {p.sequence.checks
                    .filter((check) => check.checkpoint_id === action.checkpoint_id)
                    .map((check) => (
                      <Text key={check.id} size="xs" c="dimmed">
                        {check.required ? 'Check' : 'Optional check'}: {check.description} —{' '}
                        {check.method === 'ui_element_presence_v1'
                          ? `present = ${check.expected}${check.text_filter ? `, text “${check.text_filter}”` : ''}`
                          : `${check.property} = “${check.expected}”`}
                      </Text>
                    ))}
                </li>
              ))}
            </ol>
            {!!p.questions.length && (
              <details className={`${classes.details} ${classes.notes}`}>
                <summary>
                  Review notes ({p.questions.length}{' '}
                  {p.questions.length === 1 ? 'question' : 'questions'})
                </summary>
                <ul className={classes.questions}>
                  {p.questions.map((question, i) => (
                    <li key={i}>{question}</li>
                  ))}
                </ul>
              </details>
            )}
            <details className={classes.details}>
              <summary>Edit test</summary>
              <Stack gap="sm" mt="sm">
                <TextInput
                  label="Test name"
                  value={p.title}
                  disabled={locked}
                  onChange={(e) =>
                    setProposals(
                      proposals.map((item, i) =>
                        i === index ? { ...item, title: e.currentTarget.value } : item,
                      ),
                    )
                  }
                />
                <Textarea
                  label="Expected behavior / requirement"
                  value={p.requirement}
                  disabled={locked}
                  onChange={(e) =>
                    setProposals(
                      proposals.map((item, i) =>
                        i === index ? { ...item, requirement: e.currentTarget.value } : item,
                      ),
                    )
                  }
                />
                <TaskSteps
                  value={p.sequence}
                  disabled={locked}
                  onChange={(sequence) =>
                    setProposals(
                      proposals.map((item, i) => (i === index ? { ...item, sequence } : item)),
                    )
                  }
                />
                <Text size="xs" c="dimmed">
                  Edits have not been replayed.
                </Text>
              </Stack>
            </details>
          </Stack>
        </div>
      ))}
      <details className={classes.details}>
        <summary>Exploration details · {progress.snapshots.length} screens</summary>
        <Stack gap="sm" mt="sm">
          {!!proposals.length && <DiscoveryActivity task={task} />}
          <Text size="xs" c="dimmed">
            {progress.usage.calls} AI calls ·{' '}
            {progress.usage.input_tokens + progress.usage.output_tokens} measured tokens
            {progress.usage.unknown_calls ? ' · some usage unavailable' : ''}
          </Text>
          <div className={classes.screenshots}>
            {progress.snapshots.map((s, i) => (
              <img
                key={s.id}
                src={`data:image/png;base64,${s.frame.png_base64}`}
                alt={`Explored screen ${i + 1}`}
              />
            ))}
          </div>
        </Stack>
      </details>
    </Stack>
  )
}
