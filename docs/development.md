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

CI runs pull_request and pushes to main. Superseded PR runs are cancelled. Paths are
compared against the whole PR base, including additions/deletions, rather than only the
last commit: a still-failing earlier change must not disappear from required coverage.
Root docs-only changes skip app builds. The path map lives in .github/workflows/ci.yaml.

| Changed surface | CI checks |
|---|---|
| apps/api source/config/migration/tests | API and migration format/Clippy/tests; API build |
| apps/web ordinary source/tests | Web typecheck/lint/tests/build |
| apps/mobile-worker ordinary source/tests | Worker Ruff/Pyright/tests |
| Rust browser DTOs or browser OpenAPI | API (Rust source), web consumer and contract checks |
| Rust worker DTOs or worker schema | API (Rust source), worker consumer and contract checks |
| Common Rust contract module/manifest | All contract consumers and contract checks |
| Web generator/config/dependency inputs | Web and contract checks; no Python/API app suite |
| Python generator/dependency inputs | Worker and contract checks; no web/API app suite |
| Root Rust lock/toolchain/workspace | API and contract checks; no web/Python app suite |
| Contract fixtures | Rust contract tests, API package checks and worker tests |
| CI workflow | All jobs, to verify the pipeline itself |

Generated-output edits always trigger drift checks. The contracts job has its own
Rust checks and tests, plus Node/Python generation, without starting PostgreSQL. An
API-only job does not run the contract test suite or build the exporter. Shared Rust
source still compiles as an API dependency. Compiler and package caches remain enabled.

The minimum safe typechecking unit today is an app/project or Rust crate, not one file.
A changed public signature can invalidate unchanged callers. Tests currently run within
the affected app (one API integration target, a small web suite, one worker suite).
When those suites grow, add named integration targets for each API feature and an explicit
feature-to-tests dependency map; shared routers/auth/database setup must select all API
tests. Browser related-test selection must include import dependents and fall back to the
app suite for config/deleted/shared files. Do not silently claim file-level dependency
analysis: that is not implemented. Extract Rust crates or TS project references only when
measured check time warrants them; a folder alone is not a compilation boundary.

No cloud/device/model tests run in CI. Local commands retain the explicit full-final-check
option; nothing starts checks during editing.

The refactor report records actual local gates and timings. Browser admin policy
verification previously denied access: do not use alternate browser/Playwright/HTTP
workarounds to evade it. Rendered keyboard/Retry/HMR acceptance remains explicitly
unverified until approved browser access is available. Endpoint smoke is a separate
existing automated check, not a claim of rendered browser acceptance.
