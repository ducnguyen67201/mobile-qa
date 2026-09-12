# Application and contract ownership

The root Cargo manifest is a virtual workspace. apps/api is the Loco1.1 application;
its migration package uses SeaORM2 and contains no domain tables. apps/web is a plain
React/Vite project; apps/mobile-worker is a uv project. No Turborepo/Nx or additional
application orchestration framework is needed. Each app keeps its native package
manager and tests. Shared contracts, fixtures, infrastructure and developer scripts
stay outside apps.

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
Specs 02 and03 can build independently after coordinating shared-file ownership.
