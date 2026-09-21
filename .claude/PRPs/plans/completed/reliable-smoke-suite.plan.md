# Plan: Reliable smoke suite

## Summary

Finish the existing saved-suite path as a repeatable Android smoke check. Give
the tester explicit build, device, and baseline choices; preserve an immutable
multi-case run; present an honest overall result and per-case evidence; then
prove the flow on the qualified local demo. The canonical business rules are
in [spec 10](../../../../docs/architect/implementation/10-reliable-smoke-suite.md).

## User story

As a tester preparing a release, I want to run a small saved suite against a
chosen build and compare it with an earlier run, so that I can see which
critical behaviors passed, failed, or could not be verified.

## Two-build workflow in plain terms

This shipment creates **one run for one build**. It does not install two APKs in
one emulator session or launch a paired two-build job. The saved suite and its
exact case versions are reused, while each run receives its own immutable
manifest with the selected build ID/checksum and execution context.

1. Run A: choose saved suite v3 and build 1.3, with no baseline. Execute each
   case from its declared clean start and retain the report.
2. Run B: choose the **same** saved suite v3 and build 1.4. Before submission,
   explicitly select completed Run A as the baseline; freeze its run ID with
   Run B. The same qualified device profile can be reused after verified
   cleanup, but the runs are separate.
3. When Run B finishes, compare its verified case outcomes against Run A:
   Passed→Failed is Regression, Failed→Passed is Recovered, Failed→Failed is
   Still failing, and Passed→Passed is Unchanged. A changed test version,
   execution context, or missing clean proof is Not comparable.

This is release build comparison, not a randomized user A/B experiment. The
baseline is selected **before Run B starts** and cannot be changed on its
historical report. Selecting any two completed runs afterward for a new,
ad-hoc comparison is a separate feature outside this delivery.

## Problem → solution

The library, saved-suite route, durable attempts, run map, and case-level
comparison exist. The suite button currently chooses the newest build and
first qualified profile, accepts no baseline, and has no successful live
suite-run acceptance. The real worker has a retained recovery gate. → Reuse
those paths with explicit selections, pinned suite baseline, clear result
language, secret-free HTTP coverage, and an evidence-backed real run.

## Metadata

- **Complexity:** Large; cross-cutting, about 12–16 owned source/test files plus generated outputs and docs.
- **Source PRD:** N/A; product rules in `docs/architect/product.md` and focused spec 10.
- **Phase:** Focused delivery across 04/05/07A/07B; pilot reliability campaign remains separate.
- **No migration expected:** `execution_runs.baseline_run_id` is already nullable (`apps/api/migration/src/m20260920_000009_regression.rs:9-14`). Additive DTO fields suffice.
- **Current proof:** suite route/source tests exist; simulated HTTP covers the library/plan path, not a direct suite run. `docs/architect/status.md:3-5,531-556` says a live suite run remains unproven.

## UX design

### Before

```text
Tests → Suite sequence → Run sequence
                         newest build + first qualified profile chosen silently
                         no suite baseline choice
Runs → generic release heading / first case title → case evidence
```

### After

```text
Tests → Saved suite vN → Build [chosen] · Device [chosen]
                         Compare with [run / None]
                         readiness / blocker explanation → Run suite
Runs → Saved suite · N cases · chosen build/device
       overall required-coverage result + each case's outcome/evidence
       pinned comparison and explicit gaps
```

| Touchpoint     | Before                                | After                                                         | Constraint                          |
| -------------- | ------------------------------------- | ------------------------------------------------------------- | ----------------------------------- |
| Suite editor   | “Run sequence” auto-selects setup     | “Run suite” with build/device/baseline and exact version      | Save & run pins returned version.   |
| Preview        | Readiness only                        | Readiness plus baseline candidates/suggestion                 | Preview never reserves a device.    |
| Run detail     | First case may appear as report title | Source-specific “Saved suite · N cases” and per-case evidence | Browser displays server facts only. |
| Queue/recovery | Existing queue reason                 | Existing reason remains prominent                             | Recovery never means pass.          |

## Business rule decision table

