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

| Command         | Work                                                                            |
| --------------- | ------------------------------------------------------------------------------- |
| dev             | Isolated PostgreSQL, Doppler-injected API startup, Vite; owned-child cleanup    |
| types           | Pure Rust exporter, local Hey API SDK/Zod, worker Pydantic; content-only writes |
| check-contracts | Temporary regeneration/drift + content-sync test                                |
| check-web       | Strict tsc including generator config, ESLint, Vitest                           |
| check-api       | Cargo fmt check, Clippy, workspace tests with isolated test DB                  |
| check-worker    | Ruff, strict Pyright, pytest                                                    |
| build           | Vite production bundle, Cargo workspace debug build                             |
| smoke           | Built API without dist, Vite proxy, DB bookkeeping, fixtures, import-only SDK   |

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

| Changed surface                          | CI checks                                                |
| ---------------------------------------- | -------------------------------------------------------- |
| apps/api source/config/migration/tests   | API and migration format/Clippy/tests; API build         |
| apps/web ordinary source/tests           | Web typecheck/lint/tests/build                           |
| apps/mobile-worker ordinary source/tests | Worker Ruff/Pyright/tests                                |
| Rust browser DTOs or browser OpenAPI     | API (Rust source), web consumer and contract checks      |
| Rust worker DTOs or worker schema        | API (Rust source), worker consumer and contract checks   |
| Common Rust contract module/manifest     | All contract consumers and contract checks               |
| Web generator/config/dependency inputs   | Web and contract checks; no Python/API app suite         |
| Python generator/dependency inputs       | Worker and contract checks; no web/API app suite         |
| Root Rust lock/toolchain/workspace       | API and contract checks; no web/Python app suite         |
| Contract fixtures                        | Rust contract tests, API package checks and worker tests |
| CI workflow                              | All jobs, to verify the pipeline itself                  |

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

## Explicit qualification commands

`just device-doctor profile`, `just device-smoke request`, and `just device-qualify config`
are opt-in operator commands, never dependencies of setup/check/build/smoke. The Android
fixture has its own pinned Gradle wrapper under apps/qa-demo-android. Worker checks install
the optional SDK for strict type resolution but never run a model/device. Demo-only CI
builds/lints APKs, no emulator. Device-host infrastructure selects worker checks; PostgreSQL
infrastructure selects API checks. CI filter tests run only for changed workflow/filter tests.
See [device qualification](device-qualification.md) for host prerequisites and result semantics.

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
name:<name> organization:<name> --environment development`. This grants access to
the specified Google email; no password is created. The task prints only new user/org IDs. Other operator actions require `actor:<user-id>` and
`organization:<org-id>` of an active operator:

- `link-google user:<id> subject:<verified-google-sub>`: explicitly links an unlinked
  invited account when Google is not authoritative for its email. Verify the Google
  subject out of band; an existing link cannot be silently reassigned.
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

### Google-only sign-in

Supply `GOOGLE_CLIENT_ID` through Doppler to the API. Register the application origin
in a Google OAuth **Web application** client. For local Google UI use
`http://localhost` and `http://localhost:5173` as authorized JavaScript origins,
set `MOBILE_QA_DEV_ORIGIN=http://localhost:5173`, and open that origin. Production uses
the configured HTTPS `HOST`. This GIS credential flow needs no client secret or
OAuth redirect callback. The client ID is public; the API returns it with a one-use
nonce/challenge and sets a separate HttpOnly browser-binding cookie.

Any verified Google account can register and sign in. New users have
`users.approval_status='pending'` and can view their approval status without a
workspace. Existing users remain approved after migration 000003. First-use linking
of pre-existing records still requires a Google-authoritative email; subsequent
sign-ins use Google's immutable `sub`, not email. Third-party email accounts can
register new records, but cannot claim existing email records without explicit linking.
The incremental migration drops password hashes and revokes pre-Google sessions,
while retaining users, memberships, apps and builds. Rollback cannot recover hashes.
The old password endpoint and password reset task are removed.

API tests and the HTTP smoke generate ephemeral RSA fixture keys. The public fixture
key override `MOBILE_QA_TEST_GOOGLE_JWKS` is honored **only** by Environment::Test;
development/production always fetch Google's fixed JWKS endpoint. No test private
key or real Google token is committed. Real Google consent/origin configuration and
rendered browser acceptance still require operator setup and permitted browser access.

