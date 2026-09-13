# Implementation status and evidence

Last reconciled: 2026-09-12. This branch builds phase 05 on integrated phase 02/04
commit `b4970c7`. The authoring/library implementation and deterministic validation
have passed locally. Phase 04 dispatch/report evidence below remains valid;
phase 05 rendered browser/emulator acceptance is still pending. Historical sections
record their milestone state, not the current source inventory.

## Foundation evidence before spec 03

- Separate API/web/worker apps; root Cargo workspace and pure contract crate.
- Loco health API, generated browser SDK/types/Zod validation, structured API errors,
  and actual route/OpenAPI agreement coverage.
- SeaORM migration wiring and real local PostgreSQL integration; no product tables yet.
- React navigation and health loading/error/Retry UI; other product routes are placeholders.
- Rust worker JSON Schema, generated Pydantic and deterministic passed/failed/blocked fixtures.
- Minitap 4.0.0 installed as an optional SDK extra; import tested with connections blocked.
- Explicit setup/dev/generation/check/build/smoke commands; no check watchers. CI selects
  affected apps/contracts and cancels superseded PR runs.
- Doppler project `mobile-qa`, scoped locally to `dev`; live CLI process injection verified
  from apps/api. No secret values printed. Vite does not load env files; only the API child
  receives Doppler-fetched values during normal development.

## Historical foundation validation

The apps/contracts refactor completed 53 unique tests: 21 web, 5 Rust, 26 Python and
1 exporter synchronization test. Strict TypeScript, ESLint, Rust format/Clippy, Ruff,
Pyright, generated drift, builds and runtime smoke passed. Frontend and installed-Python
advisory checks were clean after scoped dependency fixes; the unpublished local worker
is not covered by PyPI advisory lookup.

The affected-CI follow-up verified 23 path cases with picomatch, workflow YAML/command
assertions, API-only integration/build checks and contract-only checks. The Doppler
follow-up passed the 21 affected web tests, typecheck/lint/build, recipe dry run and a
synthetic process isolation/cleanup probe. A subsequent live Doppler metadata assertion
verified the configured project/config without dumping secrets.

