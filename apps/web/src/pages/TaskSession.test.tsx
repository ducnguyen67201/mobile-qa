/** Real SDK + synthetic server replies; this is not emulator acceptance. */
import { MantineProvider } from '@mantine/core'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { createMemoryRouter, RouterProvider } from 'react-router'
import { afterEach, expect, it, vi } from 'vitest'
import { routes } from '@/routes'
import { theme } from '@/theme'
import { app, appId, orgId, session, settings } from '@/test/fixtures'
import { entryId, libraryDraft, libraryEntry, libraryOptions } from '@/test/library-fixtures'
import type { PhoneSession, PhoneCommandRequest } from '@/api/generated/types.gen'
import { zOpenPhoneRequest, zPhoneCommandRequest } from '@/api/generated/zod.gen'
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
function show(blocked = false, embedded = false, protocolVersion = 4) {
  let current: PhoneSession = {
    id,
    app_id: appId,
    build_id: id,
    profile,
    state: 'ready',
    message: 'Ready',
    frame: {
      id,
      width: 400,
      height: 800,
      png_base64: 'aW5pdGlhbA==',
      controls: [
        {
          id: 'task-input',
          resource_id: 'ai.mobileqa.demo:id/task_input',
          label: 'Task input',
          editable: true,
          description: '',
          left: 20,
          top: 100,
          right: 380,
          bottom: 160,
        },
        {
          id: 'save',
          resource_id: 'ai.mobileqa.demo:id/save',
          label: 'Save',
          editable: false,
          description: '',
          left: 20,
          top: 180,
          right: 380,
          bottom: 240,
        },
      ],
    },
    protocol_version: protocolVersion,
    revision: 0,
    environment_revision: 1,
    tasks: [],
    resolved_model: {
      reference: { key: 'synthetic.openai', revision: 1 },
      display_name: 'Synthetic',
      provider: 'open_ai',
      provider_model: 'synthetic-model',
      capabilities: ['minitap_navigation', 'structured_authoring'],
    },
  }
  const opened: unknown[] = []
  const tasks: PhoneCommandRequest[] = []
  vi.stubGlobal(
    'fetch',
    vi.fn(async (request: Request) => {
      const path = new URL(request.url).pathname
      if (path.endsWith('/session')) return Response.json(session)
      if (path.endsWith('/settings')) return Response.json(settings)
      if (path === `/api/apps/${appId}`) return Response.json(app)
      if (path.endsWith('/test-library/options')) return Response.json(libraryOptions)
      if (path.endsWith(`/test-library/${entryId}`)) return Response.json(libraryEntry)
      if (path.endsWith('/draft')) return Response.json(libraryDraft)
      if (path.endsWith('/versions')) return Response.json({ items: [], next_cursor: null })
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
      if (path.endsWith('/commands')) {
        const body = zPhoneCommandRequest.parse(await request.json())
        tasks.push(body)
        current = {
          ...current,
          state: 'ready',
          revision: (current.revision ?? 0) + 1,
          frame: current.frame ? { ...current.frame, png_base64: 'dXBkYXRlZA==' } : null,
          tasks: [
            {
              id: body.id,
              goal: body.title,
              sequence: body.sequence,
              generation: null,
              progress: null,
              steps: body.sequence.actions.map((a) => ({
                action_id: a.id,
                state: 'completed',
                message: 'Done',
              })),
              control: null,
              state: 'completed',
              message: 'Actions completed',
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
            initialEntries: [
              `${embedded ? `/tests/${appId}/${entryId}` : `/apps/${appId}/try`}?workspace=${orgId}`,
            ],
          })}
        />
      </QueryClientProvider>
    </MantineProvider>,
  )
  return { opened, tasks }
}
afterEach(() => vi.unstubAllGlobals())
it.each([2, 3, 4])('protocol %i binds and runs a typed target without AI', async (protocol) => {
  const { opened, tasks } = show(false, false, protocol)
  await waitFor(() => expect(opened).toHaveLength(1))
  await userEvent.click(await screen.findByRole('combobox', { name: 'Action 1 action' }))
  await userEvent.click(screen.getByRole('option', { name: 'Enter text' }))
  await userEvent.click(screen.getByRole('button', { name: 'Pick target for action 1 on phone' }))
  await userEvent.click(screen.getByRole('button', { name: 'Select Task input' }))
  await userEvent.type(screen.getByLabelText('Action 1 text'), 'Hello')
  expect(tasks).toHaveLength(0)
  expect(screen.getByText('Direct execution · no AI calls')).toBeInTheDocument()
  await userEvent.click(screen.getByRole('button', { name: 'Run test' }))
  await waitFor(() => expect(tasks).toHaveLength(1))
  expect(tasks[0]?.sequence.actions[0]?.command).toEqual({
    operation: 'set_text',
    target: { by: 'resource_id', value: 'ai.mobileqa.demo:id/task_input' },
    text: 'Hello',
  })
  expect(tasks[0]?.frame_id).toBeNull()
})
it('controls the phone immediately and records each completed receipt once', async () => {
  const { opened, tasks } = show()
  await waitFor(() => expect(opened).toHaveLength(1))
  await userEvent.click(screen.getByRole('button', { name: 'Control phone' }))
  await userEvent.click(screen.getByLabelText('Record interactions into this test'))
  await userEvent.click(screen.getByRole('button', { name: 'Interact with Task input' }))
  await userEvent.type(screen.getByLabelText('Text to enter'), 'Xin chào')
  await userEvent.click(screen.getByRole('button', { name: 'Enter text' }))
  await waitFor(() => expect(tasks).toHaveLength(1))
  expect(tasks[0]?.frame_id).toBe(id)
  await waitFor(() => expect(screen.getByLabelText('Action 1 text')).toHaveValue('Xin chào'))
  expect(screen.queryByLabelText('Action 2 action')).not.toBeInTheDocument()
})
it('reorders direct steps and only marks an explicit Ask AI step as AI', async () => {
  const { opened, tasks } = show()
  await waitFor(() => expect(opened).toHaveLength(1))
  await userEvent.click(screen.getByRole('combobox', { name: 'Action 1 action' }))
  await userEvent.click(screen.getByRole('option', { name: 'Go back' }))
  await userEvent.click(screen.getByRole('button', { name: 'Add action' }))
  await userEvent.click(screen.getByRole('combobox', { name: 'Action 2 action' }))
  await userEvent.click(screen.getByRole('option', { name: 'Ask AI' }))
  await userEvent.type(screen.getByLabelText('Action 2 AI instruction'), 'Explore settings')
  await userEvent.click(screen.getByRole('button', { name: 'Move action 2 up' }))
  await userEvent.click(screen.getByRole('button', { name: 'Run test' }))
  await waitFor(() => expect(tasks).toHaveLength(1))
  expect(tasks[0]?.sequence.actions.map((a) => a.kind)).toEqual(['navigate', 'direct'])
})
it('preserves an embedded drafts unsaved title while opening its preview', async () => {
  const { opened } = show(false, true)
  const title = await screen.findByLabelText('Test name')
  await userEvent.clear(title)
  await userEvent.type(title, 'My unsaved test')
  expect(opened).toHaveLength(0)
  await userEvent.click(screen.getByRole('button', { name: 'Open phone preview' }))
  await waitFor(() => expect(opened).toHaveLength(1))
  expect(title).toHaveValue('My unsaved test')
  expect(screen.getByText('Unsaved changes')).toBeInTheDocument()
  expect(
    within(screen.getByRole('region', { name: 'Task setup' })).getByLabelText(
      'Action 1 AI instruction',
    ),
  ).toHaveValue('Create and save ${task_title}.')
})
it('shows missing setup and lets Escape cancel target picking', async () => {
  const { opened } = show(true)
  expect(
    await screen.findByText('A device worker needs to be connected for this app.'),
  ).toBeInTheDocument()
  expect(opened).toHaveLength(0)
  expect(screen.getByRole('button', { name: 'Run test' })).toBeDisabled()
})

it('adds optional checks inside an action and keeps their binding when reordered', async () => {
  const { opened } = show()
  await waitFor(() => expect(opened).toHaveLength(1))
  expect(screen.queryByLabelText('Check 1 description')).not.toBeInTheDocument()
  await userEvent.click(screen.getByRole('button', { name: 'Add action' }))
  expect(screen.getByRole('button', { name: 'Edit action 1' })).toHaveAttribute(
    'aria-expanded',
    'false',
  )
  expect(screen.getByRole('button', { name: 'Edit action 2' })).toHaveAttribute(
    'aria-expanded',
    'true',
  )
  await userEvent.click(screen.getByRole('button', { name: 'Add check to action 2' }))
  await waitFor(() => expect(screen.getByLabelText('Check 1 description')).toBeVisible())
  expect(screen.queryByLabelText('Check 1 after step')).not.toBeInTheDocument()
  await userEvent.click(screen.getByRole('button', { name: 'Move action 2 up' }))
  expect(screen.getByRole('button', { name: 'Edit action 1' })).toHaveAttribute(
    'aria-expanded',
    'true',
  )
  await waitFor(() => expect(screen.getByLabelText('Check 1 description')).toBeVisible())
  await userEvent.click(screen.getByRole('button', { name: 'Remove check' }))
  expect(screen.queryByLabelText('Check 1 description')).not.toBeInTheDocument()
})

it.each([false, true])(
  'repicks result targets with independent readiness: %s',
  async (independent) => {
    const { opened, tasks } = show()
    await waitFor(() => expect(opened).toHaveLength(1))
    await userEvent.click(screen.getByRole('button', { name: 'Pick target for action 1 on phone' }))
    await userEvent.click(screen.getByRole('button', { name: 'Select Save' }))
    await userEvent.click(screen.getByRole('button', { name: 'Add check to action 1' }))
    await userEvent.click(
      await screen.findByRole('button', { name: 'Pick target for check 1 on phone' }),
    )
    await userEvent.click(screen.getByRole('button', { name: 'Select Task input' }))
    if (independent) {
      await userEvent.click(screen.getByText('Screen readiness', { exact: true }))
      await userEvent.click(
        screen.getByRole('button', { name: 'Pick screen readiness for check 1 on phone' }),
      )
      await userEvent.click(screen.getByRole('button', { name: 'Select Save' }))
    }
    await userEvent.click(
      await screen.findByRole('button', { name: 'Pick target for check 1 on phone' }),
    )
    await userEvent.click(
      screen.getByRole('button', { name: independent ? 'Select Task input' : 'Select Save' }),
    )
    await userEvent.click(screen.getByRole('button', { name: 'Run test' }))
    await waitFor(() => expect(tasks).toHaveLength(1))
    expect(tasks[0]?.sequence.checks[0]).toMatchObject({
      resource_id: independent ? 'ai.mobileqa.demo:id/task_input' : 'ai.mobileqa.demo:id/save',
      ready_resource_id: 'ai.mobileqa.demo:id/save',
    })
  },
)
