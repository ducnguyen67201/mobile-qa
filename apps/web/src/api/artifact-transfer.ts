/** Binary storage capabilities never receive the application's cookies or API headers. */
export class ArtifactTransferError extends Error {
  constructor(readonly status: number) {
    super(
      status === 403
        ? 'The upload link expired. Retry to renew it.'
        : 'The file part could not be uploaded.',
    )
    this.name = 'ArtifactTransferError'
  }
}

export async function partSha256(part: Pick<Blob, 'arrayBuffer'>, signal: AbortSignal) {
  signal.throwIfAborted()
  const bytes = await part.arrayBuffer()
  signal.throwIfAborted()
  const digest = await crypto.subtle.digest('SHA-256', bytes)
  signal.throwIfAborted()
  return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, '0')).join('')
}

export async function uploadPartBytes(
  url: string,
  part: Blob,
  signal: AbortSignal,
  signedHeaders: Record<string, string> = {},
) {
  const target = new URL(url)
  if (target.protocol !== 'https:' || target.username || target.password || target.hash)
    throw new Error('The storage upload link is invalid.')
  const headers = new Headers()
  for (const [name, value] of Object.entries(signedHeaders)) {
    if (
      !['content-type', 'x-amz-checksum-sha256', 'x-amz-content-sha256'].includes(
        name.toLowerCase(),
      )
    )
      throw new Error('The storage upload headers are invalid.')
    headers.set(name, value)
  }
  const response = await fetch(target, {
    method: 'PUT',
    body: part,
    credentials: 'omit',
    redirect: 'error',
    referrerPolicy: 'no-referrer',
    headers,
    signal,
  })
  if (!response.ok) throw new ArtifactTransferError(response.status)
  const etag = response.headers.get('etag')
  if (!etag || etag.length > 1024) throw new Error('Storage did not return a valid part receipt.')
  return etag
}
