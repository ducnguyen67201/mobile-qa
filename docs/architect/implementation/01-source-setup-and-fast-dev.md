# 01 — Source setup and fast development

Status: foundation implemented; manual browser acceptance remains open.
[Current status](../status.md) records evidence and limitations. Depends on no earlier spec.
Source is this repository; [PR #1](https://github.com/ducnguyen67201/mobile-qa/pull/1).

## Deliverable

A small monorepo based on the Loco React starter whose checks and fake execution run without a device or paid model access. Development API startup uses the configured Doppler project. Keep source setup bounded: stop when the health endpoint, typed browser call and development commands work. Do not prebuild domain frameworks.

## Mandatory implementation cadence

Write the complete agreed implementation, including relevant test code, before executing verification. Then generate any changed contracts and run typechecks, lint, tests and build checks in one verification phase. Do not check after individual edits/files or run continuous checks while writing code. Batch related fixes after a failed check, then rerun the affected checks. This requirement applies to the entire spec packet and future implementation instructions.

Keep strict type/compiler settings. Do not install check-on-save tasks, automatic check hooks, background typecheck/lint/test watchers, or Rust auto-build/check watchers. Do not enable editor integrations that invoke external check commands on save. Existing editor assistance does not substitute for the explicit final checks; do not modify the user's global editor configuration.

## Source layout

Keep runnable applications under `apps/` and shared contract ownership outside them:

```text
Cargo.toml / Cargo.lock      Virtual Rust workspace and shared dependency lock
apps/api/                   Loco app: src/, config/, migration/, tests/, Cargo.toml
apps/web/                   React/Vite package and generated browser API client
apps/mobile-worker/         uv package, Minitap adapter and generated Pydantic models
crates/contracts/           Authoritative Rust DTOs and endpoint descriptions
contracts/                  Generated OpenAPI/worker schemas and shared JSON fixtures
infra/                      Local database and later device-host provisioning
scripts/                    Explicit generation/development/verification commands
docs/                       Architecture, onboarding and runbooks
justfile                    Small set of developer commands
```

Loco keeps its internal conventions inside `apps/api`. Its default combined scaffold expects `frontend/` under the application; this layout deliberately uses backend scaffolding and an independently generated web client. Document explicit working directories and output paths; do not introduce symlink-dependent scaffolding or another build framework.

## Implementation tasks

1. Generate the Rust/React starter into this repository; never edit synced historical `sources/` material. Record generator/framework versions; commit Cargo, frontend and uv lockfiles. Use SeaORM consistently rather than adding SQLx alongside it.
2. Configure the API in `apps/api` for local PostgreSQL and the React development server to proxy `/api` to Rust. Configure development so a missing frontend production build does not prevent the API starting. Verify startup in the post-implementation phase below.
3. Describe `/api/health` in the Rust contract, bind it to the real handler, and consume its generated client from `apps/web`. Preserve React Router and TanStack Query. Add one shadcn layout only.
4. Add narrow explicit exports. Use Utoipa to describe the browser HTTP contract, then generate the TypeScript SDK and Zod runtime validators with Hey API from a local OpenAPI file. Replace the browser ts-rs export and handwritten endpoint/health parser. Keep Schemars and pinned schema-to-Pydantic generation for worker messages. Generated outputs are committed and written only if content changes.
5. Scaffold the Python wrapper and a deterministic fake executor. Install mobile-use as a pinned dependency/revision rather than maintaining an upstream fork initially. Allow import/setup checks without device/model execution.
6. Use Doppler process injection without env files or examples; document variable names in the environment runbook. Add a local private artifact directory, bounded log output, and startup instructions. Use a small Compose file for PostgreSQL only until another service is actually needed.

The small contracts crate must stay free of Loco, SeaORM and model dependencies. One Rust-owned description covers each method, path, parameter, request, success response and declared error. Generated clients must actually invoke runtime validators; merely emitting Zod files is insufficient. Validate declared error bodies too, and keep malformed transport data distinct from valid API errors. Agreement tests connect the document to real handler status/body behavior. Runtime permissions, business constraints and deployed-version compatibility remain application responsibilities. Do not maintain duplicate handwritten browser/Python shapes or a second ts-rs browser pipeline.

## Developer commands

The commands are implemented in [justfile](../../../justfile), documented in
[development workflow](../development.md) and [environment setup](../environment.md).
`just device-smoke` is still an explicit unimplemented placeholder that exits 2;
real qualification belongs to spec 02. Fake execution covers only the local fixture
protocol, not production scheduling or a real phone.

Vite transpiles TypeScript without typechecking. Keep strict TypeScript checks as explicit post-implementation commands and integration CI steps, with no optional typecheck watcher. Do not attach `tsc`, lint, schema generation or Rust checks to HMR or saving files. [Vite documentation](https://vite.dev/guide/features.html)

Rust must compile before changed backend code runs. Start/rebuild it explicitly after writing the scoped implementation; do not automatically rebuild on edits. Keep incremental build caches and use the dev profile. Frontend edits and artifact writes must not restart Rust or trigger checks. Do not run release builds or `cargo clean` in the ordinary loop. [Cargo profiles](https://doc.rust-lang.org/cargo/reference/profiles.html)

## Verification and acceptance

Execute these checks only after the source-setup implementation is fully written. Acceptance criteria in all later specs follow the same timing.

- Fresh checkout starts from documented commands; a second start reuses dependencies and build caches.
- Change a UI label: HMR updates without Rust rebuild, type generation or test execution.
- Change a Rust response field: regenerate types; an outdated frontend call fails typecheck. Worker fixture mismatch fails validation when worker contracts change.
- Export handles tagged enums, null versus missing fields, UTC timestamps and UUID strings. Represent precision-sensitive integers safely. Runtime input validation and project authorization remain server responsibilities.
- CI regenerates contracts and fails on drift; a migration integration check runs against PostgreSQL. Use scope filters: docs-only changes do not build the worker; contract changes check every affected consumer.
- Record fresh-start, warm UI edit, warm Rust edit and contract-export durations on the actual development machine. Fix duplicate work before adding caches or build tools. No unmeasured latency promises.

Avoid redundant full `cargo check` immediately before equivalent Clippy/test compilation. Cache by lockfiles/toolchain. Do not force emulator/model runs into standard CI or every commit. Tests should target meaningful boundaries, not generated CRUD boilerplate or arbitrary coverage percentages.

References: [Loco SPA scaffolding](https://loco.rs/docs/how-to/build-a-spa/), [Utoipa](https://docs.rs/utoipa/latest/utoipa/), [Hey API Zod integration](https://heyapi.dev/docs/openapi/typescript/plugins/zod), [Schemars](https://docs.rs/schemars/latest/schemars/).
