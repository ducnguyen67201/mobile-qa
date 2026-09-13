/** Real SDK + synthetic server replies; this is not emulator acceptance. */
import { MantineProvider } from '@mantine/core'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { createMemoryRouter, RouterProvider } from 'react-router'
import { afterEach, expect, it, vi } from 'vitest'
import { routes } from '@/routes'
import { theme } from '@/theme'
import { app, appId, orgId, session, settings } from '@/test/fixtures'
import { entryId, libraryDraft, libraryEntry, libraryOptions } from '@/test/library-fixtures'
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
function show(blocked = false, embedded = false) {
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
          left: 20,
          top: 100,
          right: 380,
          bottom: 160,
        },
        {
          id: 'save',
          resource_id: 'ai.mobileqa.demo:id/save',
          label: 'Save',
          left: 20,
          top: 180,
          right: 380,
          bottom: 240,
        },
      ],
    },
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
      if (path.endsWith('/tasks')) {
        const body = zPhoneTaskRequest.parse(await request.json())
        tasks.push(body)
        current = {
          ...current,
          state: 'acting',
          frame: current.frame ? { ...current.frame, png_base64: 'dXBkYXRlZA==' } : null,
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
it('opens the default phone without setup fields and submits the users goal', async () => {
  const { opened, tasks } = show()
  const goal = await screen.findByLabelText('Step 1 instruction')
  await waitFor(() => expect(opened).toHaveLength(1))
  expect(goal).toBeEnabled()
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

it('runs beside the saved draft without navigating or losing unsaved edits', async () => {
  const { opened, tasks } = show(false, true)
  const title = await screen.findByLabelText('Case title')
  await userEvent.clear(title)
  await userEvent.type(title, 'My unsaved task')
  const setup = screen.getByRole('region', { name: 'Task setup' })
  const preview = screen.getByRole('complementary', { name: 'App preview' })
  const goal = within(setup).getByLabelText('Step 1 instruction')
  await userEvent.type(goal, 'Enter Buy milk and save it')
  expect(opened).toHaveLength(0)
  expect(within(setup).getByRole('button', { name: 'Run task' })).toBeDisabled()
  await userEvent.click(await within(preview).findByRole('button', { name: 'Open phone preview' }))
  const control = await within(preview).findByRole('button', { name: 'Select Task input' })
  await userEvent.click(control)
  expect(within(setup).getByText('Starting control: Task input')).toBeInTheDocument()
  await userEvent.click(within(setup).getByRole('button', { name: 'Run task' }))
  await waitFor(() => expect(tasks).toHaveLength(1))
  expect(tasks[0]).toMatchObject({
    goal: 'Enter Buy milk and save it',
    selection: { frame_id: id, control_id: 'task-input' },
  })
  await waitFor(() =>
    expect(within(preview).getByRole('img')).toHaveAttribute(
      'src',
      'data:image/png;base64,dXBkYXRlZA==',
    ),
  )
  expect(title).toHaveValue('My unsaved task')
  expect(screen.getByText('Unsaved changes')).toBeInTheDocument()
  expect(
    within(preview).queryByRole('button', { name: 'Select Task input' }),
  ).not.toBeInTheDocument()
})

it('adds and reorders action steps before submitting one ordered Minitap goal', async () => {
  const { opened, tasks } = show()
  await waitFor(() => expect(opened).toHaveLength(1))
  await userEvent.selectOptions(await screen.findByLabelText('Step 1 action'), 'type')
  await userEvent.type(screen.getByLabelText('Step 1 target'), 'task input')
  await userEvent.type(screen.getByLabelText('Step 1 text'), 'Buy milk')
  await userEvent.click(screen.getByRole('button', { name: 'Add step' }))
  expect(screen.getByRole('button', { name: 'Run task' })).toBeDisabled()
  await userEvent.selectOptions(screen.getByLabelText('Step 2 action'), 'restart')
  await userEvent.click(screen.getByRole('button', { name: 'Move step 2 up' }))
  await userEvent.click(screen.getByRole('button', { name: 'Add step' }))
  await userEvent.type(screen.getByLabelText('Step 3 instruction'), 'Unwanted step')
  await userEvent.click(screen.getByRole('button', { name: 'Remove step 3' }))
  await userEvent.click(screen.getByRole('button', { name: 'Run task' }))
  await waitFor(() => expect(tasks).toHaveLength(1))
  expect(tasks[0]?.goal).toBe(
    'Perform these steps in order. Stop if a step cannot be completed.\n1. Restart the app without clearing its saved data.\n2. Enter "Buy milk" into task input.',
  )
  expect(screen.getByLabelText('Step 2 text')).toHaveValue('Buy milk')
})

it('blocks an oversized combined task before making an API request', async () => {
  const { opened, tasks } = show()
  await waitFor(() => expect(opened).toHaveLength(1))
  fireEvent.change(await screen.findByLabelText('Step 1 instruction'), {
    target: { value: 'a'.repeat(3990) },
  })
  await userEvent.click(screen.getByRole('button', { name: 'Add step' }))
  await userEvent.selectOptions(screen.getByLabelText('Step 2 action'), 'back')
  expect(screen.getByText('Shorten your steps to fit within 4,000 characters.')).toBeInTheDocument()
  expect(screen.getByRole('button', { name: 'Run task' })).toBeDisabled()
  expect(tasks).toHaveLength(0)
})

it('picks separate targets for steps and retains their identity through reordering', async () => {
  const { opened, tasks } = show()
  await waitFor(() => expect(opened).toHaveLength(1))
  await userEvent.selectOptions(await screen.findByLabelText('Step 1 action'), 'type')
  await userEvent.click(screen.getByRole('button', { name: 'Pick target for step 1 on phone' }))
  expect(screen.getByText('Choose a target for Step 1')).toBeInTheDocument()
  await userEvent.click(screen.getByRole('button', { name: 'Select Task input' }))
  expect(screen.getByLabelText('Step 1 target')).toHaveValue('Task input')
  await userEvent.type(screen.getByLabelText('Step 1 text'), 'Buy milk')
  await userEvent.click(screen.getByRole('button', { name: 'Add step' }))
  await userEvent.selectOptions(screen.getByLabelText('Step 2 action'), 'tap')
  await userEvent.click(screen.getByRole('button', { name: 'Pick target for step 2 on phone' }))
  await userEvent.click(screen.getByRole('button', { name: 'Select Save' }))
  expect(screen.getByLabelText('Step 2 target')).toHaveValue('Save')
  expect(tasks).toHaveLength(0)
  await userEvent.click(screen.getByRole('button', { name: 'Move step 2 up' }))
  expect(screen.getByLabelText('Step 1 target')).toHaveValue('Save')
  await userEvent.click(screen.getByRole('button', { name: 'Run task' }))
  await waitFor(() => expect(tasks).toHaveLength(1))
  expect(tasks[0]?.goal).toContain(
    '1. Tap the control labelled "Save" with resource ID "ai.mobileqa.demo:id/save".',
  )
  expect(tasks[0]?.goal).toContain(
    '2. Enter "Buy milk" into the control labelled "Task input" with resource ID "ai.mobileqa.demo:id/task_input".',
  )
  expect(tasks[0]?.selection).toBeNull()
})

it('cancels picking and clears captured identity when the user describes a different target', async () => {
  const { opened, tasks } = show()
  await waitFor(() => expect(opened).toHaveLength(1))
  await userEvent.selectOptions(await screen.findByLabelText('Step 1 action'), 'tap')
  const pick = screen.getByRole('button', { name: 'Pick target for step 1 on phone' })
  await userEvent.click(pick)
  await userEvent.keyboard('{Escape}')
  expect(pick).toHaveAttribute('aria-pressed', 'false')
  await userEvent.click(pick)
  await userEvent.click(screen.getByRole('button', { name: 'Select Save' }))
  await userEvent.clear(screen.getByLabelText('Step 1 target'))
  await userEvent.type(screen.getByLabelText('Step 1 target'), 'Cancel button')
  expect(screen.queryByText('Selected from the app: Save')).not.toBeInTheDocument()
  await userEvent.click(screen.getByRole('button', { name: 'Run task' }))
  await waitFor(() => expect(tasks).toHaveLength(1))
  expect(tasks[0]?.goal).toBe('Tap Cancel button.')
})
