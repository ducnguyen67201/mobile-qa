import { MantineProvider } from '@mantine/core'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { createMemoryRouter, RouterProvider } from 'react-router'
import { afterEach, expect, it, vi } from 'vitest'
import { routes } from '@/routes'
import { app, appId, orgId, session } from '@/test/fixtures'

const runId = '22222222-2222-4222-8222-222222222222'
const plans = [
  { plan: 'starter', monthly_cents: 50000, monthly_credits: 50000, currency: 'USD' },
  { plan: 'plus', monthly_cents: 75000, monthly_credits: 75000, currency: 'USD' },
  { plan: 'business', monthly_cents: 150000, monthly_credits: 150000, currency: 'USD' },
]

function show(active = false, scheduled = false) {
  let pendingPlan: string | null = scheduled ? 'plus' : null
  const fetchMock = vi.fn(async (request: Request) => {
    const path = new URL(request.url).pathname
    if (path.endsWith('/session')) return Response.json(session)
    if (path === '/api/apps')
      return Response.json({
        items: [{ ...app, environment_name: app.environment.name }],
        next_cursor: null,
      })
    if (path === `/api/apps/${appId}`) return Response.json(app)
    if (path.endsWith('/credit-checkout') && request.method === 'POST') {
      expect(await request.json()).toEqual({ plan: 'plus' })
      return Response.json(
        {
          code: 'checkout_unavailable',
          message: 'Checkout is not configured for this environment',
          details: null,
          request_id: runId,
        },
        { status: 503 },
      )
    }
    if (path.endsWith('/credit-plan-change') && request.method === 'POST') {
      const body = await request.json()
      expect(body).toEqual({ plan: pendingPlan ? 'starter' : 'plus' })
      pendingPlan = body.plan === 'starter' ? null : body.plan
    }
    if (path.endsWith('/commercial-access') || path.endsWith('/credit-plan-change'))
      return Response.json({
        app_id: appId,
        state: active ? 'active' : 'uncontracted',
        plans,
        credit: active
          ? {
              plan: 'starter',
              pending_plan: pendingPlan,
              pending_effective_at: pendingPlan ? '2026-10-01T00:00:00Z' : null,
              period_start: '2026-09-01T00:00:00Z',
              period_end: '2026-10-01T00:00:00Z',
              granted_credits: 50000,
              charged_credits: 1500,
              held_credits: 4000,
              available_credits: 44500,
              rate_revision: 1,
              usage: [
                {
                  run_id: runId,
                  state: 'settled',
                  held_credits: 5000,
                  measured_credits: '1500',
                  charged_credits: 1500,
                  input_tokens: '100',
                  output_tokens: '50',
                  device_seconds: 1498,
                  stored_bytes: '100000',
                  reason: 'Reviewed report',
                },
              ],
            }
          : null,
        agreement: null,
        pilot_request_id: null,
        reserved_checks: 0,
        delivered_checks: 0,
        credited_checks: 0,
        delivered_check_cents: 0,
        usage: [],
      })
    throw new Error(`Unexpected fixture request ${request.method} ${path}`)
  })
  vi.stubGlobal('fetch', fetchMock)
  render(
    <MantineProvider env="test">
      <QueryClientProvider
        client={
          new QueryClient({
            defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
          })
        }
      >
        <RouterProvider
          router={createMemoryRouter(routes, {
            initialEntries: [`/settings/commercial?workspace=${orgId}&app=${appId}`],
          })}
        />
      </QueryClientProvider>
    </MantineProvider>,
  )
  return fetchMock
}

afterEach(() => vi.unstubAllGlobals())

it('shows three direct monthly plan actions and no questionnaire', async () => {
  const fetchMock = show()
  expect(await screen.findByRole('button', { name: 'Choose Plus' })).toBeInTheDocument()
  expect(screen.getByRole('button', { name: 'Choose Starter' })).toBeInTheDocument()
  expect(screen.getByRole('button', { name: 'Choose Business' })).toBeInTheDocument()
  expect(screen.getByText('50,000 credits each month')).toBeInTheDocument()
  expect(screen.queryByRole('textbox', { name: /pilot|coverage/i })).not.toBeInTheDocument()
  await userEvent.click(screen.getByRole('button', { name: 'Choose Plus' }))
  expect(
    await screen.findByText('Checkout is not configured for this environment'),
  ).toBeInTheDocument()
  expect(
    fetchMock.mock.calls.filter(([request]) =>
      new URL(request.url).pathname.endsWith('/credit-checkout'),
    ),
  ).toHaveLength(1)
})

it('shows the paid allowance, held credits, and measured run ledger', async () => {
  show(true)
  expect(await screen.findByText('44,500')).toBeInTheDocument()
  expect(
    screen.getByRole('progressbar', { name: '44500 of 50000 credits available' }),
  ).toBeInTheDocument()
  expect(screen.getByRole('cell', { name: '5,000' })).toBeInTheDocument()
  expect(screen.getByText('1,500')).toBeInTheDocument()
  expect(screen.getByText(/1498s device/)).toBeInTheDocument()
  expect(screen.getByRole('button', { name: 'Your current plan' })).toBeDisabled()
  expect(screen.getByRole('button', { name: 'Schedule Plus next renewal' })).toBeEnabled()
  await waitFor(() =>
    expect(screen.getByRole('link', { name: runId.slice(0, 8) })).toHaveAttribute(
      'href',
      `/runs/${runId}?workspace=${orgId}`,
    ),
  )
})

it('shows a pending renewal without changing the current allowance', async () => {
  show(true, true)
  expect(await screen.findByText('44,500')).toBeInTheDocument()
  expect(screen.getByText(/Plus starts at your next paid renewal/)).toBeInTheDocument()
  expect(screen.getByRole('button', { name: 'Plus scheduled' })).toBeDisabled()
  expect(screen.getByRole('button', { name: 'Keep Starter' })).toBeEnabled()
  expect(screen.getByRole('button', { name: 'Schedule Business next renewal' })).toBeDisabled()
})

it('schedules and cancels a plan while keeping the current balance', async () => {
  const fetchMock = show(true)
  await userEvent.click(await screen.findByRole('button', { name: 'Schedule Plus next renewal' }))
  expect(await screen.findByRole('button', { name: 'Plus scheduled' })).toBeDisabled()
  expect(screen.getByText('44,500')).toBeInTheDocument()
  await userEvent.click(screen.getByRole('button', { name: 'Keep Starter' }))
  expect(await screen.findByRole('button', { name: 'Schedule Plus next renewal' })).toBeEnabled()
  expect(
    fetchMock.mock.calls.filter(([request]) =>
      new URL(request.url).pathname.endsWith('/credit-plan-change'),
    ),
  ).toHaveLength(2)
})
