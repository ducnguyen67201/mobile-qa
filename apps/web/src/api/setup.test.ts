import { afterEach, describe, expect, it, vi } from 'vitest'
import { appId, app, build, buildId, session, settings, upload, uploadId } from '@/test/fixtures'
import {
  appQuery,
  appsQuery,
  buildQuery,
  buildsQuery,
  completeUpload,
  createApp,
  createUpload,
  getUpload,
  saveEnvironment,
  sessionQuery,
  settingsQuery,
  signIn,
  signOut,
  transferUpload,
} from './setup'
import { apiClient } from './runtime'
const calls = () => vi.mocked(fetch).mock.calls.map((call) => call[0] as Request)
afterEach(() => vi.unstubAllGlobals())
describe('app setup generated transport', () => {
  it('logs in with custom header and sends in-memory CSRF on mutation', async () => {
    vi.stubGlobal(
      'fetch',
      vi
        .fn()
        .mockResolvedValueOnce(Response.json(session))
        .mockResolvedValueOnce(Response.json(app, { status: 201 })),
    )
    await signIn({ credential: 'synthetic-test-only', challenge_id: uploadId })
    await createApp({
      organization_id: app.organization_id,
      name: app.name,
      android_package: app.android_package,
      environment_name: 'Staging',
      backend_origins: [],
      login_origins: [],
    })
    expect(calls()[0]?.headers.get('X-Mobile-QA-Request')).toBe('1')
    expect(calls()[1]?.headers.get('X-CSRF-Token')).toBe(session.csrf_token)
    expect(calls()[1]?.method).toBe('POST')
    expect(calls()[1]?.url).toBe(`${window.location.origin}/api/apps`)
  })
  it('serializes APK through the generated multipart serializer', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(Response.json(upload)))
    await transferUpload(appId, uploadId, new File(['test'], 'synthetic.apk'))
    const request = calls()[0]!
    expect(request.method).toBe('PUT')
    expect(request.url).toBe(`${window.location.origin}/api/apps/${appId}/build-uploads/${uploadId}/content`)
    expect(request.headers.get('content-type')).toContain('multipart/form-data')
    const payload = await request.text()
    expect(payload, payload).toContain('filename="synthetic.apk"')
  })
  it.each([200, 202])('accepts documented complete status %d and validates it', async (status) => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(Response.json(build, { status })))
    expect(await completeUpload(appId, uploadId)).toEqual(build)
  })
  it.each([201, 204])('rejects undeclared complete status %d', async (status) => {
    vi.stubGlobal(
      'fetch',
      vi
        .fn()
        .mockResolvedValue(
          status === 204 ? new Response(null, { status }) : Response.json(build, { status }),
        ),
    )
    await expect(completeUpload(appId, uploadId)).rejects.toThrow()
  })
  it('rejects malformed success instead of showing validation as passed', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue(Response.json({ ...build, validation: { state: 'ready' } })),
    )
    await expect(completeUpload(appId, uploadId)).rejects.toThrow()
  })
  it('uses upload creation and recovery operation paths', async () => {
    vi.stubGlobal(
      'fetch',
      vi
        .fn()
        .mockResolvedValueOnce(Response.json(upload, { status: 201 }))
        .mockResolvedValueOnce(Response.json(upload)),
    )
    await createUpload(appId, new File(['test'], 'synthetic.apk'))
    await getUpload(appId, uploadId)
    expect(calls().map((r) => r.method)).toEqual(['POST', 'GET'])
    expect(calls()[1]?.url).toContain(`/build-uploads/${uploadId}`)
  })
  it('passes environment revisions and scoped reference IDs', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(Response.json(app.environment)))
    await saveEnvironment(appId, {
      expected_revision: 1,
      name: 'Staging',
      backend_origins: [],
      login_origins: [],
      account_secret_reference_id: null,
      reset_secret_reference_id: null,
    })
    expect(calls()[0]?.method).toBe('PATCH')
    expect(await calls()[0]?.json()).toMatchObject({ expected_revision: 1 })
  })
  it('validates all read surfaces and logout', async () => {
    vi.stubGlobal(
      'fetch',
      vi
        .fn()
        .mockResolvedValueOnce(Response.json(session))
        .mockResolvedValueOnce(Response.json({ items: [], next_cursor: null }))
        .mockResolvedValueOnce(Response.json(app))
        .mockResolvedValueOnce(Response.json({ items: [build], next_cursor: null }))
        .mockResolvedValueOnce(Response.json(build))
        .mockResolvedValueOnce(Response.json(settings))
        .mockResolvedValueOnce(Response.json({ signed_out: true })),
    )
    // queryFn callbacks do not use context; use fetchQuery for QueryOptions context types.
    const { QueryClient } = await import('@tanstack/react-query')
    const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } })
    await client.fetchQuery(sessionQuery)
    await client.fetchQuery(appsQuery())
    await client.fetchQuery(appQuery(appId))
    await client.fetchQuery(buildsQuery(appId))
    await client.fetchQuery(buildQuery(appId, buildId))
    await client.fetchQuery(settingsQuery)
    expect(await signOut()).toEqual({ signed_out: true })
    client.clear()
    expect(apiClient.getConfig().credentials).toBe('same-origin')
  })
})
