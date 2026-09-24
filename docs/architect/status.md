# Implementation status and evidence

## Elastic Android hosts and large APKs — local implementation (2026-09-22)

The source now includes inclusive 2 GiB intake, resumable multipart transfers,
durable validation, scoped signed delivery and a bounded verified host cache.
The new host supervisor owns qualified direct-execution slots; the API owns
boot/claim/drain fences and durable power intents. OpenTofu, Packer, systemd and a
Lambda controller define the separately operated fixed EC2 pool. Multipart intake
is enabled automatically for remote storage, with no environment toggle. Automatic
capacity remains disabled by default; warm target is zero.

Local validation covers 80 API tests against disposable PostgreSQL databases,
24 pure contract tests, 243 worker tests, 38 controller tests, three deployment
checker tests, generated contract drift and 36 CI scope cases. Browser validation
passed the 189-test initial suite plus 31 targeted rollout regression tests after
fixes. Rust Clippy, Python type/lint checks, browser type/lint/build checks,
OpenTofu/Packer configuration checks and Linux C compilation passed. A durable
validation stack-overflow regression was fixed with bounded heap buffers and a
boxed job future; its normal-stack recovery test now passes. Both Docker images built locally. The API image passed a non-root, network-disabled
smoke of its binary, Java/APK validators, static assets and writable scratch; the
controller image passed an offline import/memory smoke. Details are in the report.

No AWS resources or paid AMIs were created. Railway multipart/CORS interoperability,
real near-2 GiB installation, Linux/KVM/ADB/rendering and guest packet isolation,
warm reset, density/cost measurements and a hosted stop/start cycle remain release
gates. Supervised pools currently support qualified generic direct execution;
the standalone Minitap demo needs an additional supervised adapter and qualification.
The plan remains active for these gates. See [spec 11](implementation/11-elastic-device-hosts-and-large-apks.md),
[operations](device-host-operations.md), and the
[implementation report](../../.claude/PRPs/reports/elastic-android-device-farm-and-large-apks-report.md).

## PR #16 review hardening — local verification (2026-09-22)

Review fixes keep one checkout intent and Stripe idempotency key after an
uncertain response, and require Stripe-confirmed session expiry before another
checkout can be created for the app. A checkout with no known session after 24
hours now needs billing review. The invoice path locks the app before the
checkout intent, matching plan changes; unexpected one-phase Stripe schedules
are left for billing review. Webhook signature timestamp validation no longer
overflows on extreme untrusted values. Legacy agreement totals include all
usage rows while the visible activity list remains limited to 100. Saving an
edited suite preserves its quote when the editor updates its saved-version
props; obsolete pilot copy and an unused browser helper were removed.

Generated contract drift, Rust Clippy, the API binary build, web formatting,
typecheck, lint, all 167 web tests, and the web bundle passed. Six commercial
integration tests passed on a disposable database with the configured JDK 17;
the webhook timestamp unit test passed. The signed-in Stripe plan-change click
path and an actual paid renewal remain unverified.

## Next-renewal plan changes — local implementation (2026-09-22)

The source adds an app-scoped plan-change endpoint, a Stripe subscription schedule
for the next period, pending-plan display and cancellation, and paid-invoice
validation that grants the new allowance only on confirmed renewal. The current
paid period and its usage ledger remain unchanged. Six commercial route/fixture
tests passed against a disposable PostgreSQL database, including the pending
renewal, underpayment, replay and access paths; all 166 web tests, generated
contract checks, Clippy and the web bundle passed. Stripe test mode accepted
a future-phase schedule and a separate Plus-named product/price request. The
temporary schedules were released and the test product was archived; a
read-only follow-up confirmed the existing Starter subscription retained its
price and has no schedule. The local API loaded migration 000016 and serves the
new authenticated endpoint. Restarting the API invalidated the browser session;
the signed-in plan-change click path awaits reauthentication. The full API test
command remains blocked by an older unrelated migration in the shared test
database. An actual paid renewal under a test clock, hosted plan change and
production deployment remain unverified.

## Paid suite execution and credit settlement — local verification (2026-09-22)