| Situation                                                              | Admission / result / comparison                                             |
| ---------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| Unsaved phone action sequence                                          | Save as a case with explicit checks first; it is not a direct suite member. |
| Complete suite vN with selected build/profile                          | Queue exactly its saved case-version references under one frozen manifest.  |
| Duplicate exact member                                                 | Resolve once; conflicting logical-case version/variant/requiredness fails.  |
| Later case/suite edit or new build upload                              | Existing manifest and pinned baseline do not move.                          |
| All required cases conclusive and clean                                | “Required checks passed”; list any optional failures explicitly.            |
| Required assertion fails                                               | “Failures detected”; show the failing case/check/evidence.                  |
| Required blocked, inconclusive, canceled, unrun, or recovery-held      | “Incomplete / review required”; no green suite claim.                       |
| No baseline selected                                                   | Current result still valid; comparison says No baseline.                    |
| Same pinned cases/context, conclusive prior and current, build changed | Existing pure 07B transition labels each case.                              |
| Different test/context or missing lifecycle/evidence proof             | Not comparable with the current reason; never inferred regression.          |
| Lost create response                                                   | Retry identical body and idempotency key, returning one run.                |

The production suite limit remains 1–100 resolved cases with at least one
required case. Two or three independent cases are the acceptance fixture, not a
new hard limit. No case may depend on the preceding case's device or backend
state; every real attempt obtains its own clean-start/cleanup evidence.

## Mandatory reading

| Priority | File:lines                                                                                                                                                        | Why                                                                   |
| -------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- |
| P0       | `AGENTS.md:1-74`, `docs/architect/README.md:1-59`, `docs/architect/implementation/10-reliable-smoke-suite.md:1-130`                                               | Repo rules, authority, scoped business rules.                         |
| P0       | `docs/architect/product.md:59-124`, `docs/architect/implementation/05-test-library-and-plans.md:3-40,78-120`                                                      | Case/suite/plan semantics, existing Save & run invariants.            |
| P0       | `docs/architect/implementation/04-execution-and-reports.md:52-83`, `docs/architect/implementation/07b-regression-label-and-history.md:26-78,107-139`              | Fencing, recovery, verdicts, baseline rules.                          |
| P0       | `crates/contracts/src/regression.rs:8-91`, `crates/contracts/src/execution.rs:163-203,247-284,318-359`                                                            | DTOs, manifest, summary inputs.                                       |
| P0       | `apps/api/src/services/suite_runs.rs:1-175`, `apps/api/src/services/case_runs.rs:12-199`                                                                          | Suite path and baseline pattern.                                      |
| P0       | `apps/api/src/domain/regression.rs:4-199`, `apps/api/src/services/run_comparisons.rs:1-15`, `apps/api/src/services/runs.rs:361-419,511-542`                       | Pure compatibility and server-owned report.                           |
| P1       | `apps/api/src/services/test_definitions.rs:239-281`, `apps/api/src/services/scheduler.rs:113-210`                                                                 | Deduplication, order, worker protocol, reservation.                   |
| P1       | `apps/web/src/components/runs/saved-suite-run.tsx:17-139`, `apps/web/src/components/runs/saved-case-run.tsx:25-160`                                               | Current suite UI and explicit saved-case pattern.                     |
| P1       | `apps/web/src/components/runs/run-result.tsx:20-150`, `apps/web/src/pages/RunDetail.tsx:25-235`, `apps/web/src/components/test-library/coverage-flow.tsx:840-922` | Source label, evidence, flow language.                                |
| P1       | `docs/architect/development.md:295-311,335-407`, `justfile:38-130`                                                                                                | Verification order, fake/live separation, recovery commands.          |
| P2       | `scripts/test_library_smoke.py:1-176`, `scripts/execution_smoke.py:28-76,78-305`, `apps/api/tests/execution.rs:828-911`                                           | HTTP fixture harness and route-test idiom.                            |
| P2       | `scripts/regression_acceptance.py:34-120,167-390`, `apps/qa-demo-android/app/src/main/java/ai/mobileqa/demo/MainActivity.java:65-102`                             | Existing isolated real-device harness and good/broken fixture oracle. |

`docs/CODEX-NAVIGATION-GUIDE.md` is referenced by the supplied AGENTS
supplement but is absent from this checkout. Source and canonical architect
documents above provide the required navigation; do not invent the guide.
The generated Graphify index predates current `main`; its query was used for
orientation, and every relationship here is checked in source.

## Discovery reference: eight codebase categories

