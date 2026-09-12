# 03 — App setup: UI and backend together

Status: planned. Depends on: 01. Device preflight uses 02 once qualified. Owns: Rust app/build/auth modules, migrations, transport DTOs, frontend App screen.

## Deliverable

An authenticated user creates an app, uploads a test APK and sees whether it is ready to run on the supported device. Build each operation from database to API to UI before starting the next. Do not finish all backend endpoints or all UI screens in advance.

## Ordered work

1. **Auth and shell:** add authentication using the pinned framework's supported facilities (the foundation deliberately omitted auth), then add organization/project membership enforcement, and build the App / Tests / Runs / Settings navigation. Use an operator-provisioned pilot account initially; public registration and complex invitation flows can wait.
2. **Create app:** migration and Rust service → typed create/detail endpoints → form and persisted detail page. Required fields: name, Android package, test-environment name and permitted backend/login origins. Package identity is checked against uploaded APK metadata.
3. **Upload build:** private upload session → file transfer → explicit finalization → APK metadata/checksum validation → displayed build record. Retry finalization idempotently. A file visible in storage is not automatically an accepted build.
4. **Configure test access:** store secret references and readiness metadata, display masked status, and let the operator verify the test account/reset path. Test definitions contain references, not credentials.
5. **Show readiness:** distinguish build validation, device install compatibility, backend/account prerequisites and case readiness. Use the qualified worker for install preflight after spec 04's protocol exists; before then display “not checked,” never invented success.

## Data and API shape

Initial entities: users, organizations, memberships, projects/apps, environments, builds and secret-reference metadata. Use immutable build IDs and content hashes; retain original filename as metadata only. Every customer record belongs to an organization/project.

Proposed routes:

- `POST /api/apps`, `GET /api/apps/:app_id`
- `POST /api/apps/:app_id/build-uploads`
- `POST /api/apps/:app_id/build-uploads/:upload_id/complete`
- `GET /api/apps/:app_id/builds`
- `PATCH /api/apps/:app_id/environment`

Rust request/response DTOs and endpoint descriptions are authoritative. Consume generated API functions with typed methods, paths, parameters, responses and declared errors; enable generated runtime validation. Check real route/status/serialization behavior against the contract with endpoint tests. Represent structured errors with a stable code, user-readable message and request ID.

Use authenticated server-mediated uploads to local private storage for development. Pilot storage uses short-lived scoped object uploads or bounded streaming through the API. Both share the same upload-session/finalization semantics. Verify server-side ownership, actual byte size, hash and parsed APK metadata; never trust client-provided values alone.

## Authentication details

Do not copy a starter's browser-token storage choice without review. For the same-origin pilot, use short-lived server-validated authentication in Secure/HttpOnly cookies with appropriate SameSite behavior, CSRF protection and origin checks on mutations. Enforce session expiry/logout and server-side membership checks. A frontend route guard is only navigation behavior.

If the pinned Loco starter cannot support this cleanly, record the specific gap and select a managed identity integration before customer use. Do not implement a new password system to save one dependency.

## UI acceptance

- Reload preserves the selected app/build and real server state.
- Empty, uploading, validating, unsupported, ready and network-error states are distinct.
- Invalid package/ABI, oversized/corrupt files and expired uploads have actionable messages.
- Settings contains limits, retention and access; the App page contains app-specific setup.
- Secrets never return as plain text in ordinary API responses or appear in logs/URLs.

## Verification

One meaningful browser smoke covers sign-in → create app → upload → persisted status. API tests prove a second organization's IDs cannot expose or mutate records/artifacts. Test duplicate upload finalization, malformed metadata and interrupted uploads. Keep generated CRUD tests only when they prove behavior we rely on.

Done means genuine persisted setup with honest readiness, not a polished static dashboard. Full test editing, generation and live device streaming are deferred.
