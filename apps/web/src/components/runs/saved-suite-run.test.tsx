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

const mocks = vi.hoisted(() => ({ create: vi.fn(), preview: vi.fn(), quote: vi.fn() }))
vi.mock('@/api/commercial', () => ({ createCommercialCheckQuote: mocks.quote }))
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
vi.mock('@/api/setup', () => ({
  buildQuery: (_app: string, id: string) => ({
    queryKey: ['build', id],
    queryFn: async () => ({ sha256: 'a'.repeat(64) }),
  }),
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
  mocks.quote.mockReset()
  mocks.quote.mockResolvedValue({
    id: 'quote',
    app_id: 'app',
    kind: 'saved_suite',
    build_id: 'build',
    source_version_id: 'suite-version',
    profile_id: 'profile',
    case_count: 1,
    amount_cents: 12500,
    maximum_credits: null,
    credits_after_authorization: null,
    currency: 'USD',
    checks_after_authorization: 1,
    check_cap: 8,
    expires_at: new Date(Date.now() + 600_000).toISOString(),
  })
  mocks.preview.mockResolvedValue({
    blockers: [],
    environment_revision: 7,
    baselines: [],
    suggested_baseline_id: null,
  })
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
async function chooseSetup() {
  await userEvent.click(await screen.findByRole('combobox', { name: 'Build to test' }))
  await userEvent.click(await screen.findByRole('option', { name: 'v1.1' }))
  await userEvent.click(screen.getByRole('combobox', { name: 'Test device' }))
  await userEvent.click(await screen.findByRole('option', { name: 'Device' }))
  expect(await screen.findByText(/Build checksum:/)).toBeInTheDocument()
}

it('quotes the saved suite before authorization and opens its live run map', async () => {
  const run = zRunResponse.parse(fixture)
  mocks.create.mockResolvedValue(run)
  show()
  expect(screen.getByRole('button', { name: 'Review check price' })).toBeDisabled()
  await chooseSetup()
  await userEvent.click(screen.getByRole('button', { name: 'Review check price' }))
  expect(mocks.create).not.toHaveBeenCalled()
  await userEvent.click(await screen.findByRole('button', { name: 'Authorize check and run' }))
  await waitFor(() => expect(mocks.create).toHaveBeenCalledTimes(1))
  expect(mocks.create.mock.calls[0]![1]).toEqual({
    suite_version_id: 'suite-version',
    build_id: 'build',
    profile_id: 'profile',
    environment_revision: 7,
    baseline_run_id: null,
  })
  expect(await screen.findByText(`/runs/${run.id}`)).toBeInTheDocument()
})

it('saves a changed sequence and retries the exact queued request after a lost response', async () => {
  const save = vi.fn().mockResolvedValue({ saved_version_id: 'new-suite-version', issues: [] })
  mocks.create
    .mockRejectedValueOnce(new Error('Response lost'))
    .mockResolvedValueOnce(zRunResponse.parse(fixture))
  show({ dirty: true, save })
  await chooseSetup()
  await userEvent.click(await screen.findByRole('button', { name: 'Save & review check price' }))
  await userEvent.click(await screen.findByRole('button', { name: 'Authorize check and run' }))
  await waitFor(() => expect(mocks.create).toHaveBeenCalledTimes(1))
  const first = mocks.create.mock.calls[0]!
  expect(first[1]).toMatchObject({ suite_version_id: 'new-suite-version' })
  await userEvent.click(await screen.findByRole('button', { name: 'Authorize check and run' }))
  await waitFor(() => expect(mocks.create).toHaveBeenCalledTimes(2))
  expect(mocks.create.mock.calls[1]).toEqual(first)
  expect(save).toHaveBeenCalledTimes(1)
})

it('offers a suggestion without selecting a baseline for the tester', async () => {
  mocks.preview.mockResolvedValue({
    blockers: [],
    environment_revision: 7,
    baselines: [
      {
        id: 'shown-run',
        build_id: 'old-build',
        build_label: 'v1.0',
        created_at: '2026-09-20T12:00:00Z',
        compatible: true,
        reason: 'Same suite',
      },
    ],
    suggested_baseline_id: 'shown-run',
  })
  mocks.create.mockResolvedValue(zRunResponse.parse(fixture))
  show()
  await chooseSetup()
  expect(await screen.findByText(/Suggested baseline: v1.0/)).toBeInTheDocument()
  expect(screen.getByRole('combobox', { name: 'Compare with' })).toHaveValue(
    'None — establish a first result',
  )
  await userEvent.click(screen.getByRole('button', { name: 'Review check price' }))
  await userEvent.click(await screen.findByRole('button', { name: 'Authorize check and run' }))
  await waitFor(() => expect(mocks.create).toHaveBeenCalledTimes(1))
  expect(mocks.create.mock.calls[0]![1].baseline_run_id).toBeNull()
})

it('pins the explicitly selected suite baseline even when the readiness suggestion changes', async () => {
  mocks.preview.mockResolvedValue({
    blockers: [],
    environment_revision: 7,
    baselines: [
      {
        id: 'shown-run',
        build_id: 'old-build',
        build_label: 'v1.0',
        created_at: '2026-09-20T12:00:00Z',
        compatible: true,
        reason: 'Same suite',
      },
    ],
    suggested_baseline_id: 'shown-run',
  })
  mocks.create.mockResolvedValue(zRunResponse.parse(fixture))
  show()
  await chooseSetup()
  await userEvent.click(await screen.findByRole('combobox', { name: 'Compare with' }))
  await userEvent.click(
    await screen.findByRole('option', {
      name: `v1.0 · ${new Date('2026-09-20T12:00:00Z').toLocaleString()}`,
    }),
  )
  mocks.preview.mockResolvedValue({
    blockers: [],
    environment_revision: 7,
    baselines: [],
    suggested_baseline_id: 'newer-run',
  })
  await userEvent.click(screen.getByRole('button', { name: 'Review check price' }))
  await userEvent.click(await screen.findByRole('button', { name: 'Authorize check and run' }))
  await waitFor(() => expect(mocks.create).toHaveBeenCalledTimes(1))
  expect(mocks.create.mock.calls[0]![1].baseline_run_id).toBe('shown-run')
})
