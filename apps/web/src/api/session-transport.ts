/** One CSRF cache for all generated browser transports; cleared with the session. */
import type { z } from 'zod'
import { apiClient } from './runtime'
let csrfToken = ''
export function setCsrfToken(value: string) {
  csrfToken = value
}
export function forgetSession() {
  csrfToken = ''
}
export const headers = () => ({ 'X-CSRF-Token': csrfToken })
export const options = { client: apiClient, throwOnError: true as const }
export async function checked<T>(
  promise: Promise<{ data: unknown; response: Response }>,
  schema: z.ZodType<T>,
  statuses: readonly number[] = [200],
): Promise<T> {
  const result = await promise
  if (!statuses.includes(result.response.status))
    throw new Error(`Unexpected API response status (${result.response.status})`)
  return schema.parse(result.data)
}
