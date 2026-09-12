# Implementation Report: 03 — App setup

## Summary

Implemented the first persisted workflow on `codex/03-app-setup`: an operator-created
user signs in with Google, creates an Android app/environment, uploads a private APK and sees
validation of its actual stored bytes plus persisted metadata/history. The dashboard
uses packaged Mantine 9.6.1 components following user feedback about maintaining
copied shadcn/Tailwind source. A fresh source-only review passed. Local
implementation is verified; hosted storage and rendered-browser acceptance remain open.
Implementation did not deploy services, provision cloud resources or change credentials.
The user subsequently requested publishing this branch as a pull request.

## Assessment vs reality

| Metric | Plan | Actual |
|---|---|---|
| Complexity | XL | XL: auth, tenant schema, storage, tools and UI integrated |
| Confidence | 8/10 | Local behavior verified; browser/hosted acceptance unverified |
| Files changed | About 115–125 | See working diff; includes subsequent Mantine replacement |
| Implementation | 14 tasks | Local scope complete; explicit external gates retained |

## Tasks completed

| # | Task | Status |
|---|---|---|
| 1 | Configuration and error/contracts foundation | Complete for local implementation |
| 2 | Users, organizations and revocable sessions | Complete for local implementation |
| 3 | Protected sign-in/dashboard shell | Complete for local implementation |
| 4 | App/environment persistence | Complete for local implementation |
| 5 | Create-app and persisted detail UI | Complete for local implementation |
| 6 | Private upload sessions and bounded transfer | Complete for local implementation |
| 7 | Real APK validation and idempotent completion | Complete for local implementation |
| 8 | Upload recovery and build history | Complete for local implementation |
| 9 | Scoped references and operator observations | Complete for local implementation |
| 10 | Honest readiness and Settings | Complete for local implementation |
| 11 | Explicit maintenance and operational setup | Complete for local implementation |
| 12 | Synthetic fixtures and acceptance tests | Complete for local implementation |
| 13 | CI, HTTP smoke and canonical documentation | Complete for local implementation |
| 14 | Generation and consolidated validation | Complete for local implementation |

## Validation results

| Check | Result | Evidence |
|---|---|---|
| Rust format / Clippy | Pass | Workspace/all targets, warnings denied |
| Rust tests | Pass | 5 API units, 6 app-setup integration tests, 1 health route test, 4 pure contract tests |
| TypeScript / ESLint | Pass | Full web scope |
| Web tests | Pass | 57 tests across 5 files |
| Generated drift | Pass | No changed outputs after final locked tools installed |
| Exporter synchronization regression | Pass | 1 Python helper test |
| Builds | Pass | Cargo workspace and Vite production |
| Real APK HTTP smoke | Pass | Sign in/create/stream/finalize; independent size/hash; foreign-org denial; same persisted build after API restart |
| Foundation smoke | Pass | API direct/proxy health, unknown route, migration bookkeeping, fake scenarios and network-blocked Minitap import |
| Frontend dependency audit | Pass | No known advisories |
| Rust dependency audit | Finding retained | RUSTSEC-2023-0071 in transitive rsa 0.9.10; app sessions use HMAC; Google uses RSA public-key verification, with no production RSA private-key operations |
| GAN design | Provisional source pass | Mantine source review passed; historical shadcn scores 7.27 → 7.87 |
| Rendered browser acceptance | Open | Existing admin-policy denial; no bypass attempted |
| Railway round-trip | Open | Adapter implemented, no authorized bucket credentials/provisioning used |

Measured locally: 0.147s finalization of a tiny signed synthetic resource-only APK;
0.211s warm API startup in the foundation smoke. These are not production SLAs or
customer APK performance claims. Main web chunk is 752.46 kB / 228.30 kB gzip, yielding
a nonblocking Vite size warning. No worker implementation or generated-worker diff.

## Behavior and test coverage

The real route suite covers all 16 declared operations and safe error envelopes;
login/cookie/session/logout, origin/CSRF rejection, malformed body/UUID/query limits,
active same-org ungranted and foreign-org denial, disabled/expired/revoked sessions,
login throttling, app conflicts, upload quota/size mismatch/expiry, stale receiver
recovery, duplicate sealing, concurrent/repeated completion, validator unavailability
and retry of the same build, real signed/unsigned/tampered/mismatched/unsupported APKs,
metadata/hash/byte counts, environment optimistic conflicts and stale observations,
private response fields, and dry-run/apply orphan cleanup protecting referenced builds.

Unit checks exercise CRC corruption, unsafe archive paths, missing manifests, stored
checksum mismatch, process timeout/reaping, immutable file publication and symlink
rejection. Web tests cover auth guards, forms, actual SDK/schema boundaries, multipart
serialization, uncertain-response recovery, saved build selection, exact expiry text
and polling transitions that refresh history. This is meaningful boundary coverage,
not a claim of exhaustive fault injection for disk-full, every crash window or every
hostile Android archive variant.

