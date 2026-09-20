/** DOM interactions use real generated SDK requests with synthetic HTTP responses. */
import { MantineProvider } from '@mantine/core'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { createMemoryRouter, RouterProvider } from 'react-router'
import { afterEach, expect, it, vi } from 'vitest'
import { routes } from '@/routes'
import { theme } from '@/theme'
import { app, appId, orgId, session, settings, timestamp } from '@/test/fixtures'
import {
  emptyCoverage,
  entryId,
  libraryCase,
  libraryDraft,
  libraryEntry,
  libraryOptions,
  libraryVersion,
  mutationId,
  versionId,
} from '@/test/library-fixtures'
import type {
  CreateLibraryEntryRequest,
  LibraryDraftResponse,
  LibraryVersionResponse,
} from '@/api/generated/types.gen'
import { zCreateLibraryEntryRequest, zSaveLibraryDraftRequest } from '@/api/generated/zod.gen'
function show(path: string) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } })
  const router = createMemoryRouter(routes, {
    initialEntries: [`${path}${path.includes('?') ? '&' : '?'}workspace=${orgId}`],
  })
  render(
    <MantineProvider theme={theme} env="test">
      <QueryClientProvider client={client}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    </MantineProvider>,
  )
  return { client, router }
}
function fixtureFetch(override?: (request: Request) => Promise<Response | undefined>) {
  return vi.fn(async (request: Request) => {
    const result = await override?.(request)
    if (result) return result
    const path = new URL(request.url).pathname
    if (path.endsWith('/session')) return Response.json(session)
    if (path.endsWith('/settings')) return Response.json(settings)
    if (path === '/api/apps')
      return Response.json({
        items: [{ ...app, environment_name: app.environment.name }],
        next_cursor: null,
      })
    if (path === `/api/apps/${appId}`) return Response.json(app)
    if (path.endsWith('/phone-options'))
      return Response.json({
        builds: [],
        profiles: [],
        active_session: null,
        blockers: ['Upload a build and connect a worker to preview this app.'],
      })
    if (path.endsWith('/default-test-plan'))
      return Response.json({ app_id: appId, revision: 0, plan_version_id: null })
    if (path.endsWith('/test-library/options')) return Response.json(libraryOptions)
    if (path.endsWith('/test-library'))
      return Response.json({ items: [libraryEntry], next_cursor: null })
    if (path.endsWith(`/test-library/${entryId}`)) return Response.json(libraryEntry)
    if (path.endsWith('/draft')) return Response.json(libraryDraft)
    if (path.endsWith('/versions')) return Response.json({ items: [], next_cursor: null })
    if (path.endsWith(`/versions/${versionId}`)) return Response.json(libraryVersion)
    if (path.endsWith('/runs')) return Response.json({ items: [], next_cursor: null })
    throw new Error(`Unexpected fixture request ${request.method} ${path}`)
  })
}
afterEach(() => vi.unstubAllGlobals())
it('creates without a manual key and reuses the same generated identity after a lost response', async () => {
  const bodies: CreateLibraryEntryRequest[] = []
  vi.stubGlobal(
    'fetch',
    fixtureFetch(async (request) => {
      if (request.method !== 'POST' || !new URL(request.url).pathname.endsWith('/test-library'))
        return
      bodies.push(zCreateLibraryEntryRequest.parse(await request.json()))
      if (bodies.length === 1) throw new TypeError('Failed to fetch')
      return Response.json(libraryDraft)
    }),
  )
  show('/tests')
  await userEvent.click(await screen.findByRole('button', { name: 'New case' }))
  expect(screen.queryByLabelText('Stable key')).not.toBeInTheDocument()
  const create = screen.getByRole('button', { name: 'Create test' })
  expect(create).toBeEnabled()
  await userEvent.click(create)
  await screen.findByRole('button', { name: 'Retry' })
  expect(bodies).toHaveLength(1)
  const first = zCreateLibraryEntryRequest.parse(bodies[0])
  expect(first.key).toBe(`case-${first.entry_id}`)
  expect(first.key).toMatch(/^case-[a-f0-9-]{36}$/)
  // The primary action must be as safe to retry as the explicit Retry action.
  await userEvent.click(create)
  expect(await screen.findByLabelText('Test name')).toBeInTheDocument()
  expect(bodies).toHaveLength(2)
  expect(bodies[1]).toEqual(bodies[0])
})
it('authors before an APK is uploaded and opens the saved draft from the catalog', async () => {
  const fetchMock = fixtureFetch()
  vi.stubGlobal('fetch', fetchMock)
  show('/tests')
  await waitFor(() => expect(screen.getByRole('button', { name: 'New case' })).toBeEnabled())
  expect(screen.queryByLabelText('Build')).not.toBeInTheDocument()
  await userEvent.click(
    await screen.findByRole('link', { name: /A saved task survives a restart/ }),
  )
  expect(await screen.findByLabelText('Test name')).toHaveValue(
    libraryDraft.definition.content.title,
  )
  expect(fetchMock.mock.calls.some(([r]) => new URL(r.url).pathname.endsWith('/builds'))).toBe(
    false,
  )
})
it('saves explicit typed content, preserving incomplete drafts and without requiring review', async () => {
  let current: LibraryDraftResponse = structuredClone(libraryDraft)
  const savedBodies: unknown[] = []
  vi.stubGlobal(
    'fetch',
    fixtureFetch(async (request) => {
      if (new URL(request.url).pathname.endsWith('/draft')) {
        if (request.method === 'PUT') {
          const body = zSaveLibraryDraftRequest.parse(await request.json())
          savedBodies.push(body)
          current = {
            ...current,
            definition: body.definition,
            entry: { ...current.entry, revision: 2 },
            issues: [
              {
                code: 'required',
                field: 'title',
                item_id: null,
                message: 'Give this test a descriptive title',
              },
            ],
          }
        }
        return Response.json(current)
      }
    }),
  )
  show(`/tests/${appId}/${entryId}`)
  await userEvent.clear(await screen.findByLabelText('Test name'))
  expect(screen.getByText('Unsaved changes')).toBeInTheDocument()
  expect(screen.queryByRole('button', { name: 'Request review' })).not.toBeInTheDocument()
  expect(savedBodies).toHaveLength(0)
  await userEvent.click(screen.getByRole('button', { name: 'Save' }))
  expect(await screen.findByText('Saved')).toBeInTheDocument()
  expect(screen.getByText('Give this test a descriptive title')).toBeInTheDocument()
  expect(savedBodies).toHaveLength(1)
  expect(savedBodies[0]).toMatchObject({
    expected_revision: 1,
    definition: { kind: 'case', content: { title: '' } },
  })
  expect(screen.queryByRole('button', { name: 'Request review' })).not.toBeInTheDocument()
})
it('preserves a stale editor, compares saved fields and requires an explicit revision choice', async () => {
  let reads = 0
  const savedRequests: unknown[] = []
  const remote: LibraryDraftResponse = {
    ...libraryDraft,
    entry: { ...libraryEntry, revision: 2 },
    definition: { kind: 'case', content: { ...libraryCase, title: 'Another author title' } },
  }
  vi.stubGlobal(
    'fetch',
    fixtureFetch(async (request) => {
      if (!new URL(request.url).pathname.endsWith('/draft')) return
      if (request.method === 'GET') return Response.json(++reads === 1 ? libraryDraft : remote)
      const body = zSaveLibraryDraftRequest.parse(await request.json())
      savedRequests.push(body)
      if (body.expected_revision === 1)
        return Response.json(
          {
            code: 'library_conflict',
            message: 'A newer revision exists',
            details: { kind: 'stale_revision', entry_id: entryId, current_revision: 2 },
            request_id: mutationId,
          },
          { status: 409 },
        )
      return Response.json({
        ...remote,
        entry: { ...remote.entry, revision: 3 },
        definition: body.definition,
      })
    }),
  )
  show(`/tests/${appId}/${entryId}`)
  const title = await screen.findByLabelText('Test name')
  await userEvent.clear(title)
  await userEvent.type(title, 'My carefully edited title')
  await userEvent.click(screen.getByRole('button', { name: 'Save' }))
  expect(await screen.findByText('A newer revision was saved')).toBeInTheDocument()
  expect(title).toHaveValue('My carefully edited title')
  expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled()
  await userEvent.click(screen.getByRole('button', { name: 'Compare with saved draft' }))
  const dialog = await screen.findByRole('dialog', { name: 'Your changes and the saved draft' })
  expect(within(dialog).getByText('My carefully edited title')).toBeInTheDocument()
  expect(within(dialog).getByText('Another author title')).toBeInTheDocument()
  await userEvent.click(
    within(dialog).getByRole('button', { name: 'Keep my edits against this revision' }),
  )
  expect(savedRequests).toHaveLength(1)
  expect(title).toHaveValue('My carefully edited title')
  await userEvent.click(screen.getByRole('button', { name: 'Save' }))
  expect(await screen.findByText('Saved')).toBeInTheDocument()
  expect(savedRequests[1]).toMatchObject({
    expected_revision: 2,
    definition: { content: { title: 'My carefully edited title' } },
  })
})
it('keeps expected results when an action is removed and warns before leaving an unsaved draft', async () => {
  vi.stubGlobal('fetch', fixtureFetch())
  show(`/tests/${appId}/${entryId}`)
  await userEvent.click(await screen.findByRole('button', { name: 'Remove action 2' }))
  expect(screen.getByText('Reconnect an expected result')).toBeInTheDocument()
  expect(screen.getByLabelText('Check 1 description')).toHaveValue(
    'The saved task remains visible.',
  )
  await userEvent.click(screen.getByRole('link', { name: 'Back to test library' }))
  expect(await screen.findByRole('dialog', { name: 'Leave unsaved changes?' })).toBeInTheDocument()
  await userEvent.click(screen.getByRole('button', { name: 'Keep editing' }))
  expect(screen.getByLabelText('Check 1 description')).toBeInTheDocument()
  await userEvent.click(screen.getByRole('link', { name: 'Back to test library' }))
  await userEvent.click(await screen.findByRole('button', { name: 'Discard changes and leave' }))
  expect(await screen.findByRole('heading', { name: 'Tests', level: 1 })).toBeInTheDocument()
})
it('opens previously frozen content in the current editor without review controls', async () => {
  vi.stubGlobal(
    'fetch',
    fixtureFetch(async (request) => {
      const path = new URL(request.url).pathname
      if (path.endsWith(`/test-library/${entryId}`)) return Response.json(libraryVersion.entry)
      if (path.endsWith('/draft'))
        return Response.json({
          ...libraryDraft,
          entry: libraryVersion.entry,
          saved_version_id: versionId,
        })
    }),
  )
  show(`/tests/${appId}/${entryId}`)
  expect(await screen.findByLabelText('Test name')).toHaveValue(libraryCase.title)
  expect(screen.getByRole('button', { name: 'Save' })).toBeEnabled()
  expect(screen.queryByText('Two reviews, one exact version')).not.toBeInTheDocument()
  expect(screen.queryByRole('button', { name: 'Request review' })).not.toBeInTheDocument()
})
it('restores an archived draft-only entry without requiring a frozen version', async () => {
  let current: LibraryDraftResponse = {
    ...libraryDraft,
    entry: { ...libraryEntry, archived_at: timestamp },
  }
  vi.stubGlobal(
    'fetch',
    fixtureFetch(async (request) => {
      const path = new URL(request.url).pathname
      if (path.endsWith(`/test-library/${entryId}`)) return Response.json(current.entry)
      if (path.endsWith('/draft')) return Response.json(current)
      if (path.endsWith('/archive')) {
        current = { ...current, entry: { ...current.entry, archived_at: null, revision: 2 } }
        return Response.json(current.entry)
      }
    }),
  )
  show(`/tests/${appId}/${entryId}`)
  await userEvent.click(await screen.findByRole('button', { name: 'Restore entry' }))
  const dialog = await screen.findByRole('dialog', { name: 'Restore this entry?' })
  await userEvent.click(within(dialog).getByRole('button', { name: 'Restore entry' }))
  await waitFor(() => expect(screen.getByRole('button', { name: 'Save' })).toBeEnabled())
})
it('keeps plan authoring available with no profile and no uploaded build', async () => {
  const draft: LibraryDraftResponse = {
    ...libraryDraft,
    entry: { ...libraryEntry, kind: 'plan', title: 'Release check' },
    definition: {
      kind: 'plan',
      content: {
        key: 'release',
        version: 1,
        title: 'Release check',
        suite_version_ids: [],
        cases: [],
        profile_id: null,
        budget: { duration_seconds: 600, max_steps: 30, artifact_bytes: 16777216 },
        diagnostic_retries: 0,
        exclusions: [],
      },
    },
    issues: [
      {
        code: 'required',
        field: 'profile_id',
        item_id: null,
        message: 'Choose an execution profile',
      },
    ],
    coverage: emptyCoverage,
  }
  const requests = fixtureFetch(async (request) => {
    const path = new URL(request.url).pathname
    if (path.endsWith(`/test-library/${entryId}`)) return Response.json(draft.entry)
    if (path.endsWith('/draft')) return Response.json(draft)
  })
  vi.stubGlobal('fetch', requests)
  show(`/tests/${appId}/${entryId}`)
  expect(await screen.findByLabelText('Release plan title')).toHaveValue('Release check')
  expect(screen.getByRole('combobox', { name: 'Execution profile' })).toHaveValue('')
  expect(screen.queryByRole('button', { name: 'Request review' })).not.toBeInTheDocument()
  expect(screen.getByRole('button', { name: 'Save' })).toBeEnabled()
  expect(requests.mock.calls.some(([r]) => new URL(r.url).pathname.endsWith('/builds'))).toBe(false)
})
it('changes a pinned case version only when the author explicitly chooses the newer saved version', async () => {
  const old: LibraryVersionResponse = { ...libraryVersion }
  const newer: LibraryVersionResponse = {
    ...old,
    version: {
      ...old.version,
      id: mutationId,
      definition: { kind: 'case', content: { ...libraryCase, version: 2 } },
    },
  }
  let draft: LibraryDraftResponse = {
    ...libraryDraft,
    entry: { ...libraryEntry, kind: 'suite', title: 'Core behavior' },
    definition: {
      kind: 'suite',
      content: {
        key: 'core',
        version: 1,
        title: 'Core behavior',
        cases: [{ case_version_id: versionId, required: true, data_variant: 'default' }],
      },
    },
  }
  const requests: unknown[] = []
  vi.stubGlobal(
    'fetch',
    fixtureFetch(async (request) => {
      const path = new URL(request.url).pathname
      if (path.endsWith('/options'))
        return Response.json({ ...libraryOptions, saved_versions: [old, newer] })
      if (path.endsWith(`/test-library/${entryId}`)) return Response.json(draft.entry)
      if (path.endsWith('/draft')) {
        if (request.method === 'PUT') {
          const body = zSaveLibraryDraftRequest.parse(await request.json())
          requests.push(body)
          draft = { ...draft, definition: body.definition, entry: { ...draft.entry, revision: 2 } }
        }
        return Response.json(draft)
      }
    }),
  )
  show(`/tests/${appId}/${entryId}`)
  expect(await screen.findByText(`${libraryCase.title} · v1`)).toBeInTheDocument()
  expect(requests).toHaveLength(0)
  await userEvent.click(screen.getByRole('button', { name: 'Use newer saved v2' }))
  expect(screen.getByText(`${libraryCase.title} · v2`)).toBeInTheDocument()
  await userEvent.click(screen.getByRole('button', { name: 'Save' }))
  await waitFor(() => expect(requests).toHaveLength(1))
  expect(requests[0]).toMatchObject({
    definition: {
      kind: 'suite',
      content: {
        cases: [{ case_version_id: mutationId, required: true, data_variant: 'default' }],
      },
    },
  })
})
it('runs the selected saved plan and follows its simulated report', async () => {
  const { build, buildId } = await import('@/test/fixtures')
  const { zCreateRunRequest } = await import('@/api/generated/zod.gen')
  const { zRunResponse } = await import('@/api/generated/zod.gen')
  const plan: LibraryVersionResponse = {
    ...libraryVersion,
    entry: { ...libraryVersion.entry, kind: 'plan' },
    version: {
      ...libraryVersion.version,
      definition: {
        kind: 'plan',
        content: {
          key: 'release',
          version: 1,
          title: 'Reviewed release check',
          suite_version_ids: [],
          cases: [{ case_version_id: entryId, required: true, data_variant: 'default' }],
          profile_id: mutationId,
          budget: libraryCase.budget,
          diagnostic_retries: 0,
          exclusions: ['Offline sync'],
        },
      },
    },
  }
  // Generated parser also checks the synthetic fixture rather than asserting an untyped report.
  const run = zRunResponse.parse({
    id: mutationId,
    state: 'finished',
    created_at: timestamp,
    summary: 'Synthetic release completed',
    attempts: [],
    manifest: {
      app_id: appId,
      build_id: buildId,
      build_sha256: build.sha256,
      build_bytes: build.byte_size,
      plan_version_id: versionId,
      plan_hash: plan.version.content_hash,
      environment_revision: 1,
      profile: {
        id: mutationId,
        name: 'Synthetic profile',
        driver: 'fake',
        package: libraryCase.package,
        adapter: libraryCase.adapter,
        device_identity: 'fixture',
        image: 'fixture',
        model: 'none',
        qualified: true,
        qualification_reference: 'fixture',
        max_apk_bytes: 1048576,
      },
      cases: [
        {
          definition_id: entryId,
          content_hash: 'c'.repeat(64),
          data_variant: 'default',
          required: true,
          case: libraryCase,
        },
      ],
      budget: libraryCase.budget,
      diagnostic_retries: 0,
      exclusions: ['Offline sync'],
    },
  })
  const submissions: unknown[] = []
  vi.stubGlobal(
    'fetch',
    fixtureFetch(async (request) => {
      const path = new URL(request.url).pathname
      if (path.endsWith(`/test-library/${entryId}`)) return Response.json(plan.entry)
      if (path.endsWith(`/versions/${versionId}`)) return Response.json(plan)
      if (path.endsWith('/builds')) return Response.json({ items: [build], next_cursor: null })
      if (path.endsWith('/execution-plan')) {
        expect(new URL(request.url).searchParams.get('plan_version_id')).toBe(versionId)
        return Response.json({ plan: plan.version, manifest: run.manifest, blockers: [] })
      }
      if (path.endsWith('/runs') && request.method === 'POST') {
        submissions.push(zCreateRunRequest.parse(await request.json()))
        return Response.json(run, { status: 201 })
      }
      if (path.endsWith(`/runs/${mutationId}`)) return Response.json(run)
    }),
  )
  const { router } = show(`/tests/${appId}/${entryId}/versions/${versionId}`)
  expect(await screen.findByText(/Simulated worker/)).toBeInTheDocument()
  await userEvent.click(screen.getByRole('button', { name: 'Run release check' }))
  expect(await screen.findByText(/Simulated execution/)).toBeInTheDocument()
  expect(router.state.location.pathname).toBe(`/runs/${mutationId}`)
  expect(submissions).toEqual([
    { build_id: buildId, plan_version_id: versionId, environment_revision: 1 },
  ])
})
it.each(['failed refresh', 'submitted elsewhere'])(
  'preserves unsaved input after a background %s',
  async (scenario) => {
    let changed = false
    vi.stubGlobal(
      'fetch',
      fixtureFetch(async (request) => {
        const path = new URL(request.url).pathname
        if (
          changed &&
          path.endsWith(`/test-library/${entryId}`) &&
          scenario === 'submitted elsewhere'
        )
          return Response.json(libraryVersion.entry)
        if (changed && path.endsWith('/draft'))
          return Response.json(
            {
              code: 'unavailable',
              message: 'Saved draft unavailable',
              details: null,
              request_id: mutationId,
            },
            { status: scenario === 'failed refresh' ? 503 : 404 },
          )
      }),
    )
    const { client } = show(`/tests/${appId}/${entryId}`)
    const title = await screen.findByLabelText('Test name')
    await userEvent.clear(title)
    await userEvent.type(title, 'Local work must survive')
    changed = true
    await client.invalidateQueries({ queryKey: ['test-library', orgId, appId] })
    expect(
      await screen.findByText('Saved status could not be refreshed. Your editor is preserved.'),
    ).toBeInTheDocument()
    expect(screen.getByLabelText('Test name')).toHaveValue('Local work must survive')
    await userEvent.click(screen.getByRole('link', { name: 'Back to test library' }))
    expect(
      await screen.findByRole('dialog', { name: 'Leave unsaved changes?' }),
    ).toBeInTheDocument()
  },
)