| Category               | File:lines                                                                                                                                                                | Existing pattern / implication                                                                        |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| Similar implementation | `apps/api/src/services/case_runs.rs:59-199`                                                                                                                               | Preview baselines, then create a pinned run transactionally. Extract only the common candidate scan.  |
| Naming                 | `apps/api/src/services/suite_runs.rs:73-93`, `apps/web/src/api/regression.ts:33-58`                                                                                       | `preview/create`, `SuiteRunRequest/Preview`, `suiteRunPreviewQuery/createSuiteRun`. Keep these names. |
| Error handling         | `apps/api/src/errors.rs:9-48`, `apps/api/src/services/suite_runs.rs:94-140`                                                                                               | Safe `ApiFailure::invalid`/`conflict`; preserve retry identity and field-friendly blockers.           |
| Logging                | `apps/api/src/services/suite_runs.rs:167-174`, `apps/api/src/errors.rs:70-87`                                                                                             | Structured IDs/reason codes, no secrets or raw evidence.                                              |
| Types/contracts        | `crates/contracts/src/regression.rs:54-91`, `crates/contracts/src/regression_api.rs:10-15`, `apps/web/src/api/regression.ts:33-58`                                        | Rust DTO → Utoipa/OpenAPI → generated SDK/types/Zod; validate success/error boundaries.               |
| Tests                  | `apps/api/tests/execution.rs:828-911`, `apps/web/src/components/runs/saved-suite-run.test.tsx:85-114`, `apps/api/src/domain/regression.rs:200-280`                        | Real route, UI interaction, pure policy tests.                                                        |
| Configuration          | `apps/mobile-worker/src/mobile_qa_worker/qualification/config.py:30-51`, `docs/architect/development.md:295-311`                                                          | Qualified profile, explicit toolchain/JDK, operator-only recovery. No new env files.                  |
| Dependencies           | `apps/web/src/components/runs/saved-suite-run.tsx:1-13`, `apps/api/src/services/suite_runs.rs:1-7`, `apps/mobile-worker/src/mobile_qa_worker/execution/runner.py:341-390` | Mantine/Query/React Router, Loco/SeaORM, uv/Pydantic/Android host; no package needed.                 |

## Five execution traces

1. **Entry:** `SavedSuiteRun` in `draft-editor.tsx:280-307` calls generated
   `previewSuiteRun/createSuiteRun` through `api/regression.ts:33-58`. Axum routes
   in `controllers/runs.rs:147-200` call `suite_runs::preview/create`.
2. **Data:** `suite_runs::manifest` loads admitted suite, creates a transient
   bounded `PlanDefinition`, calls `test_definitions::resolve`, then
   `runs::assemble` for build/profile/environment/readiness and `RunManifest`.
   The suite version ID is `RunSource::SavedSuiteV1`; no hidden plan is saved.
3. **State:** `suite_runs::create` locks app/environment, checks idempotency and
   admission, inserts one `execution_runs` row plus numbered attempts, commits,
   then wakes the scheduler. `scheduler::claim` filters by registered worker
   app/profile/protocol/model capability, orders by run creation/case index,
   and keeps app/device reservations fenced. Python executes one attempt,
   uploads evidence and acknowledges cleanup. No transaction spans device I/O.
4. **Contracts:** `regression.rs` owns request/preview and `regression_api.rs`
   declares exact routes. `just types` regenerates OpenAPI, browser SDK/Zod,
   worker Pydantic only when content changes. The suite baseline field needs
   `#[serde(default)]` so old clients may omit it; do not rewrite old manifests.
5. **Read:** `run_comparisons::finalize_pending` persists comparison after all
   attempts finish. `runs::detail` derives state/summary/queue status from DB;
   `RunDetail` polls the generated response and shows flow/attempt evidence.

## Patterns to mirror

### Naming and service boundary

Source: `apps/api/src/services/suite_runs.rs:73-93`.

```rust
pub async fn preview(
    ctx: &AppContext,
    actor: Uuid,
    app: Uuid,
    input: SuiteRunRequest,
) -> ApiResult<SuiteRunPreview> {
    apps::authorized(ctx, actor, app).await?;
    let preview = manifest(&ctx.db, app, &input).await?;
    let manifest = preview.manifest.ok_or_else(ApiFailure::internal)?;
    Ok(SuiteRunPreview {
        blockers: preview.blockers,
        environment_revision: manifest.environment_revision,
    })
}
```

Keep policy pure and DB work in services rather than putting business rules in
the controller or React.

### Error, SQL, transaction, and log

Source: `apps/api/src/services/suite_runs.rs:94-140,167-174` and
`apps/api/src/services/execution_store.rs:21-59`.

```rust
if !bounded(key, 128) {
    return Err(ApiFailure::invalid(
        "Idempotency-Key must contain 1–128 printable bytes",
    ));
}
let tx = ctx.db.begin().await?;
one(
    &tx,
    "SELECT id FROM apps WHERE id=$1 FOR UPDATE",
    vec![app.into()],
)
.await?;
```

Preserve the existing lock order and 409 idempotency conflict. No log should
include credentials, request bodies, hierarchy, or screenshots.

### Browser boundary and retry

Source: `apps/web/src/api/regression.ts:33-58` and
`apps/web/src/components/runs/saved-suite-run.tsx:35-105`.

