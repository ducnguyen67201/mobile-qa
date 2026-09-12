// DOM interaction assertions use the real generated SDK with fetch-only fixtures.
import { MantineProvider } from '@mantine/core'
import { theme } from '@/theme'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { createMemoryRouter, RouterProvider } from 'react-router'
import { afterEach, expect, it, vi } from 'vitest'
import { routes } from '../routes'
import { session, apiError, app, settings, orgId } from '@/test/fixtures'
import { safeReturnTo } from './SignIn'
function show(path = '/') {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } })
  const router = createMemoryRouter(routes, { initialEntries: [path] })
  render(
    <MantineProvider theme={theme} env="test">
      <QueryClientProvider client={client}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    </MantineProvider>,
  )
  return { client, router }
}
afterEach(() => vi.unstubAllGlobals())
it('protects app content when unauthenticated', async () => {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue(apiError()))
  show('/apps')
  expect(await screen.findByRole('heading', { name: 'Welcome back.' })).toBeInTheDocument()
  expect(screen.queryByText('Your first app belongs here.')).not.toBeInTheDocument()
})
it('shows the saved app empty state after validating session', async () => {
  vi.stubGlobal(
    'fetch',
    vi.fn(async (request: Request) =>
      request.url.endsWith('/session')
        ? Response.json(session)
        : Response.json({ items: [], next_cursor: null }),
    ),
  )
  show('/apps')
  expect(await screen.findByText('Your first app belongs here.')).toBeInTheDocument()
  await userEvent.click(screen.getByRole('button', { name: 'Create your first app' }))
  expect(await screen.findByLabelText('Android package')).toBeInTheDocument()
  expect(screen.getByLabelText('App name')).toHaveFocus()
})
it('keeps an unavailable session visibly retryable', async () => {
  vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('offline')))
  show()
  expect(await screen.findByRole('button', { name: 'Retry' })).toBeInTheDocument()
  expect(screen.queryByRole('heading', { name: 'Your apps.' })).not.toBeInTheDocument()
})
it('keeps tests and runs truthful placeholders behind authentication', async () => {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue(Response.json(session)))
  show('/tests')
  expect(await screen.findByRole('heading', { name: 'Tests', level: 1 })).toBeInTheDocument()
  await userEvent.click(screen.getByRole('link', { name: /Runs/ }))
  expect(screen.getByRole('heading', { name: 'Runs', level: 1 })).toBeInTheDocument()
  expect(screen.getByText(/planned for a later phase/)).toBeInTheDocument()
})
it.each(['https://evil.test', '//evil.test', '/apps\\evil', '/sign-in', 'javascript:alert(1)', null])(
  'rejects unsafe return destination %s',
  (value) => {
    expect(safeReturnTo(value)).toBe('/apps')
  },
)
it('preserves a local build destination', () => {
  expect(safeReturnTo('/apps/example?build=123')).toBe('/apps/example?build=123')
})
it('requires a backend origin and keeps login origins optional', async () => {
  vi.stubGlobal(
    'fetch',
    vi.fn(async (request: Request) =>
      request.url.endsWith('/session')
        ? Response.json(session)
        : Response.json({ items: [], next_cursor: null }),
    ),
  )
  show('/apps')
  await userEvent.click(await screen.findByRole('button', { name: 'Create your first app' }))
  expect(screen.getByLabelText('Backend origins')).toBeRequired()
  expect(screen.getByLabelText(/Login origins/)).not.toBeRequired()
  expect(screen.getByText(/Add at least one backend origin/)).toBeInTheDocument()
})

it('dismisses app creation with Escape and restores focus to its trigger', async () => {
  vi.stubGlobal(
    'fetch',
    vi.fn(async (request: Request) =>
      request.url.endsWith('/session')
        ? Response.json(session)
        : Response.json({ items: [], next_cursor: null }),
    ),
  )
  show('/apps')
  const trigger = await screen.findByRole('button', { name: 'Create your first app' })
  await userEvent.click(trigger)
  expect(await screen.findByRole('dialog', { name: 'Create an app' })).toBeInTheDocument()
  await waitFor(() => expect(screen.getByLabelText('App name')).toHaveFocus())
  await userEvent.keyboard('{Escape}')
  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument())
  await waitFor(() => expect(trigger).toHaveFocus())
})
it('opens the account menu and signs out through the API', async () => {
  const fetchMock = vi.fn(async (request: Request) => {
    if (request.url.endsWith('/logout')) return Response.json({ signed_out: true })
    if (request.url.endsWith('/session')) return Response.json(session)
    return Response.json({ items: [], next_cursor: null })
  })
  vi.stubGlobal('fetch', fetchMock)
  show('/apps')
  await userEvent.click(
    await screen.findByRole('button', { name: `Open account menu for ${session.user.display_name}` }),
  )
  const menu = await screen.findByRole('menu')
  await userEvent.click(within(menu).getByRole('menuitem', { name: 'Sign out' }))
  expect(await screen.findByRole('heading', { name: 'Welcome back.' })).toBeInTheDocument()
  expect(
    fetchMock.mock.calls.some(([request]) => request.url.endsWith('/logout') && request.method === 'POST'),
  ).toBe(true)
})

it('submits the current app draft as the generated API payload', async () => {
  let submitted: unknown
  vi.stubGlobal(
    'fetch',
    vi.fn(async (request: Request) => {
      const path = new URL(request.url).pathname
      if (path.endsWith('/session')) return Response.json(session)
      if (path.endsWith('/apps') && request.method === 'POST') {
        submitted = await request.json()
        return Response.json(app, { status: 201 })
      }
      if (path.endsWith(`/apps/${app.id}`)) return Response.json(app)
      if (path.endsWith('/settings')) return Response.json(settings)
      return Response.json({ items: [], next_cursor: null })
    }),
  )
  show('/apps')
  await userEvent.click(await screen.findByRole('button', { name: 'Create your first app' }))
  const drawer = within(await screen.findByRole('dialog', { name: 'Create an app' }))
  await userEvent.type(drawer.getByLabelText('App name'), '  Synthetic app  ')
  await userEvent.type(drawer.getByLabelText('Android package'), app.android_package)
  await userEvent.type(
    drawer.getByLabelText('Backend origins'),
    'https://api.synthetic.test, https://other.synthetic.test',
  )
  await userEvent.click(drawer.getByRole('button', { name: 'Create app' }))
  expect(await screen.findByRole('heading', { name: app.name, level: 1 })).toBeInTheDocument()
  expect(submitted).toEqual({
    organization_id: orgId,
    name: app.name,
    android_package: app.android_package,
    environment_name: 'Staging',
    backend_origins: ['https://api.synthetic.test', 'https://other.synthetic.test'],
    login_origins: [],
  })
})
it('submits sign-in field values without altering the password', async () => {
  let submitted: unknown
  vi.stubGlobal(
    'fetch',
    vi.fn(async (request: Request) => {
      if (request.url.endsWith('/login')) {
        submitted = await request.json()
        return Response.json(session)
      }
      if (request.url.endsWith('/session')) return Response.json(session)
      return Response.json({ items: [], next_cursor: null })
    }),
  )
  show('/sign-in')
  await userEvent.type(screen.getByLabelText('Email address'), session.user.email)
  await userEvent.type(screen.getByLabelText('Password'), ' synthetic password ')
  await userEvent.click(screen.getByRole('button', { name: 'Sign in' }))
  expect(await screen.findByText('Your first app belongs here.')).toBeInTheDocument()
  expect(submitted).toEqual({ email: session.user.email, password: ' synthetic password ' })
})
