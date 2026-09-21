import { MantineProvider } from '@mantine/core'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { expect, it, vi } from 'vitest'
import { theme } from '@/theme'
import { emptyCoverage, libraryCase, libraryEntry, capabilities } from '@/test/library-fixtures'
import type {
  LibraryOptionsResponse,
  LibraryVersionResponse,
  PlanDraftContent,
} from '@/api/generated/types.gen'
import { CaseMembership, PlanFields } from './membership-fields'

const caseChoice: LibraryVersionResponse = {
  entry: { ...libraryEntry, latest_version_id: 'case-version' },
  version: {
    id: 'case-version',
    app_id: libraryEntry.app_id,
    content_hash: 'a'.repeat(64),
    definition: { kind: 'case', content: libraryCase },
    approvals: [],
  },
  coverage: emptyCoverage,
  issues: [],
}
const suiteChoice: LibraryVersionResponse = {
  entry: {
    ...libraryEntry,
    id: 'suite-entry',
    kind: 'suite',
    key: 'smoke-suite',
    title: 'Smoke suite',
    latest_version_id: 'suite-version',
  },
  version: {
    id: 'suite-version',
    app_id: libraryEntry.app_id,
    content_hash: 'b'.repeat(64),
    definition: {
      kind: 'suite',
      content: { key: 'smoke-suite', title: 'Smoke suite', version: 2, cases: [] },
    },
    approvals: [],
  },
  coverage: emptyCoverage,
  issues: [],
}
const options: LibraryOptionsResponse = {
  capabilities,
  profiles: [],
  saved_versions: [caseChoice, suiteChoice],
}

it('shows independent suite cases and keeps explicit removal controls below the canvas', async () => {
  const onChange = vi.fn()
  render(
    <MantineProvider theme={theme} env="test">
      <CaseMembership
        selections={[{ case_version_id: 'case-version', required: true, data_variant: 'default' }]}
        options={options}
        onChange={onChange}
        kind="suite"
      />
    </MantineProvider>,
  )
  expect(screen.getByText(/Case 1 · A saved task survives a restart/)).toBeInTheDocument()
  await userEvent.click(screen.getByRole('button', { name: 'Remove case selection 1' }))
  expect(onChange).toHaveBeenCalledWith([])
  expect(screen.queryByRole('combobox', { name: /Add saved case/ })).not.toBeInTheDocument()
})

it('shows suite order and keeps current advanced values visible', () => {
  const onChange = vi.fn()
  const value: PlanDraftContent = {
    key: 'release',
    title: 'Release',
    version: 1,
    suite_version_ids: ['suite-version'],
    cases: [],
    profile_id: null,
    diagnostic_retries: 1,
    exclusions: ['Payments'],
    budget: { duration_seconds: 480, max_steps: 30, artifact_bytes: 16_777_216 },
  }
  render(
    <MantineProvider theme={theme} env="test">
      <PlanFields value={value} options={options} onChange={onChange} />
    </MantineProvider>,
  )
  expect(screen.getByText(/Suite 1 · Smoke suite/)).toBeInTheDocument()
  expect(screen.queryByRole('combobox', { name: /Add saved suite/ })).not.toBeInTheDocument()
  expect(screen.getByText(/No profile · 480s · 1 retry · 1 exclusion/)).toBeInTheDocument()
  expect(screen.getByText('Execution profile required')).toBeInTheDocument()
})
