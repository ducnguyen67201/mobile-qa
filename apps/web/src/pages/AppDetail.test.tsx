import { MantineProvider } from '@mantine/core'
import { webcrypto } from 'node:crypto'
import { theme } from '@/theme'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { act, render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { createMemoryRouter, RouterProvider } from 'react-router'
import { afterEach, expect, it, vi } from 'vitest'
import { routes } from '@/routes'
import { app, appId, build, buildId, session, settings, upload, uploadId } from '@/test/fixtures'
import type {
  BuildResponse,
  CreateRunRequest,
  RunManifest,
  RunResponse,
  UploadResponse,
} from '@/api/generated/types.gen'
import { zCreateRunRequest } from '@/api/generated/zod.gen'
import { ApkUpload } from '@/components/app/apk-upload'
function show(path: string) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } })
  render(
    <MantineProvider theme={theme} env="test">
      <QueryClientProvider client={client}>
        <RouterProvider router={createMemoryRouter(routes, { initialEntries: [path] })} />
      </QueryClientProvider>
    </MantineProvider>,
  )
}
function fixtureFetch(currentBuild: BuildResponse = build, currentUpload: UploadResponse = upload) {
  return vi.fn(async (request: Request) => {
    const url = new URL(request.url)
    if (url.pathname.endsWith('/execution-plan'))
      return Response.json({
        plan: null,
        manifest: null,
        blockers: ['An operator must import and approve a release check'],
      })
    if (url.pathname.endsWith('/session')) return Response.json(session)
    if (url.pathname.endsWith('/settings')) return Response.json(settings)
    if (url.pathname.endsWith(`/apps/${appId}`)) return Response.json(app)
    if (url.pathname.endsWith('/builds'))
      return Response.json({ items: [currentBuild], next_cursor: null })
    if (url.pathname.endsWith(`/builds/${buildId}`) || url.pathname.endsWith('/complete'))
      return Response.json(currentBuild)
    if (url.pathname.includes('/build-uploads/')) return Response.json(currentUpload)
    throw new Error(`Unexpected synthetic request ${request.method} ${url.pathname}`)
  })
}

const multipartSession = {
  upload_id: uploadId,
  part_size: 16 * 1024 * 1024,
  max_parallel_parts: 2,
  expires_at: upload.expires_at,
  state: 'uploading' as const,
  parts: [],
}

function showUpload(enabled = false, saved = true) {
  const router = createMemoryRouter(
    [
      {
        path: '/',
        element: (
          <ApkUpload appId={appId} maxBytes={settings.max_apk_bytes} multipartEnabled={enabled} />
        ),
      },
    ],
    { initialEntries: [saved ? `/?upload=${uploadId}` : '/'] },
  )
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } })
  render(
    <MantineProvider theme={theme} env="test">
      <QueryClientProvider client={client}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    </MantineProvider>,
  )
  return router
}