```ts
const pending = useRef<{ body: SuiteRunRequest; key: string } | null>(null)
return createSuiteRun(appId, pending.current.body, pending.current.key)
```

Use generated `zSuiteRunRequest`, `zSuiteRunPreview`, and `zRunResponse` through
`checked`. The before/after code and tests should name the new button “Run
suite”; the sequence map may retain its visual label. On first submit, snapshot
the selected body and UUID. On a lost response, submit `pending.current` again;
do not read newer form or preview values.

### Pure comparison and report

Source: `apps/api/src/domain/regression.rs:4-99,102-199` and
`apps/api/src/services/runs.rs:511-542`.

```rust
pub fn context_matches(a: &RunManifest, b: &RunManifest) -> bool {
    a.app_id == b.app_id
        && a.environment_revision == b.environment_revision
        && a.profile == b.profile
        && a.resolved_model == b.resolved_model
}
```

The pure functions already exist. Add a narrow `suite_baseline_eligible` policy
for candidate suggestion: matching `SavedSuiteV1` ID and context, every required
case present with same version/hash/variant and conclusive baseline outcome.
Explicit different completed same-app baselines remain allowed and explained by
`compare`, never silently substituted.

### Test structure

Source: `apps/api/tests/execution.rs:828-911` and
`apps/web/src/components/runs/saved-suite-run.test.tsx:85-114`.

```rust
#[tokio::test]
async fn saved_suite_route_queues_each_numbered_case_in_one_immutable_run() {
    use mobile_qa_contracts::regression::*;
    let _guard = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, ctx| async move {
        let owner = login(&server, &ctx).await;
        let (app, build, plan, worker, _) = prepared(&server, &ctx, &owner).await;
    })
    .await;
}
```

```ts
await userEvent.click(await screen.findByRole('button', { name: 'Save & run' }))
await waitFor(() => expect(mocks.create).toHaveBeenCalledTimes(1))
const first = mocks.create.mock.calls[0]!
expect(first[1]).toMatchObject({ suite_version_id: 'new-suite-version' })
```

Use exact route/response assertions rather than tests that merely mirror local
helpers. Worker tests are offline by default; real device proof is a separate
explicit acceptance run.

## External documentation

No external research needed — this delivery follows established internal
Loco/SeaORM, generated transport, React/Query, and worker patterns at pinned
versions. The local source and lockfiles, not latest web examples, determine
behavior.

## Architecture and alternatives

- **Approach:** extend existing suite request and preview; extract only the
  baseline candidate scan common to saved case/suite; keep compatibility and
  summary pure; reuse nullable baseline DB column and existing multi-case
  comparison finalizer. Let React select, submit and display only.
- **Rejected alternative:** a `smoke_suite` definition/table duplicates the
  existing suite. A new queue/verdict path would bypass proven fencing and
  evidence rules. Implicit newest-build selection cannot establish an intentional
  comparison. Persisting baseline into a mutable suite version would change
  historical runs when release context changes.
- **Scope:** local demo smoke suite, explicit selections, pinned comparison,
  honest report, automated and real acceptance. No customer or hosted claim.

## Files to change during implementation

| File                                                                                           | Action                  | Purpose                                                                                      |
| ---------------------------------------------------------------------------------------------- | ----------------------- | -------------------------------------------------------------------------------------------- |
| `crates/contracts/src/regression.rs`                                                           | Update                  | Optional suite baseline request; baseline choices in preview.                                |
| `crates/contracts/src/regression_api.rs`                                                       | Review/update if needed | Preserve exact endpoint declaration and route inventory.                                     |
| `apps/api/src/services/run_baselines.rs` and `services/mod.rs`                                 | Create/register         | Shared completed-run candidate query and compact projection.                                 |
| `apps/api/src/services/case_runs.rs`                                                           | Refactor                | Call shared baseline candidate service without behavior drift.                               |
| `apps/api/src/services/suite_runs.rs`                                                          | Update                  | Candidate preview, baseline validation/pin, existing transaction and idempotency.            |
| `apps/api/src/domain/regression.rs`                                                            | Update                  | Pure suite suggestion eligibility; existing compare remains authoritative.                   |
| `apps/api/src/services/runs.rs`                                                                | Update                  | Overall summary covers optional failures and clean required proof.                           |
| `apps/api/src/services/scheduler.rs`                                                           | Update                  | Stable claim-order tie-breaks in the existing picker; no new queue.                          |
| `apps/api/tests/execution.rs`, relevant pure policy tests                                      | Update                  | Real-route and aggregation cases.                                                            |
| `apps/web/src/components/runs/saved-suite-run.tsx` and test                                    | Update                  | Explicit choices, exact retry snapshot, clear names.                                         |
| `apps/web/src/components/runs/run-result.tsx`, `pages/RunDetail.tsx` and tests                 | Update                  | Source-specific headings, source-neutral result display.                                     |
| `apps/web/src/components/test-library/coverage-flow.tsx` and tests                             | Update                  | Button/heading wording where needed.                                                         |
| `scripts/test_library_smoke.py` or focused `scripts/smoke_suite.py`, `justfile`                | Extend                  | Two-case HTTP/worker/restart smoke without device or secrets. Reuse current fixture harness. |
| `scripts/regression_acceptance.py` or focused sibling using shared helpers                     | Extend                  | Explicit real two-case suite acceptance on isolated good/broken demo fixture.                |
| `docs/architect/implementation/10-reliable-smoke-suite.md`, `status.md`, `development.md`      | Update at delivery      | Actual behavior/evidence, precise manual steps, remaining gates.                             |
| Generated `contracts/*` and `apps/web/src/api/generated/*`; worker generated models if drifted | Regenerate only         | `just types`; never edit by hand.                                                            |

