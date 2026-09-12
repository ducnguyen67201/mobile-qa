# Mobile QA

A local source foundation: Loco Rust API, React dashboard and deterministic Python
fixture executor. No device, cloud account or model key is required. Only the App
page's health check is implemented. Auth and product data belong to later specs.

## Install

Prerequisites: Rust 1.95.0 (rustfmt/Clippy), Node 24.14.1, pnpm 11.16.0, uv 0.12.1,
just 1.56+, Python 3.12 (uv can provision it), and Docker with Compose.

```sh
just setup       # downloads dependencies; installs optional Minitap SDK; no checks
just dev         # starts isolated local PostgreSQL, Rust API and Vite
```

Open http://127.0.0.1:5173. `Ctrl-C` stops the child services this invocation owns.
Existing PostgreSQL is left running; no volume is deleted. API logs are in
`.private/api.log`, Vite logs in `.private/web.log`; each invocation truncates them.
The directory is mode 0700 and ignored by Git. `.private/artifacts/` is reserved
for local artifacts with mode 0700; no upload/storage API exists yet. Logs aren't
an artifact store or a production retention policy. Ports are 5150 (API), 5173 (Vite), 55432 (PostgreSQL).
Occupied ports fail clearly; the command never kills their existing owners.

The checked-in generated bindings are ready to consume. After changing contracts,
finish all source/tests first, then explicitly regenerate and validate:

```sh
just types
just check
just build
just smoke
```

`check` runs drift detection first, then each package's static/tests once. `build`
only builds; it does not rerun typechecking or tests. `smoke` uses the built API,
checks API/proxy/DB/fixtures, and imports Minitap 4.0.0 with telemetry disabled and
network connections blocked. It never constructs an Agent. Browser inspection is
separate from endpoint smoke.

For SDK-independent work, `uv sync --project workers/mobile --frozen` installs just
the core package/development tools. `just dev-worker-fake` runs the pass fixture;
replace the fixture with `fail.json` or `blocked.json` for other outcomes. A valid
failed/blocked observation is JSON with exit 0; invalid input exits 2.

`just dev-web` and `just dev-api` start one service. The API needs PostgreSQL first:
`docker compose -f infra/compose.yaml up -d --wait postgres`. Only explicit API
startup compiles Rust. Vite HMR does not run typechecks, lint, tests or generation.
`just device-smoke` explains that spec 02 is unimplemented and exits 2.

See [development](docs/development.md), [architecture](docs/architecture.md) and
[upstream baseline](docs/upstream-baseline.md). No Git remote or commits are created
by setup. This scaffold is local and unauthenticated; do not expose it publicly.
