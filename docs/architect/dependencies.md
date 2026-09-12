# Upstream baseline and adaptations

Graphify navigation tooling: https://github.com/Graphify-Labs/graphify
Official PyPI distribution `graphifyy==0.9.58` (CLI `graphify`), isolated under
tools/graphify with frozen transitive dependencies. Published wheel source was inspected
for code-only extraction, portable source paths, source-commit provenance and no-model
clustering/report generation. No upstream assistant installer or hook is run.
See [development](development.md) for CI publication and agent usage boundaries.

Loco v1.1.0: https://github.com/loco-rs/loco/releases/tag/v1.1.0
Commit ba726cc4d938d43309bf27bacfe27461f9c782fb. The initial scaffold materialized
loco-new/base_template conventions directly because loco new unconditionally runs
cargo fmt during authoring. Apache attribution remains in LICENSE-LOCO and NOTICE.
Its native application paths now live together under apps/api. Root is a virtual
Cargo workspace. Unneeded auth/mailers/posts/downloader features remain omitted.

The original foundation used shadcn Card/Button. Those copies were removed in the
Mantine migration; LICENSE-SHADCN is retained for historical attribution. React19,
Router8, Query5 and Vite8 remain pinned by the web lockfile.

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
pnpm11.16.0 and uv0.12.1 are retained. Lockfiles own exact resolutions; [status](status.md) owns validation evidence; no global editor/tool configuration is modified.

## Spec 03 additions

Dashboard controls come from `@mantine/core`, `@mantine/hooks` and `@mantine/form` 9.6.1 (MIT),
installed as packages rather than copied source. Product components import Mantine
directly; `apps/web/src/theme.ts` centralizes colors and defaults. Plain product CSS
does not use Mantine PostCSS mixins, so no additional PostCSS plugins are required.
Tailwind, Radix, CVA and the local `cn` helper are removed. Lucide remains the icon
package. Exact resolutions are in pnpm-lock.yaml. Official integration references:
[Mantine Vite setup](https://mantine.dev/guides/vite/),
[Drawer behavior](https://mantine.dev/core/drawer/),
[theme](https://mantine.dev/theming/theme-object/),
[DOM testing](https://mantine.dev/guides/vitest/),
[disclosure state](https://mantine.dev/hooks/use-disclosure/),
[form state](https://mantine.dev/form/use-form/).

Rust uses Loco 1.1.0's app-session JWT and S3 storage driver, SeaORM entities/migrations,
Axum multipart/cookies, SHA-256 and ZIP 8.6.0 for bounded archive checks. Concrete
versions are in Cargo.lock. No new worker dependencies or handwritten Android parser.

Android Build Tools 36.0.0 (`aapt2`, `apksigner`, `zipalign`), android-35 platform
revision 2 and JDK17 are external explicit prerequisites. `scripts/setup_android.py`
uses versioned Google archives and verifies checksums from the official
[SDK repository index](https://dl.google.com/android/repository/repository2-3.xml).
Recorded macOS Build Tools SHA-256:
`04e7f3a72044de4926fa038fa0e251a37bba1e1c3fb8beab6f8401bfd9eb4bf3`;
platform SHA-256:
`0988cacad01b38a18a47bac14a0695f246bc76c1b06c0eeb8eb0dc825ab0c8e0`.
See [Android command-line tools](https://developer.android.com/tools) for vendor
provenance. These pins are intake tooling, not qualified device evidence.

Hey API 0.99.0 needs two scoped integration adaptations: the Zod generator resolves
OpenAPI binary strings as `Blob`, and the application client wrapper presents the
original header record to generated request validators before the generated client
normalizes headers. SDK serialization/fetch and generated success/error validation
remain authoritative. Byte fields have explicit Rust schema bounds of 1–250 MiB,
avoiding incompatible browser BigInt coercion for JSON byte counts. These paths are
covered by real generated-client transport tests, including multipart serialization.

The 2026-09-12 RustSec audit reports `RUSTSEC-2023-0071` in transitive `rsa 0.9.10`,
introduced by Loco auth → jsonwebtoken's `rust_crypto` feature. There is no patched
version in the inspected advisory database. App sessions use Loco's HMAC JWT API
(HS512 default); Google identities use RSA public-key verification. Production does
not invoke RSA private-key signing/decryption. The dependency audit is **not clean**,
and the upstream advisory remains tracked rather than suppressed. Reassess it before
introducing production private-key operations. See
[RustSec advisory](https://rustsec.org/advisories/RUSTSEC-2023-0071.html).

## Google identity

`@react-oauth/google` 0.12.2 wraps the official GIS sign-in control. jsonwebtoken
10.4.0 verifies Google RS256 tokens; reqwest 0.12 fetches the fixed JWKS URL with a
timeout and bounded cache. Production performs RSA **public-key verification**, not
RSA signing/decryption. The tracked RSA advisory remains open; test fixtures use
only ephemeral synthetic signing material. No actual Google account/token is used
by automated tests. References: [Google verification](https://developers.google.com/identity/gsi/web/guides/verify-google-id-token),
[Google setup](https://developers.google.com/identity/gsi/web/guides/get-google-api-clientid),
[React wrapper](https://github.com/MomenSherif/react-oauth).
