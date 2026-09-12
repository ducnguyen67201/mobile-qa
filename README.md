# Mobile QA

Local Loco API, React dashboard and deterministic Python fixture executor. Only
App's health check is implemented; auth, app uploads and devices belong to later specs.

```text
apps/api/             Loco source, configuration, migrations and tests
apps/web/             React/Vite app and generated browser SDK/Zod validators
apps/mobile-worker/   Python/uv fake executor and generated Pydantic models
crates/contracts/     Pure Rust transport definitions and lightweight schema exporter
contracts/            Generated browser OpenAPI, worker JSON Schema and fixtures
infra/                Isolated local PostgreSQL
scripts/              Explicit setup, generation, development and verification
Cargo.toml            Virtual workspace (API, migration, contracts)
```

## Start

Prerequisites: Rust1.95.0 with rustfmt/Clippy, Node24.14.1, pnpm11.16.0, uv0.12.1,
just1.56+, Python3.12 (uv can provision), Docker with Compose.

```sh
just setup
just dev
```

Open http://127.0.0.1:5173. API uses5150; PostgreSQL uses55432, all loopback only.
Ctrl-C stops only child services and the PostgreSQL instance this command started.
Existing PostgreSQL stays running; its persistent volume is never deleted.
`.private/api.log` and `.private/web.log` truncate on explicit start. `.private/`
and reserved `.private/artifacts/` are ignored and mode0700. No artifact upload API
or production retention policy is implemented.

Finish **all** source/test/config changes for the scoped feature before generating
or running checks. Then perform the explicit final phase:

```sh
just types
just check
just build
just smoke
```

`types` exports the pure Rust crate without booting Loco or connecting to a DB.
Utoipa creates `contracts/browser.openapi.json`; the pinned local Hey API generator
produces the browser SDK/types/Zod schemas. Schemars creates the worker schema and
the pinned generator produces Pydantic. Generated outputs are committed and updated
only on content changes. There is no hosted Hey API account/schema or second ts-rs
pipeline. `check` compares staged output for drift before checking all packages once.
`build` only builds. `smoke` checks API/proxy/database, fixture outcomes and a network-
blocked import-only Minitap4.0.0 seam; it never constructs an Agent.

For individual services use `just dev-web`, `just dev-api`, `just dev-worker-fake`.
API needs DB first: `docker compose -f infra/compose.yaml up -d --wait postgres`.
For the Loco CLI directly: `cd apps/api && cargo loco start --environment development`.
API configuration paths resolve from apps/api. Only explicit API starts compile Rust;
Vite HMR never invokes typechecking/lint/tests/generation or a Rust rebuild watcher.

Full setup includes the optional Minitap SDK. For fake-only Python work:
`uv sync --project apps/mobile-worker --frozen`. Dependency reconciliation belongs to
setup; ordinary uv commands use --no-sync. Fixture CLI outcomes passed/failed/blocked
return JSON with exit0; invalid input exits2. `just device-smoke` explains spec02 is
unimplemented and exits2 without any device/cloud work.

See [architecture](docs/architecture.md), [development](docs/development.md) and
[upstream provenance](docs/upstream-baseline.md). This local scaffold is unauthenticated;
production configuration support is not a deployed or qualified customer service.
Rendered browser/keyboard/HMR acceptance remains pending due the browser tool's admin
policy verification denial. Automated UI tests do not substitute for that observation.
