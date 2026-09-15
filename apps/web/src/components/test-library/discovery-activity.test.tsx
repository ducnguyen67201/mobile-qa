import { MantineProvider } from '@mantine/core'
import { render, screen, within } from '@testing-library/react'
import { expect, it } from 'vitest'
import type { PhoneFrame, PhoneTask } from '@/api/generated/types.gen'
import {
  DiscoveryActivity,
  DiscoveryTarget,
  discoveryGap,
  explorationLabel,
} from './discovery-activity'

const frame: PhoneFrame = {
  id: 'frame',
  width: 100,
  height: 200,
  png_base64: '',
  controls: [
    {
      id: 'input',
      resource_id: 'app:id/input',
      label: 'Task name',
      left: 10,
      top: 20,
      right: 90,
      bottom: 40,
    },
  ],
}
const task: PhoneTask = {
  id: 'task',
  goal: 'Smoke',
  state: 'acting',
  control: null,
  message: '',
  progress: {
    state: 'discovering',
    proposals: [],
    gaps: [],
    trace: [],
    usage: { calls: 1, input_tokens: 20, output_tokens: 10, unknown_calls: 0 },
    snapshots: [{ id: 'before', frame }],
    journal: [
      {
        id: 'action',
        before_id: 'before',
        outcome: 'pending',
        command: {
          operation: 'set_text',
          target: { by: 'resource_id', value: 'app:id/input' },
          text: 'Hello',
        },
      },
    ],
  },
}
it('shows real numbered actions and updates their outcome', () => {
  const { rerender } = render(
    <MantineProvider>
      <DiscoveryActivity task={task} />
    </MantineProvider>,
  )
  const list = screen.getByRole('list', { name: 'Explored actions' })
  expect(within(list).getAllByRole('listitem')).toHaveLength(1)
  expect(within(list).getByText('Enter “Hello” into Task name')).toBeInTheDocument()
  expect(within(list).getByText('Acting')).toBeInTheDocument()
  rerender(
    <MantineProvider>
      <DiscoveryActivity
        task={{
          ...task,
          progress: {
            ...task.progress!,
            journal: task.progress!.journal!.map((r) => ({ ...r, outcome: 'completed' })),
          },
        }}
      />
    </MantineProvider>,
  )
  expect(screen.getByText('Done')).toBeInTheDocument()
  expect(screen.getByText('1 screens observed · 1 actions recorded')).toBeInTheDocument()
})
it('highlights only a pending target on its exact observation', () => {
  const { rerender } = render(<DiscoveryTarget task={task} frame={frame} />)
  expect(screen.getByRole('img', { name: /AI target/ })).toHaveStyle({
    left: '10%',
    top: '10%',
    width: '80%',
    height: '10%',
  })
  rerender(<DiscoveryTarget task={task} frame={{ ...frame, id: 'newer-frame' }} />)
  expect(screen.queryByRole('img')).not.toBeInTheDocument()
  rerender(
    <DiscoveryTarget
      task={task}
      frame={{
        ...frame,
        controls: [...frame.controls, { ...frame.controls[0]!, id: 'duplicate' }],
      }}
    />,
  )
  expect(screen.queryByRole('img')).not.toBeInTheDocument()
  rerender(<DiscoveryTarget task={{ ...task, state: 'completed' }} frame={frame} />)
  expect(screen.queryByRole('img')).not.toBeInTheDocument()
})
it('does not call an empty result completed or expose a raw gap code', () => {
  expect(explorationLabel({ ...task, state: 'completed' })).toBe('No tests generated')
  expect(discoveryGap('discovery_interaction_scope_required')).toContain(
    'Enable “Allow AI to tap, type and change test data”',
  )
  expect(discoveryGap('unexpected_internal_error')).not.toContain('unexpected_internal_error')
  expect(discoveryGap('Sign in to reach this screen.')).toBe('Sign in to reach this screen.')
})
