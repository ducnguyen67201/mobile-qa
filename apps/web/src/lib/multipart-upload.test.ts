import { webcrypto } from 'node:crypto'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { MultipartUpload } from '@/api/generated/types.gen'
import { appId, uploadId, upload } from '@/test/fixtures'
import * as control from '@/api/multipart'
import * as storage from '@/api/artifact-transfer'
import { getUpload } from '@/api/setup'
import { uploadMultipartFile } from './multipart-upload'

vi.mock('@/api/multipart')
vi.mock('@/api/setup', () => ({ getUpload: vi.fn() }))
vi.mock('@/api/artifact-transfer', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/artifact-transfer')>()),
  uploadPartBytes: vi.fn(),
}))

const digest = 'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad'
const session: MultipartUpload = {
  upload_id: uploadId,
  part_size: 16 * 1024 * 1024,
  max_parallel_parts: 2,
  expires_at: '2099-01-01T00:00:00Z',
  state: 'uploading',
  parts: [],
}

function fileWithBytes(value = 'abc') {
  const bytes = new TextEncoder().encode(value)
  const file = new File([bytes], 'app.apk')
  // jsdom's Blob lacks arrayBuffer; provide the browser API without altering production hashing.
  Object.defineProperty(file, 'slice', {
    value: (start: number, end: number) => {
      const part = bytes.slice(start, end)
      const blob = new Blob([part])
      Object.defineProperty(blob, 'arrayBuffer', { value: async () => part.buffer })
      return blob
    },
  })
  return file
}

beforeEach(() => {
  vi.resetAllMocks()
  vi.stubGlobal('crypto', webcrypto)
  vi.mocked(control.startMultipartUpload).mockResolvedValue(session)
  vi.mocked(control.authorizeUploadPart).mockResolvedValue({
    part_number: 1,
    url: 'https://storage.example/part',
    headers: {},
    expires_at: session.expires_at,
  })
  vi.mocked(control.confirmUploadPart).mockResolvedValue(session)
  vi.mocked(control.completeMultipartUpload).mockResolvedValue(upload)
  vi.mocked(storage.uploadPartBytes).mockResolvedValue('"etag"')
  vi.mocked(getUpload).mockResolvedValue(upload)
})
afterEach(() => vi.unstubAllGlobals())

