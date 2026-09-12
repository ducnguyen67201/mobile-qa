/** Synthetic transport fixtures only; no real account, APK or device evidence. */
import type { AppResponse, BuildResponse, SessionResponse, SettingsResponse, UploadResponse } from '@/api/generated/types.gen'
export const appId = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa'
export const orgId = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb'
export const buildId = 'cccccccc-cccc-4ccc-8ccc-cccccccccccc'
export const uploadId = 'dddddddd-dddd-4ddd-8ddd-dddddddddddd'
export const userId = 'eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee'
export const timestamp = '2026-09-12T09:00:00Z'
export const session = { user: { id: userId, email: 'synthetic@example.test', display_name: 'Synthetic Member' }, memberships: [{ organization_id: orgId, name: 'Fixture workspace', role: 'member' }], csrf_token: 'test-only-csrf', expires_at: '2099-09-12T09:00:00Z' } satisfies SessionResponse
export const readiness = { execution_ready: false, install: 'not_checked', device_profile: null, backend: 'not_checked', account: 'not_checked', reset: 'not_checked', cases: 'not_configured', account_configured: false, reset_configured: false } satisfies AppResponse['readiness']
export const app = { id: appId, organization_id: orgId, name: 'Synthetic app', android_package: 'com.example.synthetic', environment: { id: orgId, name: 'Staging', backend_origins: ['https://api.synthetic.test'], login_origins: [], revision: 1, account_secret_reference_id: null, reset_secret_reference_id: null, secret_references: [], checks: [] }, readiness, created_at: timestamp } satisfies AppResponse
export const settings = { memberships: session.memberships, max_apk_bytes: 1048576, upload_ttl_seconds: 86400, session_ttl_seconds: 28800, max_active_uploads: 3, storage: 'Private local storage', accepted_build_retention: 'Accepted builds are retained. Automated retention is not configured.' } satisfies SettingsResponse
export const upload = { id: uploadId, app_id: appId, original_filename: 'synthetic.apk', expected_size: 4, actual_size: 4, state: 'uploaded', expires_at: '2099-09-12T09:00:00Z', build_id: null, retry_after_seconds: null } satisfies UploadResponse
export const build = { id: buildId, upload_id: uploadId, app_id: appId, original_filename: 'synthetic.apk', byte_size: 4, sha256: 'a'.repeat(64), created_at: timestamp, validation: { state: 'validated', reason_code: null, message: null, started_at: timestamp, completed_at: timestamp, validator_version: 'fixture-only', intake_policy_version: 'fixture-only' }, metadata: { package_name: app.android_package, version_name: '1.0', version_code: '1', min_sdk: 23, target_sdk: 35, native_abis: [], signature_verified: true }, readiness, can_retry_validation: false } satisfies BuildResponse
export function apiError(status = 401) { return Response.json({ code: status === 401 ? 'unauthorized' : 'unavailable', message: 'Synthetic request failure', details: null, request_id: userId }, { status }) }
