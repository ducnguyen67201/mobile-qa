# Mobile QA

Rust/Loco API, authenticated Mantine dashboard, app/APK setup and an explicit Android
test runner. Tests, runs, worker dispatch and AI discovery are implemented; see the
[architecture status](docs/architect/status.md) for verified behavior and open qualification gates.

```text
apps/api/             Loco source, configuration, migrations and tests
apps/web/             React/Vite app and generated browser SDK/Zod validators
apps/mobile-worker/   Python/uv fixture and device runners, generated Pydantic models
crates/contracts/     Pure Rust transport definitions and lightweight schema exporter
contracts/            Generated browser OpenAPI, worker JSON Schema and fixtures
infra/                Local PostgreSQL and pinned device-host setup
scripts/              Explicit setup, generation, development and verification
Cargo.toml            Virtual workspace (API, migration, contracts)
```

## Start

Prerequisites: Rust1.95.0 with rustfmt/Clippy, Node24.14.1, pnpm11.16.0, uv0.12.1,
just1.56+, Python3.12 (uv can provision), Docker with Compose, and the Doppler CLI.

```sh
just setup
doppler login
doppler setup
just dev
```

Run `doppler setup` from this repository root and select its project/config. Local
credentials and directory selection are managed by the Doppler CLI; no token belongs
in Git. For the full stack, configure the existing qualified device/profile in
`.private/dev/config.json` as described in [local development](docs/architect/development.md#one-command-full-local-stack).
`just dev` builds the API once and starts API, Vite, PostgreSQL, phone and execution workers.
Doppler injects API and worker children separately; the browser and build receive no secrets.
Use `just dev-ui` for API/web development without a device profile. No `.env` or secret export file is used.
See [environment setup](docs/architect/environment.md) for required variable names and deployment.

Open http://localhost:5173. API uses5150; PostgreSQL uses55432, all loopback only.
Ctrl-C stops only child services and the PostgreSQL instance this command started.
Existing PostgreSQL stays running; its persistent volume is never deleted.
`.private/dev/latest/` contains the current per-service logs. Use `just dev-logs` to
follow them, `just dev-stop` to stop, or `just dev-restart` to rebuild and restart.
Private logs, worker state and APK artifacts stay ignored. The emulator opens when
requested through the phone preview; AI calls require an explicit AI action.

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
For the Loco CLI directly: `cd apps/api && doppler run --no-fallback --forward-signals -- cargo loco start --environment development`.
API configuration paths resolve from apps/api. Only explicit API starts compile Rust;
Vite HMR never invokes typechecking/lint/tests/generation or a Rust rebuild watcher.

Full setup includes the optional Minitap SDK. For fake-only Python work:
`uv sync --project apps/mobile-worker --frozen`. Dependency reconciliation belongs to
setup; ordinary uv commands use --no-sync. Fixture CLI outcomes passed/failed/blocked
return JSON with exit0; invalid input exits2. `just device-smoke REQUEST` explicitly runs
one controlled Android/Minitap attempt; it is never part of ordinary smoke checks.

## Test a real phone locally

Use native macOS (Apple Silicon or Intel) or Linux x86_64 with KVM, JDK 17 and the
Python/uv prerequisites above. No cloud account, database or API server is needed.

```sh
just device-local-setup
# Review/accept Android SDK licenses using the command printed by setup.
just device-local-build
just device-local
just device-local broken
just device-local unavailable
```

The default local mode uses **real Android + ADB demo taps**, without a model key. It
checks saving/reopening the demo, screenshots and a fresh-device reset. A detected
broken build is expected: command exit 0 means the expected outcome and reset matched.
It does not establish AI-agent reliability or support arbitrary customer APKs.

To exercise **Minitap on the same real phone**, configure `OPENAI_API_KEY` in the local
profile's Doppler config and explicitly select your OpenAI model:

```sh
just device-local-agent YOUR_MODEL_ID
```

This makes billed model calls. `.private/device-local/profile.toml` contains nonsecret
host settings; the default Mac phone is visible (`headless = true` hides it). Reports
are under `.private/artifacts/local-device/`; `latest.json` points to the latest report.
Close any emulator on ports 5554/5555 first. The runner refuses to borrow or kill it.
See [local device setup and recovery](docs/architect/device-qualification.md#local-mac-or-linux-development).

Start at **[docs/architect](docs/architect/README.md)**, the source of truth for product,
architecture, implementation specs and operations. The dashboard uses Google sign-in and workspace authorization;
production configuration support is not a deployed or qualified customer service.
Main's dashboard acceptance and this branch's device evidence are recorded separately
in [current status](docs/architect/status.md); hosted execution remains unqualified.
