# 07B first delivery — Regression labels and durable test history

Status: implemented locally; acceptance evidence below (2026-09-20). This is the specification for
the first 07B delivery in [phase 07](07-pilot-readiness-and-scale.md). It includes
saved-case execution, visible history, a frozen baseline and a compact comparison.
It does not complete 07B's audited triage or 07C–D's operational work.

## Problem and outcome

The editor's **Run test** currently invokes `runPhoneCommand` and persists a
`phone_tasks` payload. `Runs.tsx` reads `execution_runs` through `runsQuery`.
Consequently an actual editor execution is absent from Runs, and its result card
only says `assertion failed`. A phone command has no immutable saved-case pin in
its request and does not establish the same start/cleanup proof as a release run.

The controlled demo passed on v1.0 and failed after restarting v1.0-broken, with
the same saved test v3. This demonstrates the defect, but it is not an existing
machine-generated baseline comparison. Do not retroactively assign that verdict
to historical phone commands using their title, timestamps or current case content.

After this delivery, every new **Run test** on a saved case creates a durable run.
The editor links to it, Runs lists it, and a comparable pass-to-fail build change
shows **Regression** with the failed check, expected/observed values and both
builds. Refresh, disconnect and API restart preserve the result and baseline.

## Product decisions

1. Keep two facts separate: execution outcome (`Passed`, `Failed`, `Blocked`, etc.)
   and comparison (`Regression`, `Still failing`, `Recovered`, etc.). The failed
   check remains inspectable even when comparison is unavailable.
2. A baseline is an earlier run, not a version-name string and not an assumption
   that the previous upload worked. Show its build and date before submission.
3. Suggest the latest eligible comparable completed run, with deterministic
   timestamp/ID ordering. Include failed runs; always choosing the last passing
   run would repeatedly call a known failure new. Allow explicit Change or None.
   A deliberately pinned baseline is retained until explicitly changed.
4. Freeze the chosen baseline ID at run creation. Later runs cannot silently
   change an existing comparison. Changing the baseline means creating another run.
5. Saving expectations requires no human approval. Selecting a baseline does not
   introduce an approval workflow or certify a release as bug-free.
6. Build selection is explicit. Show a compact build selector before Run test;
   pin its ID/checksum. Do not require re-uploading an APK to select it or silently
   run against the newest upload. Preview and run build differences are visible.

## One execution path for saved tests

Use the existing execution scheduler, attempts, lease fencing, evidence store and
server verifier for both saved-case and release-plan runs. Do not add a second
regression queue or copy phone results into successful execution attempts.

- Add a saved-case admission path taking an immutable case version, explicit build,
  qualified profile, environment revision and nullable baseline run ID. Normalize
  it to the existing resolved-case manifest and execution pipeline.
- Extract only shared admission/manifest assembly from `services/runs.rs`; retain
  existing plan resolution and readiness checks. A one-case run needs no manually
  authored suite/plan and no hidden mutable library plan.
- Introduce an explicit versioned source for new manifests (`saved_case` or
  `release_plan`). Existing manifests require plan fields; preserve their stored
  JSON/hashes and decode them as legacy plan manifests. Specify migration and
  serialization fixtures before changing those non-optional contract fields.
  Old workers must not claim a manifest shape they cannot understand.
- With unsaved edits, show **Save & run**. First save, then submit its returned
  immutable version. An incomplete save exposes existing field issues and queues
  nothing. A lost response reuses the mutation identity; it never causes another
  run or silently falls back to the previous saved version.
- Show **Preparing a clean test session** while releasing a preview lease. Do not
  kill an in-flight phone action: finish or explicitly cancel it, acknowledge
  cleanup, then let the existing device scheduler acquire the device. Quarantined
  cleanup blocks reuse with actionable UI. It cannot count as a passing baseline.
- The active run panel reads execution events/evidence. An old preview image is
  clearly labeled as a last capture; it must not look like the current run's screen.

Interactive target picking, Control phone and AI exploration remain phone-session
operations. An explicit **Try actions** option can execute incomplete authoring
content; label its result **Trial**, never an established regression comparison.

## History and historical data

Runs defaults to durable test/release runs and editor trials, with a visible source
label and filters. AI exploration jobs and individual manual phone taps are not
presented as test executions. New phone commands need an explicit purpose field;
do not infer purpose from the human-readable title.

