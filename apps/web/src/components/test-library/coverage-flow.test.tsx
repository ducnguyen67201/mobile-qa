import { MantineProvider } from '@mantine/core'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { expect, it, vi } from 'vitest'
import type {
  AttemptResponse,
  CaseDefinition,
  CaseSelection,
  LibraryDraftDefinition,
  LibraryOptionsResponse,
  LibraryVersionResponse,
  RunResponse,
  SuiteDefinition,
} from '@/api/generated/types.gen'
import { theme } from '@/theme'
import { app, appId, buildId, timestamp } from '@/test/fixtures'
import { capabilities, emptyCoverage, libraryCase, libraryEntry } from '@/test/library-fixtures'
import {
  buildLibraryCoverageFlow,
  buildRunCoverageFlow,
  focusAttempt,
  LibraryCoverageFlow,
  RunCoverageFlow,
} from './coverage-flow'

function selection(id: string, required = true, dataVariant = 'default'): CaseSelection {
  return { case_version_id: id, required, data_variant: dataVariant }
}
function caseVersion(
  id: string,
  title: string,
  version: number,
  logicalKey = `case-${id}`,
): LibraryVersionResponse {
  const content: CaseDefinition = { ...libraryCase, title, version, key: logicalKey }
  return {
    entry: {
      ...libraryEntry,
      id: `entry-${logicalKey}`,
      title,
      key: logicalKey,
      latest_version_id: id,
    },
    version: {
      id,
      app_id: appId,
      content_hash: id.repeat(64).slice(0, 64),
      definition: { kind: 'case', content },
      approvals: [],
    },
    coverage: emptyCoverage,
    issues: [],
  }
}
function suiteVersion(id: string, title: string, cases: CaseSelection[]): LibraryVersionResponse {
  const content: SuiteDefinition = { key: `suite-${id}`, title, version: 3, cases }
  return {
    entry: {
      ...libraryEntry,
      id: `entry-${id}`,
      kind: 'suite',
      title,
      key: content.key,
      latest_version_id: id,
    },
    version: {
      id,
      app_id: appId,
      content_hash: id.repeat(64).slice(0, 64),
      definition: { kind: 'suite', content },
      approvals: [],
    },
    coverage: emptyCoverage,
    issues: [],
  }
}

const signIn = caseVersion('1', 'Valid sign-in', 4, 'sign-in')
const invalidPassword = caseVersion('2', 'Invalid password', 2, 'invalid-password')
const backendUnavailable = caseVersion('3', 'Backend unavailable', 1, 'backend-unavailable')
const signInV5 = caseVersion('5', 'Valid sign-in', 5, 'sign-in')
const authentication = suiteVersion('4', 'Authentication', [
  selection(signIn.version.id),
  selection(invalidPassword.version.id, false),
])
const options: LibraryOptionsResponse = {
  capabilities,
  profiles: [],
  saved_versions: [signIn, invalidPassword, backendUnavailable, signInV5, authentication],
}
const plan: LibraryDraftDefinition = {
  kind: 'plan',
  content: {
    key: 'release-check',
    title: 'Release check',
    version: 2,
    suite_version_ids: [authentication.version.id],
    cases: [selection(invalidPassword.version.id), selection(backendUnavailable.version.id)],
    profile_id: null,
    diagnostic_retries: 0,
    exclusions: [],
    budget: { duration_seconds: 600, max_steps: 30, artifact_bytes: 16_777_216 },
  },
}

