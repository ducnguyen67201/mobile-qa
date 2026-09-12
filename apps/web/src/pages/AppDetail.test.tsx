import { MantineProvider } from '@mantine/core'
import { theme } from '@/theme'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { act, render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { createMemoryRouter, RouterProvider } from 'react-router'
import { afterEach, expect, it, vi } from 'vitest'
import { routes } from '@/routes'
import { app, appId, build, buildId, session, settings, upload, uploadId } from '@/test/fixtures'
import type { BuildResponse, UploadResponse } from '@/api/generated/types.gen'
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
    if (url.pathname.endsWith('/session')) return Response.json(session)
    if (url.pathname.endsWith('/settings')) return Response.json(settings)
    if (url.pathname.endsWith(`/apps/${appId}`)) return Response.json(app)
    if (url.pathname.endsWith('/builds')) return Response.json({ items: [currentBuild], next_cursor: null })
    if (url.pathname.endsWith(`/builds/${buildId}`) || url.pathname.endsWith('/complete'))
      return Response.json(currentBuild)
    if (url.pathname.includes('/build-uploads/')) return Response.json(currentUpload)
    throw new Error(`Unexpected synthetic request ${request.method} ${url.pathname}`)
  })
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
      .some((request) => request.url.endsWith(`/build-uploads/${uploadId}`) && request.method === 'GET'),
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
    expect(fetchMock.mock.calls.filter(([request]) => request.url.endsWith('/complete'))).toHaveLength(2),
  )
  const mutations = fetchMock.mock.calls
    .map(([request]) => request)
    .filter((request) => request.method !== 'GET')
  expect(mutations.every((request) => request.url.endsWith(`/build-uploads/${uploadId}/complete`))).toBe(true)
  const requests = fetchMock.mock.calls.map(([request]) => request)
  const firstComplete = requests.findIndex((request) => request.url.endsWith('/complete'))
  expect(
    requests
      .slice(firstComplete + 1, -1)
      .some((request) => request.method === 'GET' && request.url.endsWith(`/build-uploads/${uploadId}`)),
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
            { ...build, validation: { ...build.validation, state: terminal ? 'validated' : 'validating' } },
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