it('locates saved setup errors in collapsed checks and focuses the exact field', async () => {
  const draft: LibraryDraftResponse = {
    ...libraryDraft,
    definition: {
      kind: 'case',
      content: {
        ...libraryCase,
        requirement: '',
        checks: [
          { ...libraryCase.checks[0]!, id: 'first', checkpoint_id: 'created' },
          {
            ...libraryCase.checks[0]!,
            id: 'broken',
            resource_id: '',
            ready_resource_id: '',
            description: '',
          },
        ],
      },
    },
    issues: [
      {
        code: 'required',
        field: 'checks.resource_id',
        item_id: 'broken',
        message: 'Set evidence targets belonging to this Android package',
      },
      {
        code: 'required',
        field: 'checks.description',
        item_id: 'broken',
        message: 'Describe the expected result',
      },
      {
        code: 'required',
        field: 'requirement',
        item_id: null,
        message: 'Describe the intended behavior',
      },
    ],
  }
  const requests = fixtureFetch(async (request) => {
    if (new URL(request.url).pathname.endsWith('/draft')) return Response.json(draft)
  })
  vi.stubGlobal('fetch', requests)
  show(`/tests/${appId}/${entryId}`)
  const jump = await screen.findByRole('button', { name: /Action 2 → Check 2 → Control/ })
  expect(screen.getByRole('button', { name: 'Edit action 2' })).toHaveAttribute(
    'aria-expanded',
    'false',
  )
  await userEvent.click(jump)
  await waitFor(() => expect(screen.getByLabelText('Check 2 control')).toHaveFocus())
  expect(screen.getByLabelText('Check 2 control')).toHaveAttribute('aria-invalid', 'true')
  expect(screen.getByLabelText('Check 2 control')).toHaveAccessibleDescription(
    /Pick the control to check on the phone/,
  )
  expect(screen.getByRole('button', { name: 'Show checks for action 2' })).toHaveAttribute(
    'aria-expanded',
    'true',
  )
  // Repeated jumps must reopen disclosures the user closed in between.
  await userEvent.click(screen.getByRole('button', { name: 'Edit action 2' }))
  await userEvent.click(jump)
  await waitFor(() => expect(screen.getByLabelText('Check 2 control')).toHaveFocus())
  await userEvent.type(screen.getByLabelText('Check 2 description'), 'Saved task is visible')
  expect(screen.queryByRole('button', { name: /Check 2 → Description/ })).not.toBeInTheDocument()
  expect(jump).toBeInTheDocument()
  expect(screen.getByText('Save to recheck the fields you changed.')).toBeInTheDocument()
  await userEvent.click(screen.getByRole('button', { name: /Requirement and setup —/ }))
  await waitFor(() =>
    expect(screen.getByLabelText('Expected behavior / requirement')).toHaveFocus(),
  )
  expect(screen.getByLabelText('Expected behavior / requirement')).toHaveAttribute(
    'aria-invalid',
    'true',
  )
  expect(
    requests.mock.calls.some(
      ([r]) => r.method === 'POST' && new URL(r.url).pathname.endsWith('/phone-sessions'),
    ),
  ).toBe(false)
})
