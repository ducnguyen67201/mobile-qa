import { MantineProvider } from '@mantine/core'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter, useLocation } from 'react-router'
import { beforeEach, expect, it, vi } from 'vitest'
import { zRunResponse } from '@/api/generated/zod.gen'
import type { LibraryDraftResponse } from '@/api/generated/types.gen'
import fixture from '../../../../api/tests/fixtures/execution/comparison.json'
import { SavedSuiteRun } from './saved-suite-run'

const mocks = vi.hoisted(() => ({ create: vi.fn(), preview: vi.fn() }))
vi.mock('@/hooks/use-workspace', () => ({
  useWorkspace: () => ({ workspaceId: 'workspace', href: (path: string) => path }),
}))
vi.mock('@/api/regression', () => ({
  suiteRunPreviewQuery: (_app: string, body: unknown) => ({
    queryKey: ['suite-preview', body],
    queryFn: mocks.preview,
  }),
  createSuiteRun: mocks.create,
}))
vi.mock('@/api/task-sessions', () => ({
  phoneOptionsQuery: () => ({
    queryKey: ['phone-options'],
    queryFn: async () => ({
      active_session: null,
      builds: [{ id: 'build', name: 'v1.1' }],
      profiles: [{ id: 'profile', name: 'Device', model_available: true }],
    }),
  }),
  phoneQuery: () => ({ queryKey: ['phone'], queryFn: async () => null }),
  stopPhone: vi.fn(),
}))

beforeEach(() => {
  mocks.create.mockReset()
  mocks.preview.mockReset()
  mocks.preview.mockResolvedValue({ blockers: [], environment_revision: 7 })
})

function Location() {
  return <span>{useLocation().pathname}</span>
}

function show({
  dirty = false,
  versionId = 'suite-version',
  save,
}: {
  dirty?: boolean
  versionId?: string | null
  save?: () => Promise<LibraryDraftResponse>
} = {}) {
  render(
    <MantineProvider env="test">
      <QueryClientProvider
        client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
      >
        <MemoryRouter>
          <SavedSuiteRun
            appId="app"
            versionId={versionId}
            dirty={dirty}
            save={save}
            profiles={[
              {
                id: 'profile',
                name: 'Device',
                qualified: true,
                driver: 'fake',
                package: 'ai.mobileqa.demo',
                adapter: 'demo_persistence_v1',
                model_available: true,
              },
            ]}
          />
          <Location />
        </MemoryRouter>
      </QueryClientProvider>
    </MantineProvider>,
  )
}

it('queues the saved suite in one click and opens its live run map', async () => {
  const run = zRunResponse.parse(fixture)
  mocks.create.mockResolvedValue(run)
  show()
  await userEvent.click(await screen.findByRole('button', { name: 'Run sequence' }))
  await waitFor(() => expect(mocks.create).toHaveBeenCalledTimes(1))
  expect(mocks.create.mock.calls[0]![1]).toEqual({
    suite_version_id: 'suite-version',
    build_id: 'build',
    profile_id: 'profile',
    environment_revision: 7,
  })
  expect(await screen.findByText(`/runs/${run.id}`)).toBeInTheDocument()
})

it('saves a changed sequence and retries the exact queued request after a lost response', async () => {
  const save = vi.fn().mockResolvedValue({ saved_version_id: 'new-suite-version', issues: [] })
  mocks.create
    .mockRejectedValueOnce(new Error('Response lost'))
    .mockResolvedValueOnce(zRunResponse.parse(fixture))
  show({ dirty: true, save })
  await userEvent.click(await screen.findByRole('button', { name: 'Save & run' }))
  await waitFor(() => expect(mocks.create).toHaveBeenCalledTimes(1))
  const first = mocks.create.mock.calls[0]!
  expect(first[1]).toMatchObject({ suite_version_id: 'new-suite-version' })
  await userEvent.click(await screen.findByRole('button', { name: 'Retry run' }))
  await waitFor(() => expect(mocks.create).toHaveBeenCalledTimes(2))
  expect(mocks.create.mock.calls[1]).toEqual(first)
  expect(save).toHaveBeenCalledTimes(1)
})