### Workspace creation and approval

Approve a registered account by editing `users.approval_status` to `approved` in the
local database, or run the trusted process-only task from `apps/api`:

```sh
../../target/debug/mobile-qa-cli task operator action:approval user:<uuid> status:approved --environment development
```

Use `status:pending` to revoke workspace creation approval. This does not delete
existing workspace ownership or data; `disabled_at` remains the account disable
mechanism. There is no HTTP self-approval endpoint. The user can select **Check
approval status** without signing out. `operator provision` remains a convenience
for local fixtures/explicit administration, not a sign-in requirement.

Approved users can open `/workspaces/new`, create multiple workspaces, and become
an operator/owner of each. Creation uses a client-generated UUID for safe retries.
The existing organizations/memberships tables remain the storage model. Browser
navigation uses `?workspace=<organization UUID>`; the picker updates that URL,
clears app-specific selections on switch, and retains the workspace on navigation.
A missing parameter selects the first available membership. Invalid or inaccessible
workspace IDs display an error instead of silently selecting a different workspace.
App lists and creation use the selected organization. Nested app resources derive
and verify ownership through their app IDs; the browser rejects an app link whose
organization differs from the selected workspace before loading builds/uploads.

### Combined app setup and phone runner

This branch retains main's Mantine dashboard, sign-in, workspace and APK intake flows.
Use `just dev` for that application and `just device-local-agent MODEL` for the
standalone demo runner. Phase 04 Tests/Runs now dispatch approved API jobs to a registered execution worker. An accepted uploaded
build is not an automatically executable test and device readiness remains separate.

`just setup-android` supplies intake Build Tools 36.0.0; `just device-local-setup`
supplies execution Build Tools 35.0.0, platform-tools and the emulator. They coexist
in versioned directories under `.private/android-sdk` and share android-35 revision 2.
Keep both explicit commands; neither belongs in ordinary dev/check startup.

## Execution development

After the complete implementation edit batch, run `just types`, the affected `just
check-*` recipes and `just build`; `just smoke-execution` then exercises real local
HTTP, a simulated Python worker, pass/fail/blocked evidence and API restart persistence.
Prepare signed synthetic fixtures with `just apk-fixtures` first. The additional
`execution.apk` fixture has the demo package identity but is an intake-only fixture;
it is never claimed as a runnable Android application.

Operator commands use the existing Loco task entry from apps/api:
`cargo loco task execution action:import actor:<uuid> app:<uuid> file:<absolute-json>`.
The JSON contains the case/suite/plan tagged definition only. Use `grant-reviewer`
with `user` and `purpose:business|executability`; then `approve` with `definition`,
`hash` and `purpose`. `register-profile` reads a nonsecret profile JSON; `register-worker`
takes `worker`/`profile` UUIDs and an injected token. `action:reconcile` marks expired
leases for recovery; `action:recover` requires actor/app/attempt and a physical-reset
evidence reference. Recovery is an operator assertion after stopping the old process
and verifying reset, never a substitute for that procedure.

`execution-worker --origin <api-origin> --profile-id <uuid> --state <private-path>`
long-polls the protocol for up to 30 seconds. `--once` exits after one claim, including an idle timeout. A real profile additionally needs
`--profile <absolute-host-profile>` matching the manifest image/model. `--scenario`
is an explicit synthetic/demo fixture control, not customer run input. Do not use
`dev-execution-fake` with a registered real profile: the profile controls the driver.
A dirty execution journal requires operator recovery; restarting never replays actions.

## Source formatting

`just format` explicitly formats Rust with rustfmt, all supported handwritten
JS/TS/CSS/HTML/JSON/YAML/Markdown with pinned Prettier, and Python throughout the
repository with Ruff. `just format-check` checks the same scope in CI. The formatter
selects Git-visible files, including new files, and skips generated contracts, locks
and private/build/dependency directories. Other extensions retain their native format. No format-on-save hook or watcher is
installed. For frontend-only work, use `pnpm --dir apps/web format`;
`pnpm --dir apps/web format:check` is included in `just check-web` and frontend CI.
Prettier uses 100-column lines, single quotes and no semicolons. Generated browser
contracts, dependency locks and build outputs are excluded so their generators stay
authoritative. Rust SQL literals use escaped line breaks where needed to keep query
calls readable while preserving the exact SQL bytes.