For older phone records without purpose or saved-version proof, offer **Legacy
session activity** as a separate history filter. Display recorded facts and say
**No comparable baseline**. Do not invent a case pin or clean-start receipt.
Preserve the existing creator/app authorization boundary of phone records when
listing them; a workspace run permission must not expose another user's private
authoring session. Missing retained evidence is explicit.

The filter is presented as **Earlier test activity**. Compact cards show build,
date and recorded outcome; numbered readable steps are collapsed behind **View
steps**. A single explanation above the list directs users to saved tests for a
verified build comparison. Empty current-history views offer a link to earlier
activity, rather than implying that previous executions were lost. Historical
success is labeled **Finished**, since completing actions alone need not prove
assertions passed; blocked receipts remain **Blocked**, not an app regression.

Implement this as a read projection over the existing authoritative records, with
stable source-tagged IDs and cursor pagination. No dual-written shadow run table.
All new saved test runs use `execution_runs`, so the legacy projection does not
become a second comparison engine.

## Comparison policy

The backend first establishes eligibility, then matches cases by app/stable case
identity/data variant. Require the same immutable case version/content hash,
resolved data, environment revision and qualified execution signature (device,
reset policy, verifier/runtime, and relevant model configuration for AI steps).
Build checksum is the permitted difference. Version labels alone prove nothing.

Both runs must be terminal with verified start and cleanup and sufficient check
evidence. Blocked, canceled, skipped, inconclusive, missing proof and mixed retry
outcomes cannot establish a clean transition. Retain every attempt. Until both
runs finish, show **Comparison pending**, not a final regression badge.

| Baseline → current | Comparison label |
| --- | --- |
| Passed → assertion failed, different build checksum | Regression |
| Failed → failed | Still failing |
| Failed → passed, clean attempts and lifecycle proof | Recovered |
| Passed → passed | Unchanged |
| Passed → failed, same build checksum | New failure on same build |
| No selected eligible baseline | No baseline |
| Changed test/data/context, incomplete proof or uncertain attempts | Not comparable, with reason |

Added and removed cases remain separate visible coverage changes in multi-case
runs. A changed expected value cannot appear recovered. An assertion regression
means an observed loss of previously passing behavior; it does not automatically
diagnose the root cause as an application-code defect.

Compare checkpoints/checks by stable identity. Read expected/observed data from
the retained result and verifier evidence. `observed: null` means unknown unless
a structured reason explicitly records element absence. Add that reason/evidence
field if needed; do not turn every generic failure into “task not found.”

## Architecture and ownership

```text
React: choose build / baseline → Rust admission → existing execution queue
                                                 ↓
                                worker → observations / receipts / evidence
                                                 ↓
                              server verifier → durable execution outcomes
                                                 ↓
                        pure comparison policy → persisted comparison → UI
```

| Owner | Responsibility |
| --- | --- |
| `crates/contracts` | Rust-owned request/response and versioned manifest shapes; comparison categories and reason codes; no DB or framework logic |
| `apps/api/src/domain/regression.rs` (new) | Pure compatibility, case/check matching and transition functions; no DB/network/clock/AI |
| `apps/api/src/services/runs.rs` and focused admission helper | Authorize, resolve immutable input, pin build/baseline, idempotently queue through existing execution machinery |
| `apps/api/src/services/run_comparisons.rs` (new) | Authorize both sides, load complete facts, invoke policy and finalize the comparison exactly once |
| Focused comparison/history stores + SeaORM migration | Baseline reference, policy version, case deltas/reasons/evidence references, deterministic pagination and uniqueness constraints |
| Existing worker + server verifier | Execute, capture observations and establish outcomes; never decide whether a result is a regression |
| React run components | Render generated DTOs; selection/disclosure/filter state only; shared compact result card for editor, Runs and report |

Freeze baseline selection atomically with run admission. Finalize comparison
idempotently after lifecycle verification, storing the comparison policy version
and immutable result references. A recoverable finalization job retries after API
restart; report reads do not mutate outcomes. Concurrent submissions cannot select
each other as baselines. Authorize baseline, current run and artifacts independently;
reject cross-app/workspace references without disclosing their contents.

Use Rust → OpenAPI → generated SDK/types/Zod and schema → generated Pydantic.
No handwritten matching browser interfaces or duplicated frontend verdict rules.
Use existing service/store patterns; no generic repository framework or DI container.