No database migration, new library DTO shape, new worker protocol, new package,
or new model call is planned. Change worker code only if real evidence identifies
a defect; then add a focused offline regression test and retain quarantine rules.

## Task picker policy

The word **task** here means a durable `execution_attempts` row, not a suite
definition or a browser-selected canvas node. Submission creates all case
attempts in the immutable manifest order. The Python worker supplies its
claim ID, registered profile, protocol and model capabilities; Rust is the
only authority that selects an attempt. A worker sees queued, uncanceled
attempts for its app and profile whose manifest it can execute. The intended
total order within that eligible set is `(run.created_at, run.id, case_index,
attempt.number, attempt.id)`. The current query omits the two ID tie-breaks.
An incompatible older attempt may be bypassed; there is no global FIFO or
priority promise across apps/profiles. A phone authoring session competes for
the same app/device reservation without a separate priority rule.

The claim transaction serializes on the app row, uses attempt row locking with
`SKIP LOCKED`, and inserts app and physical-device reservations before issuing
a fenced 60-second lease. Another worker cannot claim a later case for that app
while the reservation exists. Heartbeats extend the lease. Verified cleanup
releases reservations and wakes claim waiters. A failed but clean case permits
the next case. If diagnostic retry is configured, cleanup inserts attempt 2
for that case, which sorts before later cases. Unverified cleanup or lease
expiry quarantines the device; no automatic replay or skip-ahead occurs.
Readiness and queue reason remain server facts, not Python/UI heuristics.

## Step-by-step tasks

### Task 1 — Freeze suite-run transport and compatibility

- **ACTION:** Add `baseline_run_id: Option<Uuid>` to `SuiteRunRequest` with serde
  default and skip-serializing-None for omitted old-client values; add `baselines` and
  `suggested_baseline_id` to `SuiteRunPreview`.
- **IMPLEMENT:** Keep suite version/build/profile/environment fields required.
  Define old JSON decoding test and updated OpenAPI/route agreement assertions.
- **MIRROR:** `CaseRunRequest/CaseRunPreview` in `regression.rs:54-91`; endpoint
  declaration in `regression_api.rs:10-15`.
- **IMPORTS:** Existing `Uuid`, `BaselineChoice`, `serde` derives; no new crate.
- **GOTCHA:** `deny_unknown_fields` applies; older omitted baseline must mean None.
  The current idempotency fingerprint serializes `SuiteRunRequest`, so omitting
  None on serialization preserves old no-baseline retry fingerprints. Do not
  change serialized historical manifests.
- **VALIDATE:** Contract serialization fixture and real route agreement after all
  edits; `just types` once in final validation phase.

### Task 2 — Share baseline candidate selection and pin on suite creation

- **ACTION:** Move case preview's paged completed-run scan to a small shared
  service; add suite candidate suggestion and server admission.
- **IMPLEMENT:** Preserve descending `(created_at,id)` order and scan until an
  eligible result is found. For suite suggestion require exact saved suite
  version/source and `suite_baseline_eligible`; include completed failed runs.
  At create, validate a selected baseline is completed and belongs to the same
  app, then insert its ID into existing `execution_runs.baseline_run_id`. Allow
  explicit incompatible same-app baseline, as the case path does, so the report
  explains non-comparability. Fingerprint includes the baseline field. Recheck
  inside the same app/environment transaction and keep one wakeup after commit.
- **MIRROR:** `case_runs.rs:68-113,115-199`, `suite_runs.rs:94-174`,
  `execution_store.rs:21-59`.
- **IMPORTS:** `RunResponse`, `RunSource`, `BaselineChoice`, `Uuid`,
  `ConnectionTrait`, existing service modules.
