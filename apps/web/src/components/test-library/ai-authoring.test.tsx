/** Explicit discovery admission through generated request and response boundaries. */
import { MantineProvider } from '@mantine/core'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, expect, it, vi } from 'vitest'
import { GenerationPanel } from './ai-authoring'
import type { PhoneSession } from '@/api/generated/types.gen'
import { zGenerateTestsRequest } from '@/api/generated/zod.gen'

const id = '8fc2fe54-1c10-4c26-8ae6-b6882d2b3e21'
const session: PhoneSession = {
  id,
  app_id: id,
  build_id: id,
  protocol_version: 3,
  revision: 0,
  environment_revision: 1,
  state: 'ready',
  message: '',
  frame: null,
  tasks: [],
  profile: {
    id,
    name: 'Phone',
    driver: 'minitap',
    package: 'ai.mobileqa.demo',
    adapter: 'demo_persistence_v1',
    device_identity: 'test',
    image: 'test',
    model: 'test',
    qualified: true,
    qualification_reference: 'synthetic',
    max_apk_bytes: 1000,
  },
}
function show(value = session, onReconnect?: () => void) {
  const success = vi.fn()
  render(
    <MantineProvider>
      <QueryClientProvider
        client={new QueryClient({ defaultOptions: { mutations: { retry: false } } })}
      >
        <GenerationPanel session={value} onSession={success} onReconnect={onReconnect} />
      </QueryClientProvider>
    </MantineProvider>,
  )
  return success
}
afterEach(() => vi.unstubAllGlobals())
it.each([3, 4])(
  'protocol %i sends explicit exploration with the engine and scope',
  async (protocol) => {
    const fetch = vi.fn(async (r: Request) => {
      const body = zGenerateTestsRequest.parse(await r.json())
      expect(body.engine).toBe('minitap_v1')
      expect(body.allow_writes).toBe(true)
      expect(body.journey).toContain('enter sample test data')
      return Response.json(session)
    })
    vi.stubGlobal('fetch', fetch)
    const success = show({ ...session, protocol_version: protocol })
    expect(fetch).not.toHaveBeenCalled()
    expect(screen.getByRole('button', { name: 'Explore with AI' })).toBeDisabled()
    await userEvent.click(screen.getByRole('button', { name: 'Explore with AI' }))
    expect(fetch).not.toHaveBeenCalled()
    await userEvent.click(
      screen.getByRole('checkbox', { name: 'Allow AI to tap, type and change test data' }),
    )
    await userEvent.click(screen.getByRole('button', { name: 'Explore with AI' }))
    await waitFor(() => expect(success).toHaveBeenCalled())
    expect(fetch).toHaveBeenCalledTimes(1)
  },
)
it('keeps exploration unavailable for incompatible sessions and offers reconnect', async () => {
  const reconnect = vi.fn()
  show({ ...session, protocol_version: 2 }, reconnect)
  expect(screen.getByRole('button', { name: 'Explore with AI' })).toBeDisabled()
  expect(screen.getByText(/Your phone is connected/)).toBeInTheDocument()
  await userEvent.click(screen.getByRole('button', { name: 'Reconnect phone' }))
  expect(reconnect).toHaveBeenCalledTimes(1)
})
it('stops the active generation through its own route', async () => {
  const fetch = vi.fn(async (r: Request) => {
    expect(new URL(r.url).pathname).toBe(`/api/apps/${id}/test-generations/${id}/cancel`)
    return Response.json(session)
  })
  vi.stubGlobal('fetch', fetch)
  const success = show({
    ...session,
    state: 'acting',
    tasks: [
      {
        id,
        goal: 'Explore',
        control: null,
        state: 'acting',
        message: '',
        generation: {
          id,
          session_id: id,
          expected_revision: 0,
          category: 'smoke',
          journey: '',
          allow_writes: false,
          reuse_job_id: null,
          engine: 'minitap_v1',
        },
      },
    ],
  })
  await userEvent.click(screen.getByRole('button', { name: 'Stop exploration' }))
  await waitFor(() => expect(success).toHaveBeenCalled())
})

it('preserves a custom journey in the exploration request', async () => {
  const fetch = vi.fn(async (r: Request) => {
    expect(zGenerateTestsRequest.parse(await r.json()).journey).toBe('Save a task named Hello')
    return Response.json(session)
  })
  vi.stubGlobal('fetch', fetch)
  const success = show()
  await userEvent.type(
    screen.getByRole('textbox', { name: 'What should we explore? (optional)' }),
    'Save a task named Hello',
  )
  await userEvent.click(
    screen.getByRole('checkbox', { name: 'Allow AI to tap, type and change test data' }),
  )
  await userEvent.click(screen.getByRole('button', { name: 'Explore with AI' }))
  await waitFor(() => expect(success).toHaveBeenCalled())
})
