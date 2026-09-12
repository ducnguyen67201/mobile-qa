import { createClient } from './generated/client'
import type { ApiError } from './generated/types.gen'
import { zApiError } from './generated/zod.gen'

export class ApiClientError extends Error {
  constructor(readonly status: number, readonly body: ApiError | null) {
    super(body?.message ?? `Invalid API error response (HTTP ${status})`)
    this.name = 'ApiClientError'
  }
}

export const apiClient = createClient({ baseUrl: window.location.origin, credentials: 'same-origin' })
apiClient.interceptors.error.use((error: unknown, response) => {
  // Network failures and success-body validation errors are not API error envelopes.
  if (!response || response.ok) return error
  const parsed = zApiError.safeParse(error)
  return new ApiClientError(response.status, parsed.success ? parsed.data : null)
})
