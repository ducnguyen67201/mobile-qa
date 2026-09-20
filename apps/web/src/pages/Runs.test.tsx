import { MantineProvider } from '@mantine/core'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { createMemoryRouter, RouterProvider } from 'react-router'
import { afterEach, expect, it, vi } from 'vitest'
import { routes } from '@/routes'
import { theme } from '@/theme'
import { app, appId, build, orgId, session, settings, timestamp } from '@/test/fixtures'
import { entryId, libraryCase, versionId } from '@/test/library-fixtures'

const runId = '55555555-5555-4555-8555-555555555555'
const taskId = '66666666-6666-4666-8666-666666666666'

function durableRun() {
  return {
    id: runId,
    baseline_run_id: null,
    state: 'finished',
    summary: 'Required checks passed',
    created_at: timestamp,
    attempts: [],
    manifest: {
      app_id: appId,
      build_id: build.id,
      build_sha256: build.sha256,
      build_bytes: build.byte_size,
      source: {
        version: 1,
        kind: 'saved_case',
        case_version_id: versionId,
        content_hash: 'a'.repeat(64),
      },
      environment_revision: 1,
      profile: {
        id: taskId,
        name: 'Fixture phone',
        driver: 'fake',
        package: libraryCase.package,
        adapter: libraryCase.adapter,
        device_identity: 'fixture',
        image: 'fixture',
        model: 'none',
        qualified: true,
        qualification_reference: 'fixture-only',
        max_apk_bytes: 1048576,
      },
      cases: [
        {
          definition_id: versionId,
          content_hash: 'a'.repeat(64),
          data_variant: 'default',
          required: true,
          case: libraryCase,
        },
      ],
      budget: libraryCase.budget,
      diagnostic_retries: 0,
      exclusions: [],
    },
  }
}

function show() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } })
  render(
    <MantineProvider theme={theme} env="test">
      <QueryClientProvider client={client}>
        <RouterProvider
          router={createMemoryRouter(routes, {
            initialEntries: [`/runs?workspace=${orgId}`],
          })}
        />
      </QueryClientProvider>
    </MantineProvider>,
  )
}

afterEach(() => vi.unstubAllGlobals())

it('labels durable saved tests and trials and keeps legacy activity behind its filter', async () => {
  const filters: string[] = []
  vi.stubGlobal(
    'fetch',
    vi.fn(async (request: Request) => {
      const url = new URL(request.url)
      if (url.pathname.endsWith('/session')) return Response.json(session)
      if (url.pathname.endsWith('/settings')) return Response.json(settings)
      if (url.pathname === '/api/apps')
        return Response.json({
          items: [{ ...app, environment_name: app.environment.name }],
          next_cursor: null,
        })
      if (url.pathname.endsWith('/run-history')) {
        const filter = url.searchParams.get('filter') ?? ''
        filters.push(filter)
        if (filter === 'legacy')
          return Response.json({
            items: [
              {
                kind: 'legacy_session_activity',
                stable_id: `phone:${taskId}`,
                created_at: timestamp,
                task_id: taskId,
                title: 'Historical preview action',
                state: 'completed',
                message: 'Recorded fact',
              },
            ],
            next_cursor: null,
          })
        return Response.json({
          items: [
            {
              kind: 'execution',
              stable_id: `run:${runId}`,
              created_at: timestamp,
              run: durableRun(),
            },
            {
              kind: 'trial',
              stable_id: `phone:${entryId}`,
              created_at: timestamp,
              task_id: entryId,
              title: 'Try draft actions',
              state: 'completed',
              message: 'Trial completed',
            },
          ],
          next_cursor: null,
        })
      }
      throw new Error(`Unexpected fixture request ${request.method} ${url.pathname}`)
    }),
  )
  show()
  expect(await screen.findByText('Saved test')).toBeInTheDocument()
  expect(screen.getByText('Trial')).toBeInTheDocument()
  expect(screen.queryByText('Historical preview action')).not.toBeInTheDocument()

  await userEvent.click(screen.getByRole('combobox', { name: 'History' }))
  await userEvent.click(screen.getByRole('option', { name: 'Legacy session activity' }))
  expect(await screen.findByText('Historical preview action')).toBeInTheDocument()
  expect(screen.getByText(/No comparable baseline/)).toBeInTheDocument()
  await waitFor(() => expect(filters).toContain('legacy'))
})