function failure(status: number) {
  return Response.json(
    {
      code: 'multipart_lookup_failed',
      message: 'Saved upload lookup failed',
      details: null,
      request_id: appId,
    },
    { status },
  )
}
afterEach(() => {
  vi.unstubAllGlobals()
  vi.useRealTimers()
})
it('shows persisted metadata while device readiness remains not checked', async () => {
  vi.stubGlobal('fetch', fixtureFetch())
  show(`/apps/${appId}?build=${buildId}`)
  expect(await screen.findByText('APK intake checks passed.')).toBeInTheDocument()
  expect(screen.getByText('a'.repeat(64))).toBeInTheDocument()
  expect(screen.getByText(/Device checks are not available yet/)).toBeInTheDocument()
  expect(screen.getAllByText('Not checked')).toHaveLength(4)
  expect(screen.queryByText('Ready to run')).not.toBeInTheDocument()
})
it('queues the default release plan resolved by the preview', async () => {
  const planVersionId = '11111111-1111-4111-8111-111111111111'
  const runId = '22222222-2222-4222-8222-222222222222'
  const manifest: RunManifest = {
    app_id: appId,
    build_id: buildId,
    build_sha256: build.sha256,
    build_bytes: build.byte_size,
    plan_version_id: planVersionId,
    plan_hash: 'b'.repeat(64),
    environment_revision: app.environment.revision,
    profile: {
      id: runId,
      name: 'Synthetic profile',
      driver: 'fake',
      package: app.android_package,
      adapter: 'fixture',
      device_identity: 'fixture',
      image: 'fixture',
      model: 'none',
      qualified: true,
      qualification_reference: 'fixture',
      max_apk_bytes: 1048576,
    },
    cases: [],
    budget: { duration_seconds: 600, max_steps: 30, artifact_bytes: 16777216 },
    diagnostic_retries: 0,
    exclusions: [],
  }
  const run: RunResponse = {
    id: runId,
    state: 'queued',
    created_at: '2026-09-20T00:00:00Z',
    summary: 'Queued',
    attempts: [],
    manifest,
  }
  const submissions: CreateRunRequest[] = []
  let quoteRequested = false
  const base = fixtureFetch()
  vi.stubGlobal(
    'fetch',
    vi.fn(async (request: Request) => {
      const url = new URL(request.url)
      if (url.pathname.endsWith('/execution-plan')) {
        expect(url.searchParams.has('plan_version_id')).toBe(false)
        return Response.json({ plan: null, manifest, blockers: [] })
      }
      if (url.pathname.endsWith('/commercial-check-quotes')) {
        quoteRequested = true
        return Response.json(
          {
            id: '33333333-3333-4333-8333-333333333333',
            app_id: appId,
            kind: 'release_plan',
            build_id: buildId,
            source_version_id: planVersionId,
            profile_id: runId,
            case_count: 1,
            amount_cents: 12500,
            maximum_credits: null,
            credits_after_authorization: null,
            currency: 'USD',
            checks_after_authorization: 1,
            check_cap: 8,
            expires_at: new Date(Date.now() + 600_000).toISOString(),
          },
          { status: 201 },
        )
      }
      if (url.pathname.endsWith('/runs') && request.method === 'POST') {
        expect(quoteRequested).toBe(true)
        expect(request.headers.get('X-Commercial-Quote-Id')).toBe(
          '33333333-3333-4333-8333-333333333333',
        )
        submissions.push(zCreateRunRequest.parse(await request.json()))
        return Response.json(run, { status: 201 })
      }
      if (url.pathname.endsWith(`/runs/${runId}`)) return Response.json(run)
      return base(request)
    }),
  )
  show(`/apps/${appId}?build=${buildId}`)
  await userEvent.click(await screen.findByRole('button', { name: 'Review check price' }))
  expect(submissions).toHaveLength(0)
  await userEvent.click(await screen.findByRole('button', { name: 'Authorize check and run' }))
  await waitFor(() => expect(submissions).toHaveLength(1))
  expect(submissions[0]).toEqual({
    build_id: buildId,
    plan_version_id: planVersionId,
    environment_revision: app.environment.revision,
  })
})
it('distinguishes infrastructure failure from an invalid APK', async () => {
  vi.stubGlobal(
    'fetch',
    fixtureFetch({
      ...build,
      metadata: null,
      can_retry_validation: true,
      validation: {
        ...build.validation,
        state: 'error',
        message: 'Validator temporarily unavailable',
        reason_code: 'tool_unavailable',
      },
    }),
  )
  show(`/apps/${appId}?build=${buildId}`)
  expect(await screen.findByRole('button', { name: 'Retry validation' })).toBeInTheDocument()
  expect(screen.getByText(/validation service could not complete/)).toBeInTheDocument()
  expect(screen.queryByText('Invalid APK')).not.toBeInTheDocument()
})
it('recovers a sealed upload after reload without retransmitting bytes', async () => {
  const fetchMock = fixtureFetch()
  vi.stubGlobal('fetch', fetchMock)
  show(`/apps/${appId}?upload=${uploadId}`)
  await userEvent.click(await screen.findByRole('button', { name: 'Validate stored APK' }))
  await waitFor(() =>
    expect(fetchMock.mock.calls.some(([request]) => request.url.endsWith('/complete'))).toBe(true),
  )
  const requests = fetchMock.mock.calls.map(([request]) => request)
  const completeAt = requests.findIndex((request) => request.url.endsWith('/complete'))
  expect(
    requests
      .slice(0, completeAt)
      .some(
        (request) => request.url.endsWith(`/build-uploads/${uploadId}`) && request.method === 'GET',
      ),
  ).toBe(true)
  expect(requests.some((request) => request.method === 'PUT')).toBe(false)
  expect(
    requests.some((request) => request.method === 'POST' && request.url.endsWith('/build-uploads')),
  ).toBe(false)
})
it('requires original file reselection for an unfinished upload', async () => {
  vi.stubGlobal('fetch', fixtureFetch(build, { ...upload, state: 'pending', actual_size: null }))
  show(`/apps/${appId}?upload=${uploadId}`)
  await screen.findByLabelText('Choose APK file')
  await userEvent.click(screen.getByRole('button', { name: 'Upload and validate' }))
  expect(await screen.findByText('Choose the APK file to continue.')).toBeInTheDocument()
})
it.each([404, 500])(
  'handles an absent multipart session separately from a failed discard (%i)',
  async (status) => {
    const base = fixtureFetch(build, { ...upload, state: 'pending', actual_size: null })
    vi.stubGlobal(
      'fetch',
      vi.fn(async (request: Request) => {
        if (request.url.endsWith('/settings'))
          return Response.json({
            ...settings,
            multipart: { part_size: 16777216, max_parallel_parts: 2 },
          })
        if (request.url.endsWith('/multipart') && request.method === 'GET')
          return Response.json({ code: 'missing', message: 'Discard failed' }, { status })
        return base(request)
      }),
    )
    show(`/apps/${appId}?upload=${uploadId}`)
    await screen.findByRole('button', { name: 'Resume and validate' })
    await userEvent.click(screen.getByRole('button', { name: 'New upload' }))
    if (status === 404) {
      expect(await screen.findByRole('button', { name: 'Upload and validate' })).toBeInTheDocument()
      expect(screen.queryByRole('button', { name: 'New upload' })).not.toBeInTheDocument()
    } else {
      await waitFor(() => expect(screen.getByRole('button', { name: 'New upload' })).toBeEnabled())
      expect(screen.getByRole('button', { name: 'Resume and validate' })).toBeInTheDocument()
    }
  },
)

