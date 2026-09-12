// Compiled with the app: generated endpoint inputs/output remain authoritative.
import type { GetHealthData, HealthResponse } from '../api/generated/types.gen'
export const healthy: HealthResponse = { status: 'ok', service: 'mobile-qa', version: '0.1.0' }
// @ts-expect-error Missing required response fields must fail compilation.
const staleResponse: HealthResponse = { status: 'ok' }
// @ts-expect-error This read-only operation does not accept a request body.
const unexpectedBody: GetHealthData['body'] = { anything: true }
void staleResponse
void unexpectedBody
