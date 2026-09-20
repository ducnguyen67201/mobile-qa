// DOM interaction assertions use the real generated SDK with fetch-only fixtures.
import { MantineProvider } from '@mantine/core'
import { theme } from '@/theme'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { createMemoryRouter, RouterProvider } from 'react-router'
import { afterEach, expect, it, vi } from 'vitest'
import { routes } from '../routes'
import { session, apiError, app, settings, orgId, googleChallenge } from '@/test/fixtures'
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
it('keeps operator-managed tests and run history behind authentication', async () => {
  vi.stubGlobal(
    'fetch',
    vi.fn(async (request: Request) =>
      Response.json(
        new URL(request.url).pathname.endsWith('/session')
          ? session
          : { items: [], next_cursor: null },
      ),
    ),
  )
  show('/tests')
  expect(await screen.findByRole('heading', { name: 'Tests', level: 1 })).toBeInTheDocument()
  await userEvent.click(screen.getByRole('link', { name: /Runs/ }))
  expect(screen.getByRole('heading', { name: 'Runs', level: 1 })).toBeInTheDocument()
  expect(screen.getByText(/Test results, build comparisons and evidence/)).toBeInTheDocument()
})
it.each([
  'https://evil.test',
  '//evil.test',
  '/apps\\evil',
  '/sign-in',
  'javascript:alert(1)',
  null,
])('rejects unsafe return destination %s', (value) => {
  expect(safeReturnTo(value)).toBe('/apps')
})
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
    await screen.findByRole('button', {
      name: `Open account menu for ${session.user.display_name}`,
    }),
  )
  const menu = await screen.findByRole('menu')
  await userEvent.click(within(menu).getByRole('menuitem', { name: 'Sign out' }))
  expect(await screen.findByRole('heading', { name: 'Welcome back.' })).toBeInTheDocument()
  expect(
    fetchMock.mock.calls.some(
      ([request]) => request.url.endsWith('/logout') && request.method === 'POST',
    ),
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
// The provider UI is mocked; our real generated SDK still posts the returned credential.
vi.mock('@react-oauth/google', () => ({
  GoogleOAuthProvider: ({ children }: { children: React.ReactNode }) => children,
  GoogleLogin: ({ onSuccess }: { onSuccess: (response: { credential: string }) => void }) => (
    <button onClick={() => onSuccess({ credential: 'synthetic-google-token' })}>
      Continue with Google
    </button>
  ),
}))
it('offers only Google sign-in and exchanges the credential through the generated API', async () => {
  let submitted: unknown
  vi.stubGlobal(
    'fetch',
    vi.fn(async (request: Request) => {
      if (request.url.endsWith('/google/challenge')) return Response.json(googleChallenge)
      if (request.url.endsWith('/google/login')) {
        submitted = await request.json()
        return Response.json(session)
      }
      if (request.url.endsWith('/session')) return Response.json(session)
      return Response.json({ items: [], next_cursor: null })
    }),
  )
  show('/sign-in')
  expect(screen.queryByLabelText('Email address')).not.toBeInTheDocument()
  expect(screen.queryByLabelText('Password')).not.toBeInTheDocument()
  await userEvent.click(await screen.findByRole('button', { name: 'Continue with Google' }))
  expect(await screen.findByText('Your first app belongs here.')).toBeInTheDocument()
  expect(submitted).toEqual({
    credential: 'synthetic-google-token',
    challenge_id: googleChallenge.challenge_id,
  })
})
it('requests a new challenge after a rejected Google credential', async () => {
  let challenges = 0
  vi.stubGlobal(
    'fetch',
    vi.fn(async (request: Request) => {
      if (request.url.endsWith('/google/challenge')) {
        challenges += 1
        return Response.json(googleChallenge)
      }
      return apiError()
    }),
  )
  show('/sign-in')
  await userEvent.click(await screen.findByRole('button', { name: 'Continue with Google' }))
  await userEvent.click(await screen.findByRole('button', { name: 'Retry' }))
  expect(await screen.findByRole('button', { name: 'Continue with Google' })).toBeInTheDocument()
  expect(challenges).toBe(2)
})
