# Transport contracts

Status: implemented foundation. Component ownership lives in [system architecture](system.md).
The source of wire definitions is [crates/contracts](../../crates/contracts); generated
outputs are derived artifacts, never an independent specification. Product/job protocols
beyond health and local fixtures remain planned in [spec 04](implementation/04-execution-and-reports.md).

## Browser boundary

crates/contracts owns serde types and Utoipa OpenAPI declarations. It has no Loco,
Axum, DB or runtime startup dependency. The explicit lightweight exporter emits
contracts/browser.openapi.json; pinned Hey API reads this local file and generates
SDK methods, TypeScript types and Zod validators into apps/web/src/api/generated.
There are no handwritten endpoint URLs/fetch methods/response field parsers in the
browser, and no remaining ts-rs export pipeline.

Because the API preserves Loco route registration, the pure declaration and actual
handler are connected by shared HEALTH_PATH plus a real integration agreement test.
That test checks the declared GET method, no input, 200 HealthResponse/default ApiError
schemas against the registered typed Json handlers, including actual405 error JSON.
An unknown API path returns structured404; it cannot become a successful SPA fallback.
This is tested agreement, not a claim of automatic compile-time route extraction.
Add matching declaration/typed-handler/agreement coverage whenever an endpoint changes.

The query adapter calls generated getHealth, checks expected status with a
`200 satisfies keyof GetHealthResponses` compiler binding, and invokes generated
zHealthResponse exactly once. This covers204/empty/nonJSON responses, which the pinned
fetch client's responseValidator path can skip. Generated request validators run as
applicable. A tiny error interceptor invokes generated zApiError.safeParse for non2xx
responses; malformed or nonJSON errors become ApiClientError with no trusted body.
Network errors remain network errors. No unchecked transport casts or handwritten
schema shapes appear in owned application code. Generated fetch internals remain
behind this validated app boundary.

## Worker boundary

The same pure crate separately owns Schemars worker shapes. A pinned schema-to-Pydantic
v2 generator emits models under apps/mobile-worker/src/mobile_qa_worker/generated.
No browser/OpenAPI duplication of the fixture protocol is needed. The contract probe
covers UUID, UTC-normalized RFC3339 time, tagged scenarios, required-nullable and
omitted optional fields, and a precision-sensitive canonical decimal **string**.
Rust callers validate semantic constraints after deserialize; Python validates JSON
strictly. `nullable_note` must be present. `optional_note` accepts missing/null/string;
Rust omits None. Pydantic exclude_unset preserves missing input, and excluding None
normalizes optional-null output when matching Rust serialization.

Fake mode is a local version1 fixture protocol, not production worker leasing. It
reports deterministic passed/failed/blocked observations and never imports Minitap.
The explicit SDK smoke imports4.0.0 with telemetry disabled and connections blocked;
it never constructs Agent and proves no device execution or model capability.

## Future work

Keep Loco source registries and injection markers inside apps/api. Generated domain
DTOs move into crates/contracts, with re-exports from apps/api/src/dtos when useful.
Register browser endpoints in Utoipa and meaningful route agreement tests. Run one
explicit generation phase after complete edits; never hand-maintain equivalent
TypeScript/Python shapes. Future migrations register in apps/api/migration.
Health is application liveness; database readiness is tested separately. Development/
test static middleware is disabled; production serves ../web/dist relative to apps/api.
See [roadmap concurrency](implementation/00-master-spec.md#concurrent-execution-plan) for subsequent work.

## Spec 03 browser boundary

`crates/contracts/src/browser.rs` owns all 17 operations for health, cookie sessions,
apps/environments, upload intake/completion, build history and Settings. Every API
error includes a generated request ID matching `X-Request-ID`; API responses use
`Cache-Control: no-store`. Google challenge creation and credential exchange require a same-origin request and
`X-Mobile-QA-Request: 1`; other mutations require the session's `X-CSRF-Token`.
The cookie is HttpOnly/SameSite Strict, `mobile_qa_session` locally and
`__Host-mobile_qa_session` with Secure in production. Raw JWTs are never response DTOs.

Browser DTOs expose safe reference IDs/labels and static APK metadata, never password
hashes, locators or storage keys. Completion returns 200 terminal / 202 validating;
repeated completion shares one build per upload. `error` is retryable infrastructure
failure, `invalid` describes APK content, `unsupported` describes intake policy.
`validated` does not imply installation or execution readiness. The generated SDK's
multipart file field and generated success/error Zod validators are the only browser
transport path. Worker contracts and generated Pydantic are unchanged.

The pinned Hey API header/binary adaptations are documented in [dependencies](dependencies.md).
They preserve generated validation and transport ownership; consumers do not cast JSON
numbers into BigInt or substitute handwritten upload bodies/endpoints.

Google sign-in uses POST `/api/auth/google/challenge` (`GoogleLoginChallenge`) and
POST `/api/auth/google/login` (`LoginRequest`: credential plus challenge UUID).
The challenge response includes the public client ID and nonce. Its separate
HttpOnly cookie binds the browser; challenge consumption prevents replay. Google
signature/issuer/audience/expiry/nonce verification precedes account linking and
session issuance. `/api/auth/login` no longer exists. Google tokens never appear in
session DTOs or persistent browser storage. All changes use the Rust→OpenAPI pipeline.

Workspace onboarding adds `POST /api/workspaces` (`createWorkspace`) with cookie
session and CSRF/origin protection. `CreateWorkspaceRequest` carries a UUID `id`
and bounded `name`; success returns an `OrganizationMembership` with status 201.
Retries with the same ID/name by the existing owner return the same workspace;
conflicting IDs/names return 409. Non-approved accounts receive 403
`approval_required`, even with an otherwise valid session. `UserIdentity` includes
`approval_status: pending | approved`. Verified new Google users receive a session
with no memberships instead of an invitation rejection.

The browser's `workspace` URL parameter maps to the list API's `organization_id`.
List queries validate membership, and cursors from another organization are rejected.
Nested resources continue to use their persisted app/organization ownership for
server authorization; URL selection never grants access.