it('resumes a verified multipart upload without current multipart capability metadata', async () => {
  vi.stubGlobal('crypto', webcrypto)
  let sealed = false
  const fetchMock = vi.fn(async (request: Request) => {
    if (request.url.endsWith('/multipart') && request.method === 'GET')
      return Response.json({
        ...multipartSession,
        parts: [
          {
            part_number: 1,
            byte_size: 4,
            sha256: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08',
            etag: '"saved"',
          },
        ],
      })
    if (request.url.endsWith('/multipart/complete')) {
      sealed = true
      return Response.json(upload, { status: 202 })
    }
    if (request.url.endsWith('/complete')) return Response.json(build)
    return Response.json({ ...upload, state: sealed ? 'uploaded' : 'receiving' })
  })
  vi.stubGlobal('fetch', fetchMock)
  showUpload()
  const resume = await screen.findByRole('button', { name: 'Resume and validate' })
  const bytes = new TextEncoder().encode('test')
  const file = new File([bytes], upload.original_filename)
  // Supply jsdom's missing Blob reader while using the real fingerprint code.
  Object.defineProperty(file, 'slice', {
    value: (start: number, end: number) => {
      const part = bytes.slice(start, end)
      const blob = new Blob([part])
      Object.defineProperty(blob, 'arrayBuffer', { value: async () => part.buffer })
      return blob
    },
  })
  await userEvent.upload(screen.getByLabelText('Choose APK file'), file)
  await userEvent.click(resume)
  await waitFor(() =>
    expect(
      fetchMock.mock.calls.some(([request]) =>
        request.url.endsWith(`/build-uploads/${uploadId}/complete`),
      ),
    ).toBe(true),
  )
  const requests = fetchMock.mock.calls.map(([request]) => request)
  expect(
    requests.filter((request) => request.url.endsWith('/multipart') && request.method === 'GET'),
  ).toHaveLength(2)
  expect(
    requests.some((request) => request.url.endsWith('/multipart') && request.method === 'POST'),
  ).toBe(false)
  expect(requests.some((request) => request.method === 'PUT')).toBe(false)
})

