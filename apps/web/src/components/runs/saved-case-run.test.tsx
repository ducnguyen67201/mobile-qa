import { MantineProvider } from '@mantine/core'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter } from 'react-router'
import { beforeEach, expect, it, vi } from 'vitest'
import { zRunResponse } from '@/api/generated/zod.gen'
import type { LibraryDraftResponse } from '@/api/generated/types.gen'
import fixture from '../../../../api/tests/fixtures/execution/comparison.json'
import { SavedCaseRun } from './saved-case-run'
const mocks = vi.hoisted(() => ({ create: vi.fn(), preview: vi.fn(), history: vi.fn() }))
vi.mock('@/hooks/use-workspace', () => ({
  useWorkspace: () => ({ workspaceId: 'workspace', href: (p: string) => p }),
}))
vi.mock('@/api/regression', () => ({
  caseRunPreviewQuery: (_app: string, body: unknown) => ({
    queryKey: ['preview', body],
    queryFn: mocks.preview,
  }),
  createCaseRun: mocks.create,
  historyQuery: () => ({ queryKey: ['history'], queryFn: mocks.history }),
}))
vi.mock('@/api/task-sessions', () => ({
  phoneOptionsQuery: () => ({
    queryKey: ['phone-options'],
    queryFn: async () => ({
      active_session: null,
      builds: [{ id: 'build', name: 'v1.1' }],
      profiles: [{ id: 'profile', name: 'Device' }],
    }),
  }),
  phoneQuery: () => ({ queryKey: ['phone'], queryFn: async () => null }),
  stopPhone: vi.fn(),
}))
vi.mock('@/api/runs', () => ({
  runQuery: () => ({ queryKey: ['run'], enabled: false, queryFn: async () => null }),
}))
beforeEach(() => {
  mocks.create.mockReset()
  mocks.preview.mockReset()
  mocks.history.mockReset()
  mocks.history.mockResolvedValue({ items: [] })
  mocks.preview.mockResolvedValue({
    blockers: [],
    environment_revision: 7,
    baselines: [],
    suggested_baseline_id: null,
  })
})
function show(save: () => Promise<LibraryDraftResponse>) {
  render(
    <MantineProvider env="test">
      <QueryClientProvider
        client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
      >
        <MemoryRouter>
          <SavedCaseRun appId="app" versionId="old-version" dirty save={save} />
        </MemoryRouter>
      </QueryClientProvider>
    </MantineProvider>,
  )
}
async function chooseBuild() {
  await userEvent.click(await screen.findByRole('combobox', { name: 'Build to test' }))
  await userEvent.click(await screen.findByRole('option', { name: 'v1.1' }))
}
it('saves then pins the returned version and retries a lost response with the same identity', async () => {
  const save = vi.fn().mockResolvedValue({ saved_version_id: 'new-version', issues: [] })
  mocks.create
    .mockRejectedValueOnce(new Error('Response lost'))
    .mockResolvedValueOnce(zRunResponse.parse(fixture))
  show(save)
  await chooseBuild()
  await userEvent.click(screen.getByRole('button', { name: 'Save & run' }))
  await waitFor(() => expect(mocks.create).toHaveBeenCalledTimes(1))
  expect(save).toHaveBeenCalledTimes(1)
  const original = mocks.create.mock.calls[0]!
  expect(original[1]).toMatchObject({
    case_version_id: 'new-version',
    build_id: 'build',
    profile_id: 'profile',
    environment_revision: 7,
    baseline_run_id: null,
  })
  await userEvent.click(await screen.findByRole('button', { name: 'Retry run' }))
  await waitFor(() => expect(mocks.create).toHaveBeenCalledTimes(2))
  expect(mocks.create.mock.calls[1]).toEqual(original)
  expect(save).toHaveBeenCalledTimes(1)
})
it('does not queue an incomplete save', async () => {
  show(
    vi.fn().mockResolvedValue({ saved_version_id: null, issues: [{ message: 'Missing target' }] }),
  )
  await chooseBuild()
  await userEvent.click(screen.getByRole('button', { name: 'Save & run' }))
  await screen.findByText('Complete the highlighted test fields before running.')
  expect(mocks.create).not.toHaveBeenCalled()
})