- **GOTCHA:** A baseline is a run ID, not a build label. Selection at click time
  cannot be silently replaced by a newer suggestion. No cross-app disclosure.
- **VALIDATE:** Route tests for no baseline, suggested failed baseline, explicit
  incompatible choice, cross-app/nonterminal denial, idempotent replay,
  changed-body conflict, environment revision conflict.

### Task 2b — Make worker pickup order deterministic

- **ACTION:** Add `r.id` and `a.id` tie-breaks to the existing claim `ORDER BY`;
  keep its eligibility filters, app lock, row lock, reservations and lease flow.
- **IMPLEMENT:** Document the exact eligible-set and ordering policy next to
  the query. Do not introduce a separate task table, queue process or
  worker-side case selection. Preserve source/protocol/model matching so an
  incompatible older run does not block work that a worker can execute.
- **MIRROR:** `scheduler.rs:113-270`, `execution.rs:371-450`,
  `04-execution-and-reports.md:41-65`.
- **GOTCHA:** The order is per eligible worker and app; authoring phone sessions
  share reservations but have no global priority. An attempt 2 is inserted only
  after verified cleanup, and its earlier case index puts it ahead of later
  cases. Recovery must hold the reservation.
- **VALIDATE:** Real PostgreSQL tests submit a two- or three-case suite, assert
  first claim has case index 0, competing claim is empty while reserved,
  clean completion permits index 1, and configured retry precedes index 1.
  Exercise a mismatched worker/profile, equal timestamps, and recovery hold.

### Task 3 — Keep report policy honest

- **ACTION:** Add narrow pure summary/eligibility tests, then update only the
  policy needed to satisfy the defined result table.
- **IMPLEMENT:** Required pass needs all required attempts terminal and clean;
  on real profiles, verify preflight matches attempt/build/context and retained
  required check evidence. A required failure remains visible even when a later
  case is pending. Optional failure gets explicit summary text without claiming
  all cases passed. Continue rendering all attempts and all comparison cases.
  Preserve fake-run labeling; do not pretend synthetic preflight is real.
- **MIRROR:** `runs::summary:511-542`, `regression::outcome:10-79`,
  `run_comparisons::finalize_pending:1-15`.
- **IMPORTS:** Existing execution `Outcome`, `CleanupState`, `JobState`, and
  pure regression module; no UI-side verdict enum.
- **GOTCHA:** Recovery events never erase the original cleanup receipt;
  mixed retry pass/fail stays review-required. A failed assertion and a dirty
  device are separate facts; report both.
- **VALIDATE:** Pure table tests for required pass/fail/blocked/cancel/uncertain,
  optional fail, mixed attempts, invalid/absent preflight, sealed artifact loss,
  identical/different build transitions.

### Task 4 — Make suite choices explicit in the existing UI

- **ACTION:** Replace implicit `builds[0]` and first qualified profile with
  selectable build/device; add baseline choice and rename the run control.
- **IMPLEMENT:** Present saved suite version and selected build/checksum. Preview
  for current selections, show blockers and candidate build/date/compatibility.
  Default baseline may be the preview suggestion, but show that value before
  click and capture it in `pending.current` once. On Save & run, use the newly
  saved version before preview. Reset pending only on a deliberate form choice,
  never on a lost response. Preserve idle phone stop/wait and quarantine errors.
- **MIRROR:** `saved-case-run.tsx:25-160`, `saved-suite-run.tsx:35-139`,
  `api/regression.ts:33-58`, `draft-editor.tsx:280-307`.
- **IMPORTS:** Generated `SuiteRunRequest/BaselineChoice`, Mantine `Select`,
  Query hooks, `checked` and generated Zod through the API wrapper.
- **GOTCHA:** The browser may display a preview but the server must revalidate;
  a new uploaded build must not change a pending retry. No browser policy bypass
  for rendered acceptance.
- **VALIDATE:** DOM tests for explicit choices, no auto-submission, save failure,
  blocker, changed form, exact retry, old-client no-baseline, and source wording.

### Task 5 — Show a suite report, not a first-case report

- **ACTION:** Update source-specific labels on Runs and Run Detail.
- **IMPLEMENT:** For `saved_suite_v1`, use a generic stable “Saved suite · N
  cases” heading plus version link/ID rather than the first case's title.
  Keep the backend summary and comparison as read-only facts. Show per-case
  result, check evidence, start/cleanup, build/device, and pinned baseline link.
  Continue polling until persisted comparison arrives; queue/recovery reason
  stays before the map. Preserve the visual order from manifest/attempts.