## Test backend dispatch to an emulator

First run `just smoke-execution` for the existing secret-free HTTP/Python fake
acceptance. It verifies persisted runs and evidence without starting a phone.
The real path requires an explicitly started worker on the emulator host; the API
does not provision that host or start a worker daemon for you.

1. Prepare the Android toolchain and build the actual demo with
   `just device-local-build`. Upload the good demo APK from
   `apps/qa-demo-android/app/build/outputs/apk/good/debug/` through normal app setup,
   using package `ai.mobileqa.demo`. Do not upload `.private/test-apks/execution.apk`
   for this test: it is only an intake fixture, not a runnable application.
2. Select an existing qualified host TOML profile, with `headless = false` if you
   want to see the emulator. Its model, system image, SDK paths and Doppler SDK
   configuration must be valid. `just device-doctor /absolute/path/profile.toml`
   checks the host. Register the matching backend execution profile with
   `driver: minitap`, `adapter: demo_persistence_v1`, `package: ai.mobileqa.demo`,
   the exact same model/image and at most 104857600 APK bytes. Qualification must
   reference real evidence; do not label an unqualified host qualified just to run.
3. Use the execution maintenance commands above to import/review
   `contracts/fixtures/execution/persistence-case.json`, import/review a plan
   selecting that case and real profile, and register a worker for that app/profile.
   Inject the same `MOBILE_QA_WORKER_TOKEN` into registration and worker processes
   through Doppler. Backend origins belong to the demo fixture; customer account
   and reset references remain unsupported by this adapter.
4. With the API running, start the worker in another terminal on that host:

   ```bash
   # Run in a Doppler-injected shell/process with MOBILE_QA_WORKER_TOKEN available.
   just dev-execution-real http://127.0.0.1:5150 PROFILE_UUID \
     /absolute/private/execution-state /absolute/path/profile.toml
   ```

5. Submit the approved plan/build using the dashboard Run button, or the normal
   authenticated `POST /api/apps/{app_id}/runs` endpoint with an Idempotency-Key
   and body `{"build_id":"BUILD_UUID","plan_version_id":"PLAN_UUID",
"environment_revision":1}`. Browser session, CSRF and app membership checks
   still apply. Start the worker **before** submitting to observe the wakeup.
6. The waiting claim returns, the emulator boots, the APK installs, and Minitap
   creates the unique task. The worker restarts the app, captures checkpoints,
   verifies reset and uploads evidence. Read `GET /api/runs/{run_id}` or the report
   page: expect `driver: minitap`, a passed persistence check and
   `cleanup: verified_clean`. A simulated report does not validate this path.

The claim endpoint waits up to 30 seconds; the Python HTTP timeout is 35 seconds.
An idle response sets `poll_after_seconds: 0`, so the worker immediately opens its
next waiting request. Ordinary HTTP calls retain five-second timeouts. Same-process
run commits and clean resource release wake claims immediately; a five-second
database recheck covers other API processes and maintenance commands. This initial
wakeup is process-local, not PostgreSQL LISTEN/NOTIFY. No transaction is held while
waiting. Worker revocation is rechecked on each wake/recheck. Reverse proxies must
permit requests lasting longer than 30 seconds. Heartbeats and cleanup fencing are
unchanged. Real-device/model execution is not part of ordinary checks.

## Authoring workflow acceptance

After the complete phase 05 source/test/config batch, run `just types`, `just format`,
`just format-check`, the contracts/API/web/worker checks, CI scope checks, `just build`
and `just smoke-test-library`. The last command reuses the synthetic sign-in/APK
and worker fixtures on the owned test API port. It performs customer mutations
through HTTP, checks exact retries and restarts, and never consumes Doppler secrets
or starts an emulator. Preserve existing local development processes.

For an allowed browser acceptance: Tests → app → new case or controlled demo template
→ Save → Request review → both explicitly granted reviews → suite/release plan →
review → Set as default → choose a build → Run → report. An operator must already
register a compatible qualified profile and grant the reviewer purposes. Customer
apps without a qualified adapter can be authored and receive business review;
automatic execution remains blocked with a visible readiness explanation.

The existing browser admin-policy restriction is binding. Source design review,
DOM tests and HTTP acceptance must be reported separately from rendered/device proof.
