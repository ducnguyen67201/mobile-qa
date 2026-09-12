# Plan: 03 — App setup

## Summary

Turn the health-only scaffold into the first persisted customer workflow: an operator-provisioned user signs in, creates an Android app and environment, uploads an APK, and sees validation of the actual stored file. Implement authentication, app creation, upload/finalization and readiness as successive database → API → UI authoring slices, followed by one final generation and verification phase.

This plan covers spec 03. APK intake validation is distinct from installation, backend/account verification and test readiness. Device preflight stays `not_checked` until specs 02 and 04 supply qualified execution evidence.

## User Story

As a developer at a pilot customer, I want to sign in, register my app and upload a test APK, so that I can see its persisted identity, validation result and remaining setup requirements.

## Problem → Solution

Unauthenticated navigation and API liveness → authorized app/build records, private file intake, real metadata validation and actionable readiness states that survive reload.

## Metadata

- **Complexity:** XL; first authentication, tenant schema and artifact subsystem.
- **Source PRD:** N/A; free-form request, grounded in `docs/architect/implementation/03-app-setup-and-ui-backend.md` and product US-01/US-11.
- **PRD Phase:** 03 — App setup.
- **Baseline inspected:** `d137e35`, 2026-09-12; clean working tree before planning.
- **Estimated files:** Approximately 115–125 files plus generated SDK outputs; 14 tasks in five vertical slices. Many files are small SeaORM entities, registries or imported upstream UI components, not custom implementations.
- **Implementation status:** Local implementation verified on `codex/03-app-setup`; hosted/rendered acceptance remains open. See the implementation report.
- **Dependencies:** Implemented source foundation from 01. Its rendered-browser acceptance remains open. Specs 02/04 gate device checks, not local app setup.
- **Authority:** `docs/architect/README.md`; this plan is an implementation snapshot, not a second architecture authority.
- **Navigation note:** The supplied supplement references `docs/CODEX-NAVIGATION-GUIDE.md`, but that file is absent. The architecture index supplies repository ownership/navigation.

## UX Design

### Before

```text
App | Tests | Runs | Settings
App: “App onboarding will be added in the next phase.”
API: Connecting… / Ready / Cannot connect [Retry]
```

### After

```text
/sign-in → protected shell → /apps → /apps/:app_id?build=:build_id

Desktop: reusable sidebar (App / Tests / Runs / Settings)
Mobile: the same navigation in the sidebar’s sheet
Content header: [Toggle navigation] App             [Account menu]

App: PocketTasks                  Android: com.example.pockettasks
Environment: Staging              [Edit environment]
[Choose APK] [Upload]

Build history                     Selected build
v1.4 · 2026-09-12                 package / version / bytes / SHA-256
v1.3 · 2026-09-11                 APK validation: Validated
                                 Device installation: Not checked
                                 Backend: Not checked
                                 Test account/reset: Not checked
                                 Tests: Not configured
                                 “Device checks are not available yet.”
```

The App page never labels an APK “ready to run” based on parsing alone. “Validated” means APK intake checks passed. A failed validation includes a stable reason and a next action. Settings shows actual session/access and configured intake limits; retention is explicitly described as not automated for accepted builds yet.

### UI framework and reuse decision