it('adds saved coverage directly from the canvas editor', async () => {
  const onAddCase = vi.fn()
  const suite: LibraryDraftDefinition = {
    kind: 'suite',
    content: { key: 'smoke', title: 'Smoke', version: 1, cases: [] },
  }
  const { unmount } = render(
    <MantineProvider theme={theme} env="test">
      <LibraryCoverageFlow definition={suite} options={options} onAddCase={onAddCase} />
    </MantineProvider>,
  )
  await userEvent.click(screen.getByRole('combobox', { name: 'Add saved case to sequence' }))
  await userEvent.click(screen.getByRole('option', { name: 'Valid sign-in · v4' }))
  expect(onAddCase).toHaveBeenCalledWith(signIn.version.id)
  unmount()

  const onAddSuite = vi.fn()
  const emptyPlan: LibraryDraftDefinition = {
    kind: 'plan',
    content: {
      ...(plan.kind === 'plan' ? plan.content : {}),
      suite_version_ids: [],
      cases: [],
    } as Extract<LibraryDraftDefinition, { kind: 'plan' }>['content'],
  }
  render(
    <MantineProvider theme={theme} env="test">
      <LibraryCoverageFlow
        definition={emptyPlan}
        options={options}
        onAddCase={vi.fn()}
        onAddSuite={onAddSuite}
      />
    </MantineProvider>,
  )
  await userEvent.click(screen.getByRole('combobox', { name: 'Add saved suite to sequence' }))
  await userEvent.click(screen.getByRole('option', { name: /Authentication/ }))
  expect(onAddSuite).toHaveBeenCalledWith(authentication.version.id)
})

it('connects suite cases as an explicit numbered sequence', () => {
  const suite: LibraryDraftDefinition = {
    kind: 'suite',
    content: {
      key: 'ordered',
      title: 'Ordered smoke',
      version: 1,
      cases: [
        selection(signIn.version.id),
        selection(invalidPassword.version.id),
        selection(backendUnavailable.version.id),
      ],
    },
  }
  const model = buildLibraryCoverageFlow(suite, options)
  expect(
    model.nodes.filter((node) => node.type === 'case').map((node) => node.data.eyebrow),
  ).toEqual(['Step 1', 'Step 2', 'Step 3'])
  expect(model.edges.filter((item) => item.id.startsWith('sequence-'))).toHaveLength(2)
  expect(model.edges[0]).toMatchObject({
    source: 'coverage-root',
    target: expect.stringContaining('case-1-'),
  })
})

it('deduplicates identical logical cases across suites and direct selections', () => {
  const secondSuite = suiteVersion('6', 'Identity', [selection(signIn.version.id)])
  const exactPlan: LibraryDraftDefinition = {
    kind: 'plan',
    content: {
      ...(plan.kind === 'plan' ? plan.content : {}),
      suite_version_ids: [authentication.version.id, secondSuite.version.id],
      cases: [selection(signIn.version.id), selection(backendUnavailable.version.id)],
    } as Extract<LibraryDraftDefinition, { kind: 'plan' }>['content'],
  }
  const model = buildLibraryCoverageFlow(exactPlan, {
    ...options,
    saved_versions: [...options.saved_versions, secondSuite],
  })
  expect(model.nodes.filter((node) => node.type === 'case').map((node) => node.data.title)).toEqual(
    ['Valid sign-in', 'Backend unavailable', 'Invalid password'],
  )
  expect(model.nodes.filter((node) => node.type === 'case').map((node) => node.data.order)).toEqual(
    [1, 2, 3],
  )
  expect(model.nodes.find((node) => node.id.startsWith('suite-1'))?.data.count).toBe(0)
})

it('numbers mixed plan coverage in the same direct-then-suite order as the manifest', () => {
  if (plan.kind !== 'plan') throw new Error('fixture must be a plan')
  const model = buildLibraryCoverageFlow(
    {
      kind: 'plan',
      content: { ...plan.content, cases: [selection(backendUnavailable.version.id)] },
    },
    options,
  )
  expect(model.nodes.filter((node) => node.type === 'case').map((node) => node.data.title)).toEqual(
    ['Backend unavailable', 'Valid sign-in', 'Invalid password'],
  )
  expect(model.nodes.filter((node) => node.type === 'case').map((node) => node.data.order)).toEqual(
    [1, 2, 3],
  )
  expect(model.nodes.find((node) => node.type === 'group')?.id).toBe('direct-cases')
})