## Deviations and integration fixes

- One coherent initial SeaORM migration replaces three empty-schema migration steps.
  Domain data access remains SeaORM; SQL is confined to the migration and DB assertions.
- Thin routes and cohesive service/storage modules are grouped by responsibility rather
  than mirroring every proposed filename. Operator actions use the explicit `operator`
  task; cleanup is `artifact-cleanup`, dry-run by default.
- Local sealing uses a same-filesystem hard link with no overwrite, followed by the DB
  attempt fence. Scratch cleanup cannot remove the sealed link. S3 uses immutable UUID
  attempt keys and bounded streams. AWS cutover remains a separate verified copy job.
- Archive directory preflight bounds allocation and rejects ZIP64/multidisk before
  parsing; regular APKs are capped at 250 MiB. No archive entries are extracted.
- Hey API 0.99 required explicit bounded numeric schemas, a generated Blob resolver,
  and a narrow original-header wrapper for generated request validation. No generated
  source or equivalent handwritten DTO was substituted.
- Android Build Tools 36 reports `minSdkVersion`; parser compatibility now covers it.
  Missing JDK17/tooling is infrastructure error, not a false invalid signature.
- CLI smoke exposed that Loco CLI bypasses `Hooks::boot`. Auth configuration moved into
  `Hooks::load_config`, covering both real startup and tests. Test bootstrap is serialized
  around the initial shared DB migration, while completion concurrency remains tested.
- Source/test/config authoring preceded all checks. Reported failures were batched and
  only failed/invalidated checks rerun. The worktree lacked its Python installation;
  installing existing frozen dependencies fixed foundation smoke, and generated drift
  was rechecked with those exact tools. No Python dependency/source change was needed.
- GAN was necessarily code-only under the existing browser restriction. The score is
  provisional and does not establish rendered visual quality or keyboard acceptance.

## Remaining acceptance gates

1. Allowed rendered sign-in/create/upload/reload/keyboard/error-state acceptance.
2. Authorized Railway private-bucket round-trip, access policy, multipart-abort lifecycle,
   and deployed parser resource isolation. No hosted availability/security claim yet.
3. Device installation/preflight and execution remain specs 02/04: `not_checked` and
   `execution_ready=false`, even for a validated APK. Tests/Runs remain placeholders.
4. Track the unpatched transitive RSA advisory; audit result is not suppressed or called
   clean. Accepted-build retention and AWS data migration remain explicit later operations.

## Artifacts

- Executed plan: `../plans/completed/03-app-setup.plan.md`
- Canonical status: `../../../docs/architect/status.md`
- GAN reports: `../../../gan-harness/feedback/iteration-01.md`, `iteration-02.md`
- Operator/tool setup: `../../../docs/architect/development.md`

## Files changed

