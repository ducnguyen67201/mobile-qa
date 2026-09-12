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
| dev | Isolated PostgreSQL, Doppler-injected API startup, Vite; owned-child cleanup |
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

Runtime secrets use [Doppler injection](environment.md), with no env files. Only the
API child is wrapped; checks/fake runs need no Doppler access. Development and test
use fixed local database URLs, ignoring inherited DATABASE_URL.
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

The [status record](status.md) records local and hosted evidence. Browser admin policy
verification previously denied access: do not use alternate browser/Playwright/HTTP
workarounds to evade it. Rendered keyboard/Retry/HMR acceptance remains explicitly
unverified until approved browser access is available. Endpoint smoke is a separate
existing automated check, not a claim of rendered browser acceptance.

## App setup development and operator commands

Install a JDK 17 and set `JAVA_HOME` in the invoking shell. Explicitly run
`just setup-android` to install pinned Build Tools 36.0.0 and platform android-35
under ignored `.private/android-sdk`, then `just apk-fixtures` after authoring and
before the final API test pass. Installation uses Google's versioned SDK archives;
it changes no global SDK configuration. API startup and tests never download tools.
The synthetic resource-only APK and ephemeral signing key prove intake, not execution.

After finishing source/test/config edits: `just types`, `cargo fmt --all`,
`just apk-fixtures`, `just check-contracts`, `just check-web`, `just check-api`,
`just build`, `just smoke`, `just smoke-app-setup`. Rerun only failed/invalidated
checks. The second smoke owns isolated test accounts/API processes, verifies real
HTTP upload and validation, restarts the API, and checks persisted data and tenant
isolation. It does not inspect a rendered browser or reset a database.

Run explicit Loco tasks from `apps/api` with process injection appropriate to the
environment, for example `doppler run --no-fallback --forward-signals --
../../target/debug/mobile-qa-cli task operator action:provision email:<email>
name:<name> organization:<name> --environment development`. Inject
`MOBILE_QA_OPERATOR_PASSWORD`; never place it in the arguments. The task prints only
new user/org IDs. Other operator actions require `actor:<user-id>` and
`organization:<org-id>` of an active operator:

- `reset-password user:<id>`: updates the hash and revokes existing sessions.
- `membership user:<id> role:member|operator active:true|false`: manage org access.
- `grant` / `revoke-grant user:<id> app:<id>`: manage explicit member app access.
- `reference app:<id> kind:account|reset label:<label>`: records an injected
  `MOBILE_QA_SECRET_LOCATOR` beginning `doppler://`, prints only reference ID.
- `observe app:<id> revision:<n> kind:backend|account|reset
  state:operator_reported_ok|operator_reported_blocked [note:<safe-note>]`: records
  an operator observation at the current environment revision. Never include
  credentials in notes. Editing the environment invalidates old observations.

`task artifact-cleanup` is dry-run by default. `apply:true` deletes expired upload
attempts and stale scratch after a safety hour, keeping every referenced build object
and active lease. It also expires old session/rate-limit rows. Run it as an explicitly
scheduled operator action; no background scheduler or accepted-build retention is
configured. Hosted cleanup/round-trip and rendered acceptance remain separate gates.

### Browser state ownership

Use Mantine `useDisclosure` for controlled drawers and `useForm` for field values,
normalization and submission. Generated SDK/Zod boundaries remain authoritative for
wire validation. TanStack Query owns saved apps/builds/session data and mutation
status; router search parameters own selected build/upload IDs. `useApkUpload` owns
transient file selection, request phases, cancellation and saved-upload reconciliation.
Keep this workflow outside presentation components. Use plain React state for small
independent values; add memoization/context only when there is a concrete sharing or
identity requirement. Do not recreate the removed SidebarContext state container.