The current-format saved suite `ecfe8f15-5dff-44df-87d9-e6b5071e82ed`
ran against the validated demo APK in local Stripe test mode. Run
`09e56244-5679-433f-931c-b21dc601609e` passed all three required checks:
entering "Buy milk", saving it, and finding it after an app restart. The run
report exposes six sealed evidence artifacts and records verified clean device
cleanup. Operator review marked the report delivered. The 3,302-credit maximum
hold settled at 118 measured credits (117 device seconds, 288,140 sealed
evidence bytes, zero provider tokens), returning 3,184 credits. The Starter
period and Settings page both show 50,000 granted, zero held, 118 used, and
49,882 available. Two earlier pre-delivery attempts, one with a missing build
artifact and one missing the worker's optional device dependency, were released
at zero charge after recovery review. This verifies the local execution and
settlement path; hosted delivery, renewal, and unit economics remain open.

## Test-library catalog compatibility — local fix (2026-09-22)

The paid-preview local database contains five suites, including two drafts with
newer membership fields that this checkout does not parse. Those records caused
the Suites list to return 500; newer saved versions and an unsupported sequence
also caused the options request to return 500. The catalog now keeps incompatible
entries visible and read-only, excludes incompatible versions from selection,
and returns an explicit unsupported-format error on a detail read. It does not
rewrite or delete the saved records. Six test-library integration tests and all
164 web tests passed; Rust Clippy, web typecheck and lint passed. A read-only
probe against the actual local app database loaded all five suites, marked the
two newer-format drafts read-only and returned usable version options. API and
web are running locally; the newer-format notice and both saved-version links
were confirmed in the signed-in browser. The phone worker reported
`worker_claim_recovery_required`; its recovery journal was preserved. A
separately started execution worker later completed the suite run above.
The newer-format "Demo sequential smoke suite" has two compatible saved versions
referenced by 12 historical runs, so it is retained as useful history. Its
current draft cannot be edited in this checkout; the detail page now presents
a specific newer-format notice while retaining its saved-version links.

## Monthly credit plans — local implementation (2026-09-22)

The local source now defines Starter (US$500/50,000), Plus (US$750/75,000)
and Business (US$1,500/150,000) monthly cards, with one direct Checkout action
each and no pilot questionnaire. New database tables record Stripe checkout
intents, paid invoice grants, credit quotes and run holds. A signed
`invoice.paid` event is required to create a grant. The rate card uses measured
provider tokens, server-recorded device occupancy and sealed evidence bytes.
After operator report review, the service settles measured credits up to the
held cap and returns unused credits; a queued pre-claim cancellation releases
the hold. The monthly allowance resets without rollover.

Stripe test-mode Checkout credentials and local CLI webhook forwarding are
configured through Doppler in this environment. One sandbox Starter checkout
completed on 2026-09-22. The first `invoice.paid` delivery was acknowledged
without a grant because the current Stripe Invoice payload supplies
`status: paid` rather than the older `paid: true` field. After correcting that
parser, replay of the signed Stripe event marked the checkout intent paid and
created a 50,000-credit period for the selected app in the local database.
Hosted webhook registration, a live payment, renewal event, customer-app meter,
unit economics and hosted deployment remain unverified. Existing fixed-check
agreements remain as legacy records.

`just types` and `just check-contracts` passed with zero generated drift.
`just check-web` passed formatting, TypeScript and lint; its only test failure
was an ambiguous text selector, and the corrected paywall test passed. The
Rust workspace check passed on a fresh disposable PostgreSQL database with
process-scoped JDK 17, including five commercial route tests. A later focused
commercial campaign passed the measured 1,061-credit settlement, duplicate
invoice, underpayment, hold and cancellation cases after the final meter
change. The web production bundle also built. Those initial checks did not
include a device run; the later local execution is recorded above.
A focused regression test now covers the current
Stripe Invoice shape, duplicate delivery, underpayment and out-of-band payment
rejection. The later full API check compiled and passed Clippy, but its shared
test database contained a migration from another branch. With an isolated
database and the JDK 17 `Contents/Home` path, all seven app setup tests passed.
The combined workspace test run then hit cumulative sign-in rate limits; on a
fresh isolated database, all five commercial tests passed.

## Commercial access and usage — local implementation (2026-09-21)