## Compact UI

Before running, one strip: **Build v1.1 ▾ · Compare with v1.0, Sep 20 · Change**.
With no baseline: **First comparable run**; execution remains possible.

```text
A saved task survives an app restart          [Regression]
v1.0 Passed → v1.0-broken Failed · Test v3
Step 3 · After restarting, the saved task was missing.
Expected: “Buy milk”     Observed: task not found
[Compare evidence]                         [Run again]
```

The example requires verified absence evidence. Otherwise render the actual reason
or **Observed result unavailable**. Use a red Regression badge with text/icon, never
color alone. Keep failure reason visible without expanding technical details.
Compare evidence opens the existing report layout with labeled Baseline/Current
captures, build/date and the matching failed check. On narrow screens use tabs.
Missing/expired captures keep their labels and explanation. Collapse hashes,
attempts and lifecycle details; do not add a second large dashboard or warning stack.

## Delivery order and verification

1. **Contracts and policy:** define versioned manifest compatibility, source/purpose,
   baseline input and comparison response; pure table-driven transition tests cover
   every row above, added/removed/changed cases, missing observations and retries.
2. **Durable admission/history:** implement single-case runs and explicit build choice;
   retain legacy plan serialization. Route tests verify real paths/DTOs, auth,
   stale save/environment handling, idempotency and history after API restart.
3. **Baseline/finalization:** migrate nullable baseline references and comparison
   records; integrate finalization into existing lifecycle. Test concurrent creation,
   deterministic candidate selection, pinned baselines, restart recovery, cleanup
   failure, immutable results and unauthorized baseline/artifact access.
4. **UI integration:** replace editor Run test submission, add source-aware history,
   compact badge/explanation and evidence comparison. Test Save & run, no-baseline,
   pending/blocked/not-comparable states, links after disconnect, keyboard and narrow
   layouts. Remove obsolete duplicate result rendering where it is superseded.
5. **Acceptance:** run the exact same saved persistence test on clean good/broken
   APKs, using the good durable run as the explicitly displayed baseline. Verify
   both records in Runs after reload/API restart, Regression at the restart check,
   and matching expected/observed evidence. A follow-up failure against the latest
   failed baseline reads Still failing. A changed expectation reads Not comparable.

Finish the agreed code/test/config batch before running affected checks. Run pure,
route, generated-contract and UI validation without device/model calls; perform
the explicit emulator acceptance separately. Update this spec and status with
observed results, without claiming demo acceptance qualifies arbitrary customer APKs.

## Completion boundary

Ship only when a tester can select an existing build, run a saved test, reopen the
result from Runs, see why it is or is not a regression, and inspect the chosen
baseline without losing either execution. A badge inferred from `failed`, a title
match, or whichever passing run happens to be newest does not meet this boundary.
Audited human triage, accepted-release baseline management, exports, retention and
fleet scaling remain subsequent work; none is required to add another save approval.


## Implementation report — 2026-09-20

Implemented the first delivery across contracts, pure policy, admission, persistence,
worker compatibility and browser presentation. Saved-case runs share the execution
scheduler and verifier; no shadow run table, AI verdict or review workflow was added.

- `POST /api/apps/{app_id}/case-runs/preview` returns admission blockers and baseline
  choices. Recent choices are bounded; keyset scanning continues until the latest
  eligible completed result is found. Failed baselines are eligible.
- `POST /api/apps/{app_id}/case-runs` atomically pins case/build/profile/environment/
  baseline and queues an attempt under the existing app-scoped idempotency key.
  Explicit incompatible baselines remain pinned and receive Not comparable.
- `GET /api/apps/{app_id}/run-history` projects saved runs and creator-private trials,
  with stable pagination and an explicit legacy-activity filter. Manual phone taps
  and AI discovery are excluded. Existing untyped phone history gains no verdict.
- Migration 000009 allows saved-case manifests without a plan foreign key and stores
  the baseline/comparison and explicit command purpose. Existing manifest JSON stays
  unchanged. Saved-case manifests require execution protocol 4. Release-plan creation
  deliberately retains its legacy manifest shape and existing compatibility; this
  delivery adds baseline selection to saved cases, not the release-plan form.