- **MIRROR:** `run-result.tsx:20-150`, `RunDetail.tsx:25-235`,
  `coverage-flow.tsx:894-922`.
- **IMPORTS:** Existing generated `RunResponse`, React Router `Link`, Mantine
  components, `runQuery`.
- **GOTCHA:** Old manifests may have no `source`; label them Release plan.
  The run manifest lacks suite title, so do not infer it from case zero or
  rewrite the manifest just for display.
- **VALIDATE:** DOM tests for multi-case suite, optional failure, pending
  comparison, recovery queue, old manifest and sealed evidence links.

### Task 6 — Exercise the full HTTP path without a phone

- **ACTION:** Add a deterministic two-case saved-suite smoke using the existing
  isolated sign-in, PostgreSQL, API process, and fake worker helpers.
- **IMPLEMENT:** Save two distinct case versions and a suite through actual
  browser routes, preview/create the suite, verify idempotent response, run both
  numbered attempts, restart API, and verify manifest, baseline and reports.
  Exercise pass, deliberately failed assertion, and blocked prerequisite
  variants without model/device calls. Avoid changing `execution_smoke`'s
  one-case assumptions unless a narrow shared helper is extracted; a focused
  script may invoke its fixture helpers. Add a `just` recipe only for an
  explicit smoke, not an ordinary check hook.
- **MIRROR:** `scripts/test_library_smoke.py:20-176`,
  `scripts/execution_smoke.py:28-76,78-305` and route fixture
  `apps/api/tests/execution.rs:828-911`.
- **IMPORTS:** Existing Python stdlib fixture helpers and fake worker command.
- **GOTCHA:** The intake-only synthetic APK is not a runnable Android app;
  label this smoke simulated. Run one fake worker claim per queued attempt if
  using `--once`.
- **VALIDATE:** Secret-free HTTP smoke after the full edit batch; assert actual
  routes, durable bytes, and every attempt rather than success badges alone.

### Task 7 — Perform explicit real-device acceptance and reconcile docs

- **ACTION:** After the complete implementation and one validation pass,
  recover the held host only through documented operator evidence and run the
  real demo suite on two builds.
- **IMPLEMENT:** Inspect the worker journal, host dirty marker, process/ports,
  and qualified profile. A fresh host boot and explicit recovery may be
  necessary per status. Never delete state to force a claim. Extend the
  isolated `regression_acceptance.py` harness or extract narrow reusable
  fixture helpers for a focused sibling: case A checks a task appears after
  Save; case B checks it survives restart. The broken flavor displays the
  immediate row but omits durable commit, so A stays passed and B regresses.
  Run good build, then broken build pinned to the first run. Record run IDs,
  fixture hashes, case/check outcomes, preflight/cleanup receipts, comparison,
  model usage and unresolved blockers. If the worker exposes a reproducible
  defect, make its smallest owned fix and rerun only invalidated checks.
- **MIRROR:** `scripts/regression_acceptance.py:34-120,167-390`,
  `docs/architect/development.md:295-311,335-407`,
  `docs/architect/status.md:3-5`; worker recovery in
  `execution/runner.py:341-390` and `qualification/runner.py:336-363`.
- **IMPORTS:** Existing operator `execution` task and `just` commands only.
- **GOTCHA:** Real-device execution is an explicit acceptance step, never part
  of ordinary checks. Browser admin-policy denial remains binding. `just dev`
  injects API/worker secrets itself; do not wrap it in Doppler or write env files.
- **VALIDATE:** Update spec 10 and status only with observed evidence. A blocked
  device or browser gate remains open; source/tests cannot close it.

## Testing strategy

| Test                   | Input                                                                   | Expected result                                                            |
| ---------------------- | ----------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| Contract compatibility | Old `SuiteRunRequest` JSON without baseline                             | Decodes as None; current JSON round-trips.                                 |
| Pure suite eligibility | Same suite/context, all required prior outcomes conclusive              | Suggested; failed prior result allowed.                                    |
| Pure suite eligibility | Changed version, context, or one required uncertain case                | Not suggested.                                                             |
| Route admission        | Two saved cases, suite, explicit build/profile/baseline                 | One run, two ordered attempts, exact manifest/source/pin.                  |
| Route denial           | Cross-app baseline/reference, archived member, stale env, revoked actor | 403/404/409/422 as existing boundary requires; no run.                     |
| Retry                  | Same ID/body then changed body                                          | Same run, then 409 conflict; no extra attempts.                            |
| Result policy          | Required pass plus optional failure                                     | Explicit optional issue; no “all cases passed”.                            |
| Result policy          | Required failure/blocked/dirty/mixed                                    | Failed or incomplete as specified; no false green.                         |
| Browser                | Choice/change/save/lost response                                        | Exact request and key pinned; clear blocked state.                         |
| HTTP smoke             | Two fake attempts and API restart                                       | Persisted manifest/evidence/comparison; labeled simulated.                 |
| Real acceptance        | Good → broken build on qualified demo                                   | Clean per-case proof and correct pinned comparison, or explicit open gate. |