| File | Action |
|---|---|
| `.github/workflows/ci.yaml` | Modified |
| `AGENTS.md` | Modified |
| `Cargo.lock` | Modified |
| `Cargo.toml` | Modified |
| `NOTICE` | Modified |
| `apps/api/Cargo.toml` | Modified |
| `apps/api/migration/src/lib.rs` | Modified |
| `apps/api/src/app.rs` | Modified |
| `apps/api/src/controllers/health.rs` | Modified |
| `apps/api/src/controllers/mod.rs` | Modified |
| `apps/api/src/lib.rs` | Modified |
| `apps/api/src/models/mod.rs` | Modified |
| `apps/api/src/tasks/mod.rs` | Modified |
| `apps/api/tests/health.rs` | Modified |
| `apps/web/components.json` | Deleted |
| `apps/web/openapi-ts.config.ts` | Modified |
| `apps/web/package.json` | Modified |
| `apps/web/pnpm-lock.yaml` | Modified |
| `apps/web/src/App.tsx` | Modified |
| `apps/web/src/api/generated/index.ts` | Modified |
| `apps/web/src/api/generated/sdk.gen.ts` | Modified |
| `apps/web/src/api/generated/types.gen.ts` | Modified |
| `apps/web/src/api/generated/zod.gen.ts` | Modified |
| `apps/web/src/api/runtime.ts` | Modified |
| `apps/web/src/api/transport.test.ts` | Modified |
| `apps/web/src/components/ui/button.tsx` | Deleted |
| `apps/web/src/components/ui/card.tsx` | Deleted |
| `apps/web/src/index.css` | Modified |
| `apps/web/src/lib/utils.ts` | Deleted |
| `apps/web/src/main.tsx` | Modified |
| `apps/web/src/pages/Home.test.tsx` | Modified |
| `apps/web/src/pages/Home.tsx` | Modified |
| `apps/web/src/pages/Placeholder.tsx` | Modified |
| `apps/web/src/routes.tsx` | Modified |
| `apps/web/src/test/contracts.compile.ts` | Modified |
| `apps/web/src/test/setup.ts` | Modified |
| `apps/web/vite.config.ts` | Modified |
| `contracts/browser.openapi.json` | Modified |
| `crates/contracts/src/browser.rs` | Modified |
| `docs/architect/README.md` | Modified |
| `docs/architect/contracts.md` | Modified |
| `docs/architect/decisions.md` | Modified |
| `docs/architect/dependencies.md` | Modified |
| `docs/architect/development.md` | Modified |
| `docs/architect/environment.md` | Modified |
| `docs/architect/implementation/00-master-spec.md` | Modified |
| `docs/architect/implementation/01-source-setup-and-fast-dev.md` | Modified |
| `docs/architect/implementation/03-app-setup-and-ui-backend.md` | Modified |
| `docs/architect/product.md` | Modified |
| `docs/architect/status.md` | Modified |
| `docs/architect/system.md` | Modified |
| `justfile` | Modified |
| `.claude/PRPs/plans/completed/03-app-setup.plan.md` | Created |
| `.claude/PRPs/reports/03-app-setup-report.md` | Created |
| `apps/api/migration/src/m20260912_000001_app_setup.rs` | Created |
| `apps/api/src/config.rs` | Created |
| `apps/api/src/controllers/setup.rs` | Created |
| `apps/api/src/errors.rs` | Created |
| `apps/api/src/middleware/mod.rs` | Created |
| `apps/api/src/models/_entities/app_memberships.rs` | Created |
| `apps/api/src/models/_entities/apps.rs` | Created |
| `apps/api/src/models/_entities/build_uploads.rs` | Created |
| `apps/api/src/models/_entities/builds.rs` | Created |
| `apps/api/src/models/_entities/environment_checks.rs` | Created |
| `apps/api/src/models/_entities/environments.rs` | Created |
| `apps/api/src/models/_entities/login_attempts.rs` | Created |
| `apps/api/src/models/_entities/memberships.rs` | Created |
| `apps/api/src/models/_entities/mod.rs` | Created |
| `apps/api/src/models/_entities/organizations.rs` | Created |
| `apps/api/src/models/_entities/secret_references.rs` | Created |
| `apps/api/src/models/_entities/sessions.rs` | Created |
| `apps/api/src/models/_entities/users.rs` | Created |
| `apps/api/src/services/apk_validation.rs` | Created |
| `apps/api/src/services/apps.rs` | Created |
| `apps/api/src/services/auth.rs` | Created |
| `apps/api/src/services/mod.rs` | Created |
| `apps/api/src/services/uploads.rs` | Created |
| `apps/api/src/storage/mod.rs` | Created |
| `apps/api/src/tasks/cleanup.rs` | Created |
| `apps/api/src/tasks/operator.rs` | Created |
| `apps/api/tests/app_setup.rs` | Created |
| `apps/api/tests/fixtures/apk/AndroidManifest.xml` | Created |
| `apps/api/tests/fixtures/apk/README.md` | Created |
| `apps/web/src/api/setup.test.ts` | Created |
| `apps/web/src/api/setup.ts` | Created |
| `apps/web/src/components/app/apk-upload.tsx` | Created |
| `apps/web/src/components/app/app-form.tsx` | Created |
| `apps/web/src/components/app/build-status.tsx` | Created |
| `apps/web/src/components/app/environment.tsx` | Created |
| `apps/web/src/components/app/feedback.test.ts` | Created |
| `apps/web/src/components/app/feedback.tsx` | Created |
| `apps/web/src/components/app/session.tsx` | Created |
| `apps/web/src/hooks/use-apk-upload.ts` | Created |
| `apps/web/src/lib/format.ts` | Created |
| `apps/web/src/pages/AppDetail.test.tsx` | Created |
| `apps/web/src/pages/AppDetail.tsx` | Created |
| `apps/web/src/pages/Apps.tsx` | Created |
| `apps/web/src/pages/Settings.tsx` | Created |
| `apps/web/src/pages/SignIn.tsx` | Created |
| `apps/web/src/test/fixtures.ts` | Created |
| `apps/web/src/theme.ts` | Created |
| `gan-harness/eval-rubric.md` | Created |
| `gan-harness/feedback/iteration-01.md` | Created |
| `gan-harness/feedback/iteration-02.md` | Created |
| `gan-harness/feedback/mantine-review.md` | Created |
| `gan-harness/generator-state.md` | Created |
| `gan-harness/spec.md` | Created |
| `scripts/apk_fixtures.py` | Created |
| `scripts/app_setup_smoke.py` | Created |
| `scripts/setup_android.py` | Created |

## Mantine maintenance refactor (2026-09-12)