describe('resumable multipart orchestration', () => {
  it('hashes, authorizes, uploads and confirms bytes before completing', async () => {
    const progress = vi.fn()
    const signal = new AbortController().signal
    expect(await uploadMultipartFile(appId, uploadId, fileWithBytes(), signal, progress)).toEqual(
      upload,
    )
    expect(control.startMultipartUpload).toHaveBeenCalledWith(appId, uploadId, signal)
    expect(control.authorizeUploadPart).toHaveBeenCalledWith(
      appId,
      uploadId,
      { part_number: 1, byte_size: 3, sha256: digest },
      expect.any(AbortSignal),
    )
    expect(control.confirmUploadPart).toHaveBeenCalledWith(
      appId,
      uploadId,
      1,
      '"etag"',
      expect.any(AbortSignal),
    )
    expect(progress).toHaveBeenLastCalledWith({ completedBytes: 3, totalBytes: 3 })
    expect(control.completeMultipartUpload).toHaveBeenCalledOnce()
  })

  it('verifies saved bytes and skips an already confirmed part', async () => {
    vi.mocked(control.startMultipartUpload).mockResolvedValue({
      ...session,
      parts: [{ part_number: 1, byte_size: 3, sha256: digest, etag: '"etag"' }],
    })
    await uploadMultipartFile(
      appId,
      uploadId,
      fileWithBytes(),
      new AbortController().signal,
      vi.fn(),
    )
    expect(storage.uploadPartBytes).not.toHaveBeenCalled()
    expect(control.completeMultipartUpload).toHaveBeenCalledOnce()
  })

  it('resumes a supplied session without starting it again and skips confirmed bytes', async () => {
    const progress = vi.fn()
    const existingSession: MultipartUpload = {
      ...session,
      parts: [{ part_number: 1, byte_size: 3, sha256: digest, etag: '"etag"' }],
    }
    expect(
      await uploadMultipartFile(
        appId,
        uploadId,
        fileWithBytes(),
        new AbortController().signal,
        progress,
        existingSession,
      ),
    ).toEqual(upload)
    expect(control.startMultipartUpload).not.toHaveBeenCalled()
    expect(control.authorizeUploadPart).not.toHaveBeenCalled()
    expect(storage.uploadPartBytes).not.toHaveBeenCalled()
    expect(control.completeMultipartUpload).toHaveBeenCalledOnce()
    expect(progress).toHaveBeenLastCalledWith({ completedBytes: 3, totalBytes: 3 })
  })

  it.each([null, '"etag"'])(
    'rejects changed contents against a supplied session with receipt %s before authorization',
    async (etag) => {
      const existingSession: MultipartUpload = {
        ...session,
        parts: [{ part_number: 1, byte_size: 3, sha256: digest, etag }],
      }
      await expect(
        uploadMultipartFile(
          appId,
          uploadId,
          fileWithBytes('xyz'),
          new AbortController().signal,
          vi.fn(),
          existingSession,
        ),
      ).rejects.toThrow('differs')
      expect(control.startMultipartUpload).not.toHaveBeenCalled()
      expect(control.authorizeUploadPart).not.toHaveBeenCalled()
      expect(storage.uploadPartBytes).not.toHaveBeenCalled()
      expect(control.completeMultipartUpload).not.toHaveBeenCalled()
    },
  )

  it('rejects a supplied session belonging to another upload before authorization', async () => {
    await expect(
      uploadMultipartFile(appId, uploadId, fileWithBytes(), new AbortController().signal, vi.fn(), {
        ...session,
        upload_id: 'ffffffff-ffff-4fff-8fff-ffffffffffff',
      }),
    ).rejects.toThrow()
    expect(control.startMultipartUpload).not.toHaveBeenCalled()
    expect(control.authorizeUploadPart).not.toHaveBeenCalled()
    expect(storage.uploadPartBytes).not.toHaveBeenCalled()
    expect(control.completeMultipartUpload).not.toHaveBeenCalled()
  })

  it('rejects different contents with the same filename and size before any transfer', async () => {
    vi.mocked(control.startMultipartUpload).mockResolvedValue({
      ...session,
      parts: [{ part_number: 1, byte_size: 3, sha256: digest, etag: null }],
    })
    await expect(
      uploadMultipartFile(
        appId,
        uploadId,
        fileWithBytes('xyz'),
        new AbortController().signal,
        vi.fn(),
      ),
    ).rejects.toThrow('differs')
    expect(storage.uploadPartBytes).not.toHaveBeenCalled()
    expect(control.completeMultipartUpload).not.toHaveBeenCalled()
  })

  it('does not reuse receipts belonging to a different file size', async () => {
    vi.mocked(control.startMultipartUpload).mockResolvedValue({
      ...session,
      parts: [{ part_number: 1, byte_size: 7, sha256: digest, etag: '"etag"' }],
    })
    await expect(
      uploadMultipartFile(appId, uploadId, fileWithBytes(), new AbortController().signal, vi.fn()),
    ).rejects.toThrow('does not match')
  })

  it('renews an expired signed URL with the API before retrying', async () => {
    vi.useFakeTimers()
    try {
      vi.mocked(storage.uploadPartBytes)
        .mockRejectedValueOnce(new storage.ArtifactTransferError(403))
        .mockResolvedValueOnce('"etag"')
      const transfer = uploadMultipartFile(
        appId,
        uploadId,
        fileWithBytes(),
        new AbortController().signal,
        vi.fn(),
      )
      // Hashing uses a native promise; wait until the failed transfer has scheduled its backoff.
      await vi.waitFor(() => expect(storage.uploadPartBytes).toHaveBeenCalledOnce())
      await vi.runAllTimersAsync()
      await transfer
      expect(control.authorizeUploadPart).toHaveBeenCalledTimes(2)
    } finally {
      vi.useRealTimers()
    }
  })

  it('keeps cancellation from completing a multipart object', async () => {
    const controller = new AbortController()
    controller.abort()
    await expect(
      uploadMultipartFile(appId, uploadId, fileWithBytes(), controller.signal, vi.fn()),
    ).rejects.toThrow()
    expect(control.completeMultipartUpload).not.toHaveBeenCalled()
  })

  it('reconciles a sealing upload instead of transferring it again', async () => {
    vi.mocked(control.startMultipartUpload).mockResolvedValue({ ...session, state: 'completing' })
    await uploadMultipartFile(
      appId,
      uploadId,
      fileWithBytes(),
      new AbortController().signal,
      vi.fn(),
    )
    expect(getUpload).toHaveBeenCalled()
    expect(storage.uploadPartBytes).not.toHaveBeenCalled()
  })
})