it.each([401, 500])(
  'preserves the saved upload after a fresh multipart lookup fails (%i)',
  async (status) => {
    let lookups = 0
    const fetchMock = vi.fn(async (request: Request) => {
      if (request.url.endsWith('/multipart')) {
        lookups += 1
        return lookups === 1 ? Response.json(multipartSession) : failure(status)
      }
      return Response.json({ ...upload, state: 'receiving' })
    })
    vi.stubGlobal('fetch', fetchMock)
    const router = showUpload()
    const resume = await screen.findByRole('button', { name: 'Resume and validate' })
    await userEvent.upload(
      screen.getByLabelText('Choose APK file'),
      new File(['test'], upload.original_filename),
    )
    await userEvent.click(resume)
    expect(await screen.findByText('Upload needs attention')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Upload and validate' })).toBeDisabled()
    await userEvent.click(screen.getByRole('button', { name: 'New upload' }))
    await waitFor(() => expect(lookups).toBe(3))
    expect(router.state.location.search).toContain(uploadId)
    expect(fetchMock.mock.calls.every(([request]) => request.method === 'GET')).toBe(true)
  },
)

it('aborts an existing multipart session without current multipart capability metadata', async () => {
  const fetchMock = vi.fn(async (request: Request) => {
    if (request.url.endsWith('/multipart'))
      return Response.json({
        ...multipartSession,
        state: request.method === 'DELETE' ? 'aborted' : 'uploading',
      })
    return Response.json({ ...upload, state: 'receiving' })
  })
  vi.stubGlobal('fetch', fetchMock)
  const router = showUpload()
  await screen.findByRole('button', { name: 'Resume and validate' })
  await userEvent.click(screen.getByRole('button', { name: 'New upload' }))
  await waitFor(() => expect(router.state.location.search).toBe(''))
  const methods = fetchMock.mock.calls
    .filter(([request]) => request.url.endsWith('/multipart'))
    .map(([request]) => request.method)
  expect(methods).toEqual(['GET', 'GET', 'DELETE'])
})

it('keeps a legacy receiving upload blocked when no multipart descriptor exists', async () => {
  const fetchMock = vi.fn(async (request: Request) =>
    request.url.endsWith('/multipart')
      ? failure(404)
      : Response.json({ ...upload, state: 'receiving' }),
  )
  vi.stubGlobal('fetch', fetchMock)
  showUpload()
  await waitFor(() =>
    expect(fetchMock.mock.calls.some(([request]) => request.url.endsWith('/multipart'))).toBe(true),
  )
  expect(screen.getByRole('button', { name: 'Upload and validate' })).toBeDisabled()
  expect(screen.queryByLabelText('Choose APK file')).not.toBeInTheDocument()
  expect(fetchMock.mock.calls.every(([request]) => request.method === 'GET')).toBe(true)
})

it('uses streaming transfer for local storage without multipart support', async () => {
  const fetchMock = vi.fn(async (request: Request) => {
    if (request.url.endsWith('/build-uploads') && request.method === 'POST')
      return Response.json({ ...upload, state: 'pending' }, { status: 201 })
    if (request.url.endsWith('/complete')) return Response.json(build)
    return Response.json(upload)
  })
  vi.stubGlobal('fetch', fetchMock)
  showUpload(false, false)
  await userEvent.upload(
    screen.getByLabelText('Choose APK file'),
    new File(['test'], upload.original_filename),
  )
  await userEvent.click(screen.getByRole('button', { name: 'Upload and validate' }))
  await waitFor(() =>
    expect(fetchMock.mock.calls.some(([request]) => request.url.endsWith('/complete'))).toBe(true),
  )
  expect(fetchMock.mock.calls.some(([request]) => request.method === 'PUT')).toBe(true)
  expect(fetchMock.mock.calls.some(([request]) => request.url.includes('/multipart'))).toBe(false)
})
it('reconciles a lost completion response before retrying the same upload', async () => {
  const base = fixtureFetch()
  let failed = false
  const fetchMock = vi.fn(async (request: Request) => {
    if (request.url.endsWith('/complete') && !failed) {
      failed = true
      throw new Error('response lost')
    }
    return base(request)
  })
  vi.stubGlobal('fetch', fetchMock)
  show(`/apps/${appId}?upload=${uploadId}`)
  await userEvent.click(await screen.findByRole('button', { name: 'Validate stored APK' }))
  expect(await screen.findByText('Upload needs attention')).toBeInTheDocument()
  await userEvent.click(screen.getByRole('button', { name: 'Validate stored APK' }))
  await waitFor(() =>
    expect(
      fetchMock.mock.calls.filter(([request]) => request.url.endsWith('/complete')),
    ).toHaveLength(2),
  )
  const mutations = fetchMock.mock.calls
    .map(([request]) => request)
    .filter((request) => request.method !== 'GET')
  expect(
    mutations.every((request) => request.url.endsWith(`/build-uploads/${uploadId}/complete`)),
  ).toBe(true)
  const requests = fetchMock.mock.calls.map(([request]) => request)
  const firstComplete = requests.findIndex((request) => request.url.endsWith('/complete'))
  expect(
    requests
      .slice(firstComplete + 1, -1)
      .some(
        (request) => request.method === 'GET' && request.url.endsWith(`/build-uploads/${uploadId}`),
      ),
  ).toBe(true)
})
it('updates history when selected detail changes from validating to terminal while polling', async () => {
  vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval'] })
  const base = fixtureFetch()
  let inspections = 0
  let terminal = false
  vi.stubGlobal(
    'fetch',
    vi.fn(async (request: Request) => {
      const path = new URL(request.url).pathname
      if (path.endsWith(`/builds/${buildId}`)) {
        inspections += 1
        terminal = inspections > 1
        return Response.json({
          ...build,
          validation: { ...build.validation, state: terminal ? 'validated' : 'validating' },
        })
      }
      if (path.endsWith('/builds'))
        return Response.json({
          items: [
            {
              ...build,
              validation: { ...build.validation, state: terminal ? 'validated' : 'validating' },
            },
          ],
          next_cursor: null,
        })
      return base(request)
    }),
  )
  show(`/apps/${appId}?build=${buildId}`)
  await waitFor(() => expect(screen.getAllByText('Validating')).toHaveLength(2))
  await act(async () => {
    await vi.advanceTimersByTimeAsync(3000)
  })
  await waitFor(() => expect(screen.getAllByText('Validated')).toHaveLength(2))
  expect(screen.queryByText('Validating')).not.toBeInTheDocument()
})

it('polls an unselected validating row until its status becomes terminal', async () => {
  vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval'] })
  const base = fixtureFetch()
  let terminal = false
  vi.stubGlobal(
    'fetch',
    vi.fn(async (request: Request) => {
      if (new URL(request.url).pathname.endsWith('/builds'))
        return Response.json({
          items: [
            build,
            {
              ...build,
              id: 'ffffffff-ffff-4fff-8fff-ffffffffffff',
              validation: { ...build.validation, state: terminal ? 'validated' : 'validating' },
            },
          ],
          next_cursor: null,
        })
      return base(request)
    }),
  )
  show(`/apps/${appId}?build=${buildId}`)
  expect(await screen.findByText('Validating')).toBeInTheDocument()
  terminal = true
  await act(async () => {
    await vi.advanceTimersByTimeAsync(3000)
  })
  await waitFor(() => expect(screen.queryByText('Validating')).not.toBeInTheDocument())
  expect(screen.getAllByText('Validated')).toHaveLength(3)
})

it('saves the environment draft and reopens the latest saved revision', async () => {
  const base = fixtureFetch()
  let submitted: unknown
  let saved = false
  const updated = {
    ...app.environment,
    name: 'QA',
    revision: 2,
    backend_origins: ['https://qa.synthetic.test'],
  }
  vi.stubGlobal(
    'fetch',
    vi.fn(async (request: Request) => {
      const path = new URL(request.url).pathname
      if (path.endsWith('/environment') && request.method === 'PATCH') {
        submitted = await request.json()
        saved = true
        return Response.json(updated)
      }
      if (path.endsWith(`/apps/${appId}`))
        return Response.json({ ...app, environment: saved ? updated : app.environment })
      return base(request)
    }),
  )
  show(`/apps/${appId}`)
  await userEvent.click(await screen.findByRole('button', { name: 'Edit' }))
  const drawer = within(await screen.findByRole('dialog', { name: 'Edit environment' }))
  await userEvent.clear(drawer.getByLabelText('Environment name'))
  await userEvent.type(drawer.getByLabelText('Environment name'), ' QA ')
  await userEvent.clear(drawer.getByLabelText('Backend origins'))
  await userEvent.type(drawer.getByLabelText('Backend origins'), 'https://qa.synthetic.test')
  await userEvent.click(drawer.getByRole('button', { name: 'Save environment' }))
  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument())
  expect(await screen.findByText('QA')).toBeInTheDocument()
  expect(submitted).toEqual({
    expected_revision: 1,
    name: 'QA',
    backend_origins: ['https://qa.synthetic.test'],
    login_origins: [],
    account_secret_reference_id: null,
    reset_secret_reference_id: null,
  })
  await userEvent.click(screen.getByRole('button', { name: 'Edit' }))
  expect(await screen.findByLabelText('Environment name')).toHaveValue('QA')
})