it('surfaces requiredness, version and variant conflicts using the backend logical key', () => {
  const requiredness = buildLibraryCoverageFlow(plan, options)
  const invalidNode = requiredness.nodes.find((node) => node.data.title === 'Invalid password')
  expect(invalidNode?.data.required).toBe(true)
  expect(invalidNode?.data.status).toBe('needs_setup')
  expect(invalidNode?.data.detail).toBe('Conflicting selection')
  expect(requiredness.nodes.find((node) => node.id === 'coverage-root')?.data.status).toBe(
    'needs_setup',
  )

  if (plan.kind !== 'plan') throw new Error('fixture must be a plan')
  const versionConflict = buildLibraryCoverageFlow(
    { kind: 'plan', content: { ...plan.content, cases: [selection(signInV5.version.id)] } },
    options,
  )
  expect(
    versionConflict.nodes.find((node) => node.data.title === 'Valid sign-in')?.data.status,
  ).toBe('needs_setup')
  const variantConflict = buildLibraryCoverageFlow(
    {
      kind: 'plan',
      content: { ...plan.content, cases: [selection(signIn.version.id, true, 'account-b')] },
    },
    options,
  )
  expect(
    variantConflict.nodes.find((node) => node.data.title === 'Valid sign-in')?.data.status,
  ).toBe('needs_setup')
})

it('keeps unavailable pinned references visibly missing', () => {
  if (plan.kind !== 'plan') throw new Error('fixture must be a plan')
  const model = buildLibraryCoverageFlow(
    { kind: 'plan', content: { ...plan.content, suite_version_ids: ['archived-suite-version'] } },
    options,
  )
  const node = model.nodes.find((candidate) => candidate.id.includes('archived-suite-version'))
  expect(node?.data.title).toBe('Unavailable saved suite')
  expect(node?.data.status).toBe('missing')
  expect(node?.ariaLabel).toMatch(/missing/i)
})

it('uses a legible top-to-bottom narrow layout and handles zero or 100 cases', () => {
  const vertical = buildLibraryCoverageFlow(plan, options, 'vertical')
  expect(vertical.orientation).toBe('vertical')
  expect(vertical.nodes.find((node) => node.id === 'coverage-root')?.position).toEqual({
    x: 28,
    y: 20,
  })
  expect(vertical.nodes.filter((node) => node.type === 'case').every((node) => node.parentId)).toBe(
    true,
  )

  const zero: LibraryDraftDefinition = {
    kind: 'suite',
    content: { key: 'empty', title: 'Empty', version: 1, cases: [] },
  }
  expect(buildLibraryCoverageFlow(zero, options, 'vertical').contentHeight).toBe(360)

  const hundredCases = Array.from({ length: 100 }, (_, index) =>
    caseVersion(`bulk-${index}`, `Case ${index + 1}`, 1),
  )
  const hundred: LibraryDraftDefinition = {
    kind: 'suite',
    content: {
      key: 'hundred',
      title: 'One hundred checks',
      version: 1,
      cases: hundredCases.map((item) => selection(item.version.id)),
    },
  }
  const large = buildLibraryCoverageFlow(
    hundred,
    { ...options, saved_versions: hundredCases },
    'vertical',
  )
  expect(large.itemCount).toBe(100)
  expect(large.contentHeight).toBeGreaterThan(11_000)
})