Edge cases: empty/incomplete suite, 100-case boundary, duplicate logical case,
changed case version, unavailable/invalid APK, no qualified profile, offline or
incompatible worker, active phone action, quarantined phone, missing/invalid
baseline, API response loss, permission revocation, artifact failure, cancellation,
API restart, and mixed retry history. Add tests only where they verify a real
remaining behavior risk; avoid duplicate tests that mirror implementation.

## Validation commands and order

Finish **all** code, tests, generated-source edits, docs and config for this
scope first. Then run one consolidated validation pass; batch fixes and rerun
only failed/invalidated checks. No check-on-save, watchers, device or model
calls in ordinary checks.

```bash
just types
just format
just format-check
just check-contracts
just check-api
just check-web
just check-worker
just build
just smoke-test-library
just smoke-execution
# Run the new focused suite smoke recipe if added.
```

Expected: generated drift clean, static checks/build/tests pass; HTTP smokes
prove actual routes/persistence with fake evidence. `just check-api` needs the
documented isolated test DB; do not reset a shared dev DB. `just apk-fixtures`
only if fixture inputs changed. `npm audit`/`pip audit` from AGENTS applies
before a commit, using the relevant locked project dependency context.

Real acceptance is separate: use the explicit isolated
`scripts/regression_acceptance.py` pattern with the qualified nonsecret host
profile and locally built good/broken demo APKs. Check/recover the retained host
first. Follow `docs/architect/development.md:335-407` and the recovery journal.
For an authorized rendered UI session, use `just dev` and its `dev-logs`,
`dev-restart`, and `dev-stop` controls. Do not run either harness or boot an
emulator as part of plan verification.

## Acceptance criteria

- [ ] All business rules in spec 10 are implemented without a new suite model,
      queue, approval step, or handwritten consumer types.
- [ ] Explicit build/device/baseline are visible and frozen; old omitted
      baseline requests still work.
- [ ] One suite version produces one immutable multi-case run with every case
      accounted for and no false green from missing proof.
- [ ] Two-build comparison persists after restart and labels only verified,
      compatible cases as regressions/recoveries.
- [ ] Route, policy, browser and simulated HTTP checks pass once after edits.
- [ ] Real demo device acceptance is recorded with run IDs/evidence, or its
      specific gate remains open without a claim of live completion.
- [ ] Rendered browser acceptance is recorded only through allowed access;
      denial is left open.

## Completion checklist

- [ ] Source follows service/domain/controller boundaries; pure rules have no
      DB, network, clock or React dependencies.
- [ ] Existing transaction/lease/idempotency/cleanup invariants remain intact.
- [ ] Rust is the only wire-shape source; generated SDK/Zod/Pydantic are checked.
- [ ] Error messages are safe and actionable; logs contain IDs/reason codes,
      not secrets or raw device evidence.
- [ ] Canonical spec, decisions and status match observed behavior; historical
      snapshots are unchanged.
- [ ] `git diff` reviewed; no `.private`, tokens, env files or temporary GAN
      artifacts are committed.

## Risks

| Risk                                                    | Likelihood / impact | Mitigation                                                                                                                                                  |
| ------------------------------------------------------- | ------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Dirty AVD or server reservation blocks live run         | Known / high        | Evidence-backed operator recovery before new claims; preserve journals.                                                                                     |
| Baseline suggestion chooses an unrelated multi-case run | Medium / high       | Pure suite eligibility checks source/version/context and all required cases; route tests.                                                                   |
| Optional failures make a misleading green summary       | Medium / high       | Pure summary table test and explicit optional-issue wording.                                                                                                |
| New DTO breaks old clients or history                   | Low / high          | Additive serde default; serialization fixtures; never rewrite manifest rows.                                                                                |
| Rendered browser inspection denied by admin policy      | Known / medium      | Respect denial; record source/DOM and live API/device evidence separately.                                                                                  |
| Two-case fixture accidentally shares state              | Low / high          | Use the existing good/broken oracle: immediate-save case plus restart-persistence case, each on its own fresh AVD and with separate start/cleanup receipts. |

## Confidence

**7/10** for a single-pass source implementation: most paths already exist.
Live host recovery and allowed rendered-browser access are external acceptance
gates, so they cannot be assumed from source or simulated checks.
