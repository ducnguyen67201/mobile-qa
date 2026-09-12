import { afterEach, describe, expect, it, vi } from 'vitest'
import { getHealth, parseHealth } from './health'
import { ApiClientError } from './client'
const healthy = { status: 'ok', service: 'mobile-qa', version: '0.1.0' }
afterEach(() => vi.unstubAllGlobals())
describe('health boundary', () => {
  it('consumes the real wire shape', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(Response.json(healthy)))
    expect(await getHealth()).toEqual(healthy)
    expect(fetch).toHaveBeenCalledWith('/api/health', expect.any(Object))
  })
  it.each([null, [], {}, { ...healthy, status: 'unknown' }, { ...healthy, version: 1 }])('rejects malformed data %j', data => { expect(() => parseHealth(data)).toThrow('Invalid health response') })
  it('preserves a valid structured HTTP error', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(Response.json({ code: 'unavailable', message: 'Try later' }, { status: 503 })))
    await expect(getHealth()).rejects.toMatchObject({ status: 503, message: 'Try later' })
  })
  it('does not trust error fields with wrong scalar types', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(Response.json({ code: 1, message: {} }, { status: 500 })))
    await expect(getHealth()).rejects.toEqual(new ApiClientError(500, null))
  })
  it('rejects network failures and invalid success JSON', async () => {
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('offline')))
    await expect(getHealth()).rejects.toThrow('offline')
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response('not json')))
    await expect(getHealth()).rejects.toThrow('Invalid health response')
  })
})
