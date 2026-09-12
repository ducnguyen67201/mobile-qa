# Explicit development workflow

Finish every scoped source/test/config change before generation, formatting,
typecheck, lint, tests, builds or runtime verification. Run one final phase, batch
related fixes, and rerun only failed/invalidated checks. There are no check-on-save
hooks, lifecycle checks, check watchers, Rust rebuild loops or global configuration
changes. Vite only performs normal transpilation/HMR.

`just setup` downloads Cargo/pnpm/uv dependencies, including the optional SDK. Existing
locks use locked/frozen resolution; pnpm dependency scripts are disabled. The web-local
pnpm-workspace.yaml prevents discovery of unrelated ancestor workspaces. Normal uv
commands use --no-sync. Moved virtual environments can be refreshed explicitly with
`uv sync --project apps/mobile-worker --frozen --extra sdk --reinstall`.

| Command | Work |
|---|---|
| dev | Isolated PostgreSQL, explicit API startup, Vite; owned-child cleanup |
| types | Pure Rust exporter, local Hey API SDK/Zod, worker Pydantic; content-only writes |
| check-contracts | Temporary regeneration/drift + content-sync test |
| check-web | Strict tsc including generator config, ESLint, Vitest |
| check-api | Cargo fmt check, Clippy, workspace tests with isolated test DB |
| check-worker | Ruff, strict Pyright, pytest |
| build | Vite production bundle, Cargo workspace debug build |
| smoke | Built API without dist, Vite proxy, DB bookkeeping, fixtures, import-only SDK |

API processes run with apps/api as cwd; workspace target/cache stay at repository
root. Test binaries resolve apps/api/config from their Cargo package directory.
Production static assets resolve ../web/dist. Worker tests resolve root fixtures;
ordinary fake commands can run from the root with an explicit --project.

Development and test use fixed local database URLs, ignoring inherited DATABASE_URL.
All destructive flags stay false; integration uses the existing mobile_qa_test DB
without create/drop helpers. Compose project mobile-qa-local binds loopback, initializes
the separate test database and persists its named volume. Startup never resets that
volume. Existing Compose PostgreSQL is left running; commands stop only what they own.
Logs truncate on explicit startup. Keep secrets/customer data out of source and logs.
.private/artifacts is reserved only; no storage API is included.

CI runs pull_request and pushes to main, avoiding duplicate feature push jobs. Docs-only
changes skip app builds. apps/api/infra changes check Rust/database; apps/web changes
check/build web; apps/mobile-worker changes check Python. Shared contracts, generator
configuration/manifests/locks and tooling trigger drift plus all affected consumers.
The contracts job installs both Node and Python generators; it does not start a DB.
Caches key Cargo/pnpm/uv by locks/toolchain. No cloud/device/model tests run in CI.

The refactor report records actual local gates and timings. Browser admin policy
verification previously denied access: do not use alternate browser/Playwright/HTTP
workarounds to evade it. Rendered keyboard/Retry/HMR acceptance remains explicitly
unverified until approved browser access is available. Endpoint smoke is a separate
existing automated check, not a claim of rendered browser acceptance.
