/** Resume proves the selected bytes match every recorded part before sending new bytes. */
import { ArtifactTransferError, partSha256, uploadPartBytes } from '@/api/artifact-transfer'
import {
  authorizeUploadPart,
  completeMultipartUpload,
  confirmUploadPart,
  startMultipartUpload,
} from '@/api/multipart'
import { getUpload } from '@/api/setup'
import type { MultipartUpload } from '@/api/generated/types.gen'

export type TransferProgress = { completedBytes: number; totalBytes: number }
type Progress = (progress: TransferProgress) => void

function pause(milliseconds: number, signal: AbortSignal) {
  signal.throwIfAborted()
  return new Promise<void>((resolve, reject) => {
    const cancel = () => {
      clearTimeout(timer)
      reject(signal.reason)
    }
    const timer = setTimeout(() => {
      signal.removeEventListener('abort', cancel)
      resolve()
    }, milliseconds)
    signal.addEventListener('abort', cancel, { once: true })
  })
}

async function awaitSealed(appId: string, uploadId: string, signal: AbortSignal) {
  const deadline = Date.now() + 20 * 60_000
  while (Date.now() < deadline) {
    signal.throwIfAborted()
    const upload = await getUpload(appId, uploadId, signal)
    if (upload.state === 'uploaded' || upload.state === 'finalized') return upload
    if (upload.state !== 'receiving')
      throw new Error('The saved upload needs attention. Check its status before retrying.')
    await pause(2000, signal)
  }
  throw new Error('The server is still verifying the upload. You can check its status later.')
}

function validateSession(session: MultipartUpload, file: File) {
  if (
    session.part_size !== 16 * 1024 * 1024 ||
    session.max_parallel_parts < 1 ||
    session.max_parallel_parts > 2
  )
    throw new Error('The server returned an unsupported transfer configuration.')
  const count = Math.ceil(file.size / session.part_size)
  const seen = new Set<number>()
  for (const part of session.parts) {
    const size = Math.min(session.part_size, file.size - (part.part_number - 1) * session.part_size)
    if (
      part.part_number < 1 ||
      part.part_number > count ||
      seen.has(part.part_number) ||
      part.byte_size !== size
    )
      throw new Error('The saved upload does not match this file.')
    seen.add(part.part_number)
  }
  return count
}

async function sendPart(
  appId: string,
  uploadId: string,
  partNumber: number,
  part: Blob,
  digest: string,
  signal: AbortSignal,
) {
  for (let attempt = 0; attempt < 3; attempt++) {
    signal.throwIfAborted()
    const capability = await authorizeUploadPart(
      appId,
      uploadId,
      { part_number: partNumber, byte_size: part.size, sha256: digest },
      signal,
    )
    try {
      const etag = await uploadPartBytes(capability.url, part, signal, capability.headers)
      await confirmUploadPart(appId, uploadId, partNumber, etag, signal)
      return
    } catch (error) {
      const retryable =
        error instanceof ArtifactTransferError &&
        (error.status === 403 || error.status === 429 || error.status >= 500)
      if (signal.aborted || !retryable || attempt === 2) throw error
      await pause(500 * 2 ** attempt, signal)
    }
  }
}

export async function uploadMultipartFile(
  appId: string,
  uploadId: string,
  file: File,
  signal: AbortSignal,
  onProgress: Progress,
  existingSession?: MultipartUpload,
) {
  signal.throwIfAborted()
  const session = existingSession ?? (await startMultipartUpload(appId, uploadId, signal))
  if (session.upload_id !== uploadId) throw new Error('The saved upload reference does not match.')
  if (session.state === 'aborted') throw new Error('This upload was discarded. Start a new upload.')
  if (session.state !== 'uploading') return awaitSealed(appId, uploadId, signal)
  const count = validateSession(session, file)
  const digests = new Map<number, string>()
  const completed = new Set<number>()
  let completedBytes = 0
  // Verify every known part first: a mismatch must not produce a mixed-file object.
  for (const saved of session.parts) {
    const part = file.slice(
      (saved.part_number - 1) * session.part_size,
      saved.part_number * session.part_size,
    )
    const digest = await partSha256(part, signal)
    if (digest !== saved.sha256)
      throw new Error(
        'This file differs from the original upload. Select the original file or start a new upload.',
      )
    digests.set(saved.part_number, digest)
    if (saved.etag) {
      completed.add(saved.part_number)
      completedBytes += part.size
    }
  }
  onProgress({ completedBytes, totalBytes: file.size })
  let next = 1
  const transfers = new AbortController()
  const joined = AbortSignal.any([signal, transfers.signal])
  const transfer = async () => {
    while (next <= count) {
      const number = next++
      if (completed.has(number)) continue
      joined.throwIfAborted()
      const part = file.slice((number - 1) * session.part_size, number * session.part_size)
      const digest = digests.get(number) ?? (await partSha256(part, joined))
      await sendPart(appId, uploadId, number, part, digest, joined)
      completedBytes += part.size
      onProgress({ completedBytes, totalBytes: file.size })
    }
  }
  const jobs = Array.from({ length: session.max_parallel_parts }, () =>
    transfer().catch((error: unknown) => {
      transfers.abort(error)
      throw error
    }),
  )
  // Settle sibling transfers before returning so a failed batch cannot keep mutating.
  const results = await Promise.allSettled(jobs)
  for (const result of results) if (result.status === 'rejected') throw result.reason
  await completeMultipartUpload(appId, uploadId, signal)
  return awaitSealed(appId, uploadId, signal)
}
