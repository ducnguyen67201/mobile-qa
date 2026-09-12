# Upstream baseline and adaptations

Loco v1.1.0: https://github.com/loco-rs/loco/releases/tag/v1.1.0
Commit ba726cc4d938d43309bf27bacfe27461f9c782fb. The initial scaffold materialized
loco-new/base_template conventions directly because loco new unconditionally runs
cargo fmt during authoring. Apache attribution remains in LICENSE-LOCO and NOTICE.
Its native application paths now live together under apps/api. Root is a virtual
Cargo workspace. Unneeded auth/mailers/posts/downloader features remain omitted.

Card/Button are adapted from shadcn/ui (MIT), with local LICENSE-SHADCN:
https://github.com/shadcn-ui/ui/tree/main/apps/v4/registry/new-york-v4/ui
React19, Router8, Query5, Vite8 and Tailwind4 remain pinned by the web lockfile.

Browser transport now uses Utoipa5, replacing the initial ts-rs pipeline entirely.
Pure Utoipa definitions export OpenAPI without application/DB dependencies:
https://docs.rs/utoipa/latest/utoipa/
Loco handlers retain framework registration, with real tests checking agreement with
the pure endpoint declaration. Utoipa-Axum automatic routing was inspected, but adding
an Axum dependency to the pure exporter or a separate routing crate is unnecessary
for the one-endpoint foundation.

Hey API openapi-ts0.99.0 and Zod4.6.2 are pinned exactly. Generator config consumes a
local OpenAPI file; no hosted input, account, remote service or watch mode is used.
Primary docs and installed source were inspected before authoring:
https://heyapi.dev/docs/openapi/typescript/plugins/sdk
https://heyapi.dev/docs/openapi/typescript/plugins/zod
https://heyapi.dev/docs/openapi/typescript/clients/fetch
The bundled fetch client skips automatic response validation for204/empty-content
bodies and nonJSON branches, and does not validate non2xx errors. App query boundaries
therefore invoke generated success schemas once and check declared status; a generated
ApiError schema guards non2xx error bodies. Request validation stays SDK-generated.

Worker transport remains Schemars1 → datamodel-code-generator0.76.0 → Pydantic2.
The earlier0.33.0 generator lost required-nullable semantics;0.76.0 is covered by
shared fixture tests. Generated worker output is never handwritten.

SDK distribution minitap-mobile-use==4.0.0, Python3.12:
https://github.com/minitap-ai/mobile-use
Inspected revision12a1dbd3774e96fbc6029ba4d2a7801aeb527764 is not asserted identical
to the published wheel. Import seam is minitap.mobile_use.sdk.Agent; construction
initializes telemetry, so setup never constructs it. Telemetry is disabled before
explicit import. The adapter uses function-local direct imports and upstream typed
Agent, builder, model configuration and TaskRequest classes; no dynamic Any-typed
module facade. The pinned wheel lacks a typing marker and has incomplete return
annotations, so `apps/mobile-worker/typings` describes only the consumed SDK surface;
a compatibility test checks it against the installed package. Task return values
remain opaque `object` because our verifier owns the verdict. Callback metadata is
narrowed before use, and SDK output still never decides the QA verdict. The SDK extra keeps fake development independent. The existing
python-dotenv1.2.2 override and pytest9.0.3 update are preserved for audit compatibility.

PostgreSQL17-alpine is digest-pinned in infra/compose.yaml. Rust1.95.0, Node24.14.1,
pnpm11.16.0 and uv0.12.1 are retained. Lockfiles own exact resolutions; [status](status.md) owns validation evidence; no global editor/tool configuration is modified.

## Phase 02 qualification dependencies

Minitap remains pinned to4.0.0. langchain-core1.6.3 is declared directly in the sdk extra
for the completion-usage callback (same existing transitive resolution). No second agent
framework is added. The tiny Java/Views demo adds an isolated AGP8.13.0/Gradle8.13/JDK17
build, compile/target35 and Build Tools35.0.0. Gradle wrapper files come from the upstream
v8.13.0 tag; retain their license headers. Android archive metadata and vendor checksums
are pinned in infra/device-host/toolchain.lock.json. Required vendor licenses still apply;
no installer silently accepts SDK terms or modifies a user's existing SDK.


The first live Minitap demo exposed private ToolNode API drift: Minitap 4.0.0
omits both the config argument to state extraction and the tools list in ToolRuntime.
`qualification/sdk_compat.py` supplies a typed ExecutorToolNode subclass only inside
the isolated SDK process and selects it in Minitap's graph factory. It is guarded
against the exact Minitap 4.0.0 / langgraph-prebuilt 1.1.0 pair; no installed package
files are modified. The subclass preserves sequential execution and error handling.
[Upstream fix #214](https://github.com/minitap-ai/mobile-use/pull/214) tracks state
extraction; our adapter also supplies the tools and execution context fields.
A real offline graph tool call verifies the compatibility path before device use.
Remove the adapter after a fixed SDK release passes that test and qualification.
Downgrading was rejected because the resulting LangChain version had a known advisory.