The proposed operated-service pricing is represented locally by app-scoped pilot and
recurring agreements, one qualified profile and saved coverage allowlist, short-lived
server quotes, atomic per-run check reservations, a customer usage ledger, manual
operator review/credit commands, and a pilot-request record. New release-plan and
saved-suite runs require explicit quote authorization. A queued run canceled before
device claim is credited. Standalone saved-case execution and interactive phone/model
entry points require separately agreed scope. The Settings pricing page distinguishes
proposed terms, agreement state, reserved checks, delivered charges and credits.
The legacy API still retains manual pilot-request and fixed-check routes for
existing agreements. The customer page now presents the three monthly plans,
the paid allowance, and a credit activity table; it no longer asks for a pilot
coverage note or shows the per-check estimate. New credit runs still require
an exact quote authorization on a run screen.

`just types`, `just check-contracts`, `just check-web` (164 tests), `just check-api`
(including three new real-route commercial tests), and `just build` passed in the
isolated worktree. API tests used a fresh disposable PostgreSQL database, locally
available signed synthetic APK fixtures and process-scoped JDK 17. The retained
shared test database still has a stale migration ledger; it was not modified for
this work. The tests cover pilot inclusion and cap, concurrent last-slot requests,
idempotent replay, pre-claim cancellation credit, foreign access, resource bypass
denial, recurring quote/base and closing an expired period. They do not establish
real-device quality, a usable customer report or sustainable unit economics.

Hosted enforcement has not been deployed. Existing hosted-app backfill, customer
agreement, real-device/report review, measured operator and provider costs, and the
pilot gates below remain open. Rendered visual inspection remains blocked by the
recorded browser admin-policy denial; no alternate browser path was used.

The refreshed commercial page passed web formatting, typecheck, lint, all 164 web
tests and a production web build on 2026-09-22. Its pilot dialog and estimate
were verified in component tests; rendered visual acceptance remains open.

## Reliable smoke suite — local implementation (2026-09-21)

Saved-suite run setup now requires an explicit build and qualified device,
shows the chosen build checksum, offers completed-run baseline choices without
preselecting a suggestion, and pins the selected baseline on the immutable
multi-case run. The API shares candidate scanning with saved-case runs, enforces
the same-app completed baseline rule, and keeps worker pickup in the existing
fenced claim path with stable tie-breaks. Saved suite cases are independent:
the editor's display order no longer sets pickup order, and the coverage map
does not draw case-to-case execution arrows. The report names the saved suite
and distinguishes required
passing checks from optional failures and missing real-device proof. Older
suite requests without a baseline remain valid and keep their idempotency bytes.

Contract drift, scoped formatting, API Clippy/workspace tests, 161 web tests,
185 worker tests and the production build passed. The API test pass used a
disposable PostgreSQL container and process-scoped JDK 17 because the retained
test database has a stale migration ledger and the default Java is 8. Existing
test-library and execution HTTP smokes, plus the new two-case saved-suite HTTP
smoke, passed against a fresh migrated disposable database. All three use the
fake worker and intake-only APK; they prove routes, independent case pickup,
idempotency,
baseline pinning and restart persistence, **not** Android verdicts.

The clarified suite picker passed a focused real-database route test using a
fresh temporary PostgreSQL instance: the second displayed case was claimed
first, a concurrent claim remained blocked by the reservation, and recovery
held the app until verified. The two-case simulated HTTP suite smoke passed
again after the change. This checkout's local JDK setting was corrected to
the JDK 17 home directory for APK intake validation.

The real good/broken two-case harness is implemented as the explicit `--suite`
mode of `scripts/regression_acceptance.py`. It has not run in this delivery.
During the original validation, another worktree owned the API and workers for
the same physical host profile. That stack has since been stopped cleanly and
this checkout's local API, web and workers have connected, but device state and
recovery history still need operator verification before real acceptance.
The previous rendered-browser
admin-policy denial remains open. There are no new real suite run IDs or device
evidence to report.

## Execution stall recovery hardening — local changes (2026-09-21)

The real execution worker now checks the Android command-line tools before claiming a job and passes the configured JDK to its supervised child. An AVD setup failure before an emulator has ever launched may finish with verified clean cleanup after the owned directory and free ports are checked. Once an emulator has launched, uncertain cleanup remains quarantined and requires operator recovery. The previously held local attempt was explicitly recovered after confirming no active process, dirty marker or current AVD; a later queued attempt exposed a Java 8 selection and left a new dirty AVD state. A fresh host boot and operator recovery are still required for that new attempt. Source and offline tests do not establish a successful live suite run.

