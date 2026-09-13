import { MantineProvider } from '@mantine/core'
import { theme } from '@/theme'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { createMemoryRouter, RouterProvider } from 'react-router'
import { afterEach, expect, it, vi } from 'vitest'
import { routes } from '@/routes'
import { app, orgId, session, settings, buildId } from '@/test/fixtures'
import type { SessionResponse } from '@/api/generated/types.gen'

const secondId = '99999999-9999-4999-8999-999999999999'
const second = {
  organization_id: secondId,
  name: 'Second workspace',
  role: 'operator' as const,
}
function show(path: string) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0 } },
  })
  const router = createMemoryRouter(routes, { initialEntries: [path] })
  render(
    <MantineProvider theme={theme} env="test">
      <QueryClientProvider client={client}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    </MantineProvider>,
  )
  return { router, client }
}
function fixture(account: SessionResponse = session) {
  return vi.fn(async (request: Request) => {
    const url = new URL(request.url)
    if (url.pathname.endsWith('/session')) return Response.json(account)
    if (url.pathname.endsWith('/settings')) return Response.json(settings)
    if (url.pathname.endsWith(`/apps/${app.id}`)) return Response.json(app)
    if (url.pathname.endsWith('/apps'))
      return Response.json({
        items:
          url.searchParams.get('organization_id') === orgId
            ? [{ ...app, environment_name: app.environment.name }]
            : [],
        next_cursor: null,
      })
    return Response.json({ items: [], next_cursor: null })
  })
}
afterEach(() => vi.unstubAllGlobals())
it('allows pending Google accounts to sign in but hides and guards workspace creation', async () => {
  const pending: SessionResponse = {
    ...session,
    user: { ...session.user, approval_status: 'pending' },
    memberships: [],
  }
  const fetchMock = fixture(pending)
  vi.stubGlobal('fetch', fetchMock)
  const { router } = show('/workspaces/new')
  expect(await screen.findByText('Awaiting approval')).toBeInTheDocument()
  expect(router.state.location.pathname).toBe('/workspaces')
  expect(screen.queryByRole('button', { name: 'Create workspace' })).not.toBeInTheDocument()
  expect(screen.queryByLabelText(/Workspace name/)).not.toBeInTheDocument()
  expect(fetchMock.mock.calls.some(([r]) => r.method === 'POST')).toBe(false)
})
it('refreshes approval and exposes creation without signing out', async () => {
  let approved = false
  vi.stubGlobal(
    'fetch',
    vi.fn(async () =>
      Response.json({
        ...session,
        user: {
          ...session.user,
          approval_status: approved ? 'approved' : 'pending',
        },
        memberships: [],
      }),
    ),
  )
  show('/apps')
  await screen.findByText('Awaiting approval')
  approved = true
  await userEvent.click(screen.getByRole('button', { name: 'Check approval status' }))
  await userEvent.click(await screen.findByRole('link', { name: 'Create workspace' }))
  expect(await screen.findByLabelText(/Workspace name/)).toBeInTheDocument()
})
it('creates a workspace for an approved account and navigates using its refreshed membership', async () => {
  let created = false
  let payload: unknown
  vi.stubGlobal(
    'fetch',
    vi.fn(async (request: Request) => {
      const path = new URL(request.url).pathname
      if (path.endsWith('/session'))
        return Response.json({
          ...session,
          memberships: created ? [second] : [],
        })
      if (path.endsWith('/workspaces')) {
        payload = await request.json()
        created = true
        return Response.json(second, { status: 201 })
      }
      return Response.json({ items: [], next_cursor: null })
    }),
  )
  const { router } = show('/apps')
  await userEvent.type(await screen.findByLabelText(/Workspace name/), 'Second workspace')
  await userEvent.click(screen.getByRole('button', { name: 'Create workspace' }))
  await screen.findByText('Your first app belongs here.')
  expect(payload).toEqual({ id: expect.any(String), name: 'Second workspace' })
  expect(router.state.location.search).toBe(`?workspace=${secondId}`)
  await userEvent.click(screen.getByRole('button', { name: 'Expand sidebar' }))
  expect(screen.getByLabelText('Current workspace')).toHaveValue(secondId)
})
it('switches app lists by URL, preserves navigation context, and restores browser history', async () => {
  const fetchMock = fixture({
    ...session,
    memberships: [...session.memberships, second],
  })
  vi.stubGlobal('fetch', fetchMock)
  const { router } = show(`/apps?workspace=${orgId}`)
  await screen.findByRole('heading', { name: app.name })
  await userEvent.click(screen.getByRole('button', { name: 'Create app' }))
  await screen.findByLabelText('App name')
  await userEvent.keyboard('{Escape}')
  await userEvent.click(screen.getByRole('button', { name: 'Expand sidebar' }))
  await userEvent.selectOptions(screen.getByLabelText('Current workspace'), secondId)
  await screen.findByText('Your first app belongs here.')
  expect(screen.queryByText(app.name)).not.toBeInTheDocument()
  expect(router.state.location.search).toBe(`?workspace=${secondId}`)
  await userEvent.click(screen.getByRole('link', { name: /Tests Soon/ }))
  expect(router.state.location.search).toBe(`?workspace=${secondId}`)
  await router.navigate(-1)
  await router.navigate(-1)
  await screen.findByRole('heading', { name: app.name })
  expect(screen.getByLabelText('Current workspace')).toHaveValue(orgId)
  const lists = fetchMock.mock.calls
    .map(([r]) => new URL(r.url))
    .filter((u) => u.pathname.endsWith('/apps'))
  expect(
    lists.every((u) => [orgId, secondId].includes(u.searchParams.get('organization_id')!)),
  ).toBe(true)
})
it('keeps reload and deep-link workspace selection and creates apps only in that workspace', async () => {
  let body: unknown
  const base = fixture({
    ...session,
    memberships: [...session.memberships, second],
  })
  vi.stubGlobal(
    'fetch',
    vi.fn(async (request: Request) => {
      if (request.method === 'POST') {
        body = await request.json()
        return Response.json({ ...app, organization_id: secondId }, { status: 201 })
      }
      if (new URL(request.url).pathname.endsWith(`/apps/${app.id}`))
        return Response.json({ ...app, organization_id: secondId })
      return base(request)
    }),
  )
  const { router } = show(`/apps?workspace=${secondId}`)
  await userEvent.click(await screen.findByRole('button', { name: 'Create app' }))
  expect(screen.queryByLabelText('Organization')).not.toBeInTheDocument()
  await userEvent.type(screen.getByLabelText('App name'), 'Second app')
  await userEvent.type(screen.getByLabelText('Android package'), 'com.second.app')
  await userEvent.type(screen.getByLabelText('Backend origins'), 'https://second.example.com')
  await userEvent.click(
    within(screen.getByRole('dialog', { name: 'Create an app' })).getByRole('button', {
      name: 'Create app',
    }),
  )
  await waitFor(() => expect(body).toMatchObject({ organization_id: secondId }))
  await waitFor(() => expect(router.state.location.pathname).toBe(`/apps/${app.id}`))
  expect(router.state.location.search).toBe(`?workspace=${secondId}`)
})
it('rejects a tampered workspace URL before fetching app data', async () => {
  const fetchMock = fixture()
  vi.stubGlobal('fetch', fetchMock)
  show(`/apps?workspace=${secondId}`)
  await screen.findByText('Workspace unavailable')
  expect(fetchMock.mock.calls.every(([r]) => r.url.endsWith('/session'))).toBe(true)
})
it('does not load build or upload data for an app under a different workspace URL', async () => {
  const fetchMock = fixture({
    ...session,
    memberships: [...session.memberships, second],
  })
  vi.stubGlobal('fetch', fetchMock)
  show(`/apps/${app.id}?workspace=${secondId}&build=${buildId}`)
  await screen.findByText('This app belongs to a different workspace.')
  expect(fetchMock.mock.calls.some(([r]) => /builds|build-uploads/.test(r.url))).toBe(false)
  expect(screen.queryByLabelText('Choose APK file')).not.toBeInTheDocument()
})
it('clears app-specific parameters when switching workspace from a detail page', async () => {
  vi.stubGlobal('fetch', fixture({ ...session, memberships: [...session.memberships, second] }))
  const { router } = show(
    `/apps/${app.id}?workspace=${secondId}&build=${buildId}&upload=${buildId}`,
  )
  await screen.findByText('This app belongs to a different workspace.')
  await userEvent.click(screen.getByRole('button', { name: 'Switch workspace' }))
  await userEvent.click(await screen.findByRole('menuitem', { name: 'Fixture workspace' }))
  await screen.findByRole('heading', { name: app.name })
  expect(router.state.location.pathname).toBe('/apps')
  expect(router.state.location.search).toBe(`?workspace=${orgId}`)
})
