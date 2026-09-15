/** Review and explicit confirmation stay intact when saving is reduced to one button. */
import { MantineProvider } from '@mantine/core'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter } from 'react-router'
import { beforeEach, expect, it, vi } from 'vitest'
import type { GenerationProposal, PhoneTask } from '@/api/generated/types.gen'
import { saveAuthoredTests } from '@/api/test-authoring'
import { libraryCase } from '@/test/library-fixtures'
import { ProposalReview } from './ai-authoring'

vi.mock('@/hooks/use-workspace', () => ({
  useWorkspace: () => ({ workspaceId: 'workspace', href: (path: string) => path }),
}))
vi.mock('@/api/test-library', () => ({
  libraryQuery: () => ({
    queryKey: ['library'],
    queryFn: async () => ({ items: [{ title: 'Save a task' }] }),
  }),
}))
vi.mock('@/api/test-authoring', () => ({ saveAuthoredTests: vi.fn() }))
const proposal: GenerationProposal = {
  id: 'proposal-1',
  title: 'Save a task',
  requirement: 'The saved task is visible.',
  category: 'smoke',
  questions: [],
  source_ids: ['screen'],
  path_ids: ['action'],
  sequence: {
    actions: [
      {
        id: 'action',
        checkpoint_id: 'saved',
        kind: 'direct',
        instruction: '',
        command: { operation: 'tap', target: { by: 'resource_id', value: 'app:id/save' } },
      },
    ],
    checks: [
      {
        ...libraryCase.checks[0]!,
        id: 'check',
        checkpoint_id: 'saved',
        method: 'ui_property_equals_v1',
        description: 'Saved text',
        property: 'text',
        expected: 'Hello',
      },
    ],
  },
}
function show(proposals = [proposal]) {
  const task: PhoneTask = {
    id: 'task',
    goal: 'Generate Smoke tests',
    state: 'completed',
    message: '',
    control: null,
    progress: {
      state: 'ready',
      proposals,
      gaps: [],
      trace: [],
      journal: [],
      usage: { calls: 2, input_tokens: 100, output_tokens: 20, unknown_calls: 0 },
      snapshots: [
        {
          id: 'screen',
          frame: { id: 'frame', width: 1080, height: 1920, controls: [], png_base64: '' },
        },
      ],
    },
  }
  return render(
    <MantineProvider>
      <QueryClientProvider
        client={new QueryClient({ defaultOptions: { mutations: { retry: false } } })}
      >
        <MemoryRouter>
          <ProposalReview task={task} appId="app" />
        </MemoryRouter>
      </QueryClientProvider>
    </MantineProvider>,
  )
}
beforeEach(() => {
  vi.mocked(saveAuthoredTests).mockReset()
  vi.mocked(saveAuthoredTests).mockResolvedValue({ entry_ids: ['saved'] })
})
it('shows actions and checks with collapsed evidence and saves one test on explicit confirmation', async () => {
  show()
  expect(screen.getByRole('list', { name: 'Suggested test actions' })).toBeInTheDocument()
  expect(screen.getByText('Check: Saved text — text = “Hello”')).toBeInTheDocument()
  expect(
    screen.getByText('Exploration details · 1 screens').closest('details'),
  ).not.toHaveAttribute('open')
  expect(screen.getByText('Edit test').closest('details')).not.toHaveAttribute('open')
  expect(screen.queryByRole('checkbox')).not.toBeInTheDocument()
  expect(saveAuthoredTests).not.toHaveBeenCalled()
  await userEvent.click(screen.getByRole('button', { name: 'Confirm & save test' }))
  await waitFor(() =>
    expect(saveAuthoredTests).toHaveBeenCalledWith(
      'app',
      expect.objectContaining({
        expectations_confirmed: true,
        source_task_id: 'task',
        tests: [expect.objectContaining({ proposal_id: proposal.id, sequence: proposal.sequence })],
      }),
    ),
  )
  expect(await screen.findByRole('link', { name: 'Open saved test' })).toHaveAttribute(
    'href',
    '/tests/app/saved',
  )
  expect(screen.getByRole('button', { name: 'Saved' })).toBeDisabled()
})
it('lets the tester deselect suggestions and sends only the selected test', async () => {
  show([proposal, { ...proposal, id: 'proposal-2', title: 'Another test' }])
  await userEvent.click(screen.getByRole('checkbox', { name: 'Include Another test' }))
  await userEvent.click(screen.getByRole('checkbox', { name: 'Include Save a task' }))
  expect(screen.getByRole('button', { name: 'Confirm & save 0 tests' })).toBeDisabled()
  await userEvent.click(screen.getByRole('checkbox', { name: 'Include Save a task' }))
  await userEvent.click(screen.getByRole('button', { name: 'Confirm & save test' }))
  await waitFor(() => expect(saveAuthoredTests).toHaveBeenCalledTimes(1))
  expect(
    vi.mocked(saveAuthoredTests).mock.calls[0]?.[1].tests.map((test) => test.proposal_id),
  ).toEqual(['proposal-1'])
})
it('saves an edited name and preserves the draft when saving fails', async () => {
  vi.mocked(saveAuthoredTests).mockRejectedValue(new Error('Save unavailable'))
  show()
  await userEvent.click(screen.getByText('Edit test'))
  const input = screen.getByRole('textbox', { name: 'Test name' })
  await userEvent.clear(input)
  await userEvent.type(input, 'My smoke test')
  await userEvent.click(screen.getByRole('button', { name: 'Confirm & save test' }))
  await waitFor(() => expect(saveAuthoredTests).toHaveBeenCalledTimes(1))
  expect(vi.mocked(saveAuthoredTests).mock.calls[0]?.[1].tests[0]?.title).toBe('My smoke test')
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Confirm & save test' })).toBeEnabled(),
  )
  expect(input).toHaveValue('My smoke test')
  expect(screen.queryByRole('link', { name: 'Open saved test' })).not.toBeInTheDocument()
})

it('keeps duplicate notice and checks visible while grouping every question into one disclosure', async () => {
  const questions = [
    'What happens for duplicate tasks?',
    'Should the input clear?',
    'Is there a maximum length?',
    'What happens for an empty name?',
    'Should saving show feedback?',
  ]
  show([{ ...proposal, questions }])
  const notes = screen.getByText('Review notes (5 questions)')
  const disclosure = notes.closest('details')
  expect(disclosure).not.toHaveAttribute('open')
  for (const question of questions)
    expect(screen.getByText(question).closest('details')).toBe(disclosure)
  expect(screen.getByText('Check: Saved text — text = “Hello”').closest('details')).toBeNull()
  const duplicate = await screen.findByText('Name already exists')
  expect(duplicate.closest('details')).toBeNull()
  await userEvent.click(notes)
  expect(disclosure).toHaveAttribute('open')
  for (const question of questions) expect(screen.getByText(question)).toBeVisible()
  expect(screen.getByRole('button', { name: 'Confirm & save test' })).toBeEnabled()
})
