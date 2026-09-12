import type { ApiError } from '../bindings/ApiError'
export class ApiClientError extends Error {
  constructor(readonly status: number, readonly body: ApiError | null) {
    super(body?.message ?? `Request failed with status ${status}`)
    this.name = 'ApiClientError'
  }
}
export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}
export async function request<T>(path: string, parse: (data: unknown) => T): Promise<T> {
  const response = await fetch(path, { credentials: 'same-origin', headers: { Accept: 'application/json' } })
  const data: unknown = await response.json().catch(() => null)
  if (!response.ok) {
    const body: ApiError | null = isRecord(data) && typeof data.code === 'string' && typeof data.message === 'string'
      ? { code: data.code, message: data.message, details: data.details ?? null } : null
    throw new ApiClientError(response.status, body)
  }
  return parse(data)
}