[Hosted workflow run for fe3cf30](https://github.com/ducnguyen67201/mobile-qa/actions/runs/34682742656)
completed successfully for scope, API, web, worker and contracts. CodeRabbit status was
also successful when inspected; this is not a claim of a separate human review.

## Open gates and planned work

Graphify refresh workflow and agent navigation guidance are authored locally. Validation
with graphifyy 0.9.58 produced 305 nodes and 399 edges across 53 source files, including
Rust, TypeScript and Python; source paths were portable and generated consumers excluded.
A health-route query returned the route and Rust contract. Repeated generation was
byte-identical for unchanged input. Actionlint passed. Local bare-Git scenarios verified
no-change behavior, output-only publication, stale-result discard and non-fast-forward
rejection. The local preview is not committed because its working files include edits
newer than the embedded HEAD provenance.

Hosted generation and bot publication remain unverified until this change is merged to
main and the first `Refresh codebase graph` run succeeds. These checks do not establish
hosted token permissions or repeat the unrelated application suites.

| Item                                                                       | Status / next evidence                                                                               |
| -------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| Spec 01 rendered browser/keyboard/Retry/HMR acceptance                     | Pending: browser tool could not verify admin policy; no bypass attempted                             |
| Spec 02 real Android/cloud qualification                                   | Local ADB and one live Minitap demo passed; full campaign and cloud qualification pending            |
| Spec 03 auth/app creation/APK upload                                       | Local implementation verified; hosted Railway round-trip and allowed rendered acceptance remain open |
| Spec 04 worker HTTP leases/runs/evidence reports                           | Planned; local fake protocol is not a scheduler                                                      |
| Spec 05 versioned case/suite/plan editor and approvals                     | Planned                                                                                              |
| Spec 06 requirements-to-tests generation                                   | Planned                                                                                              |
| Spec 07 regression, retention, production deployment and pilot reliability | Planned                                                                                              |
| Paid pilot/customer validation                                             | No accepted customer app, signed pilot or demonstrated willingness to pay recorded                   |

Doppler project creation does not provision database/model credentials, deploy a service,
or implement customer-secret resolution. Model/device performance, reset reliability,
HMR timing and production readiness must not be inferred from foundation tests.

The source foundation supports independent device feasibility (02) and app setup (03)
work with agreed ownership. Preserve spec 01's open browser gate during that work.
Current implementation reports are linked below; this remains the canonical status entry point.

## Spec 03 implementation evidence (2026-09-12)

Implemented: Google-only sign-in and app cookie sessions with revocation/origin/CSRF controls;
tenant-scoped apps/environments; private local storage and a hosted S3-compatible
adapter; real bounded Android metadata/signature validation; immutable persisted build
history; Mantine dashboard; explicit reference/observation/cleanup tasks.

Observed on the working branch: 56 web tests, 15 Rust tests and the exporter helper
regression pass; TypeScript, ESLint, Rust format/Clippy, generated drift and both builds
pass. The signed synthetic APK HTTP smoke validates bytes/hash/metadata, rejects a
foreign organization and retrieves the same build after API restart (0.145s completion
for this tiny fixture). Foundation smoke passes with a 0.621s warm built API startup.
These timings are local synthetic measurements, not customer APK or hosted SLAs.
Worker sources/contracts are unchanged; the existing fake and import-only SDK smoke
remain the only worker evidence. No device execution occurred.

The original shadcn GAN source review improved 7.27 → 7.87. After the user's
maintenance feedback, Mantine 9.6.1 replaced all copied UI primitives, `cn`, the
mobile hook and Tailwind configuration. A fresh source-only review passed after
fixing drawer close labels, resize/scroll-lock behavior, navbar scrolling and long
text wrapping. The earlier numeric scores do not evaluate this new implementation.
The post-refactor frontend validation passes: 56 tests across 5 files, TypeScript,
ESLint, production build, clean pnpm audit and diff whitespace checks. New DOM
coverage checks drawer Escape/focus return, account-menu logout, form submission
payloads and reopening the latest environment revision. State now uses Mantine
useDisclosure/useForm with a dedicated useApkUpload workflow hook; generated transport
and server reconciliation remain authoritative. No browser was
opened. Vite retains a nonblocking chunk warning (749.13 kB / 227.28 kB gzip main JS;
235.58 kB / 34.68 kB gzip CSS). RustSec flags unpatched transitive `rsa` advisory RUSTSEC-2023-0071;
App sessions use HMAC JWTs; Google identities use RSA public-key verification. See [dependencies](dependencies.md).

Hosted acceptance still requires authorized Railway bucket/streaming round-trip,
private access policy, abandoned multipart lifecycle and parser resource isolation
checks. Browser/keyboard acceptance remains blocked by the existing admin-policy
restriction; no alternate access was attempted. Device readiness remains `not_checked`
and overall execution readiness remains false.

Implementation details, deviations and final checks:
[Spec 03 report](../../.claude/PRPs/reports/03-app-setup-report.md).

### Google-only follow-up (2026-09-12)

Google Identity Services replaces password authentication. A second migration removes
password hashes and revokes earlier sessions while retaining users and product data.
Access remains invitation-only. The API verifies the Google signature, issuer,
audience, expiry, verified email and a one-use browser-bound nonce; linked identities
use the immutable Google subject. The web uses the official Google button and a
separate sign-in hook; no password form or reset-password task remains.

Current checks: 57 web tests pass; TypeScript, ESLint, Vite build, Rust format/Clippy
and generated drift pass. Google route tests cover rejected claims/signatures,
missing browser binding, replay, expired challenges and unauthorized identities.
All 16 Rust tests pass. The HTTP smoke completes synthetic Google sign-in, APK
validation and persisted retrieval after API restart (0.147s fixture finalization);
foundation smoke passes (0.211s warm API startup). These are local synthetic results. Frontend audit is clean; the existing
unpatched RSA advisory remains. Main JS is 752.46 kB / 228.30 kB gzip with the existing
nonblocking size warning. Prior baseline counts above are historical.

Real Google account consent is unverified: configure a Google web client and its
origins, and inject GOOGLE_CLIENT_ID through Doppler. No Google/cloud configuration
or browser access was changed during this implementation.

### Workspace onboarding follow-up

Source now supports Google registration without invitation, approval-gated workspace
creation, multiple owned workspaces and URL-based selection. Migration 000003 adds
account approval and preserves existing access. Scoped listing, invalid workspace
links, app/workspace mismatches and ownership checks are covered by new route/DOM
cases. Validation passes: 65 web tests, 17 Rust tests, TypeScript, ESLint,
Rust format/Clippy, both builds and generated-contract drift. The synthetic APK HTTP
smoke verifies tenant denial and persistence after API restart (0.251s finalization).
Frontend dependency audit is clean; the existing RustSec RSA advisory and Vite bundle
size warning remain. The earlier invitation-only descriptions above are historical.

A local Google OAuth web client has now been configured through the user-authorized
browser flow, with GOOGLE_CLIENT_ID injected through Doppler. Real Google sign-in
was verified against the local app. This supersedes the earlier unverified Google
setup and browser-access notes; hosted storage and device acceptance remain open.

The restarted local app was verified through real Google sign-in: the existing
account lands on its workspace URL, the chooser shows its owned workspace, and
the approved Create Workspace form renders. Creation/switching across two workspaces
and pending-account gating are covered by automated route and DOM tests.

### Sidebar workspace controls

Workspace selection now lives in the sidebar. Desktop navigation minimizes to an
80px icon rail, with tooltips, a workspace menu and an expand control; expanded
navigation is 256px. Mantine disclosure state is separate from the mobile drawer.
Browser verification covers collapse, workspace menu selection and expansion while
preserving the selected URL. All 65 web tests, TypeScript, ESLint, production build
and frontend audit pass. The existing nonblocking bundle warning remains.

Settings and the account identity/sign-out menu now sit at the bottom of the
sidebar, including its minimized layout. The account menu was verified in the
browser; 65 web tests, TypeScript, ESLint, build and frontend audit pass.

## Phase 02 implementation and offline evidence

The operator qualification harness, Rust-owned contracts, generated Python models,
controlled good/broken Android demo, pinned tool installer and runbook are implemented
on `feat/02-cloud-phone-and-feasibility`. The qualification protocol is separate from
the fake fixture protocol and the future production worker leases.

Observed locally after complete authoring: 80 worker tests (54 new), 6 contract tests,
10 CI selection cases and the exporter synchronization test passed. Ruff, strict
Pyright, Rust formatting/Clippy, generated drift, worker packaging, network-blocked
Minitap import and both Android APK builds passed. Android lint checks both debug
flavors; expected fixture/version warnings remain. No API/web runtime source changed;
their full suites were not rerun for this phase. Hosted CI has not run on this branch.

At that initial checkpoint, no cloud host, emulator execution, model call or real device
evidence had been produced. The native local-device follow-up below supersedes the local
device status; it does not close Minitap/cloud qualification.
Phase 02 remains unqualified and phase 04 real execution remains gated. Host access or
region/AMI/network, the approved model, current rates, spend and shutdown deadline are
still operator inputs. A launch template is not an approved launch packet. Phase 03
app setup can proceed independently.

See the [implementation report](../../.claude/PRPs/reports/02-cloud-phone-and-feasibility-report.md)
and authoritative [device qualification runbook](device-qualification.md). The PRP plan
remains unarchived until task 12 has real qualification and teardown evidence.

## Hosting direction follow-up

The user selected Railway for the initial application deployment and AWS for a later
migration when needed. [Hosting](hosting.md) records the service layout, portability
constraints and deployment work still to implement. Railway KVM/device execution is
not verified; selecting Railway does not close the phase 02 device-host gate. This
follow-up changes documentation only and has created no provider resources.

## Native local-device follow-up

The runner now has macOS ARM64/Intel and Linux x86_64 host selection, pinned native
Android images, platform acceleration checks and real boot identity for crash recovery.
Explicit device-local setup/build/run commands share the supervisor, assertions and
cleanup with cloud qualification. Default ADB demo navigation uses a real emulator
without model calls; explicit agent mode uses Minitap and Doppler. Reports identify
which navigation path ran. API worker leases and customer APK execution remain planned.

Observed on macOS 15.7 / ARM64: all 100 worker tests, strict Pyright, Ruff, 11 CI scope
cases, both demo APK builds and Android flavor lint passed after the implementation.
Native doctor verified the pinned Android API 35 ARM64 image and Hypervisor.Framework.
Real ADB demo outcomes (reports remain private under `.private/artifacts/local-device`):

| Scenario                 | Outcome                  | Reset          | Attempt                                |
| ------------------------ | ------------------------ | -------------- | -------------------------------------- |
| Good APK                 | passed                   | verified_clean | `4b52a6a0-d97a-4b3a-9362-af43bc4c0d67` |
| Broken APK               | failed                   | verified_clean | `06e29435-7652-4a79-bc91-c36ca7c483c7` |
| Backend unavailable      | blocked                  | verified_clean | `76de6d90-8ef9-435f-945c-faad66fc487c` |
| SIGINT during navigation | inconclusive / cancelled | verified_clean | `cb812944-46d7-4497-9ab2-f957a6d41a7e` |

The interruption probe signalled the actual local launcher after navigation started,
waited for its result, and verified the dirty marker was absent and emulator ports
were available. The wrapper replaces itself with the supervisor so it cannot kill
that supervisor while Ctrl-C cleanup is in progress.

The local runs exposed and corrected Mac acceleration-output parsing, timezone/locale
handling, unattended emulator prompts, stale TCP connection checks, fixture-network
startup races and input focus timing. Earlier failed attempts were retained; completed
ADB-only quarantine was recovered explicitly using matching result/profile evidence.
Cold boots in these accepted runs took about 45–46 seconds; full reset took 55–57 seconds.
These timings describe this Mac, not a cloud capacity promise.

No model calls or cloud resources were used. Minitap mode is implemented but its live
11-attempt qualification remains pending, as do HTTP worker leases and dashboard jobs.

## First live Minitap demo

On 2026-09-12, a visible macOS ARM64 demo using Minitap 4.0.0 and OpenAI gpt-4.1
passed the create/save/reopen persistence test and verified a clean device reset.
Credentials were injected from Doppler `mobile-qa / dev_personal`; no key was stored
in the repo. Attempt `ee08c402-43ae-40d3-b462-cac4753356da` recorded 8 model calls,
34,817 input tokens and 941 output tokens, with no unknown usage calls. Private
screenshots, video, SDK traces and the report remain under `.private/artifacts/local-device`.

The preceding attempt `6e3fb69e-e33f-4ebc-91a5-ead5011f7c31` was inconclusive due to
Minitap/LangGraph ToolRuntime incompatibility; its clean reset passed. It made 5 model
calls (22,297 input / 605 output tokens), retained in its SDK child result. Parent
inconclusive reports currently omit usage when the SDK process exits nonzero; that
failure-path usage aggregation remains an accounting limitation.

A typed, version-guarded SDK compatibility subclass now supplies the required config,
tools and execution context without changing installed packages or dependency locks.
Strict Pyright, focused Ruff and all 4 adapter tests passed, including a real offline
Minitap tool execution through LangGraph. See dependencies.md for scope and removal.
This single live demo does not complete the full 11-attempt reliability campaign,
cloud-host qualification, customer APK execution or dashboard job integration.

## Main dashboard integration

Merged main `9eb5dba` into the device branch, preserving apps/web and apps/api byte
for byte from main and the worker/demo sources from the verified device branch.
Shared commands and architecture records retain both workflows. No UI run endpoint
or worker lease API exists in main; Tests/Runs remain placeholders. No customer
upload is silently routed through the controlled-demo qualification protocol.

Merge validation passed: 65 frontend tests, 101 worker tests, 6 Rust contract tests,
exporter synchronization test, strict TypeScript/Pyright, frontend/worker lint,
production UI build, zero generated-contract drift and 15 combined CI scope cases.
Frontend and Python dependency audits found no known vulnerabilities. The existing
Vite chunk-size warning remains. API code is unchanged from main; API runtime tests
and billed/device runs were not repeated for this merge. Historical API and device
validation above retain their original scope.

## Phase 04 implementation work

Local source on `codex/04-execution-and-reports` adds versioned operator test imports,
approval grants, immutable run manifests, worker HTTP leases/reservations, private
checkpoint evidence and Runs/read-only Tests UI. Local validation passed: 29 Rust
unit/route/contract tests, 69 frontend tests, 115 worker tests, strict type/lint checks,
Rust/web builds, zero generated-contract drift and 17 CI scope cases. Test totals
include targeted reruns after fixes; they are not a single uninterrupted run.

`just smoke-execution` passed through real HTTP, PostgreSQL and the Python worker:
queued-job restart, simulated passed/failed/blocked evidence, and identical saved
reports after API restart. The separate foundation `just smoke` could not start
because existing servers occupy 5150/5173; those processes were left running.
No manual browser, model/device, cloud or hosted S3 acceptance ran for this phase.
The frontend production audit found no known vulnerabilities; Cargo audit still
reports the previously documented `RUSTSEC-2023-0071` in rsa 0.9.10.

The [implementation report](../../.claude/PRPs/reports/04-execution-and-reports-report.md)
records changes, validation, deviations and remaining gates. The plan remains active;
this is local execution/reporting implementation, not phase-wide acceptance.

Phase 04 follow-up: bounded long-poll claims and an explicit real-worker launch
recipe are implemented. Validation passed: seven execution route/database tests
(including wake-on-commit, idle timeout and revocation), the health route test,
15 worker execution tests, Rust build, Clippy, Ruff/Pyright, formatting and the
simulated HTTP/restart smoke. Real emulator/API acceptance remains pending; this
change does not run Minitap or provision a device.

## Real API-to-emulator acceptance — 2026-09-12

Run `5f942465-1661-4f7b-ad3c-7b555cb61a37`, attempt
`ecffe2bf-173d-4328-b44e-a94b6e6ff56c`, passed through the current Rust API,
long-poll Python worker, visible macOS ARM64 emulator and Minitap/gpt-4.1.
The real good demo APK was uploaded through normal intake. The worker was started
before HTTP run submission; it claimed the queued attempt, created the unique task,
restarted the app and published four checkpoint artifacts. Rust independently
recorded both created and persisted checks as passed. Cleanup was verified_clean;
the worker exited 0, emulator ports were released, and no dirty marker or active
AVD remained. The owned test API was stopped; existing development servers were
left running.

The SDK recorded 10 calls, 46,829 input tokens and 1,258 output tokens, with no
unknown usage calls. Model credentials were injected only into the SDK child via
the existing Doppler configuration. The test API used isolated synthetic identity
and worker credentials; device execution and model calls were real.

Private report/evidence: `.private/live-execution/aad6f00b-7dff-4174-852f-00e5f5aebf87/`
(`api-report.json` plus the attempt's evidence directory). This proves one good-path
backend-to-device run, not manual browser acceptance, broken/unavailable scenarios
through this new path, cancellation under live navigation, full reliability or
hosted storage acceptance. Those gates remain open.

## Phase 05 authoring implementation

Draft catalog, immutable submission/review, atomic mutation retries, archive and
explicit default-plan source are implemented with generated browser contracts.
The Tests UI connects case/suite/plan editing to existing execution preview and
reports. Validation passes: 90 web tests, 42 Rust tests, 117 worker tests, one exporter
helper test and 20 CI scope cases. TypeScript, ESLint, Clippy, Ruff, Pyright,
generated drift and both builds pass. The HTTP smoke authors and reviews a case,
suite and plan, selects an explicit default, runs the fake Python worker after
API restart and preserves passed/failed/blocked reports plus the queued manifest
after a later draft edit. See the [phase 05 report](../../.claude/PRPs/reports/05-test-library-and-plans-report.md).
The GAN review is source-only; rendered layout and the new UI-to-emulator flow
remain unverified under the existing browser restriction.

### Phase 06 task sessions — in progress

Task-first source is being implemented on `codex/06-task-sessions`; see
[the plan](../../.claude/PRPs/plans/06-task-sessions.plan.md). No completion or real
UI-to-Minitap acceptance is claimed yet. This checkout retains local phase 05
changes; GitHub PR #5 was the phase 04 execution/report PR.

Phase 06 primary task-session source and real API-to-Minitap acceptance now pass:
run `a8101844-8f33-4292-b093-8dd02e719b1c` saved two different texts through the actual
sample APK, including a selected-control task, and closed its owned emulator.
Generated contracts, 43 Rust tests, 93 browser tests and the worker checks passed
(with one existing local HTTP test requiring a successful targeted rerun).
See [the implementation report](../../.claude/PRPs/reports/06-task-sessions-report.md).
Rendered browser acceptance, persistent local worker credential/registration,
restart-goal acceptance and the broader generation/discovery scope remain open.
