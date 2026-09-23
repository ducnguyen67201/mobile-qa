import { MantineProvider } from '@mantine/core'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { createMemoryRouter, RouterProvider } from 'react-router'
import { afterEach, expect, it, vi } from 'vitest'
import { routes } from '@/routes'
import { theme } from '@/theme'
import { app, appId, buildId, session, settings } from '@/test/fixtures'
import type { RunResponse } from '@/api/generated/types.gen'
const runId = '33333333-3333-4333-8333-333333333333'
const blockingRunId = '44444444-4444-4444-8444-444444444444'
function report(): RunResponse {
  return {
    id: runId,
    created_at: '2026-09-12T00:00:00Z',
    state: 'running',
    summary: 'Incomplete / review required',
    attempts: [],
    manifest: {
      app_id: appId,
      build_id: buildId,
      build_sha256: 'a'.repeat(64),
      build_bytes: 100,
      plan_version_id: runId,
      plan_hash: 'b'.repeat(64),
      environment_revision: 1,
      profile: {
        id: runId,
        name: 'Fixture phone',
        driver: 'fake',
        package: app.android_package,
        adapter: 'demo_persistence_v1',
        device_identity: 'synthetic',
        image: 'synthetic',
        model: 'none',
        qualified: true,
        qualification_reference: 'test-only',
        max_apk_bytes: 1000,
      },
      cases: [],
      budget: { duration_seconds: 600, max_steps: 30, artifact_bytes: 16777216 },
      diagnostic_retries: 0,
      exclusions: [],
    },
  }
}
function show() {
  const query = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } })
  render(
    <MantineProvider theme={theme} env="test">
      <QueryClientProvider client={query}>
        <RouterProvider
          router={createMemoryRouter(routes, {
            initialEntries: [`/runs/${runId}?workspace=${app.organization_id}`],
          })}
        />
      </QueryClientProvider>
    </MantineProvider>,
  )
}
afterEach(() => vi.unstubAllGlobals())
it.each([
  ['host_starting', 'Starting a device host for this run.'],
  [
    'host_draining',
    'The device host is finishing cleanup. Your run will start when a clean device is available.',
  ],
  [
    'pool_paused',
    'Device capacity is paused. An operator must resume it before this run can start.',
  ],
  ['waiting_for_slot', 'Waiting for a free, qualified device.'],
  ['profile_incompatible', 'No device slot is qualified for this run’s device profile.'],
] as const)('explains %s without implying execution has started', async (reason, message) => {
  const current = report()
  current.state = 'queued'
  current.queue_status = {
    reason,
    blocking_run_id: null,
    last_compatible_worker_at: null,
    wait_seconds: 15,
  }
  vi.stubGlobal(
    'fetch',
    vi.fn(async (request: Request) => {
      const path = new URL(request.url).pathname
      if (path.endsWith('/session')) return Response.json(session)
      if (path.endsWith('/settings')) return Response.json(settings)
      if (path === `/api/apps/${appId}`) return Response.json(app)
      if (path.endsWith(runId)) return Response.json(current)
      return Response.json(
        { code: 'not_found', message: 'No fixture', request_id: runId },
        { status: 404 },
      )
    }),
  )
  show()
  expect(await screen.findByText(message)).toBeInTheDocument()
  expect(screen.getByRole('button', { name: 'Cancel run' })).toBeEnabled()
})
it('shows simulated run and pending cancellation without claiming physical stop', async () => {
  let current = report()
  vi.stubGlobal(
    'fetch',
    vi.fn(async (request: Request) => {
      const path = new URL(request.url).pathname
      if (path.endsWith('/session')) return Response.json(session)
      if (path.endsWith('/settings')) return Response.json(settings)
      if (path === `/apps/${appId}` || path === `/api/apps/${appId}`) return Response.json(app)
      if (path.endsWith('/cancel')) {
        current = { ...current, state: 'cancel_requested' }
        return Response.json(current)
      }
      if (path.endsWith(runId)) return Response.json(current)
      throw new Error(`Unexpected fixture route ${path}`)
    }),
  )
  show()
  expect(await screen.findByText(/Simulated execution/)).toBeInTheDocument()
  await userEvent.click(screen.getByRole('button', { name: 'Cancel run' }))
  expect(await screen.findByText(/Waiting for your phone to stop/)).toBeInTheDocument()
  expect(screen.getByRole('button', { name: 'Cancel run' })).toBeDisabled()
})
it('shows cancellation separately from required device recovery', async () => {
  const current = report()
  current.state = 'recovery_required'
  current.cancel_requested = true
  vi.stubGlobal(
    'fetch',
    vi.fn(async (request: Request) => {
      const path = new URL(request.url).pathname
      if (path.endsWith('/session')) return Response.json(session)
      if (path.endsWith('/settings')) return Response.json(settings)
      if (path === `/apps/${appId}` || path === `/api/apps/${appId}`) return Response.json(app)
      if (path.endsWith(runId)) return Response.json(current)
      throw new Error(`Unexpected fixture route ${path}`)
    }),
  )
  show()
  expect(
    await screen.findByText('Cancellation recorded; device recovery required'),
  ).toBeInTheDocument()
  expect(
    screen.getByText(/reservation remains until an operator recovers the device/),
  ).toBeInTheDocument()
  expect(screen.getByRole('button', { name: 'Cancellation requested' })).toBeDisabled()
})
it('explains a queued run held for device recovery', async () => {
  const current = report()
  current.state = 'queued'
  current.queue_status = {
    reason: 'device_recovery_required',
    blocking_run_id: blockingRunId,
    last_compatible_worker_at: null,
    wait_seconds: 30,
  }
  vi.stubGlobal(
    'fetch',
    vi.fn(async (request: Request) => {
      const path = new URL(request.url).pathname
      if (path.endsWith('/session')) return Response.json(session)
      if (path.endsWith('/settings')) return Response.json(settings)
      if (path === `/apps/${appId}` || path === `/api/apps/${appId}`) return Response.json(app)
      if (path.endsWith(runId)) return Response.json(current)
      throw new Error(`Unexpected fixture route ${path}`)
    }),
  )
  show()
  const status = await screen.findByText('Waiting for device recovery')
  expect(screen.getByText(/A previous attempt still holds the phone/)).toBeInTheDocument()
  expect(screen.getByText(/No test has been evaluated/)).toBeInTheDocument()
  expect(screen.getByRole('link', { name: 'View the run requiring recovery' })).toHaveAttribute(
    'href',
    `/runs/${blockingRunId}?workspace=${app.organization_id}`,
  )
  expect(
    status.compareDocumentPosition(screen.getByRole('heading', { name: 'Run suite' })) &
      Node.DOCUMENT_POSITION_FOLLOWING,
  ).toBeTruthy()
})
it('explains when a polling worker cannot claim a run with its protocol', async () => {
  const current = report()
  current.state = 'queued'
  current.queue_status = {
    reason: 'worker_upgrade_required',
    last_compatible_worker_at: null,
    wait_seconds: 30,
  }
  vi.stubGlobal(
    'fetch',
    vi.fn(async (request: Request) => {
      const path = new URL(request.url).pathname
      if (path.endsWith('/session')) return Response.json(session)
      if (path.endsWith('/settings')) return Response.json(settings)
      if (path === `/apps/${appId}` || path === `/api/apps/${appId}`) return Response.json(app)
      if (path.endsWith(runId)) return Response.json(current)
      throw new Error(`Unexpected fixture route ${path}`)
    }),
  )
  show()
  expect(await screen.findByText(/newer execution protocol/)).toBeInTheDocument()
})
it('does not present malformed persisted results as a report', async () => {
  vi.stubGlobal(
    'fetch',
    vi.fn(async (request: Request) => {
      const path = new URL(request.url).pathname
      if (path.endsWith('/session')) return Response.json(session)
      if (path.endsWith('/settings')) return Response.json(settings)
      return Response.json({ state: 'finished', summary: 'Everything passed' })
    }),
  )
  show()
  expect(await screen.findByRole('button', { name: /Retry/i })).toBeInTheDocument()
  expect(screen.queryByText('Everything passed')).not.toBeInTheDocument()
})
it('shows the exact frozen model assignment used by the run', async () => {
  const current = report()
  current.manifest.resolved_model = {
    reference: { key: 'approved.navigation', revision: 3 },
    display_name: 'Approved navigation',
    provider: 'open_ai',
    provider_model: 'provider-model',
    capabilities: ['minitap_navigation'],
  }
  vi.stubGlobal(
    'fetch',
    vi.fn(async (request: Request) => {
      const path = new URL(request.url).pathname
      if (path.endsWith('/session')) return Response.json(session)
      if (path.endsWith('/settings')) return Response.json(settings)
      if (path === `/apps/${appId}` || path === `/api/apps/${appId}`) return Response.json(app)
      if (path.endsWith(runId)) return Response.json(current)
      throw new Error(`Unexpected fixture route ${path}`)
    }),
  )
  show()
  expect(await screen.findByText('Approved navigation')).toBeInTheDocument()
  expect(screen.getByText(/approved\.navigation@3.*provider-model/)).toBeInTheDocument()
})
it('does not warn about model context for direct-only historical runs', async () => {
  const current = report()
  vi.stubGlobal(
    'fetch',
    vi.fn(async (request: Request) => {
      const path = new URL(request.url).pathname
      if (path.endsWith('/session')) return Response.json(session)
      if (path.endsWith('/settings')) return Response.json(settings)
      if (path === `/apps/${appId}` || path === `/api/apps/${appId}`) return Response.json(app)
      if (path.endsWith(runId)) return Response.json(current)
      throw new Error(`Unexpected fixture route ${path}`)
    }),
  )
  show()
  expect(await screen.findByText('Build checksum')).toBeInTheDocument()
  expect(screen.queryByText('Legacy model context unavailable')).not.toBeInTheDocument()
})