- Pure comparison uses exact saved version/hash, app, data variant, environment and
  profile equality; both sides require verified start/cleanup and sealed conclusive
  checks. Ambiguous targets, missing evidence, mixed retries and recovery remain Not
  comparable. A ready-screen absence is structured separately from an unknown value.
- Cleanup attempts comparison finalization immediately. Existing scheduler
  reconciliation retries pending projections after restart; reads do not write.
  Final comparisons are write-once with policy version 1. Deployment must keep the
  existing worker poll/reconciliation process running for recovery.
- The editor has explicit Build / Test device / Compare with selectors and Save & run.
  Retries retain the saved request and key. Phone cleanup precedes queueing; active
  actions and quarantine block automatic reuse. Try actions stays separate.
  A shared result card renders the server label, failed action, expected/observed
  values, both run links and matching captures behind Compare evidence. Runs provides
  source filters and pagination. No frontend code computes regression eligibility.

### Validation and acceptance

Automated validation: Rust contracts/domain/route suites, worker tests, browser tests,
Clippy, TypeScript/ESLint, Ruff/Pyright, generated-contract drift and API/web builds
passed across the consolidated pass and focused corrections. A separate worktree test
DB avoided another branch's migration; its test-only sign-in buckets were cleared
between repeated route runs. Production auth limits were unchanged. Vite retains its
existing large-bundle advisory. The new local-only `sampleBroken` Android fixture
builds alongside `sample`; APKs/evidence remain ignored private artifacts.

`scripts/regression_acceptance.py` is an explicit real-device acceptance command,
excluded from ordinary checks. It first proves persistence on one owned AVD and
empty state on a second, then registers that local fixture profile in an isolated
API test workspace. It exercises the same immutable case against good/broken builds,
checks pass → Regression → Still failing, and compares history/reports after API restart.
Use the built API binary, a compatible isolated test database (`LOCO_CONFIG_FOLDER`
may select one), JDK 17 and the worker's uv environment:

```sh
uv run --project apps/mobile-worker --no-sync python scripts/regression_acceptance.py \
  --profile /absolute/path/to/nonsecret-host.toml \
  --good apps/qa-demo-android/app/build/outputs/apk/sample/debug/app-sample-debug.apk \
  --broken apps/qa-demo-android/app/build/outputs/apk/sampleBroken/debug/app-sampleBroken-debug.apk
```

Real-device acceptance passed (2026-09-20). This fixture qualification
does not qualify customer APKs, account/backend resets, or a production fleet.

The implementation report lives here rather than adding obsolete PRP plan/report
copies. This first delivery does not include audited triage, accepted-release baseline
management, exports, retention or fleet scaling. Legacy demo profiles lacking start
proof can still execute, but cannot become comparable baselines by inference.


### Observed real-device result

Acceptance directory (ignored/private):
`.private/regression-acceptance/2f172b68-5b97-4b0c-80fd-032ae72b3d7d`.
The test-only workspace and records use the isolated test database, not the user's
existing demonstration workspace.

| Durable run | Result | Comparison |
| --- | --- | --- |
| `4256cc75-835d-4d51-b996-1d9e8ccb42bc` · sample | All three checks passed | No baseline |
| `94ca7116-dba8-4ada-996f-edc784e4a1c5` · sampleBroken | Enter/save passed; restart persistence failed | Regression against the first run |
| `b7fdba04-4e70-4cc5-a1fb-faf8f70b6397` · sampleBroken | Same restart failure | Still failing against the second run |

The failing check retained expected `Buy milk`, structured `absent`, and
`target_absent_on_ready_screen`, with PNG/XML evidence. All runs had verified
preflight and disposal receipts. The chooser suggested the latest failure for run
three. After stopping and restarting the test API, each full report was unchanged
and all three appeared in history. No model calls were made.

Automated totals across the consolidated pass and scoped retries: 59 Rust tests
(contracts, domain and API routes), 129 browser tests and 181 worker tests passed.
Generated drift, formatting, Clippy, TypeScript, ESLint, Ruff, Pyright and API/web/
fixture APK builds passed. Authenticated browser inspection verified the new editor controls and compact
history cards, including real earlier successful, failed and blocked results.
The default empty view links to earlier activity; raw step IDs are hidden behind
readable step summaries. Full narrow-screen acceptance remains open. No
authentication bypass was used. The latest card changes passed focused DOM tests,
TypeScript, ESLint and the web build; the production dependency audit reported no
known vulnerabilities.
