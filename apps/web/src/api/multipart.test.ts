import { afterEach, expect, it, vi } from 'vitest'
import { appId, uploadId, upload } from '@/test/fixtures'
import type { MultipartUpload } from './generated/types.gen'
import {
  abortMultipartUpload,
  authorizeUploadPart,
  completeMultipartUpload,
  confirmUploadPart,
  getMultipartUpload,
  startMultipartUpload,
} from './multipart'

const session: MultipartUpload = {
  upload_id: uploadId,
  part_size: 16777216,
  max_parallel_parts: 2,
  expires_at: '2099-01-01T00:00:00Z',
  state: 'uploading',
  parts: [],
}
afterEach(() => vi.unstubAllGlobals())

it('uses generated scoped multipart operations and validates their responses', async () => {
  const fetcher = vi
    .fn<(request: Request) => Promise<Response>>()
    .mockResolvedValueOnce(Response.json(session))
    .mockResolvedValueOnce(Response.json(session))
    .mockResolvedValueOnce(
      Response.json({
        part_number: 1,
        url: 'https://storage.example/part',
        expires_at: session.expires_at,
        headers: {},
      }),
    )
    .mockResolvedValueOnce(Response.json(session))
    .mockResolvedValueOnce(Response.json(upload, { status: 202 }))
    .mockResolvedValueOnce(Response.json({ ...session, state: 'aborted' }))
  vi.stubGlobal('fetch', fetcher)
  const signal = new AbortController().signal
  await startMultipartUpload(appId, uploadId, signal)
  expect(await getMultipartUpload(appId, uploadId, signal)).toEqual(session)
  await authorizeUploadPart(
    appId,
    uploadId,
    { part_number: 1, byte_size: 3, sha256: 'a'.repeat(64) },
    signal,
  )
  await confirmUploadPart(appId, uploadId, 1, '"etag"', signal)
  await completeMultipartUpload(appId, uploadId, signal)
  await abortMultipartUpload(appId, uploadId, signal)
  const requests = fetcher.mock.calls.map(([request]: [Request]) => request)
  expect(requests.map((request) => request.method)).toEqual([
    'POST',
    'GET',
    'POST',
    'PUT',
    'POST',
    'DELETE',
  ])
  expect(requests.map((request) => new URL(request.url).pathname)).toEqual([
    `/api/apps/${appId}/build-uploads/${uploadId}/multipart`,
    `/api/apps/${appId}/build-uploads/${uploadId}/multipart`,
    `/api/apps/${appId}/build-uploads/${uploadId}/multipart/parts`,
    `/api/apps/${appId}/build-uploads/${uploadId}/multipart/parts/1`,
    `/api/apps/${appId}/build-uploads/${uploadId}/multipart/complete`,
    `/api/apps/${appId}/build-uploads/${uploadId}/multipart`,
  ])
})

it('rejects malformed multipart state at the generated boundary', async () => {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue(Response.json({ ...session, state: 'trusted' })))
  await expect(
    startMultipartUpload(appId, uploadId, new AbortController().signal),
  ).rejects.toThrow()
})

it('rejects malformed saved multipart state at the generated GET boundary', async () => {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue(Response.json({ ...session, state: 'trusted' })))
  await expect(getMultipartUpload(appId, uploadId, new AbortController().signal)).rejects.toThrow()
})