## Model catalog capacity — locally validated (2026-09-21)

An additive model assignment revision and audit store, protocol-6 qualified worker
set, compatible claim selection and computed run queue reasons are implemented on
this branch. New navigation and structured-authoring work resolve active assignments
independently of the physical device profile and freeze them in run/session payloads.
The worker accepts the old ADB-only `no-model-adb-demo` sentinel as model-free, so
that private historical profile does not abort startup. A pre-existing
recovery-required reservation still requires explicit operator recovery before any
worker can claim the physical phone.
Queue creation logs the bounded reason code; run details refresh the reason as worker
and reservation state changes. Direct-only runs no longer show a legacy model warning.
Review hardening requires one live phone worker to support both assigned purposes and
counts a worker as run-compatible only when its execution poll protocol can claim the run.

Contract drift, formatting, Clippy, Rust workspace tests (65), web typecheck/lint
and 153 web tests, Ruff/Pyright and 182 worker test cases pass across the consolidated
pass and focused retries. The web production build and Cargo workspace build pass.
The API used a disposable
PostgreSQL container because the retained local test database has a different branch's
migration ledger; signed synthetic APK fixtures were generated with local Android
tools. The route tests cover immutable assignment revisions, distinct phone authoring
models, compatible claims, and offline/mismatch/busy/recovery queue reasons. No live
device/provider qualification or rollout was performed. Assignment `max_calls` is
recorded but not yet enforced as a runtime stop condition.

## 07B regression labels and history — local implementation (2026-09-20)

The [focused delivery](implementation/07b-regression-label-and-history.md) now creates
saved-case runs through the existing execution pipeline, with explicit build selection,
an immutable test version and pinned baseline. Runs lists durable executions and
creator-private authoring trials; legacy phone activity has its own filter and no
inferred regression verdict. The backend persists comparisons; the shared result card
shows the label, failed action, expected/observed values and baseline/current evidence.

Execution protocol 4 gates saved-case manifests. Legacy release-plan manifests remain
unchanged. Comparison requires conclusive checks and verified clean start/cleanup on
both sides; old demo phone results cannot establish that proof. The local real-device
fixture acceptance and automated validation are recorded in the specification report.
Real local fixture acceptance passed: good → Passed, broken → Regression at the
restart check, repeated broken → Still failing. Reports and pinned baselines stayed
unchanged after API restart and all runs appeared in history. The fixture used an
isolated test workspace, not the existing demonstration workspace. 59 Rust, 127 web
and 181 worker tests passed across the consolidated pass and focused retries; static
checks, generated drift and builds passed. Full authenticated rendered acceptance is
still pending sign-in after the dev restart. Audited triage, release-baseline
management, retention and fleet work remain planned.

## Save without human reviews (2026-09-20)

Implemented Edit → Save → Run for cases, suites and release plans, including selected
AI proposals and templates. The current entry stays editable. Complete Save creates
or reuses an immutable snapshot; incomplete content persists with Needs setup. Review
buttons, capabilities, routes and operator approval commands are removed. Membership,
CSRF, revisions, technical admission, archive permissions, exact pins and defaults remain.

Migration 000008 retains prior review states and events as legacy audit, without
inventing approvals. Historical content hashes, report payloads and run manifests stay
unchanged. Old mutation IDs fail closed under the new save namespace; refresh clients
after upgrade. This supersedes the historical phase 05 review workflow documented below.

Validation: 50 Rust, 119 browser and 175 worker tests pass across the consolidated run
and targeted retries. Formatting, Clippy, TypeScript/ESLint, Ruff/Pyright, generated
contract drift and API/web builds pass. All three simulated HTTP smokes pass:
test-library, direct-authoring and execution (operator imports). They preserve exact
retry results, pinned manifests, pass/fail/blocked reports and API restart persistence.

The shared Compose test database contained another branch's migration. Validation
used a disposable PostgreSQL container, the same fixed test database name and an
external temporary test configuration; no development data was reset. JDK 17 was
explicitly selected for APK intake checks. Tests that ran before container readiness
passed after it was ready; one existing worker loopback HTTP test passed on retry.
Vite still reports the existing large-bundle advisory. No real device/model calls or
rendered browser acceptance were performed for this change.

