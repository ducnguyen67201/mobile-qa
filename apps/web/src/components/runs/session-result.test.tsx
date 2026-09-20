import { render, screen } from '@testing-library/react'
import { MantineProvider } from '@mantine/core'
import { describe, expect, it } from 'vitest'
import type { RunHistoryItem } from '@/api/generated/types.gen'
import { SessionResult } from './session-result'

const item: RunHistoryItem = {
  id: 'legacy:example',
  source: 'legacy',
  build_label: '1.0-broken',
  created_at: '2026-09-20T13:30:00Z',
  trial: {
    id: 'example',
    goal: 'A saved task survives a restart',
    state: 'failed',
    message: 'Step stopped: assertion failed',
    sequence: {
      actions: [
        { id: 'opaque-step-id', kind: 'restart_app', instruction: '', checkpoint_id: 'restart' },
      ],
      checks: [],
    },
    steps: [{ action_id: 'opaque-step-id', state: 'failed', message: 'assertion failed' }],
  },
}
describe('earlier test results', () => {
  it('shows a failure without inventing a regression and keeps steps collapsed', () => {
    render(
      <MantineProvider>
        <SessionResult item={item} />
      </MantineProvider>,
    )
    expect(screen.getByText('Failed')).toBeInTheDocument()
    expect(screen.getByText('Action 1: An expected result did not match.')).toBeInTheDocument()
    expect(screen.queryByText('Regression')).not.toBeInTheDocument()
    expect(screen.queryByText(/opaque-step-id/)).not.toBeInTheDocument()
    expect(screen.getByText('View steps').closest('details')).not.toHaveAttribute('open')
    expect(screen.getByText(/Restart the app, preserving saved data/)).toBeInTheDocument()
  })
  it('distinguishes a blocked action from an assertion failure', () => {
    render(
      <MantineProvider>
        <SessionResult
          item={{
            ...item,
            trial: {
              ...item.trial!,
              steps: [
                { action_id: 'opaque-step-id', state: 'blocked', message: 'target not unique' },
              ],
            },
          }}
        />
      </MantineProvider>,
    )
    expect(screen.getByText('Blocked')).toBeInTheDocument()
    expect(screen.queryByText('Failed')).not.toBeInTheDocument()
  })
})
