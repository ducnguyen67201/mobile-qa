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
explicit import. The SDK extra keeps fake development independent. The existing
python-dotenv1.2.2 override and pytest9.0.3 update are preserved for audit compatibility.

PostgreSQL17-alpine is digest-pinned in infra/compose.yaml. Rust1.95.0, Node24.14.1,
pnpm11.16.0 and uv0.12.1 are retained. Exact resolutions and validation outcomes belong
to the report; no global editor/tool configuration is modified.
