import { webcrypto } from 'node:crypto'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { ArtifactTransferError, partSha256, uploadPartBytes } from './artifact-transfer'

afterEach(() => vi.unstubAllGlobals())

describe('private artifact transfer', () => {
  it('hashes only the supplied bounded part and honors cancellation', async () => {
    vi.stubGlobal('crypto', webcrypto)
    const part = { arrayBuffer: async () => new TextEncoder().encode('abc').buffer }
    const controller = new AbortController()
    expect(await partSha256(part, controller.signal)).toBe(
      'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad',
    )
    controller.abort()
    await expect(partSha256(part, controller.signal)).rejects.toThrow()
  })

  it('sends only file bytes without credentials or redirects', async () => {
    const fetcher = vi.fn().mockResolvedValue(new Response(null, { headers: { etag: '"part"' } }))
    vi.stubGlobal('fetch', fetcher)
    const body = new Blob(['test'])
    const signal = new AbortController().signal
    expect(
      await uploadPartBytes('https://storage.example/part?signature=synthetic', body, signal),
    ).toBe('"part"')
    expect(fetcher).toHaveBeenCalledWith(expect.any(URL), {
      method: 'PUT',
      body,
      signal,
      credentials: 'omit',
      redirect: 'error',
      referrerPolicy: 'no-referrer',
      headers: expect.any(Headers),
    })
  })

  it.each([
    'http://storage.example/part',
    'https://user:password@storage.example/part',
    'https://storage.example/part#secret',
  ])('rejects unsafe capability %s before fetching', async (url) => {
    const fetcher = vi.fn()
    vi.stubGlobal('fetch', fetcher)
    await expect(uploadPartBytes(url, new Blob(), new AbortController().signal)).rejects.toThrow(
      'invalid',
    )
    expect(fetcher).not.toHaveBeenCalled()
  })

  it('distinguishes expiry from a missing storage receipt without exposing signed URLs', async () => {
    vi.stubGlobal(
      'fetch',
      vi
        .fn()
        .mockResolvedValueOnce(new Response(null, { status: 403 }))
        .mockResolvedValueOnce(new Response()),
    )
    const signal = new AbortController().signal
    await expect(
      uploadPartBytes('https://storage.example/part', new Blob(), signal),
    ).rejects.toBeInstanceOf(ArtifactTransferError)
    await expect(
      uploadPartBytes('https://storage.example/part', new Blob(), signal),
    ).rejects.toThrow('receipt')
  })
})
