import { afterEach, expect, it, vi } from 'vitest'
import { planQuery, createRun } from './runs'
import { forgetSession, setCsrfToken } from './session-transport'
const app = '11111111-1111-4111-8111-111111111111'
const build = '22222222-2222-4222-8222-222222222222'
afterEach(() => {
  vi.unstubAllGlobals()
  forgetSession()
})
it('validates plan response and surfaces invalid success bodies', async () => {
  vi.stubGlobal(
    'fetch',
    vi.fn(async () =>
      Response.json({ plan: null, manifest: null, blockers: ['Operator setup required'] }),
    ),
  )
  const query = planQuery('workspace', app, build)
  const fn = query.queryFn as () => Promise<unknown>
  await expect(fn()).resolves.toEqual({
    plan: null,
    manifest: null,
    blockers: ['Operator setup required'],
  })
  vi.stubGlobal(
    'fetch',
    vi.fn(async () => Response.json({ ready: true })),
  )
  await expect(fn()).rejects.toThrow()
})
it('shares session CSRF and retains submission identity on transport failure', async () => {
  setCsrfToken('csrf-test')
  const requests: Request[] = []
  vi.stubGlobal(
    'fetch',
    vi.fn(async (r: Request) => {
      requests.push(r)
      return Response.json(
        { code: 'execution_conflict', message: 'Refresh preview', details: null, request_id: app },
        { status: 409 },
      )
    }),
  )
  const body = { build_id: build, plan_version_id: app, environment_revision: 1 }
  await expect(createRun(app, body, 'same-key')).rejects.toThrow('Refresh preview')
  await expect(createRun(app, body, 'same-key')).rejects.toThrow()
  expect(requests).toHaveLength(2)
  expect(requests[0]?.headers.get('x-csrf-token')).toBe('csrf-test')
  expect(requests.every((r) => r.headers.get('idempotency-key') === 'same-key')).toBe(true)
})
it('pins an explicit plan version in the preview request and cache identity', async () => {
  const plan = '44444444-4444-4444-8444-444444444444'
  const requests: Request[] = []
  vi.stubGlobal(
    'fetch',
    vi.fn(async (request: Request) => {
      requests.push(request)
      return Response.json({ plan: null, manifest: null, blockers: ['Fixture'] })
    }),
  )
  const { QueryClient } = await import('@tanstack/react-query')
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } })
  await client.fetchQuery(planQuery('workspace', app, build, plan))
  expect(new URL(requests[0]!.url).searchParams.get('plan_version_id')).toBe(plan)
  expect(planQuery('workspace', app, build).queryKey).not.toEqual(
    planQuery('workspace', app, build, plan).queryKey,
  )
  client.clear()
})
