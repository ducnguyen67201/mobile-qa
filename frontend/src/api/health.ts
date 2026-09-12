import type { HealthResponse } from '../bindings/HealthResponse'
import { isRecord, request } from './client'
export function parseHealth(data: unknown): HealthResponse {
  if (!isRecord(data) || data.status !== 'ok' || typeof data.service !== 'string' || typeof data.version !== 'string') {
    throw new Error('Invalid health response')
  }
  return { status: data.status, service: data.service, version: data.version }
}
export function getHealth(): Promise<HealthResponse> { return request('/api/health', parseHealth) }
