/** AI proposals are editable drafts. Accepting them never approves or runs a regression plan. */
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
import { generateTests, saveAuthoredTests } from '@/api/test-authoring'
import { useWorkspace } from '@/hooks/use-workspace'
import { ErrorNotice } from '@/components/app/feedback'
import { TaskSteps } from '@/components/task-session/task-steps'

export function GenerationPanel({
  session,
  onSession,
}: {
  session?: PhoneSession
  onSession: (s: PhoneSession) => void
}) {
  const [category, setCategory] = useState<CoverageKind>('smoke')
  const [journey, setJourney] = useState('')
  const [writes, setWrites] = useState(false)
  const [reuse, setReuse] = useState(false)
  const previous = session?.tasks
    .slice()
    .reverse()
    .find(
      (t) =>
        t.state === 'completed' &&
        t.generation?.journey === journey &&
        t.generation?.allow_writes === writes &&
        !!t.progress?.snapshots.length,
    )
  const start = useMutation({
    mutationFn: (r: GenerateTestsRequest) => generateTests(session!.app_id, r),
    onSuccess: onSession,
  })
  const ready = session?.state === 'ready' && session.protocol_version === 2
  const ai = !!session?.profile.model && session.profile.driver === 'minitap'
  return (
    <Card withBorder>
      <Stack>
        <Title order={2} size="h3">
          Generate tests with AI
        </Title>
        <Text size="sm" c="dimmed">
          Explore a small journey, then review named scenarios. Existing tests stay unchanged.
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
          placeholder="Create a task and make sure it remains after restarting"
          value={journey}
          maxLength={4000}
          onChange={(e) => setJourney(e.currentTarget.value)}
        />
        <Checkbox
          label="Allow form submission and test-data changes within this journey"
          checked={writes}
          onChange={(e) => setWrites(e.currentTarget.checked)}
        />
        <Text size="xs" c="dimmed">
          Up to 8 screen states, 12 actions, 3 minutes and 5 proposed tests. AI calls are limited;
          this does not discover every screen.
        </Text>
        {session && !ai && (
          <Alert>AI needs a configured model. You can still use templates and direct steps.</Alert>
        )}
        {previous && (
          <Checkbox
            label="Reuse captured screens from this session"
            checked={reuse}
            onChange={(e) => setReuse(e.currentTarget.checked)}
          />
        )}
        <Button
          loading={start.isPending}
          disabled={!ready || !ai || (writes && !journey.trim())}
          onClick={() =>
            session &&
            start.mutate({
              id: crypto.randomUUID(),
              session_id: session.id,
              expected_revision: session.revision ?? 0,
              category,
              journey,
              allow_writes: writes,
              reuse_job_id: reuse && previous ? previous.id : null,
            })
          }
        >
          Discover & generate
        </Button>
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
  const [selected, setSelected] = useState<string[]>([])
  const [confirmed, setConfirmed] = useState(false)
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
  return (
    <Stack>
      <Text size="sm">
        {progress.snapshots.length} screen states explored · {progress.usage.calls} AI calls ·{' '}
        {progress.usage.input_tokens + progress.usage.output_tokens} measured tokens
        {progress.usage.unknown_calls ? ' · some usage unavailable' : ''}
      </Text>
      {progress.gaps.map((gap, i) => (
        <Alert key={i}>{gap}</Alert>
      ))}
      <Group>
        {progress.snapshots.map((s) => (
          <img
            key={s.id}
            src={`data:image/png;base64,${s.frame.png_base64}`}
            alt={`Discovery source ${s.id}`}
            style={{ width: 90, borderRadius: 8 }}
          />
        ))}
      </Group>
      {proposals.map((p, index) => (
        <Card withBorder key={p.id}>
          <Stack>
            {existing.data?.items.some(
              (item) => item.title.trim().toLowerCase() === p.title.trim().toLowerCase(),
            ) && (
              <Alert color="orange">
                A test with this name already exists. Review it before saving another.
              </Alert>
            )}
            <Checkbox
              label="Save this test"
              checked={selected.includes(p.id)}
              onChange={(e) =>
                setSelected(
                  e.currentTarget.checked
                    ? [...selected, p.id]
                    : selected.filter((id) => id !== p.id),
                )
              }
            />
            <TextInput
              label="Suggested test name"
              value={p.title}
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
              onChange={(e) =>
                setProposals(
                  proposals.map((item, i) =>
                    i === index ? { ...item, requirement: e.currentTarget.value } : item,
                  ),
                )
              }
            />
            {p.questions.map((q, i) => (
              <Alert color="orange" key={i}>
                {q}
              </Alert>
            ))}
            <Text size="xs" c="dimmed">
              Based on {p.source_ids.length} captured screen states. Edit unresolved controls in the
              saved draft.
            </Text>
            <TaskSteps
              value={p.sequence}
              onChange={(sequence) =>
                setProposals(
                  proposals.map((item, i) => (i === index ? { ...item, sequence } : item)),
                )
              }
            />
          </Stack>
        </Card>
      ))}
      {!!proposals.length && (
        <>
          <Checkbox
            label="I reviewed and confirm the selected tests’ expected behavior"
            checked={confirmed}
            onChange={(e) => setConfirmed(e.currentTarget.checked)}
          />
          <Button
            disabled={!selected.length || !confirmed || save.isSuccess}
            loading={save.isPending}
            onClick={() =>
              save.mutate({
                mutation_id: crypto.randomUUID(),
                source_task_id: task.id,
                expectations_confirmed: confirmed,
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
            Save selected tests
          </Button>
        </>
      )}
      {save.isError && (
        <ErrorNotice
          error={save.error}
          retry={() => save.variables && save.mutate(save.variables)}
        />
      )}
      {save.data?.entry_ids.map((id, i) => (
        <Button key={id} component={Link} to={href(`/tests/${appId}/${id}`)} variant="light">
          Open saved test {i + 1}
        </Button>
      ))}
    </Stack>
  )
}
