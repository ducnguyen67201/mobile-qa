/** Application-owned configuration around the generated HTTP client. */
import { createClient, type Client, type RequestOptions } from './generated/client'
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
const transportClient = createClient({ baseUrl: window.location.origin, credentials: 'same-origin' })
// Pinned client 0.99 normalizes headers to Headers before invoking its generated
// z.object validator. Preserve the SDK's original header record for validation;
// serialization, auth, fetch and error handling still belong to the generated client.
const requestMethods = new Set(['get', 'post', 'put', 'patch', 'delete', 'head', 'options', 'request'])
export const apiClient: Client = new Proxy(transportClient, {
  get(target, property, receiver) {
    const value = Reflect.get(target, property, receiver)
    if (typeof property !== 'string' || !requestMethods.has(property)) return value
    const method = value as Client['get']
    return (options: RequestOptions) => {
      const validator = options.requestValidator
      return method({ ...options, requestValidator: validator
        ? data => validator(Object.assign({}, data, { headers: options.headers }))
        : undefined })
    }
  },
})
apiClient.interceptors.error.use((error: unknown, response) => {
  // Network failures and success-body validation errors are not API error envelopes.
  if (!response || response.ok) return error
  if (response.status === 401 && !response.url.endsWith("/auth/login")) {
    queueMicrotask(() => window.dispatchEvent(new Event("mobile-qa:unauthorized")))
  }
  const parsed = zApiError.safeParse(error)
  return new ApiClientError(response.status, parsed.success ? parsed.data : null)
})