The user requested packaged controls instead of locally copied primitives and long
conditional class strings. All shell/pages/forms/status views now import Mantine
directly. `theme.ts` owns colors, typography and component defaults. Product CSS is
limited to surfaces and layout. Removed shadcn components, configuration, provenance
manifest, `cn`, the copied mobile hook, Radix, CVA and Tailwind dependencies. Historical
license attribution is retained. No API/contracts/backend changes were needed.

Source review identified and resolved accessible close labels, mobile drawer closure
on desktop resize, scrollable desktop navigation and long-text wrapping. The final
review is a provisional source pass, not observed rendered quality. Existing policy
still prevents browser acceptance. All 53 web tests, TypeScript, ESLint, Vite build,
pnpm audit and diff whitespace checks pass after the refactor. Tests preserve generated
transport coverage and add drawer Escape/focus restoration and menu logout behavior.

## Hook/state follow-up

Replaced drawer boolean setters with Mantine useDisclosure handlers and separate
form field setters with useForm/getInputProps/onSubmit for app creation, environment
editing and sign-in. Form submission snapshots values before mutations; normalization
preserves passwords and generated SDK/Zod validation remains authoritative. Extracted
upload phases/recovery/cancellation to useApkUpload, keeping server inspection before
retry. Plain presentation components no longer own the upload workflow. No new global
context or speculative memoization was introduced.

All 56 web tests pass, including new current-draft payload, password preservation and
saved-environment revision tests. TypeScript caught layout-component event typing;
semantic form elements now own submit events, with Mantine Stack used for layout.
Both affected suites (24 tests) passed again after that fix. TypeScript, ESLint,
production build, clean dependency audit and diff whitespace checks pass. The
nonblocking main chunk warning remains (749.13 kB / 227.28 kB gzip). Browser acceptance
is unchanged and no backend/contract changes were required.

## Google-only sign-in correction (2026-09-12)

Google Identity Services is the only sign-in method. The generated API contract now
includes a browser-bound one-use challenge and credential exchange. The API checks
RS256 signatures against Google's fixed public key endpoint, issuer, audience,
expiry, nonce and verified email. Existing invitations link automatically only when
Google is authoritative for the email; subsequent logins use Google's immutable sub.
Other Google email identities require explicit operator linking. No public signup
policy was introduced.

The second versioned SeaORM migration removes password hashes and revokes old
sessions without deleting apps or build history. The old password endpoint and reset
command are removed. The Mantine sign-in page uses the official Google control,
TanStack Query and a dedicated useGoogleSignIn hook, including retry and failure
states. Tests use ephemeral RSA fixture keys accepted only by the test environment.

Google OAuth client/origin setup and real Google consent remain open acceptance
requirements. Supply GOOGLE_CLIENT_ID to the API through Doppler; no client secret
is needed. See the canonical development guide. No cloud configuration was changed.

Follow-up verification passes: 57 web tests, 16 Rust tests, generated drift,
TypeScript/ESLint, Rust format/Clippy, both builds, APK HTTP smoke with synthetic
Google assertions and restart persistence, and foundation smoke. The Java launcher
initially selected Java 8; selecting the installed Homebrew JDK 17 fixed the two APK
checks. The health contract path-count assertion was updated for the new challenge
route. Only failed or invalidated checks were repeated. Frontend audit is clean;
RustSec still reports RUSTSEC-2023-0071.


## Public Google registration and owned workspaces (2026-09-12)

Google sign-in now registers unknown identities without a workspace invitation.
New accounts start pending; migration 000003 retains approved status for existing
accounts. Only approved accounts can access workspace creation, enforced again
against the current user row inside the API transaction. Operators manage approval
through the trusted process task or users.approval_status in the local database.
Workspace creation atomically grants creator ownership through the existing operator
membership, with a client UUID for idempotent retries. Existing memberships and
app permissions remain the workspace isolation boundary.

The Mantine shell includes a workspace switcher and workspace list/create screens.
The selected workspace is carried by ?workspace=<uuid>; app lists, creation,
settings and links follow it. Switching from app detail clears previous build/upload
parameters and remounts form/upload state. Invalid workspace links and mismatched
app links cannot mount product data screens. Tests/Runs remain placeholders.

Validation: 65 web tests and 17 Rust tests pass, including pending approval,
approval refresh, concurrent workspace retry, multiple workspace ownership,
cross-workspace denial, deep links, switching, history and app creation scope.
TypeScript, ESLint, Rust format/Clippy, generated drift and builds pass. Synthetic
APK HTTP smoke passes with restart persistence (0.251s fixture finalization).
Frontend audit is clean; the pre-existing RSA advisory and bundle warning remain.
Google web-client setup and real local Google sign-in have since been verified
through the user-authorized browser flow, superseding the earlier open setup note.
Hosted storage and device acceptance are still outstanding.