it('renders singular copy, long-title disclosure and lets the page own wheel scrolling', () => {
  const longTitle =
    'A very long saved case title that remains distinguishable in the sequence without hiding its complete meaning'
  const longCase = caseVersion('long', longTitle, 1)
  const suite: LibraryDraftDefinition = {
    kind: 'suite',
    content: {
      key: 'single',
      title: 'Single check',
      version: 1,
      cases: [selection(longCase.version.id)],
    },
  }
  render(
    <MantineProvider theme={theme} env="test">
      <LibraryCoverageFlow
        definition={suite}
        options={{ ...options, saved_versions: [longCase] }}
      />
    </MantineProvider>,
  )
  expect(screen.getByText('1 case')).toBeInTheDocument()
  expect(screen.getByText('1 unique case')).toBeInTheDocument()
  expect(screen.getByTitle(longTitle)).toBeInTheDocument()
  const canvas = screen.getByLabelText('Suite coverage flow')
  const wheel = new WheelEvent('wheel', { bubbles: true, cancelable: true, deltaY: 100 })
  canvas.dispatchEvent(wheel)
  expect(wheel.defaultPrevented).toBe(false)
})

function attempt(
  id: string,
  caseId: string,
  state: AttemptResponse['state'],
  outcome?: AttemptResponse['outcome'],
): AttemptResponse {
  return {
    id,
    case_version_id: caseId,
    number: 1,
    generation: 1,
    state,
    outcome,
    reason: null,
    cleanup: 'pending',
    artifacts: [],
    checks: [],
    events: [],
    usage: [],
  }
}
function runFixture(): RunResponse {
  return {
    id: 'run-1',
    created_at: timestamp,
    state: 'running',
    summary: 'Release check v2',
    attempts: [
      attempt('attempt-1', signIn.version.id, 'finished', 'passed'),
      attempt('attempt-2', invalidPassword.version.id, 'running'),
    ],
    manifest: {
      app_id: appId,
      build_id: buildId,
      build_sha256: 'a'.repeat(64),
      build_bytes: 100,
      environment_revision: 1,
      profile: {
        id: 'profile-1',
        name: 'Fixture phone',
        driver: 'fake',
        package: app.android_package,
        adapter: 'demo_persistence_v1',
        device_identity: 'synthetic',
        image: 'synthetic',
        model: 'none',
        qualified: true,
        qualification_reference: 'test-only',
        max_apk_bytes: 1000,
      },
      cases: [signIn, invalidPassword, backendUnavailable].map((value, index) => ({
        definition_id: value.version.id,
        content_hash: `${index}`.repeat(64),
        case:
          value.version.definition.kind === 'case' ? value.version.definition.content : libraryCase,
        required: index !== 1,
        data_variant: 'default',
      })),
      budget: { duration_seconds: 600, max_steps: 30, artifact_bytes: 16_777_216 },
      diagnostic_retries: 0,
      exclusions: [],
    },
  }
}

it('derives a complete dynamic legend from real attempts', () => {
  const model = buildRunCoverageFlow(runFixture())
  expect(
    model.nodes.filter((node) => node.type === 'case').map((node) => node.data.status),
  ).toEqual(['passed', 'running', 'queued'])
  render(
    <MantineProvider theme={theme} env="test">
      <RunCoverageFlow run={runFixture()} />
    </MantineProvider>,
  )
  expect(screen.getByLabelText('Flow status legend')).toHaveTextContent('RunningPassedQueued')
  expect(screen.queryByText('Failed')).not.toBeInTheDocument()
})

it('activates an attempt from a complete keyboard-action name and focuses evidence', () => {
  const activate = vi.fn()
  const model = buildRunCoverageFlow(runFixture(), activate)
  const node = model.nodes.find((candidate) => candidate.data.title === 'Valid sign-in')
  expect(node?.data.onActivate).toBeTypeOf('function')
  expect(node?.ariaLabel).toMatch(/Case 1, Valid sign-in, version 4, required, Passed/)
  node?.data.onActivate?.()
  expect(activate).toHaveBeenCalledWith('attempt-1')

  const target = document.createElement('div')
  target.id = 'attempt-attempt-1'
  target.tabIndex = -1
  document.body.append(target)
  focusAttempt('attempt-1')
  expect(target).toHaveFocus()
  target.remove()
})
