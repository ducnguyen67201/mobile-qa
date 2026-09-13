/** Real SDK + synthetic server replies; this is not emulator acceptance. */
import { MantineProvider } from '@mantine/core'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { createMemoryRouter, RouterProvider } from 'react-router'
import { afterEach, expect, it, vi } from 'vitest'
import { routes } from '@/routes'
import { theme } from '@/theme'
import { app, appId, orgId, session, settings } from '@/test/fixtures'
import type { PhoneSession, PhoneTaskRequest } from '@/api/generated/types.gen'
import { zOpenPhoneRequest, zPhoneTaskRequest } from '@/api/generated/zod.gen'
const id = '8fc2fe54-1c10-4c26-8ae6-b6882d2b3e21'
const profile = {
  id,
  name: 'Local phone',
  driver: 'minitap' as const,
  package: 'ai.mobileqa.demo',
  adapter: 'demo_persistence_v1',
  device_identity: 'local-test',
  image: 'synthetic',
  model: 'synthetic',
  qualified: true,
  qualification_reference: 'synthetic',
  max_apk_bytes: 104857600,
}
function show(blocked = false) {
  let current: PhoneSession = {
    id,
    app_id: appId,
    build_id: id,
    profile,
    state: 'ready',
    message: 'Ready',
    frame: null,
    tasks: [],
  }
  const opened: unknown[] = []
  const tasks: PhoneTaskRequest[] = []
  vi.stubGlobal(
    'fetch',
    vi.fn(async (request: Request) => {
      const path = new URL(request.url).pathname
      if (path.endsWith('/session')) return Response.json(session)
      if (path.endsWith('/settings')) return Response.json(settings)
      if (path === `/api/apps/${appId}`) return Response.json(app)
      if (path.endsWith('/phone-options'))
        return Response.json({
          builds: [{ id, name: 'sample.apk' }],
          profiles: blocked ? [] : [profile],
          active_session: null,
          blockers: blocked ? ['A device worker needs to be connected for this app.'] : [],
        })
      if (path.endsWith('/phones')) {
        opened.push(zOpenPhoneRequest.parse(await request.json()))
        return Response.json(current)
      }
      if (path.endsWith('/tasks')) {
        const body = zPhoneTaskRequest.parse(await request.json())
        tasks.push(body)
        current = {
          ...current,
          state: 'acting',
          tasks: [
            {
              id: body.id,
              goal: body.goal,
              control: null,
              state: 'queued',
              message: 'Waiting for Minitap',
            },
          ],
        }
        return Response.json(current)
      }
      if (path.endsWith('/stop')) {
        current = { ...current, state: 'stopping' }
        return Response.json(current)
      }
      if (path === `/api/phones/${id}`) return Response.json(current)
      throw new Error(`Unexpected fixture request ${path}`)
    }),
  )
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } })
  render(
    <MantineProvider theme={theme} env="test">
      <QueryClientProvider client={client}>
        <RouterProvider
          router={createMemoryRouter(routes, {
            initialEntries: [`/apps/${appId}/try?workspace=${orgId}`],
          })}
        />
      </QueryClientProvider>
    </MantineProvider>,
  )
  return { opened, tasks }
}
afterEach(() => vi.unstubAllGlobals())
it('opens the default phone without setup fields and submits the users goal', async () => {
  const { opened, tasks } = show()
  const goal = await screen.findByLabelText('What should Minitap do?')
  await waitFor(() => expect(goal).toBeEnabled())
  expect(opened).toHaveLength(1)
  expect(screen.queryByLabelText('Stable key')).not.toBeInTheDocument()
  expect(screen.queryByLabelText('Choose a device')).not.toBeInTheDocument()
  await userEvent.type(goal, 'Save Buy milk and restart')
  await userEvent.click(screen.getByRole('button', { name: 'Run task' }))
  await waitFor(() => expect(tasks).toHaveLength(1))
  expect(tasks[0]?.goal).toBe('Save Buy milk and restart')
  expect(screen.getByRole('button', { name: 'Run task' })).toBeDisabled()
  await userEvent.click(screen.getByRole('button', { name: 'Stop session' }))
  await waitFor(() => expect(screen.getByRole('button', { name: 'Stop session' })).toBeDisabled())
})
it('explains missing device setup without pretending there is a phone', async () => {
  const { opened } = show(true)
  expect(
    await screen.findByText('A device worker needs to be connected for this app.'),
  ).toBeInTheDocument()
  expect(opened).toHaveLength(0)
  expect(
    screen.queryByRole('img', { name: 'Current screen of your Android app' }),
  ).not.toBeInTheDocument()
  expect(screen.getByRole('button', { name: 'Run task' })).toBeDisabled()
})