See [spec 05](implementation/05-test-library-and-plans.md) for the active behavior.

## Phase 07A — qualified clean-start replay (2026-09-15)

07A source and synthetic validation are complete. The new `android_direct_v1`
profile declares one local-state app, Android35 image/ABI, launcher, fixed geometry,
reset-policy/runtime/verifier revisions, stage limits and required starting assertions.
Profiles remain immutable and operator-owned. Missing legacy context stays unknown.

The worker retains clean-start PNG/XML, waits for the API's fenced acknowledgement,
then executes direct commands without model calls. Start and cleanup have separate
budgets. Missing start proof or uncertain cleanup quarantines the reservation; no
mutation is retried after an uncertain response. Recovery appends a record without
replacing the initial cleanup receipt or changing the machine verdict.

The generic Android boundary owns install/launcher verification, fresh AVD disposal,
package-aware captures and shared password masking. The demo wrapper retains its
fixture backend and persistence reset oracle. Phone protocol4 and execution protocol3
protect old workers from new profile fields. AI discovery remains demo-qualified;
this increment does not establish real-app exploration or remote account reset.
Phone-session admission also binds the host model to the registered model for
AI-capable profiles before acquiring the device or making network calls. Model-free
direct profiles do not require a host model match.
Review hardening preserves a pre-existing AVD until explicit operator recovery,
expands `${task_title}` consistently with the server verifier without changing the
frozen manifest, and keeps server quarantine visible even when local disposal succeeded.
Protocol 4 is admitted by both the command queue and phone UI; direct commands and
AI discovery retain their existing protocol 2/3 compatibility limits.

Run details show a compact clean-start/cleanup strip with evidence behind a disclosure.
The GAN evaluator accepted the compact readiness design at 7.98/10 (threshold 7.5).
This assessment is source/DOM only; rendered browser inspection remains restricted.
Actual non-demo APK qualification, build comparison, pilot metrics and the reliability
campaign remain pending in 07B–D. No customer APK was supplied for this acceptance.

Validation: generated-contract drift, formatting, Clippy, TypeScript/ESLint,
Ruff/Pyright and API/web builds pass. 49 Rust tests, 116 browser tests and 159 worker
tests pass across the consolidated run and targeted retries. The pre-existing loopback
HTTP worker test failed intermittently before passing alone; no transport behavior was
changed to mask it. The CI-scope tests and contract-export helper pass. Both simulated
HTTP smokes (`just smoke-execution`, `just smoke-direct-authoring`) preserve reports
through API restart, including direct authoring and reviewed immutable manifests.
Review follow-up: 50 Rust tests, 120 web tests and 175 worker tests pass, with
formatting, Clippy, TypeScript/ESLint, Ruff/Pyright and API/web builds. The final API
rerun hit the shared test database's sign-in rate limit; all six test-library tests
passed on targeted retry after that window expired. Production limits were unchanged.
The simplification pass removed duplicate plan/profile loading and a repeated readiness
label branch. Earlier contract-drift and simulated HTTP smoke evidence still applies;
this follow-up changes no transport shapes.
These checks use no emulator or model calls and do not prove real-device qualification.

