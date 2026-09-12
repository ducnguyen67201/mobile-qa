// Mock only fetch: exercise the actual generated SDK plus our validation boundary.
// This proves transport handling, not live server or device availability.
import { afterEach, describe, expect, it, vi } from 'vitest'
import { getHealth } from './generated/sdk.gen'
import { healthQuery } from './queries'
import { apiClient, ApiClientError } from './runtime'

const healthy = { status: 'ok', service: 'mobile-qa', version: '0.1.0' }
afterEach(() => vi.unstubAllGlobals())
describe('generated browser transport', () => {
  it('generates the GET method/path and receives the typed success', async () => {
    const fetchMock = vi.fn().mockResolvedValue(Response.json(healthy))
    vi.stubGlobal('fetch', fetchMock)
    const result = await getHealth({ client: apiClient, throwOnError: true })
    expect(result.data).toEqual(healthy)
    const request: Request = fetchMock.mock.calls[0]?.[0]
    expect(request.url).toBe(`${window.location.origin}/api/health`)
    expect(request.method).toBe('GET')
    expect(request.body).toBeNull()
  })
  it('validates success through the generated Zod schema', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(Response.json(healthy)))
    expect(await healthQuery.queryFn()).toEqual(healthy)
  })
  it.each([null, [], {}, { ...healthy, status: 'unknown' }, { ...healthy, version: 1 }])('rejects malformed success %j', async data => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(Response.json(data)))
    await expect(healthQuery.queryFn()).rejects.toThrow()
  })
  it.each([new Response(null, { status: 204 }), new Response('', { headers: { 'Content-Length': '0' } }), new Response('not json')])('rejects empty/nonJSON success %#', async response => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(response))
    await expect(healthQuery.queryFn()).rejects.toThrow()
  })
  it('rejects an undeclared success status', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(Response.json(healthy, { status: 201 })))
    await expect(healthQuery.queryFn()).rejects.toThrow('Unexpected health response status (201)')
  })
  it('validates and preserves a structured non2xx error', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(Response.json({ code: 'unavailable', message: 'Try later', details: null }, { status: 503 })))
    await expect(healthQuery.queryFn()).rejects.toMatchObject({ status: 503, message: 'Try later', body: { code: 'unavailable' } })
  })
  it.each([{}, { code: 1, message: {} }, { code: 'failure', message: 3 }, null])('does not trust malformed errors %j', async data => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(Response.json(data, { status: 500 })))
    await expect(healthQuery.queryFn()).rejects.toEqual(new ApiClientError(500, null))
  })
  it('rejects nonJSON HTTP errors without exposing their body', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response('<html>upstream</html>', { status: 502 })))
    await expect(healthQuery.queryFn()).rejects.toEqual(new ApiClientError(502, null))
  })
  it('preserves a network failure', async () => {
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('offline')))
    await expect(healthQuery.queryFn()).rejects.toThrow('offline')
  })
})
