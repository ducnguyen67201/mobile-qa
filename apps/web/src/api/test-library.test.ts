import { QueryClient } from '@tanstack/react-query'
import { afterEach, expect, it, vi } from 'vitest'
import { appId } from '@/test/fixtures'
import {
  entryId,
  libraryDraft,
  libraryEntry,
  libraryOptions,
  libraryVersion,
  mutationId,
  versionId,
} from '@/test/library-fixtures'
import {
  archiveLibraryEntry,
  createLibraryEntry,
  defaultPlanQuery,
  libraryDraftQuery,
  libraryEntryQuery,
  libraryErrorDetails,
  libraryHistoryQuery,
  libraryOptionsQuery,
  libraryQuery,
  libraryVersionQuery,
  saveLibraryDraft,
  setDefaultPlan,
} from './test-library'
import { ApiClientError } from './runtime'
import { forgetSession, setCsrfToken } from './session-transport'
const client = () => new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } })
afterEach(() => {
  vi.unstubAllGlobals()
  forgetSession()
})
it('validates every library read through generated response schemas', async () => {
  vi.stubGlobal(
    'fetch',
    vi
      .fn()
      .mockResolvedValueOnce(Response.json({ items: [libraryEntry], next_cursor: null }))
      .mockResolvedValueOnce(Response.json(libraryOptions))
      .mockResolvedValueOnce(Response.json(libraryEntry))
      .mockResolvedValueOnce(Response.json(libraryDraft))
      .mockResolvedValueOnce(Response.json({ items: [libraryVersion], next_cursor: null }))
      .mockResolvedValueOnce(Response.json(libraryVersion))
      .mockResolvedValueOnce(Response.json({ app_id: appId, revision: 0, plan_version_id: null })),
  )
  const query = client()
  expect(await query.fetchQuery(libraryQuery('workspace', appId, { kind: 'case' }))).toHaveProperty(
    'items',
    [libraryEntry],
  )
  expect(await query.fetchQuery(libraryOptionsQuery('workspace', appId))).toEqual(libraryOptions)
  expect(await query.fetchQuery(libraryEntryQuery('workspace', appId, entryId))).toEqual(
    libraryEntry,
  )
  expect(await query.fetchQuery(libraryDraftQuery('workspace', appId, entryId))).toEqual(
    libraryDraft,
  )
  expect(await query.fetchQuery(libraryHistoryQuery('workspace', appId, entryId))).toHaveProperty(
    'items',
    [libraryVersion],
  )
  expect(
    await query.fetchQuery(libraryVersionQuery('workspace', appId, entryId, versionId)),
  ).toEqual(libraryVersion)
  expect(await query.fetchQuery(defaultPlanQuery('workspace', appId))).toHaveProperty(
    'plan_version_id',
    null,
  )
  query.clear()
})
it('uses documented methods and shares CSRF while validating all mutations', async () => {
  setCsrfToken('library-csrf')
  const calls: Request[] = []
  const replies = [
    Response.json(libraryDraft, { status: 201 }),
    Response.json(libraryDraft),
    Response.json(libraryEntry),
    Response.json({ app_id: appId, revision: 1, plan_version_id: versionId }),
  ]
  vi.stubGlobal(
    'fetch',
    vi.fn(async (request: Request) => {
      calls.push(request)
      return replies.shift()!
    }),
  )
  await createLibraryEntry(appId, {
    mutation_id: mutationId,
    entry_id: entryId,
    kind: 'case',
    key: 'example',
    template_profile_id: null,
  })
  await saveLibraryDraft(appId, entryId, {
    mutation_id: mutationId,
    expected_revision: 1,
    definition: libraryDraft.definition,
  })
  await archiveLibraryEntry(appId, entryId, {
    mutation_id: mutationId,
    expected_revision: 2,
    archived: true,
  })
  await setDefaultPlan(appId, {
    mutation_id: mutationId,
    expected_revision: 0,
    plan_version_id: versionId,
  })
  expect(calls.map((r) => r.method)).toEqual(['POST', 'PUT', 'POST', 'PUT'])
  expect(calls.every((r) => r.headers.get('X-CSRF-Token') === 'library-csrf')).toBe(true)
  expect(new URL(calls[1]!.url).pathname).toBe(`/api/apps/${appId}/test-library/${entryId}/draft`)
})
it.each([201, 202, 204])('rejects undeclared draft-save success status %d', async (status) => {
  vi.stubGlobal(
    'fetch',
    vi.fn(async () =>
      status === 204 ? new Response(null, { status }) : Response.json(libraryDraft, { status }),
    ),
  )
  await expect(
    saveLibraryDraft(appId, entryId, {
      mutation_id: mutationId,
      expected_revision: 1,
      definition: libraryDraft.definition,
    }),
  ).rejects.toThrow()
})
it('rejects malformed success, but preserves typed stale revisions and safely ignores malformed details', async () => {
  vi.stubGlobal(
    'fetch',
    vi.fn(async () =>
      Response.json({ ...libraryDraft, entry: { ...libraryEntry, revision: 'two' } }),
    ),
  )
  await expect(
    client().fetchQuery(libraryDraftQuery('workspace', appId, entryId)),
  ).rejects.toThrow()
  const details = { kind: 'stale_revision', entry_id: entryId, current_revision: 3 }
  const error = new ApiClientError(409, {
    code: 'library_conflict',
    message: 'Draft changed',
    details,
    request_id: mutationId,
  })
  expect(libraryErrorDetails(error)).toEqual(details)
  expect(
    libraryErrorDetails(
      new ApiClientError(409, {
        ...error.body!,
        details: { ...details, current_revision: 'latest' },
      }),
    ),
  ).toBeUndefined()
  expect(libraryErrorDetails(new Error('offline'))).toBeUndefined()
})
it('reuses mutation identity and payload after response loss', async () => {
  const bodies: unknown[] = []
  vi.stubGlobal(
    'fetch',
    vi.fn(async (request: Request) => {
      bodies.push(await request.json())
      throw new Error('response lost')
    }),
  )
  const body = {
    mutation_id: mutationId,
    expected_revision: 1,
    definition: libraryDraft.definition,
  }
  await expect(saveLibraryDraft(appId, entryId, body)).rejects.toThrow()
  await expect(saveLibraryDraft(appId, entryId, body)).rejects.toThrow()
  expect(bodies).toEqual([body, body])
})
it('scopes drafts, history and selected filters to workspace and app identities', () => {
  expect(libraryDraftQuery('one', appId, entryId).queryKey).not.toEqual(
    libraryDraftQuery('two', appId, entryId).queryKey,
  )
  expect(libraryQuery('one', appId, { kind: 'case' }).queryKey).not.toEqual(
    libraryQuery('one', appId, { kind: 'suite' }).queryKey,
  )
  expect(libraryVersionQuery('one', appId, entryId, versionId).queryKey).not.toEqual(
    libraryVersionQuery('one', appId, entryId, mutationId).queryKey,
  )
})