Implementation report and remaining gates are in the owning
[phase 07 specification](implementation/07-pilot-readiness-and-scale.md#07a-implementation-notes--2026-09-15).
Temporary GAN specifications and evaluation files were removed after applying the review.
The Phase07 plan remains active; the later 07B–D operational deliverables remain open.

## Full local development supervisor (2026-09-15)

`just dev` now builds the API once and starts PostgreSQL, API, Vite, phone and execution
workers. `dev-stop`, `dev-restart` and `dev-logs` manage this stack. Each API/worker process
gets its own Doppler injection; build tools and Vite receive an allowlisted environment.
The existing registered local device profile is configured in ignored `.private/dev/config.json`.
Logs are private per run, with `.private/dev/latest` pointing to the current folder.
The phone worker advertises protocol 3 and drains pending idle claims on shutdown.

Four supervisor policy tests, 149 worker tests, Ruff/Pyright and 28 CI scope cases passed.
Real local startup authenticated both workers and returned HTTP 200 from the API, Vite
and the proxied Google login challenge. Full restart preserves database contents and
worker recovery journals. This verifies local orchestration, not browser rendering,
arbitrary APK qualification or an additional paid AI discovery run.

## Spec 08 — Minitap flow discovery (2026-09-14)

[Spec 08](implementation/08-minitap-flow-discovery.md) is implemented on
`codex/08-minitap-discovery`, based on merged main `0aaa20f`. Minitap's pinned graph explores
through parent-owned observations/direct tools, with protocol 3, acknowledged action
intents, immutable outcomes, measured call budgets and recorded-path proposals.
The UI starts/stops exploration explicitly and saves editable drafts for direct replay.
The old structured-model decision loop is removed; legacy payloads remain readable.
Strict generated draft schemas use action numbers; code constructs replay and evidence IDs.

Validation: 148 worker tests (including the real pinned SDK graph with synthetic model
responses), 103 web tests, API integration/route tests, Rust contract tests, generated drift,
Ruff/Pyright, ESLint/TypeScript, Clippy and production builds passed. The HTTP lease route
covers protocol 2/3 compatibility, intent-before-effect, immutable receipts, chronological
redraw observations, invalid proposal paths, idempotent draft saving and lease expiry.
CI path selection passed 26 cases. One local HTTP fixture timed out under concurrent checks;
its isolated rerun and the subsequent full worker suite passed.

Real sample acceptance passed under Minitap 4.0.0 / gpt-4.1 on the qualified Android 35
arm64 emulator. Attempt `6eff28bb-b289-4b34-a31b-1c0ed65aa4d2`, generation
`6784f747-84a2-4aec-89a9-dd9d24751e15`, recorded `set_text("Hello")` then `tap(Save)`
and proposed **Save task 'Hello' and verify it appears in the list**. A named draft was
saved through the real API. Direct replay passed an independently supplied Hello assertion;
a second replay correctly failed a deliberately wrong assertion. Discovery used 9 calls,
19,111 input tokens and 993 output tokens, with zero unknown calls; both replays added
zero model calls. Private evidence is under `.private/task-session-acceptance/`.

Earlier live attempts rejected invalid draft references and exposed passive redraw handling;
those failures led to action-number draft contracts and chronological observation validation.
They are not counted as successful acceptance. Same-session sample replay does not establish
clean-reset replay or arbitrary APK support. Rendered browser acceptance remains open under
the existing browser access restriction. No old PRP/GAN report trees are retained.

## Earlier implementation evidence

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
| Spec 07 regression, retention, production deployment and pilot reliability | First regression/history delivery implemented locally; later operational phases planned              |
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
[phase 03 specification](implementation/03-app-setup-and-ui-backend.md).

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

See the [phase 02 specification](implementation/02-cloud-phone-and-feasibility.md)
and authoritative [device qualification runbook](device-qualification.md). Real qualification and teardown evidence remain required.

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

The [phase 04 specification](implementation/04-execution-and-reports.md)
defines the execution/reporting scope and remaining gates;
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
after a later draft edit. See the [phase 05 specification](implementation/05-test-library-and-plans.md).
The GAN review is source-only; rendered layout and the new UI-to-emulator flow
remain unverified under the existing browser restriction.

### Phase 05 coverage-flow follow-up — locally validated (2026-09-20)

The Tests editor, frozen versions and run detail now project their existing ordered
suite/plan/manifest data through a non-connectable React Flow control surface. The
browser never persists layout or creates case dependencies. Authors add saved cases and
suites directly from the canvas; each selection appends the next numbered step, while
arrowed case rails make sequential execution order visible. Explicit move/remove/required
controls and one Save action remain. The run view derives labels only from manifest and
attempt state, and the authoring view matches the API's logical-case conflict rules.
Ready saved suites now expose one Run sequence action in the canvas. It refreshes
readiness, queues the exact suite version through the durable manifest/attempt pipeline and
opens the polling run map; unsaved content uses Save & run and pins the returned version.
Suite submission derives a transient bounded policy and does not create a hidden plan.

The two-iteration GAN design evaluation improved from 6.79 to 8.39/10. Web typecheck,
ESLint, all 147 browser tests, contract drift checks and the production build pass. Rust
compilation and all 11 library tests pass. The new database-backed route test compiles but
cannot execute against the retained local test database because its migration ledger names a
removed historical migration; the database was not reset. Signed-in in-app browser
inspection passed at desktop and 390×844: the narrow flow uses readable vertical layout,
ordinary document scrolling works over the canvas and the saved-suite page has no redundant
Add button. The API-only development origin now defaults to the documented
`http://localhost:5173`; a fresh browser challenge returned 200 and the Google Continue button
rendered after restart. The new Run sequence control and a real live run remain source/test
verified because this acceptance pass did not sign back in or execute a device.
This follow-up did not execute a device or model.

### Phase 06 task sessions — in progress

Task-first source is being implemented on `codex/06-task-sessions`; see
[phase 06 specification](implementation/06-test-generation.md). No completion or real
UI-to-Minitap acceptance is claimed yet. This checkout retains local phase 05
changes; GitHub PR #5 was the phase 04 execution/report PR.

Phase 06 primary task-session source and real API-to-Minitap acceptance now pass:
run `a8101844-8f33-4292-b093-8dd02e719b1c` saved two different texts through the actual
sample APK, including a selected-control task, and closed its owned emulator.
Generated contracts, 43 Rust tests, 93 browser tests and the worker checks passed
(with one existing local HTTP test requiring a successful targeted rerun).
See [phase 06 specification](implementation/06-test-generation.md).
Rendered browser acceptance, persistent local worker credential/registration,
restart-goal acceptance and the broader generation/discovery scope remain open.

## Phase 06 revamp — locally validated (2026-09-13)

The [phase 06 specification](implementation/06-test-generation.md)
revises phase 06 following tester feedback. It covers direct device commands and recording,
reusable predefined templates, and an explicit Generate with AI flow that discovers bounded
journeys and proposes named test drafts. Both preview and saved regression execution must
preserve structured actions; direct-only reruns must use no model calls.

Structured direct steps, phone control/recording, four reusable templates, and bounded AI
proposal generation are implemented in source. Legacy approved definitions retain their wire
serialization. Runtime state uses existing session payloads and library receipt transactions;
no new database migration is required. Protocol 2 fences direct work from old workers.
Validation: TypeScript, ESLint, Ruff, Pyright, Clippy, formatting and contract drift pass;
99 browser tests, 46 Rust tests and 136 worker tests pass (one pre-existing local HTTP worker
test intermittently times out in the full run and passes individually). Both builds pass.
The direct-authoring HTTP smoke preserves passed/failed/blocked reports through API restarts.
The actual shared direct executor also entered/saved/restarted the local sample successfully
for `Hello direct` and `Xin chào`, with zero AI calls and verified emulator shutdown. It exposed
and fixed competing Android hierarchy dump mechanisms. This device check does not prove the
whole new UI/API route on a real device. Real-model proposal quality, rendered browser acceptance,
broader app qualification and hosted gates remain open. See the
[phase 06 specification](implementation/06-test-generation.md).

## Shared model registry and resolver — locally validated (2026-09-21)

The API now owns an immutable, nonsecret model registry and resolves exact model references into
frozen run and phone-session assignments. Model-enabled protocol-5 claims are fenced against the
worker's recent qualified reference before resource reservation, while direct profiles remain
model-free. Python provider construction and credential allowlisting are centralized in one runtime
module; generated browser and worker contracts carry the same reference, capability and audit
shapes. Browser controls use explicit capabilities without adding a customer model-selection step.

Observed validation: eight targeted contract tests, all 149 browser tests and 11 API library tests
pass; API integration binaries compile; contract drift, browser typecheck/lint, worker Ruff/Pyright,
formatting, the production build and `git diff --check` pass. The full worker run collected 179
tests; its two reported failures both passed on their exact targeted reruns (one is the previously
documented local HTTP boundary test, and one was a corrected protocol-5 fixture). PostgreSQL-backed
API tests and the three model-free HTTP smokes could not start against the retained local test
database because its migration ledger contains `m20260918_000008_regression`, which is absent from
this worktree. The database was not reset or altered for validation.

No real device, provider/model, Doppler, or rendered-browser acceptance ran for this change. Registry
registration does not itself requalify a host; rollout still requires an approved definition, an
exact private host `model_ref`, worker restart and a matching protocol-5 heartbeat.
