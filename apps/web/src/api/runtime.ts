/** Application-owned configuration around the generated HTTP client. */
import { createClient } from './generated/client'
import type { ApiError } from './generated/types.gen'
import { zApiError } from './generated/zod.gen'

/**
 * A rejected HTTP response. body is null when its payload fails the generated schema;
 * callers must not treat an arbitrary upstream error page as a trusted API message.
 */
export class ApiClientError extends Error {
  constructor(readonly status: number, readonly body: ApiError | null) {
    super(body?.message ?? `Invalid API error response (HTTP ${status})`)
    this.name = 'ApiClientError'
  }
}

// Same-origin transport works through Vite's dev proxy and the future Rust static host.
// Do not put a provider key or Doppler secret in browser configuration.
export const apiClient = createClient({ baseUrl: window.location.origin, credentials: 'same-origin' })
apiClient.interceptors.error.use((error: unknown, response) => {
  // Network failures and success-body validation errors are not API error envelopes.
  if (!response || response.ok) return error
  const parsed = zApiError.safeParse(error)
  return new ApiClientError(response.status, parsed.success ? parsed.data : null)
})
