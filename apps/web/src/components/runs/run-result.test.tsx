import { render, screen } from '@testing-library/react'
import { MantineProvider } from '@mantine/core'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter } from 'react-router'
import { describe, it, expect, vi } from 'vitest'
import { RunResult } from './run-result'
import { zRunResponse } from '@/api/generated/zod.gen'
import fixture from '../../../../api/tests/fixtures/execution/comparison.json'
vi.mock('@/hooks/use-workspace', () => ({
  useWorkspace: () => ({ workspaceId: 'test', href: (path: string) => path }),
}))
const display = (comparison?: unknown) => {
  const run = zRunResponse.parse({ ...fixture, comparison })
  render(
    <MantineProvider>
      <QueryClientProvider client={new QueryClient()}>
        <MemoryRouter>
          <RunResult run={run} compact />
        </MemoryRouter>
      </QueryClientProvider>
    </MantineProvider>,
  )
}
describe('run comparison display', () => {
  it('does not invent a regression from an execution failure', () => {
    display()
    expect(screen.getByText('Finalizing comparison…')).toBeInTheDocument()
    expect(screen.queryByText('Regression')).not.toBeInTheDocument()
  })
  it('shows the server verdict and its reason', () => {
    display({
      policy_version: 1,
      baseline_run_id: null,
      cases: [
        {
          case_key: 'persistence',
          title: 'Persistence',
          data_variant: 'default',
          kind: 'regression',
          reason: 'Same saved test failed on the new build',
          baseline_case_id: null,
          current_case_id: null,
          baseline_outcome: 'passed',
          current_outcome: 'failed',
          baseline_checks: [],
          current_checks: [],
        },
      ],
    })
    expect(screen.getByText('Regression')).toBeInTheDocument()
    expect(screen.getByText('Same saved test failed on the new build')).toBeInTheDocument()
  })
})
