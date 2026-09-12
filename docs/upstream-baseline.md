# Upstream baseline

Loco v1.1.0: https://github.com/loco-rs/loco/releases/tag/v1.1.0
Commit: ba726cc4d938d43309bf27bacfe27461f9c782fb.
Reference paths: `loco-new/base_template/{Cargo.toml.t,src/app.rs.t,src/bin/main.rs.t,
config/*.yaml.t,migration/}`, frontend main/routes/API client and request home test.
The scaffold was materialized directly from these conventions. `loco new` was not
run because its success branch unconditionally invokes cargo fmt during authoring.
Unneeded auth, mailers, posts and downloader features were removed. Apache license
and attribution are preserved in LICENSE-LOCO and NOTICE.

The minimal Card/Button implementations are adapted from shadcn/ui (MIT):
https://github.com/shadcn-ui/ui/tree/main/apps/v4/registry/new-york-v4/ui
The local LICENSE-SHADCN preserves its notice. Tailwind 4 provides styling.

The transport toolchain is ts-rs 12, Schemars 1 and datamodel-code-generator 0.76.0
(Pydantic v2 output), with exact resolutions in Cargo.lock and uv.lock. TS browser
output comes from explicit export_all(Config::with_out_dir) calls, not test attributes. React 19,
Router 8, Query 5 and Vite 8 preserve the pinned starter's package majors; pnpm lock
records exact resolved versions. Rust is pinned to 1.95.0; Node to 24.14.1.

SDK: distribution `minitap-mobile-use==4.0.0`, Python 3.12.
Official source: https://github.com/minitap-ai/mobile-use
Inspected reference revision: 12a1dbd3774e96fbc6029ba4d2a7801aeb527764. This source
revision is not asserted to be byte-identical to the published wheel. Actual import
is verified separately from installed distribution metadata. Export is
`minitap.mobile_use.sdk.Agent`; Agent construction initializes telemetry, so setup
never constructs it. MOBILE_USE_TELEMETRY_ENABLED=false is set before SDK import.
The dependency is an optional `sdk` extra to retain lightweight fake development;
full `just setup` and import smoke explicitly include that extra.

Local PostgreSQL uses cached upstream 17-alpine pinned by manifest digest in Compose.
Exact runtime/lock resolutions and gate outcomes belong to the implementation report.
Generator 0.33.0 lost required-nullable semantics; 0.76.0 is pinned after validation.
Other tool pins stay on known compatible lint versions; no global
package manager, editor or tool configuration is altered.

PR preparation audit (2026-09-12): pytest is pinned to 9.0.3 for
CVE-2025-71176. A scoped uv override selects python-dotenv 1.2.2 for
CVE-2026-28684 because Minitap 4.0.0 still pins 1.1.1. Install through uv
with the checked-in lockfile; ordinary pip resolution does not apply uv overrides.
The 26 worker tests and network-blocked SDK import passed after this update.
Frontend and installed-Python dependency audits report no known vulnerabilities.
Real device behavior still requires spec 02 qualification.