Use **shadcn/ui + Tailwind CSS**, extending the existing `new-york` configuration in `apps/web/components.json`. The user explicitly requested framework reuse; this follows architecture decision A03. Start the dashboard shell from the official [Sidebar block](https://ui.shadcn.com/blocks/sidebar) (`sidebar-01`) and adapt navigation to React Router. Use its [Radix Sidebar components](https://ui.shadcn.com/docs/components/radix/sidebar) for responsive navigation. Keep one consistent Radix-backed component family.

shadcn distributes component source into the application. Import the upstream implementation and required dependencies; do not recreate its primitives. Custom components such as AppForm and ApkUpload compose the kit and connect generated API adapters.

| Dashboard surface | Reuse | Product-specific work |
|---|---|---|
| Navigation shell | Sidebar block, SidebarProvider/Inset/Trigger, Sheet, Tooltip, Separator | Four product links, active route and responsive layout |
| Sign-in and setup forms | Card, Button, Input, Label, Textarea, Select | Field values, generated validation, errors and submissions |
| Account actions | DropdownMenu and Button | Current account and sign-out |
| Build history | Table, Badge and Button | Actual build rows, URL selection and server pagination |
| APK picker | Input type=file, Button, Card | Upload/finalization state and recovery |
| Loading/errors | Skeleton, Alert and text with live-region semantics | Real state and Retry; indeterminate transfer feedback |
| Metadata/readiness | Card, Badge, Separator | Persisted values and truthful readiness labels |

Follow the existing-project [Vite integration](https://ui.shadcn.com/docs/installation/vite). Preserve Vite, Router, Query, the `@/` alias and `rsc:false`; do not scaffold another app. Pin the chosen official registry revision/CLI version when importing components, record it in dependencies.md, and lock runtime dependencies with pnpm. Import only the listed components and the sidebar’s actual dependencies. Review changes to existing Button/Card before replacement: preserve `type="button"` as the safe default and existing heading/accessibility semantics while adding upstream variants/composition support.

Extend `src/index.css` with the semantic tokens required by the imported components: primary, accent, destructive, input, ring, popover and sidebar families. Map them to the existing neutral/green palette; preserve focus and skip-link behavior. Reuse component styles and reserve feature-specific Tailwind for layout. Keep MIT attribution. Registry import is source authoring, distinct from API contract generation; avoid any CLI path that triggers formatting/checks during authoring and import reviewed upstream source directly if necessary.

Use the kit’s simple Table with server pagination. Small forms use React state, kit controls and generated Zod; no additional form-state library is needed. Omit template demo charts/statistics, extra routing and another UI kit. Feature-specific code implements the actual app setup behavior.

### Interaction Changes

| Touchpoint | Before | After | Notes |
|---|---|---|---|
| Entry | Public health panel | Sign-in or app list | Return destination limited to local protected routes |
| Create app | Absent | Name, Android package, environment name, backend/login origins | Inline errors; save disabled only while submitting |
| Upload | Absent | Choose file → create session → transfer → finalize | Indeterminate transfer indicator; no fabricated percentage |
| Finalization | Absent | Validating → validated / invalid / unsupported / error | Error means validator infrastructure failed; not necessarily bad APK |
| Interrupted request | Absent | Reload server upload status; retry appropriate step | Never automatically create another build after an uncertain response |
| App/build selection | Absent | URL identifies selected app/build | Browser file selection cannot survive reload; reselect only if bytes never sealed |
| Account setup | Absent | Masked reference status and operator-reported checks | Stored reference is not proof of credentials working |
| Network failure | Health-only Retry | Keep last known data, show stale/error indicator and Retry | Do not replace stale data with a green state |
| Auth expiry/logout | Absent | Clear cached tenant data and return to sign-in | Server revocation remains authoritative |
| Accessibility | Existing skip link/navigation | Labeled fields, field errors, focus to first error, live status | Status uses text; do not depend on color |

## Mandatory Reading

Line references refer to the inspected baseline; new modules listed later do not exist yet.

| Priority | File | Lines | Why |
|---|---|---|---|
| P0 | `AGENTS.md` | all | Ownership, secrets, final-only verification, browser restriction |
| P0 | `docs/architect/README.md` | all | Canonical authority |
| P0 | `docs/architect/implementation/03-app-setup-and-ui-backend.md` | all | Feature scope and acceptance |
| P0 | `docs/architect/status.md` | all | Existing evidence and open gates |
| P0 | `docs/architect/contracts.md` | all | Generated boundary and route agreement |
| P0 | `docs/architect/environment.md` | all | Doppler isolation and no env files |
| P0 | `apps/api/src/app.rs` | 20–60 | Hook, routing, task and migration integration |
| P0 | `apps/api/src/controllers/health.rs` | 5–50 | Typed responses and 404/405 handling |
| P0 | `crates/contracts/src/browser.rs` | 9–62 | Pure DTO and Utoipa declaration |
| P0 | `apps/api/tests/health.rs` | 10–49 | Real route/OpenAPI/DB test |
| P1 | `apps/api/migration/src/lib.rs` | 1–14 | Registry and injection marker |
| P1 | `apps/web/src/api/runtime.ts` | 1–25 | Same-origin cookies and generated error validation |
| P1 | `apps/web/src/api/queries.ts` | 7–23 | Expected status plus generated success validation |
| P1 | `apps/web/src/api/transport.test.ts` | 1–53 | Test generated SDK using fetch-only mocks |
| P1 | `apps/web/src/pages/Home.test.tsx` | 1–36 | Memory router, QueryClient, accessible assertions |
| P1 | `apps/web/src/routes.tsx`, `apps/web/src/App.tsx` | all | Existing shell and markers |
| P1 | `apps/web/components.json`, `apps/web/src/index.css`, `apps/web/src/components/ui/button.tsx`, `apps/web/src/components/ui/card.tsx` | all | Existing shadcn setup, palette and primitive behavior |
| P1 | `apps/web/src/main.tsx` | 8–12 | Query client lifetime and retry policy |
| P1 | `scripts/runtime.py` | 27–38, 67–156 | Owned processes, secret-free smoke, DB checks |
| P1 | `scripts/contracts.py` | 15–66 | Content-only generation/drift |
| P1 | `.github/workflows/ci.yaml` | all | Affected surfaces; new files must trigger consumers |
| P1 | `Cargo.toml`, `apps/api/Cargo.toml`, `Cargo.lock` | all | Loco 1.1.0; SeaORM 2.0.2; Axum 0.8 |
| P1 | `apps/web/package.json`, `apps/web/openapi-ts.config.ts` | all | Pinned client generator 0.99.0 and Zod 4.6.2 |
| P2 | `docs/architect/implementation/02-cloud-phone-and-feasibility.md` | all | Candidate API 35/x86_64 profile is unqualified |
| P2 | `docs/architect/product.md`, `docs/architect/commenting.md`, `docs/architect/development.md` | all | Scope, comments and validation rules |

## Discovery and Traces

### Unified Discovery Table

| Category | File:Lines | Pattern | Key snippet / finding |
|---|---|---|---|
| Similar implementation | `apps/api/src/controllers/health.rs:10–17` | Typed Axum response in Loco controller | `async fn current() -> Json<HealthResponse>` |
| Naming | `crates/contracts/src/browser.rs:12–25` | Rust PascalCase DTOs, snake_case enum wire values | `#[serde(rename_all = "snake_case")]` |
| Errors | `apps/web/src/api/runtime.ts:10–24` | Structured errors validated with generated Zod | `zApiError.safeParse(error)` |
| Logging | `apps/api/config/development.yaml:4–8`, `apps/api/config/production.yaml:4–8` | Compact info locally, JSON info in production | No owned domain tracing calls yet |
| Types | `crates/contracts/src/browser.rs:39–62` | Pure OpenAPI declarations | `operation_id = "getHealth"` |
| Tests | `apps/api/tests/health.rs:29–48` | Loco request harness, actual HTTP and local DB | `request::<App, _, _>(...)` |
| Configuration | `apps/web/vite.config.ts:8–12`, `scripts/runtime.py:78–89` | Env-file loading off; API-only secret injection | `envDir: false` |
| Dependencies | `Cargo.toml:9–10`, `apps/web/package.json` | Fixed framework and generated-client baseline | Auth feature currently disabled; no product ORM/storage service |

### Five Traces

1. **Entry:** `main.tsx` → RouterProvider → `routes.tsx` → `Home` → TanStack query. Proposed protected layout resolves session before mounting tenant queries.
2. **Data:** `healthQuery` → generated `getHealth` → same-origin `/api` (Vite proxy in dev) → Loco `App::routes` → typed JSON → explicit status check and generated Zod. Every new JSON operation follows this chain.
3. **State:** Currently only query state and migration bookkeeping. New state lives in PostgreSQL; immutable upload bytes live outside the static directory. React state stores form/transfer UI, not canonical build status.
4. **Contracts:** Pure Rust DTOs/Utoipa → local OpenAPI → generated SDK/types/Zod; Python fixture shapes remain untouched. Real registered route tests bind declarations to behavior.
5. **Patterns:** Loco hook and module registries exist; domain model, repository and service implementations do not. Introduce small feature services using SeaORM transactions, without inventing a generic domain framework or claiming an existing repository pattern.

## External Documentation and Version Findings

Research performed 2026-09-12. Version-specific installed Loco source was also inspected under Cargo's registry; it is read-only evidence, never an edit target.

| Topic | Source | Key takeaway |
|---|---|---|
| Loco cookie auth | [Pinned extractor source](https://github.com/loco-rs/loco/blob/v1.1.0/src/controller/extractor/auth.rs) | Supports cookie-only JWT extraction; default bearer behavior must be overridden |
| Password hashing | [Pinned Loco hash source](https://github.com/loco-rs/loco/blob/v1.1.0/src/hash.rs) | Reuse framework Argon2id hashing/verification |
| ORM transactions | [SeaORM transactions](https://www.sea-ql.org/SeaORM/docs/advanced-query/transaction/) | Commit/rollback define atomic state changes; file storage still needs reconciliation |
| Request limits | [Axum DefaultBodyLimit](https://docs.rs/axum/latest/axum/extract/struct.DefaultBodyLimit.html) | Extractor limit is not a universal streaming-body cap |
| APK metadata | [AAPT2](https://developer.android.com/tools/aapt2) | `dump badging` and `dump xmltree` inspect compiled APK metadata |
| Alternative metadata tool | [apkanalyzer](https://developer.android.com/tools/apkanalyzer) | Supplies individual package/version/SDK queries; requires command-line tools |
| Signature validation | [apksigner](https://developer.android.com/tools/apksigner) | `verify` checks APK signatures; do not modify bytes after verification |
| Tool distribution | [Build Tools](https://developer.android.com/tools/releases/build-tools) | Use an explicit Build Tools version, not an unpinned PATH binary |
| ZIP inspection | [zip 8.6.0 ZipArchive](https://docs.rs/zip/8.6.0/zip/read/struct.ZipArchive.html) | Read entries without extracting; enforce our own budgets and entry policy |
| CSRF | [OWASP CSRF guidance](https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html) | SameSite alone is insufficient; validate origin and a session-bound token |

- **KEY_INSIGHT:** Loco 1.1.0 already exposes `hash::hash_password`, `hash::verify_password`, `auth::jwt::JWT::generate_token`, and `controller::extractor::auth::extract_jwt_from_request_parts`. **APPLIES_TO:** Authentication slice. **GOTCHA:** Enable `auth`; YAML uses `from: Cookie` with uppercase C (`src/config/auth.rs:40–48`); JWT signing expects a base64-encoded key (`src/auth/jwt.rs:100–139`). Add a session ID in custom claims and look it up in the DB; built-in JWT validation alone does not implement logout revocation.
- **KEY_INSIGHT:** There is no owned session or product service to copy. **APPLIES_TO:** New service layout. **GOTCHA:** Use Loco crypto facilities and small adapters; do not add a separate password algorithm, bearer fallback or browser token persistence.
- **KEY_INSIGHT:** APK static inspection does not execute the app. **APPLIES_TO:** Validation and readiness. **GOTCHA:** An intake-compatible ABI/min-SDK is not proof of installation, login, Play Services or emulator compatibility.
- **KEY_INSIGHT:** File bytes and DB commits cannot share a transaction. **APPLIES_TO:** Finalization. **GOTCHA:** Seal bytes first with a server-owned immutable key; use unique upload/build linkage, fencing tokens and retryable reconciliation.

## Strategic Design

### Selected approach

Use Loco's authentication primitives, SeaORM product/session records, same-origin cookies, private API-mediated multipart transfer, and bounded Rust-owned APK inspection. Finalization runs synchronously within a bounded request after persisting `validating`; concurrent/repeated requests observe the same build. A retry can reclaim an expired validation lease. This avoids needing the future device scheduler to validate a file.

Complete each slice's source, tests, contracts and UI before authoring the next slice. Do not run checks or generators between those slices: the repository requires the whole agreed scope to be authored before final verification. Generated imports can be authored against the operation/DTO names fixed below; generate once in Task 14.

### Alternatives considered

| Alternative | Decision |
|---|---|
| Managed identity service immediately | Unnecessary for this pilot slice: inspected Loco supports the required primitives. Reconsider if implementation exposes a concrete unsupported requirement |
| Starter JWT in localStorage | Rejected by canonical cookie/session requirements |
| Automatic acceptance when file exists | Rejected: cannot prove completeness, package identity or validity |
| Browser-only APK parsing | Useful optional hint later; cannot establish trusted build metadata |
| Pure custom binary Android manifest parser | Avoid new format implementation; pin official Android tools |
| Python device worker for validation | Would couple spec 03 to future worker protocol; keep Python unchanged |
| New queue/broker | Bounded synchronous validation with durable status and reclaimable lease suffices initially |
| Hosted APK storage | Implement an S3-compatible adapter for Railway Buckets now; preserve AWS S3 migration. Bucket provisioning/deployment remain separate actions |

### Scope / NOT Building

Build pilot sign-in/logout, org/app permissions, app/environment forms, private upload sessions, immutable builds, actual file validation, build history, reference metadata and honest readiness. Keep Tests/Runs as placeholders.

Do not build public signup, email/reset flows, invitations, OAuth providers, device allocation/install/run, AI generation, test editing, reports, APK downloads/sharing, resumable chunk upload, AAB/split APK sets, malware certification, AWS migration execution or a customer-secret resolver. No cloud/paid resource creation, credential changes, push or deployment is authorized by this planning task.

## Data Model and Invariants

All IDs are server-generated UUIDs; timestamps are UTC. Public IDs are not authorization. Use explicit SeaORM entities, ActiveModels and versioned migrations. Use text plus CHECK constraints for small lifecycle enums; UUID foreign keys and indexes; server-side length validation plus DB constraints. Do not serialize ORM entities directly.

| Table | Fields and constraints |
|---|---|
| `users` | id, normalized email UNIQUE, password_hash, display_name, disabled_at, created_at, updated_at. No public registration |
| `organizations` | id, name, created_at |
| `memberships` | user_id, organization_id, role `operator/member`, active; UNIQUE pair |
| `app_memberships` | user_id, organization_id, app_id; UNIQUE user/app; FK to org membership and matching app/org. Explicit per-app grant for members |
| `sessions` | id, user_id, created_at, expires_at, revoked_at, csrf_token. Never return session ID or JWT in JSON; token is exposed only in the protected session response. Index user/expiry |
| `login_attempts` | bounded rate bucket key, window_start, count, expires_at. Hash normalized-email/network buckets, no password or raw login body |
| `apps` | id, organization_id, name, android_package, created_by, created_at, updated_at; UNIQUE org/package; UNIQUE id/org for composite child FKs |
| `environments` | id, organization_id, app_id UNIQUE, name, backend_origins JSON, login_origins JSON, revision, account_secret_reference_id nullable, reset_secret_reference_id nullable, updated_at; matching org/app FKs |
| `secret_references` | id, org/app, label, private operator-managed locator, kind `account/reset`, created_at, updated_at. Public view returns label/present only, never locator or value |
| `environment_checks` | id, org/app/environment, environment_revision, kind `backend/account/reset`, state `not_checked/operator_reported_ok/operator_reported_blocked`, safe note, checked_by, checked_at. Preserve check history |
| `build_uploads` | id, org/app, created_by, original_filename, expected_size, state, expires_at, attempt_id nullable, lease_until nullable, storage_backend, sealed_storage_key nullable, actual_size nullable, sha256 nullable, created_at; index app/created, state/expiry |
| `builds` | id, org/app, upload_id UNIQUE, immutable storage_backend/storage_key/sha256/byte_size/original_filename, validation_state, reason_code/message nullable, parsed metadata nullable, validator_version, intake_policy_version, attempt_id/lease_until, created_at, validated_at nullable |

Projects and apps are one customer boundary in this phase: implement `apps`, not separate overlapping project/app hierarchies. An active org operator may administer all apps in that org; a member may create an app and receives its explicit app grant in the same transaction. Other app operations require both active organization membership and app grant, or the same-org operator role. Check joins on every scoped API and CLI operation. Foreign-tenant/missing resources both return 404; a known same-org resource with a disallowed operation returns 403 only where existence is already authorized.

Package identity is immutable after creation. One environment per app initially. App name 1–100 trimmed characters; environment name 1–80; package 1–255 with Java-style dot-separated identifiers. Normalize origin lists using a URL parser: scheme + host + optional nondefault port; reject userinfo, query, fragment, non-root paths, wildcards and non-http(s). Require at least one backend origin; login origins may be empty. Production uses HTTPS; explicit loopback HTTP is allowed only in development/test. Store only; do not fetch arbitrary supplied URLs. Limit each list to 20 origins, each 2048 bytes.

Environment edits use `expected_revision`; update under a conditional revision check or return 409. A change increments revision; prior checks become stale and cannot make the new configuration ready. Reference IDs must belong to this app/org and match kind. Operators enter reference locators through a local maintenance task, not through public API JSON. Customer APIs can select an already-authorized opaque reference ID or clear it.

Storage references contain an internal logical backend ID plus a provider-neutral object key, never a public URL or credential. Build hash/identity never changes. Validation fields may move through their declared lifecycle; terminal validated/invalid/unsupported outcomes cannot be overwritten. A separate upload of identical bytes can create a new build record; do not impose global hash uniqueness or leak another tenant's deduplication result.

## Authentication and Error Contract

Use `mobile_qa_session` only on loopback dev/test; production uses `__Host-mobile_qa_session`, Secure, HttpOnly, SameSite=Lax, Path=/, no Domain. Dev's non-Secure cookie exception is explicitly environment-scoped, never inferred from a forwarded header. Production origin comes from HOST and startup rejects HTTP. Dev browser origin is exactly `http://127.0.0.1:5173`, not the API's 5150 origin. Tests supply their configured trusted origin. Disable CORS credentials to other origins.

Reuse Loco JWT (default HS512), cookie-only extraction and framework password hash functions. JWT pid identifies user; a server-minted `sid` custom claim identifies a session. Session lifetime is 60 minutes, no refresh token in this slice; DB expiry/revocation and user disablement are checked on every request. Each login creates fresh sid/CSRF values and revokes the cookie's preceding session if present. Logout revokes the DB session before clearing the cookie. Password changes via operator task revoke all sessions. Blocking password verification goes through `spawn_blocking` with concurrency capped at four.

Every mutation requires an exact trusted Origin (or same-origin Referer fallback when Origin absent); reject missing/null/untrusted values. Login additionally accepts only JSON and requires `X-Mobile-QA-Request: 1`; no permissive CORS. Authenticated mutations also require `X-CSRF-Token`, compared in constant time with the session token. Bootstrap token comes from `GET /api/auth/session`, with `Cache-Control: no-store`; retain it in memory, never URLs/localStorage. Test login CSRF as well as post-login CSRF.

Login uses a generic 401 for unknown email/bad password/disabled account. Verify a dummy hash for unknown users to avoid a trivial fast path. Enforce DB-backed counters before expensive verification: 10 attempts/email/15 minutes and 100/network bucket/15 minutes, plus bounded global hashing concurrency; respond 429 with Retry-After. Trust forwarded client address only behind explicitly configured trusted proxy; otherwise use transport peer address. Keep counter cardinality bounded/expired rows cleaned. These are configurable initial limits, not measured capacity.

Extend existing `ApiError` with required UUID `request_id`; retain code/message/details, with details restricted to safe field errors. Introduce a typed application `ApiFailure: IntoResponse` and request ID middleware. Normalize JSON/multipart/path/query rejections, body limit errors, auth/CSRF failures and 404/405 to this shape. Do not let native Loco errors bypass the declared contract. Do not echo DB errors, filesystem paths, command output or secrets. Generate request IDs server-side; return matching X-Request-ID. Adapt exact health tests and all web fixture envelopes.

Logging is new domain behavior: use `tracing` fields `request_id`, authorized org/app/build/upload IDs, phase, duration_ms and stable reason_code. Info for lifecycle changes, warn for denied/rejected operations, error for internal failures. Never log password/JWT/CSRF/secret locators, raw command output, arbitrary URLs or uploaded content. Place narrow module comments at auth, fake/tool and generated boundaries.

## API Inventory and DTOs

All routes remain under `/api`; shared path constants in pure Rust declarations connect to Axum registrations. OpenAPI uses `{app_id}` etc. Each operation declares cookie security where required, path/query/body schemas, CSRF/custom headers, success status and the `ApiError` error envelope. Operation IDs below determine generated SDK names. Use `limit` (default 20/max 100) and optional cursor for lists; stable `(created_at,id)` ordering, app/org-bound cursors validated server-side.

| Method / path | Operation ID | Input | Success |
|---|---|---|---|
| POST `/api/auth/login` | `login` | LoginRequest {email,password}; custom header/origin | 200 SessionResponse + Set-Cookie |
| GET `/api/auth/session` | `getSession` | Cookie | 200 SessionResponse |
| POST `/api/auth/logout` | `logout` | Cookie + CSRF | 200 LogoutResponse {signed_out:true}; absent/expired cookie can also be cleared after origin check |
| GET `/api/apps` | `listApps` | Optional organization_id, cursor, limit | 200 AppListResponse; only permitted apps |
| POST `/api/apps` | `createApp` | CreateAppRequest {organization_id,name,android_package,environment_name,backend_origins,login_origins} | 201 AppResponse |
| GET `/api/apps/{app_id}` | `getApp` | Authorized app | 200 AppResponse including environment/readiness |
| PATCH `/api/apps/{app_id}/environment` | `updateEnvironment` | UpdateEnvironmentRequest incl. expected_revision, names/origins/reference IDs | 200 EnvironmentResponse |
| POST `/api/apps/{app_id}/build-uploads` | `createBuildUpload` | CreateBuildUploadRequest {original_filename,expected_size} | 201 UploadResponse |
| GET `/api/apps/{app_id}/build-uploads/{upload_id}` | `getBuildUpload` | Authorized upload | 200 UploadResponse |
| PUT `/api/apps/{app_id}/build-uploads/{upload_id}/content` | `uploadBuildContent` | multipart/form-data with exactly one binary `file` part | 200 UploadResponse after sealing |
| POST `/api/apps/{app_id}/build-uploads/{upload_id}/complete` | `completeBuildUpload` | No JSON body; cookie + CSRF | 200 BuildResponse terminal; 202 BuildResponse if another attempt holds lease |
| GET `/api/apps/{app_id}/builds` | `listBuilds` | cursor, limit | 200 BuildListResponse |
| GET `/api/apps/{app_id}/builds/{build_id}` | `getBuild` | Authorized build | 200 BuildResponse |
| GET `/api/settings` | `getSettings` | Cookie | 200 SettingsResponse with access, configured limits, retention disclosure |

Relevant failures: 400 malformed data/cursor, 401 unauthenticated, 403 CSRF/role denial, 404 absent/foreign resource, 409 duplicate app/conflicting transfer/revision/incomplete upload, 410 expired upload, 413 too large, 415 wrong content type, 422 invalid field value, 429 bounded capacity, 500/503 internal/unavailable. List actual applicable codes per operation plus default ApiError. A well-formed completion returning a persisted `invalid` build uses 200: file validation is a domain result, not a transport failure.

`SessionResponse`: safe user identity, active organization memberships/roles, csrf_token, expires_at. No session/JWT/password hash. `UploadResponse`: id, app_id, original_filename, expected/actual_size, state, expires_at, build_id nullable, retry_after_seconds nullable; no storage path or presigned URL. `BuildResponse`: id/app_id, original_filename, byte_size, sha256, created_at, validation object, metadata nullable, readiness object, can_retry_validation. Metadata contains actual package_name, version_name nullable, version_code as canonical decimal string (including versionCodeMajor), min_sdk, target_sdk nullable, sorted unique native_abis and signature_verified. Integer sizes are bounded below JS safe-integer limits. Validation has state, reason code/message, started_at, completed_at, validator_version, intake_policy_version. Failure details are sanitized.

Keep named DTOs/enums in `crates/contracts/src/browser.rs` for this slice so the current CI path map continues to select web on every browser contract edit. Do not introduce browser submodules without updating its path globs. `ApiError` remains shared. Add schema-derived validators/serialization tests; no handwritten equivalent TS response shape. For multipart, document the binary property in Utoipa and use the generated SDK serializer; do not add a handwritten URL/fetch escape hatch.

## Storage rollout: Railway first, AWS S3 later

**Planned hosting choice:** Railway private Buckets for the initial hosted APKs; AWS S3 is the later migration destination. Local development and normal CI retain private local storage and require no cloud credentials. [Railway documents its buckets as private and S3-compatible](https://docs.railway.com/storage-buckets). This records the user's storage direction, not an already-provisioned bucket or a decision to migrate the entire application host.

Implement one narrow `ArtifactStore` interface: publish an immutable attempt key from a bounded file stream; materialize an object to bounded private scratch; stat; list an owned prefix; delete an unreferenced key. Use local and S3-compatible implementations. The inspected Loco 1.1.0 driver provides `loco_rs::storage::drivers::aws::with_credentials_and_endpoint`; combine it with `Storage::single`, `upload_stream` and `download_stream`. Enable its `storage_aws_s3` feature. Do not use whole-object collect/upload APIs for APKs. Endpoint, bucket and region are configured per internal backend ID; credentials are API-only Doppler process inputs. Proposed variables: ARTIFACT_S3_ENDPOINT, ARTIFACT_S3_BUCKET, ARTIFACT_S3_REGION, ARTIFACT_S3_ACCESS_KEY_ID and ARTIFACT_S3_SECRET_ACCESS_KEY. Tests never read real values. Never Debug-log the driver Credential struct.

Keep the same browser upload/session/finalization API across backends; the browser sends files through the authenticated API in this slice. No bucket credentials or public object URLs enter responses. Object keys use only server UUIDs, scoped by organization/app/upload/attempt. Store the logical backend ID with each upload/build so configuration changes cannot silently redirect existing records to a different bucket.

The numbered transfer sequence below describes the local backend. For Railway, replace its durable local rename step with this sequence:

1. Stream the request to a size-limited private scratch file and calculate size/SHA-256 as already specified. Scratch on the application disk is temporary; it is not the durable APK store.
2. Stream the completed scratch file to a fresh immutable attempt object key in the private bucket. Bound multipart chunk buffers/concurrency and abort failed multipart writes where supported. A storage error leaves the session retryable, never uploaded/accepted. Unknown outcomes are reconciled against that attempt key; fresh retries receive fresh keys. Apply transfer deadlines to both request intake and bucket publication; release/fence the attempt on expiry.
3. Only after object publication succeeds commit uploaded state, backend/key, size and checksum. Remove scratch on every exit. An object written before a failed DB commit is an orphan handled by cleanup. Object storage has no filesystem atomic rename assumption.
4. At finalization, stream the durable object back to private scratch with the actual-byte limit and recompute checksum. Validate these downloaded bytes, not an earlier local upload copy; persist the result only if they match. Bound materialization to 120 seconds. The existing 60-second inspection budget starts after download; the hosted validation lease is **240 seconds total** to cover both stages and persistence. Reclaim only after the total lease expires. Local inspection retains its 90-second lease.
5. Cleanup uses backend-aware references, owned prefixes and attempt fencing. Never delete a build-referenced object. Abandoned multipart uploads also need explicit cleanup/lifecycle configuration before hosted acceptance; do not assume a local file cleanup task covers them.

Add adapter tests for failed/uncertain publication, missing object, truncated download, checksum mismatch, private access and DB-commit orphan recovery. Normal tests use a local protocol double; label them accordingly. Before hosted acceptance, run an authorized small fixture upload/readback/finalization against a dedicated Railway prefix and verify its SHA-256 after discarding local scratch. That live check needs scoped credentials and is separate from secret-free CI; do not mark it passed from mock tests. Railway resource creation is not performed during planning.

**Later migration:** provision AWS S3 separately, pause new uploads/finalization/cleanup for a controlled cutover, inventory referenced objects, copy them preserving keys, and verify byte sizes plus full SHA-256 reads. Switch backend configuration only after every referenced object is verified; preserve Railway source objects for rollback. Re-enable writes and verify new uploads and old build reads on AWS. Storage transport is reusable, but copying data, validating API compatibility/access policy and switching configuration are real migration work—not just changing an endpoint. Build IDs, hashes, public APIs and dashboard behavior remain unchanged. AWS cutover implementation is outside spec 03.

## Upload and Validation State Machines

```text
Upload: pending → receiving → uploaded → finalized
        pending/receiving/uploaded → expired (only if not finalized)
        receiving → pending on failed attempt, guarded by attempt_id

Build: created as validating at first complete
       validating → validated | invalid | unsupported | error
       error or expired validating lease → validating on explicit retry
```

Defaults to implement/configure: 250 MiB APK byte limit, 30-minute upload TTL, four nonterminal uploads/app, two concurrent transfers/process, one validator/process. Transfer total deadline 5 minutes and idle deadline 30 seconds; lease 6 minutes. Validation total deadline 60 seconds and lease 90 seconds. API JSON body cap 64 KiB; multipart route has a scoped envelope cap of APK limit + 1 MiB and an independent file byte counter. Limit original filename to 255 characters; retain safe display text only and ignore it for storage/tool arguments.

1. Create upload after permission and quota checks in a transaction. Client expected_size is a declaration, not evidence.
2. Claim a pending upload with compare-and-set and a fresh attempt UUID. Stream `Multipart::next_field` / field chunks to `.private/artifacts/<org>/<app>/<upload>/<attempt>.part`; directories 0700, files 0600, create-new semantics. Reject extra parts and count actual bytes, including absent/incorrect Content-Length. Hash SHA-256 incrementally. On any stream error delete only this attempt's part and release state conditionally.
3. On full transfer, flush/sync and rename to an immutable attempt-specific sealed key on the same filesystem. Commit uploaded state/key/actual size/hash with an attempt_id fence. Both expected and actual size must agree; cap enforcement is based on actual bytes. A losing attempt cannot replace winning bytes. A crash after rename leaves an orphan, not an accepted build. PUT on an already-sealed upload returns 409; client GETs state and proceeds to complete, never overwrites it.
4. Complete locks the upload row (`TransactionTrait`, short `SELECT FOR UPDATE` via SeaORM). Check scope/expiry/sealed state, insert one build via UNIQUE upload_id, mark upload finalized and establish build validation attempt/lease. Commit before expensive file work. Two callers share build ID; active lease returns 202 with Retry-After. Terminal calls return the existing build unchanged.
5. Reopen only the server-owned sealed key; recompute hash and byte count to detect storage mutation. Perform bounded ZIP structural checks, reject encrypted entries, unsafe paths, duplicate names, overlapping payloads, unsupported compression, missing/duplicate binary manifest and truncated/CRC-corrupt entries. Limits: 100,000 entries, 1 GiB total expanded bytes, 250 MiB per entry and 1000:1 expansion ratio; budgets count real reads as well as ZIP declarations. Never extract entries into the filesystem. CRC checking must read bounded entries, not just open the central directory.
6. Run pinned Build Tools `aapt2 dump badging FILE`, `aapt2 dump xmltree FILE --file AndroidManifest.xml`, and `apksigner verify --verbose FILE`. Use absolute tool/file paths, argument arrays, cleared child environment with explicit minimal Java/tool requirements, fixed locale, piped output capped at 1 MiB/tool, deadline, and kill/reap on timeout/cancellation. Do not forward the API's Doppler environment. Parse only required fields using a bounded parser covered by fixtures, not whitespace splitting of version names. Reject split manifests/required splits; accept only a self-contained APK.
7. Require parsed package equality with app.android_package; derive actual version/min-SDK/target-SDK/native ABI and verify signature. Initial intake policy is explicitly a **candidate** API 35/x86_64 profile from spec 02: min_sdk > 35 or native libraries with no x86_64 support yields `unsupported` with a targeted rebuild message. An APK with no native libraries is ABI-neutral, not invalid. This metadata comparison does not check a phone. Persist policy/version so future qualification changes do not rewrite historical decisions.
8. Persist terminal state using build attempt_id fencing; old work cannot overwrite newer attempts. Missing tool, output-budget exhaustion, timeout or infrastructure failure yields `error`, with Retry validation; corrupt/signature/package failures yield `invalid`; unsupported bundle/profile yields `unsupported`. Validated requires all checks and complete required metadata. Partial metadata is allowed only for nonvalidated outcomes.
9. On browser request failure, GET upload/build state first. A stale validating lease becomes retryable in the response; POST complete reclaims it. Do not automatically accept or silently restart during a GET. Recovery runs the same immutable bytes; no device side effect exists. Expiry after upload.finalized does not invalidate the associated build or prevent validation retry.
10. Provide explicit `cleanup-app-uploads` maintenance task: reconcile stale transfer leases, expire uploads, remove their owned part/sealed files and unreferenced attempt orphans after TTL + grace, clean expired sessions/rate buckets. Recheck DB references and fencing before deletion. Never remove a key referenced by builds. Default dry-run; explicit apply. No automatic accepted-build retention/deletion in this phase.

Resource-limit checks in ZIP parsing must be cancellable/cooperative; timing out a `spawn_blocking` future does not stop its thread. Run decompression in bounded chunks with deadline checks and a process-wide permit; cap central-directory/entry allocation before it can consume arbitrary memory. Production hardening should isolate parsers from customer/runtime credentials; cleared process environment alone is not an OS sandbox.

## Readiness Semantics

The server returns separate build validation, install, backend, account, reset and cases facets. `install = not_checked`, qualified device profile = null, cases = not_configured in this slice. Backend/account/reset can show an operator-reported observation bound to an environment revision, explicitly labeled with actor/time. Missing secret reference means missing configuration; a present reference means configured, never verified. No automatic backend calls or secret dereference occur.

The UI may say “APK validated” and list unfinished prerequisites. Overall execution readiness remains false. Changes to environment/references invalidate previous observations. No button starts a device run, and no static metadata check emits a test pass/fail.

## Patterns to Mirror

For dashboard presentation, follow the UI framework/reuse mapping above. New feature components compose imported shadcn primitives; they do not introduce a parallel component system.

### NAMING_CONVENTION

Source: `crates/contracts/src/browser.rs:12–16`.

```rust
#[derive(Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Ok,
}
```

Use snake_case Rust modules/functions/JSON fields, PascalCase Rust DTOs/React components and camelCase operation IDs. New TS API adapter files stay lowercase (`auth.ts`, `apps.ts`, `builds.ts`); generated transport definitions are imported, not copied.

### ERROR_HANDLING

Source: `apps/api/src/controllers/health.rs:23–27`; extend the existing envelope with request_id.

```rust
Json(ApiError {
    code: "not_found".into(),
    message: "API route not found".into(),
    details: None,
})
```

Source: `apps/web/src/api/runtime.ts:23–24`.

```typescript
const parsed = zApiError.safeParse(error)
return new ApiClientError(response.status, parsed.success ? parsed.data : null)
```

### LOGGING_PATTERN

Source: `apps/api/config/production.yaml:4–8`.

```yaml
logger:
  enable: true
  pretty_backtrace: false
  level: info
  format: json
```

No existing domain logging snippet exists. The tracing fields/security rules above are the new narrow convention, not a pattern discovered in an existing service.

### REPOSITORY_PATTERN

Source: `apps/api/src/models/mod.rs:1–2`.

```rust
//! SeaORM entity/model extension location. Empty until product tables are added.
//! Transport DTOs stay in crates/contracts; ORM records are not the public API.
```

There is no existing CRUD/repository implementation. Use `models/_entities/<table>.rs` with SeaORM `DeriveEntityModel`, explicit `Relation`, `ActiveModelBehavior`; small query helpers in feature services. Import `EntityTrait`, `QueryFilter`, `ColumnTrait`, `ActiveModelTrait`, `TransactionTrait`, `QuerySelect`, `Set`. Never copy the health test's raw diagnostic SQL into domain data access.

### SERVICE_PATTERN

Source: `apps/api/src/app.rs:1–2`.

```rust
//! Connect this application to Loco's startup, routing and migration hooks.
//! Product rules belong in future feature services, not in these framework hooks.
```

Add `services/{auth,apps,uploads,apk_validation,readiness}.rs`; handlers own extraction/auth/status, services own transactions and transitions, storage adapter owns local filesystem or private S3-compatible object operations. Inject `SetupServices` through `AppContext.shared_store` in `Hooks::after_context`; installed Loco `src/app.rs:256–273,518–556` provides that seam. Keep test storage, clock and inspector injectable with explicit fake names. No generic repository abstraction.

### TEST_STRUCTURE

Source: `apps/api/tests/health.rs:29–31`.

```rust
request::<App, _, _>(|request, ctx| async move {
    let res = request.get(HEALTH_PATH).await;
    res.assert_status_ok();
```

Source: `apps/web/src/pages/Home.test.tsx:7–9`.

```tsx
function show(path = '/') {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } })
  return render(<QueryClientProvider client={client}><RouterProvider router={createMemoryRouter(routes, { initialEntries: [path] })} /></QueryClientProvider>)
```

Use unique UUID fixture scopes in shared `mobile_qa_test`; no whole-table truncation/drop helpers. For cookie tests use Loco `request_with_config` and `RequestConfigBuilder::save_cookies(true)` with trusted test scheme/origin, or explicitly pass response cookies. Simultaneous requests require separate cookie jars. Inject inspector doubles only for transition/error tests; a separate real-tool test is mandatory.

## Files to Change

Braces below enumerate concrete proposed files. Generated outputs are owned solely by `just types`.

| File / group | Action | Justification |
|---|---|---|
| `Cargo.toml`, `Cargo.lock`, `apps/api/Cargo.toml` | UPDATE | Enable Loco auth; add direct API dependencies and lock resolution |
| `apps/api/src/{lib.rs,app.rs,controllers/mod.rs,models/mod.rs,tasks/mod.rs}` | UPDATE | Register feature modules, hooks/tasks without losing markers |
| `apps/api/src/config.rs`, `apps/api/src/errors.rs` | CREATE | Typed setup configuration and uniform request-aware failures |
| `apps/api/src/middleware/{mod.rs,request_context.rs,session.rs,csrf.rs}` | CREATE | Request IDs, authenticated context, mutation protection |
| `apps/api/src/models/_entities/{mod.rs,users.rs,organizations.rs,memberships.rs,app_memberships.rs,sessions.rs,login_attempts.rs,apps.rs,environments.rs,secret_references.rs,environment_checks.rs,build_uploads.rs,builds.rs}` | CREATE | Explicit persistence models; no generator that runs checks during authoring |
| `apps/api/migration/src/{m20260912_000001_auth.rs,m20260912_000002_apps.rs,m20260912_000003_builds.rs}` | CREATE | Auth/access, app/environment, upload/build migrations |
| `apps/api/migration/src/lib.rs` | UPDATE | Register migrations in order |
| `apps/api/src/controllers/{auth.rs,apps.rs,build_uploads.rs,builds.rs,settings.rs,health.rs}` | CREATE/UPDATE | Real handlers and consistent errors |
| `apps/api/src/services/{mod.rs,auth.rs,apps.rs,uploads.rs,apk_validation.rs,readiness.rs}` | CREATE | Focused logic and injected clock/inspector seams |
| `apps/api/src/storage/{mod.rs,local.rs,s3.rs}` | CREATE | Private local/Railway writes, immutable keys and cleanup |
| `apps/api/src/tasks/{provision_pilot.rs,set_app_access.rs,cleanup_app_uploads.rs}` | CREATE | Operator provisioning, references/check observations, cleanup |
| `apps/api/config/{development,test,production}.yaml` | UPDATE | Auth cookie location, origin, storage and validation settings |
| `crates/contracts/src/browser.rs` | UPDATE | All browser DTOs, operations, security/errors |
| `contracts/browser.openapi.json`, `apps/web/src/api/generated/**` | GENERATE | Authoritative browser artifacts |
| `apps/web/src/api/{runtime.ts,auth.ts,apps.ts,builds.ts,settings.ts}` | UPDATE/CREATE | Generated SDK adapters and expected-status/Zod validation |
| `apps/web/src/{App.tsx,routes.tsx,main.tsx}` | UPDATE | Protected shell, durable selection, cache clearing |
| `apps/web/{components.json,package.json,pnpm-lock.yaml}`, `apps/web/src/index.css` | UPDATE | shadcn setup, required dependencies and theme tokens |
| `apps/web/src/components/AppSidebar.tsx`, `apps/web/src/hooks/use-mobile.ts`, `apps/web/src/test/setup.ts` | CREATE/UPDATE | Adapt upstream sidebar block/hook and required DOM-test shims |
| `apps/web/src/components/ui/{button.tsx,card.tsx,sidebar.tsx,sheet.tsx,tooltip.tsx,separator.tsx,input.tsx,label.tsx,textarea.tsx,select.tsx,dropdown-menu.tsx,table.tsx,badge.tsx,skeleton.tsx,alert.tsx}` | IMPORT/UPDATE | Reuse reviewed upstream primitives and required block dependencies |
| `apps/web/src/pages/{SignIn.tsx,Apps.tsx,AppDetail.tsx,Settings.tsx,Home.tsx}` | CREATE/UPDATE | Actual onboarding; remove unused Home if replaced entirely |
| `apps/web/src/components/{RequireSession.tsx,AppForm.tsx,ApkUpload.tsx,BuildStatus.tsx,EnvironmentForm.tsx}` | CREATE | Focused accessible controls and session boundary |
| `apps/api/tests/{health.rs,auth.rs,apps.rs,build_uploads.rs,apk_tools.rs,storage_s3.rs,support/mod.rs}` | UPDATE/CREATE | Route agreement, tenant isolation and real APK checks |
| `apps/api/tests/fixtures/apk/{AndroidManifest.xml,README.md}` | CREATE | Controlled input source/provenance; no APK binary or signing key in Git |
| `apps/web/src/pages/{Home.test.tsx,SignIn.test.tsx,AppDetail.test.tsx}`, `apps/web/src/api/{transport.test.ts,builds.test.ts}`, `apps/web/src/test/contracts.compile.ts` | UPDATE/CREATE | Auth/form/reload/error/SDK and static contract coverage |
| `scripts/apk_fixtures.py`, `scripts/app_setup_smoke.py`, `justfile`, `.github/workflows/ci.yaml` | CREATE/UPDATE | Deterministic real-tool fixture generation, HTTP smoke and affected CI |
| `docs/architect/{system.md,contracts.md,environment.md,development.md,dependencies.md,decisions.md,status.md}` | UPDATE | Implemented design/config, dependencies and evidence after execution |
| `docs/architect/implementation/03-app-setup-and-ui-backend.md`, `AGENTS.md` | UPDATE | Link plan; later reconcile foundation-only no-auth sentence with implemented phase |

Do not alter `apps/mobile-worker`, worker shapes/fixtures, parent sources or global configuration. Existing `apps/api/src/workers` remains a scaffold; synchronous APK validation does not introduce the future worker protocol.

### Dependency delta

Keep framework/frontend versions. Enable Loco `auth` and `storage_aws_s3` in workspace features. Promote existing SeaORM 2.0 to API dependencies with PostgreSQL/Tokio-Rustls features; add direct serde derive, uuid v4/serde, chrono serde, tracing, sha2, url, subtle, and axum-extra cookie feature compatible with Axum 0.8. Enable Axum multipart; Tokio fs/io-util/process/sync/time; add zip 8.6.0 with only required stored/deflate support and no encryption. Use tempfile for tests. Reuse compatible lockfile versions for already-present crates; resolve new direct dependencies through Cargo, never edit lockfile manually. For the browser, add the Radix primitive packages, Lucide icons and animation dependency actually imported by the reviewed shadcn components. Follow one selected registry revision’s dependency declarations, record provenance and resolve with the existing pnpm version. Reuse class-variance-authority, clsx and tailwind-merge. No blanket upgrades to React/Vite/Router/Query/Zod, second UI kit, chart package or table engine.

Pin Android Build Tools **36.0.0**, platform **android-35** for fixture linking, and JDK **17** for the fixture/signature process. Tools are external runtime dependencies; package revision/output goes into validator_version. Use explicit SDK-local absolute paths under ignored `.private/android-sdk` in local setup, or a read-only supplied SDK root; never modify the user's global SDK config. Do not auto-download/accept SDK licenses at API startup or inside tests. Fixture preparation/CI provisioning is an explicit setup action before verification. This toolchain pin is independent of the candidate target device's API 35.

## Step-by-Step Tasks

Each task's VALIDATE entry specifies evidence to implement and collect in Task 14, not permission to run an intermediate check.

### Task 1: Establish scoped configuration and error/contract foundations

- **ACTION:** Update manifests, import the dashboard UI kit and introduce config/request-error plumbing.
- **IMPLEMENT (UI):** Import the reviewed official sidebar block/components into the existing shadcn setup; add their dependencies and semantic tokens. Preserve Button defaults, Card heading semantics and licenses. Complete source authoring without triggering lifecycle checks.
- **IMPLEMENT:** Add dependencies above; typed settings and `SetupServices` wiring; `ApiFailure`, server request IDs, normalized extractor/fallback errors and ApiError.request_id. Config includes limits, origin, local root, absolute tool paths and validator pin. Keep API liveness public. Clarify in AGENTS that the original no-auth restriction described phase 01 and spec 03 now owns auth; retain other invariants.
- **MIRROR:** ERROR_HANDLING, SERVICE_PATTERN; `health.rs` fallbacks.
- **IMPORTS:** `axum::{http, response::IntoResponse, Json}`, `loco_rs::app::{AppContext, Hooks}`, `mobile_qa_contracts::browser::ApiError`, `serde::Deserialize`, `tracing`, `uuid::Uuid`.
- **GOTCHA:** `HOST` is production public origin; dev Vite origin differs from API host. Invalid/missing tools fail upload/finalization actionably, not liveness. Keep secret-free smoke working.
- **VALIDATE:** Tests for configured origins/cookie flags, malformed JSON and 404/405 envelope/request ID; update original health agreement without assuming paths.len()==1.

### Task 2: Persist pilot users, organizations and revocable sessions

- **ACTION:** Add auth migration, entities, service and operator provisioning task.
- **IMPLEMENT:** users/org memberships/sessions/login buckets; Loco hash/JWT integration, rate/concurrency controls and DB revocation. Provision one org/account via explicit task with no default password. Read password from hidden TTY input or protected stdin, never CLI argv/logs; no startup auto-seeding. Existing user update/password reset must be explicit and scoped.
- **MIRROR:** REPOSITORY_PATTERN, migration registry marker, `App::register_tasks`.
- **IMPORTS:** `loco_rs::{hash, auth::jwt::JWT, task}`, `sea_orm::{EntityTrait, TransactionTrait, Set}`, `tokio::task::spawn_blocking`.
- **GOTCHA:** Loco signing secret is base64; no hardcoded development signing secret. Generate ephemeral keys in process for dev/test when runtime secret absent; production requires injected `JWT_SECRET`. Restart invalidates dev cookies and is acceptable. Tests create synthetic users through helpers.
- **VALIDATE:** Migration constraints; hashed-password persistence, generic invalid login, limits, disabled user, expiry and revoked session; no secrets in logs/output.

### Task 3: Finish sign-in through the protected UI

- **ACTION:** Implement auth endpoints, middleware and sign-in/shell consumers.
- **IMPLEMENT:** Session/JWT/cookie contract above; origin/CSRF; generated login/session/logout adapters; `RequireSession` loading/error/authenticated states; safe returnTo; no protected query before session success. Sign-out/401 cancels requests and clears all customer query caches. Assemble sign-in with shadcn Card/Input/Label/Button. Adapt AppSidebar to App/Tests/Runs/Settings NavLinks; mount SidebarProvider/SidebarInset/SidebarTrigger around Outlet and use DropdownMenu for account/sign-out. Preserve the skip link and main landmark; use the upstream mobile sheet behavior.
- **MIRROR:** `api/runtime.ts`, `queries.ts`, Home.test memory-router setup.
- **IMPORTS:** `loco_rs::controller::extractor::auth::extract_jwt_from_request_parts`, `axum_extra::extract::cookie`, generated auth SDK/Zod, `useQuery`, `useMutation`, `useQueryClient`, `Navigate`, `Outlet`; `@/components/ui/{sidebar,dropdown-menu,card,input,label,button}` and `@/components/AppSidebar`.
- **GOTCHA:** Do not turn a session-network/500/schema error into “signed out”; only trusted 401 triggers it. StrictMode must not submit login twice. Reject CSRF on login as well as later mutations.
- **VALIDATE:** Cookie flags, tamper/expiry/logout, cookie-only behavior, CSRF rejection, cache isolation and accessible sign-in errors; preserve health tests separately. Cover active sidebar links, mobile trigger, menu keyboard interaction and focus return using actual kit components. Add only required matchMedia/ResizeObserver test shims; do not mock away the controls.

### Task 4: Create the app/environment persistence slice

- **ACTION:** Add app/access/environment migrations, entities and services.
- **IMPLEMENT:** Schema/permissions above; transactional app + environment + creator grant. List only granted apps; detail includes environment/readiness. Enforce unique org/package and full origin validation. Add CreateApp/App/List/Environment DTOs and operations before consumer code.
- **MIRROR:** SERVICE_PATTERN, pure Utoipa declaration and real route agreement.
- **IMPORTS:** `sea_orm::{ColumnTrait, QueryFilter, QueryOrder, TransactionTrait}`, `url::Url`, `mobile_qa_contracts::browser::*` (prefer explicit names in implementation).
- **GOTCHA:** Never accept arbitrary org_id as proof of membership. Same-org users without an app grant must not obtain access through nested IDs.
- **VALIDATE:** API tests cover create/reload/list/cursor, duplicate package, invalid origins and cross-org/same-org grant denial.

### Task 5: Finish create-app and persisted detail UI

- **ACTION:** Add app list/form/detail routes and generated query/mutation adapters.
- **IMPLEMENT:** `/` redirects to `/apps`; `/apps` lists apps and create form; `/apps/:app_id` displays persisted details. Query keys include session user/org and app IDs. Select organization from verified session membership when user has multiple memberships. Mutation invalidates only relevant app/list keys. Empty list/error/retry are explicit.
- **MIRROR:** Imported shadcn Card/Button/Input/Label/Select/Textarea/Alert/Skeleton, Home loading/error pattern and generated response validation. AppForm composes the kit rather than hand-building equivalent controls.
- **IMPORTS:** generated createApp/listApps/getApp + Zod; `react-router` params/navigation; TanStack query/mutation; existing `components/ui`.
- **GOTCHA:** URL selection must be authorized by server; avoid mutable singleton “current app” in the API client. Replace foundation Home and adapt its tests to preserve separate API liveness coverage.
- **VALIDATE:** Form constraints, actual SDK path/body, reload detail, permission error, stable selection and keyboard behavior via DOM tests; rendered acceptance later remains gated.

### Task 6: Implement private upload sessions and bounded transfer

- **ACTION:** Add upload/build migration, local and Railway storage adapters, and session/content handlers.
- **IMPLEMENT:** Upload state machine, quotas/deadlines, multipart exactly-one-file contract, streaming counters/hash, attempt fencing, sealed keys and GET recovery. Implement the hosted storage branch below using Loco’s S3 driver; durable Railway publication precedes uploaded state. Add backend identity to stored object references. Add constants/DTOs and route declarations for creation/status/content. Use server-generated directory names; permit no raw artifact route.
- **MIRROR:** Typed handler/status pattern; SeaORM transaction service boundary; owned cleanup in `scripts/runtime.py` as process-ownership guidance only.
- **IMPORTS:** `axum::extract::{Multipart, Path, State}`, `tokio::{fs, io::AsyncWriteExt}`, `sha2::{Digest, Sha256}`, `std::path::PathBuf`, SeaORM traits; `loco_rs::storage::{Storage, drivers::aws, stream::BytesStream}` for the S3 adapter.
- **GOTCHA:** Auth and origin checks happen before consuming large body data. Do not globally disable body limits. File flush/rename and DB commit need orphan handling; a request cancellation cannot delete another attempt.
- **VALIDATE:** Exact/over-limit, missing or lying Content-Length, interrupted/duplicate transfer, expiry, extra multipart field, disk error, symlink/path attack and tenant ownership tests.

### Task 7: Implement real APK inspection and idempotent finalization

- **ACTION:** Add bounded inspector and complete/build endpoints.
- **IMPLEMENT:** Persist validating build first, unique upload_id, short transaction locks and reclaimable lease; materialize the authoritative stored object through its backend into bounded private scratch; hash/ZIP/metadata/signature checks and intake policy; fenced terminal updates; explicit retry. Implement `ApkInspector` trait with RealApkInspector and test-only named fake, injected via SetupServices. Expose list/detail/status with no private paths.
- **MIRROR:** SERVICE_PATTERN, transport semantic validation in `crates/contracts/tests/contracts.rs`.
- **IMPORTS:** `zip::ZipArchive`, `std::io::Read`, `tokio::{process::Command,time,sync::Semaphore}`, `sha2`, `sea_orm::{TransactionTrait, QuerySelect}`.
- **GOTCHA:** Kill/reap child processes and bound output; parsing a ZIP header is not APK validation. ARM-only is unsupported for the candidate policy, not corrupt; ABI-neutral is allowed. Cancellation must not leave permanent validating state.
- **VALIDATE:** Known metadata/hash/signature, package mismatch, split/unsigned/corrupt files, policy incompatibility, timeout/missing tool, duplicate concurrent complete, DB/file crash windows and stale-fence rejection.

### Task 8: Finish upload/build-history UI with server recovery

- **ACTION:** Add ApkUpload and BuildStatus, generated upload/build adapters and history.
- **IMPLEMENT:** File choose/client limit hint → create session → generated multipart SDK transfer → explicit complete. Save active upload ID in URL query while pending; selected build ID in URL after completion. Poll only active states every two seconds with bounded backoff on failures; stop on terminal/logout/unmount. Server `can_retry_validation` controls retry. For upload/complete network ambiguity, GET state before another mutation. Use mutation retry:false.
- **MIRROR:** Expected status `satisfies keyof ...Responses` and single generated Zod parse per success boundary, existing retry controls.
- **IMPORTS:** generated upload/complete/list/get SDK + Zod; TanStack query/mutation; `useSearchParams`; shadcn Input/Button/Card/Table/Badge/Alert/Skeleton. Build history uses the kit’s Table; file selection uses Input type=file.
- **GOTCHA:** Generated fetch offers no truthful upload percentage by default; use phase text/indeterminate progress. 202 is accepted only where declared. Never recover file bytes from localStorage or accidentally reuse another app's upload ID.
- **VALIDATE:** File selection, cancel/unmount, failed transfer, lost complete response, reload pending/finalized build, unsupported/invalid/error messages, foreign/stale URL IDs, no duplicate finalization side effects.

### Task 9: Add environment access references and operator observations

- **ACTION:** Implement revision-safe environment update and explicit operator access task.
- **IMPLEMENT:** Environment/reference/check tables from Task 4, private locator writes through `set-app-access`; masked public metadata; same-app reference authorization. Operator task records backend/account/reset observation, actor and environment revision, without fetching secrets or making backend requests. Add safe observation enums/DTOs. UI can select existing references and display check provenance.
- **MIRROR:** Apps scoped service/DTO pattern; commenting rules for nonimplemented secret resolution.
- **IMPORTS:** SeaORM transaction/conditional update; generated updateEnvironment and schemas; existing form components.
- **GOTCHA:** Do not let customers supply arbitrary Doppler names and then claim verified access. A configured reference and an operator-reported check remain different states. Masking is server-side, not CSS.
- **VALIDATE:** Cross-app references denied, concurrent revision conflict, stale checks after edit, no locator/value in response/logs and operator role enforcement.

### Task 10: Finish honest readiness and Settings

- **ACTION:** Centralize server readiness derivation and show it on App/Settings.
- **IMPLEMENT:** Facets above, current build selection, revision-specific observations, unavailable device explanation and missing case setup. Settings serves limits/access and accurate accepted-build retention disclosure. No fake run controls or ready badge. Attach human next actions to invalid/unsupported/infra-error reasons.
- **MIRROR:** Existing honest Placeholder boundary; named contract enums and Zod.
- **IMPORTS:** generated readiness/settings types, `getSettings`, generated response validators.
- **GOTCHA:** Do not expose a writable ready boolean. Device success cannot be inferred from installed SDK tools, health, or policy compatibility.
- **VALIDATE:** Truth table with valid APK + no device, missing account/reset, stale observation, unsupported ABI and API/network failure; no combination yields ready-to-run in this slice.

### Task 11: Add upload recovery maintenance and operational setup

- **ACTION:** Implement scoped cleanup task, documented commands and runtime configuration.
- **IMPLEMENT:** Dry-run/apply cleanup behavior above; add just recipe wrappers for provision-pilot/set-app-access/cleanup without secret argv. Document development ephemeral auth and production JWT_SECRET via API-only Doppler; tool paths/storage root/limits are nonsecret settings. Setup instructions use local SDK directory and explicit version pins. Keep ordinary startup free of provisioning/reset actions.
- **MIRROR:** `App::register_tasks`, justfile explicit recipes, runtime owned-process cleanup.
- **IMPORTS:** `loco_rs::task::{Task, TaskInfo, Vars}`, SeaORM/clock/storage services.
- **GOTCHA:** Cleanup never deletes any build-referenced file or active lease; expired upload TTL does not expire a finalized build. No `.env` files, downloaded secrets or global SDK/Doppler changes.
- **VALIDATE:** Dry-run has no writes, apply touches only eligible keys/rows, concurrent lease renewal survives cleanup, provisioning cannot overwrite an existing account unintentionally.

### Task 12: Author acceptance tests and controlled APK fixture generation

- **ACTION:** Complete all test sources and offline acceptance tooling before running anything.
- **IMPLEMENT:** API/DOM/SDK test matrix below. Add `scripts/apk_fixtures.py` to produce a small signed, resource-only APK from tracked synthetic AndroidManifest (`android:hasCode=false`), using AAPT2 link with android-35/android.jar, then ephemeral signing key in a temporary/private directory. Produce mismatched-package, unsupported-min-SDK, tampered-signature and corrupt variants. No app/customer binaries/signing keys in Git. Real `apk_tools` test must fail with actionable setup error if tools absent, not silently skip. Fake-inspector tests prove lifecycle only. Add generated compile assertions for new DTO fields/binary input/statuses.
- **MIRROR:** TEST_STRUCTURE, existing fetch-only mocking and fixture provenance comments.
- **IMPORTS:** `loco_rs::testing::prelude`, `tempfile`, Vitest/Testing Library; Python standard library subprocess/tempfile/pathlib.
- **GOTCHA:** Resource-only synthetic APK validates intake but proves no app launch. ABI parser fixtures can use synthetic entry lists and must be labeled synthetic; do not present fake .so bytes as executable compatibility evidence. Use session/task-local fixture credentials; never embed a real account password.
- **VALIDATE:** Meaningful state/security assertions and mandatory real-tool route test; DB fixtures isolated by unique IDs and cleanup limited to owned data.

### Task 13: Integrate CI, HTTP smoke and owning documentation

- **ACTION:** Finish workflow, smoke and canonical documentation edits.
- **IMPLEMENT:** CI API job explicitly provisions pinned Android tools/JDK, builds synthetic fixtures, then runs real APK tests; no emulator/model/Doppler. Extend affected paths for new fixture/smoke scripts; all shared auth/config changes select full API tests. New `just smoke-app-setup` calls `scripts/app_setup_smoke.py`: isolated test DB/app fixtures and temporary storage, synthetic key/password held in memory, sign-in/create/upload/finalize/GET from a new session, separate-org denial. Reuse runtime process ownership; do not alter a user's real app or pilot account. Update system/contracts/environment/dependencies/decisions/spec 03; status only with collected evidence.
- **MIRROR:** Existing `.github/workflows/ci.yaml`, runtime database/services ownership and docs authority.
- **IMPORTS:** Python standard library HTTP/cookiejar/subprocess; existing runtime helpers if safe to reuse with explicit environment/ports.
- **GOTCHA:** HTTP smoke is not rendered UI acceptance. Preserve the browser policy denial and spec 01's open gate. Do not add an automated browser route around that denial. Record actual finalization timing, not a promised target.
- **VALIDATE:** CI path coverage includes scripts and real tool prerequisite; smoke process cleanup on failure; docs distinguish operator observation/static validation/device execution.

### Task 14: Generate and verify the completed implementation once

- **ACTION:** Resolve remaining lock changes, run generation, then final checks in the order below.
- **IMPLEMENT:** All source/test/config/docs ready; generate `just types` once; format owned Rust; run contract/web/API/build and smoke. Batch fixes and rerun only failed or invalidated checks. Review resulting diff including generated changes and all scaffold markers.
- **MIRROR:** `docs/architect/development.md` and scripts/contracts content-only synchronization.
- **IMPORTS:** N/A — command phase.
- **GOTCHA:** Generated source edits can invalidate multiple consumers. Worker source/contracts should have no diff. Browser/manual acceptance is gated, not a silent pass. Do not run a full unrelated worker suite solely because this is a feature plan.
- **VALIDATE:** All executable checks below pass; record exact remaining acceptance gates. Do not mark spec 03 fully accepted until allowed browser smoke is completed.

## Testing Strategy

### Unit, Route and UI Tests

| Test | Input | Expected output | Edge case? |
|---|---|---|---|
| Login/session/logout | Provisioned user and correct password | Cookie + safe session; reload works; logout cookie replay gives 401 | No |
| Auth failure | Bad signature, expired/revoked session, disabled user | Structured 401; no protected data | Yes |
| Login throttling | Exceed per-email/network window or hash concurrency | 429 before expensive work; generic failures | Yes |
| CSRF | Cross-origin login; missing/wrong CSRF on each mutation | Rejected before state/file change | Yes |
| App transaction | Valid create request | App, environment, grant committed together | No |
| Scope enforcement | Foreign org/app/upload/build/reference; same-org ungranted user | No metadata or artifact disclosure; 404 | Yes |
| Schema/route agreement | Every registered operation | Correct method/path/input/security/status/DTO; errors match generated schemas | No |
| Boundary validation | Malformed JSON/multipart/UUID, unexpected 204/nonJSON success | Structured API rejection or client validation error | Yes |
| Bounded transfer | Exact max, max+1, missing Content-Length, extra parts | Exact max allowed, excess rejected, no memory-buffered APK | Yes |
| Interrupted transfer | Disconnect/disk failure/stale attempt | No accepted build; retryable/expired upload; no winning-file deletion | Yes |
| Real APK route | Generated signed APK through real transfer + completion | Stored byte count/hash/package/version match independently known fixture | No |
| Invalid APK | Corrupt ZIP/binary manifest/signature/package mismatch | Persist invalid with actionable reason | Yes |
| Unsupported intake | Split set, min SDK too high, ARM-only metadata | Unsupported reason; no device claim | Yes |
| ABI-neutral | No native libraries | Validated when other checks pass | Yes |
| Tool failure | Absent executable, timeout, bad/oversized output | Persist error; retry available; no false invalid/pass | Yes |
| Exactly one build | Concurrent complete and lost response retry | Same build ID; one terminal outcome and immutable bytes | Yes |
| Crash/recovery | After seal/before DB; after build creation/before terminal write | Orphan cleanup or fenced lease retry; never autoaccept | Yes |
| Environment revision | Edit after operator observation | Old check stale; optimistic conflict prevents lost update | Yes |
| Sensitive fields | All ordinary responses/logs | No JWT/password hash/secret locator/storage key | Yes |
| Readiness truth table | Validated APK without worker/cases | Device not checked; execution_ready=false | Yes |
| Client uncertainty | Transfer/complete response lost | GET authoritative state before retry mutation | Yes |
| Reload/cache | Navigate direct URL; logout and another user login | Persisted selection; no previous user's cached data | Yes |
| Cleanup | Expired nonbuild objects + active/build objects | Delete only eligible owned files on apply | Yes |

### Edge Cases Checklist

- [ ] Empty/whitespace fields; invalid origins; duplicate package.
- [ ] Oversize/truncated content; extension/MIME spoofing; malicious filename.
- [ ] Archive duplicates/path traversal/overlap/CRC failure/expansion budget.
- [ ] Session expires during upload; authorization revoked before complete.
- [ ] Two organizations and two members of the same organization.
- [ ] Concurrent transfer/finalization/retry/environment edit/cleanup.
- [ ] Network loss before and after durable commit; reload while validating.
- [ ] Infrastructure unavailable, disk full, tool cancellation and process restart.
- [ ] New environment revision invalidates previous observations.
- [ ] Keyboard, accessible status/error text and no invented progress/readiness.

## Validation Commands

These are **implementation-time** commands, not commands to execute for this documentation-only plan. All authoring must be finished first. Start from repository root; existing recipes handle API cwd and isolated PostgreSQL. Install/provide explicit Android/JDK dependencies first through documented scoped setup. Tests must not fetch tools themselves.

### Generation and Static Analysis

```sh
just types
cargo fmt --all
python3 scripts/apk_fixtures.py
just check-contracts
just check-web
just check-api
```

EXPECT: No drift after generation, strict TypeScript/ESLint/Vitest pass; Rust formatting/Clippy/workspace tests pass with real local PostgreSQL and mandatory real APK tooling. The fixture script (created in Task 12) writes only ignored `.private/test-apks` inputs and an expected-metadata manifest, discovers tools from explicit project settings, and fails if the pinned tools are absent; tests use that path rather than downloading files. `just check-api` already runs Rust unit/route/contract tests; do not repeat them as another full suite.

### Targeted Unit/Route Reruns (only for failed/invalidated checks)

```sh
cargo test -p mobile-qa --test auth --locked
cargo test -p mobile-qa --test apps --locked
cargo test -p mobile-qa --test build_uploads --locked
cargo test -p mobile-qa --test apk_tools --locked
pnpm --dir apps/web test -- src/pages/AppDetail.test.tsx src/api/builds.test.ts
```

EXPECT: Named affected tests pass. Direct Rust commands require the existing isolated test DB to be running; prefer the normal recipe for the initial full validation. Do not reset/create/drop databases to make tests pass.

### Build and Runtime

```sh
just build
just smoke
just smoke-app-setup
```

EXPECT: Web/Cargo build; prior foundation smoke still works without dist dependency or Doppler; new HTTP acceptance proves persisted metadata, cookie flow and tenant denial. `smoke-app-setup` is created by Task 13, not an existing command today.

### Database Validation

The integration harness runs registered migrations via Loco on `mobile_qa_test`. Assert migration bookkeeping and new FK/unique/CHECK constraints in route tests. Boot twice without resetting DB and verify existing scoped app/build data persists. No down/drop/truncate command is part of routine validation. Migration rollback logic is reviewed and may only be tested in a separately disposable database explicitly designated for that purpose.

### Dependency Review

```sh
pnpm --dir apps/web audit
git diff --check
git diff --stat
```

Review Rust dependency advisories with available project tooling before customer use; do not silently install a global auditor. No Python dependency delta is expected. If committing later, follow the repository's audit requirement; no commit/push is requested here.

### Browser Validation / Manual Acceptance

`just dev` is the existing developer launch command (API-only Doppler, Vite HMR). Rendered access currently has an admin-policy denial. Do not use alternate browser/Playwright/HTTP tools to evade that restriction. The existing/new HTTP tests above are independent automated integration evidence, not a substitute for rendered acceptance.

Once normal browser access is permitted:

- [ ] Sign in using a provisioned synthetic/local account, including an incorrect-password attempt.
- [ ] Create app/environment through keyboard-accessible form and reload its URL.
- [ ] Upload a known APK, see actual metadata/hash and terminal validation status.
- [ ] Reload while validating and after terminal status; recover a lost completion response.
- [ ] Upload corrupt/mismatched/unsupported APK and verify actionable, distinct states.
- [ ] Confirm device/account/tests are not falsely marked ready.
- [ ] Sign out, use Back/direct URL, then sign in as another organization and confirm isolation.
- [ ] Exercise visible network-error/Retry states and original shell keyboard/HMR gate.

## Acceptance Criteria

- [ ] Operator-provisioned login uses framework hashing and revocable server-checked cookies.
- [ ] Origin/CSRF/session expiry/logout and org/app permissions enforced by API tests.
- [ ] App/environment and creator grant persist atomically and survive reload.
- [ ] Actual uploaded bytes are privately stored, size/hash measured, metadata parsed and signature checked.
- [ ] Local and S3-compatible adapters satisfy the same storage invariants; an authorized Railway round-trip is recorded before hosted acceptance.
- [ ] Concurrent/retried finalization creates one immutable build per upload.
- [ ] Corrupt, invalid-package, unsupported-policy, infrastructure-error and expired-upload states are distinct.
- [ ] All public success/error bodies pass generated runtime schemas; route/OpenAPI agreement includes every operation.
- [ ] Device preflight stays not checked and overall execution readiness stays false without qualified evidence.
- [ ] Reference presence/operator observations are masked, scoped, revision-aware and truthfully labeled.
- [ ] Real-tool API acceptance and HTTP persisted workflow smoke pass; fake tests are identified as fake.
- [ ] All affected static/test/build checks pass; source/generated drift is clean.
- [ ] Rendered sign-in → create → upload → persisted status smoke completes when policy permits; otherwise gate remains open.

## Completion Checklist

- [ ] Dashboard reuses the official shadcn sidebar block and primitives; custom code composes product workflows.
- [ ] Source follows named conventions and all scaffold markers remain.
- [ ] All 14 tasks complete; no handwritten consumer DTOs or fetch URLs.
- [ ] Error/logging semantics consistent, with no secret/artifact leakage.
- [ ] Migrations, transactions, leases/fences and cleanup invariants covered.
- [ ] Complete authoring precedes one generation/verification phase.
- [ ] Canonical docs updated alongside material changes; status contains observed evidence only.
- [ ] No worker/device/model/scheduling or global configuration scope added.
- [ ] Implementation uses the fixed decisions above without needing new product decisions.

## Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| First domain/auth slice spans many surfaces | High | Large integration batch | Ordered vertical authoring, shared API inventory, one integration owner and final check phase |
| Session correctness accidentally reduced to JWT validity | Medium | Unauthorized/revoked access | Session row and membership lookup on every request; cookie-only extraction and replay tests |
| File/DB crash windows and duplicate finalization | High | Lost/stuck/incorrect builds | Immutable attempt keys, unique upload/build linkage, short locks, fences and recovery tests |
| Tools missing or parser behavior changes | Medium | Validation unavailable | Explicit version pins, real synthetic APK tests and persisted infrastructure-error states |
| Hostile APK resource consumption/parser vulnerability | Medium | API availability/security | Strict transfer/archive/process limits, minimal subprocess environment, production isolation gate |
| Static validation mistaken for installation success | High | Misleading customer promise | Separate facets; candidate policy named; no execution-ready state without later evidence |
| Browser policy gate unresolved | Known | Rendered acceptance incomplete | Preserve pending gate; provide allowed source/API/DOM evidence without bypass |
| Local storage fills | Medium | Upload failure | Quotas, expired-upload cleanup, actionably fail disk errors; accepted-build retention remains a later operational requirement |

## Notes and Readiness Assessment

Planning reconciles the user's scope with existing canonical decisions; no further product decision is required to start implementation. Limits and the candidate intake profile above are proposed implementation defaults, not production capacity/qualification evidence. Real cloud/device work, customer secret resolution and accepted-build retention remain explicitly deferred.

The source request was free-form, not a PRD path. Link this plan from spec 03 but keep its status **planned**; do not advance any milestone to in-progress/implemented simply because planning is complete. All links/snippets are derived from inspected source or primary documentation; no service/repository example was invented and presented as existing code.

**Confidence: 8/10.** The architecture and interfaces are defined; primary uncertainty is integrating the new authentication/storage/tool boundaries in one complete authoring batch and the unresolved rendered-browser gate.

Executed through prp-implement; final evidence is in `../../reports/03-app-setup-report.md`.

## Implementation authoring checkpoint

Tasks 1–13 coding complete: migration/entities/auth; session routes/contracts; tenant
apps/environments; shadcn dashboard; private storage/intake; real validation/fences;
operator/cleanup tasks; integration/DOM tests; CI/fixtures/smoke/docs. Task 14 begins
with generation after this complete authoring checkpoint. Differences: one coherent
initial schema migration, thin routes grouped by app setup, cohesive service modules;
GAN review is explicitly source-only because of the existing browser denial.


## Final implementation checkpoint

Tasks 1–14 completed for local implementation. The final API/DOM/contract/build and
HTTP smoke checks pass; signed APK metadata survives a real API restart. The external
acceptance gates (rendered browser, Railway and device qualification) remain open as
specified above. The Rust dependency audit reports one unpatched transitive RSA issue,
tracked in canonical dependencies.md; no advisory is suppressed. The report records
all deviations, failures resolved, test evidence and remaining gates. Unchecked test
ideas in this planning snapshot are not claims of executed exhaustive fault injection.
