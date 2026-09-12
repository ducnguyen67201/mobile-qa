# Plan: Phase 04 — execution and evidence reports

## Summary

Connect an approved, versioned test case and uploaded APK to durable Rust jobs, a polling Python worker, an Android device and a persisted evidence report. Introduce the small test-definition model now through operator import/review commands; phase 05 adds its editor. Minitap performs semantic navigation, while explicit checkpoints and independent verification establish correctness.

## User story

As an app developer, I want to run an approved check against an uploaded build and inspect its evidence, so that I can distinguish a demonstrated defect from a missing prerequisite or an unfinished check.

## Problem → solution

A working app/build dashboard and a separate hard-coded Android demo → a browser-triggered, version-pinned execution with progress, cancellation, recovery and retained evidence.

## Metadata and baseline

- Complexity: XL; approximately 100 authored files plus generated consumers; 14 ordered tasks.
- Source: `docs/architect/implementation/04-execution-and-reports.md`; product sections 4–8 own domain and verdict semantics.
- Status: planning complete; implementation and acceptance remain pending.
- Inspected on 2026-09-12: origin/main `9eb5dba` contains specs 01 and 03. PR #2 is open; device branch `d4c4729` includes that main plus spec 02. This plan uses `d4c4729` as its integrated reference, not this older detached checkout (`d137e35`).
- Read-only reference was extracted to `/tmp/mobile-qa-04-plan-reference`; file/line references below are relative to the integrated Git revision, not the current checkout or an enduring dependency on that temporary folder.
- Implementation must start from main containing both streams, or an explicitly integrated local branch, before applying these tasks. Do not overwrite newer app setup with this checkout's scaffold.
- Full Minitap reliability campaign/cloud qualification and hosted artifact acceptance remain open. Local/fake implementation can proceed; real execution acceptance cannot be claimed until the chosen device configuration is qualified. A local demo is not cloud qualification.
- `docs/CODEX-NAVIGATION-GUIDE.md` was absent. Follow `AGENTS.md` and `docs/architect/README.md`; do not read parent sources.

## What exists today: tests and actions

There is one executable qualification case, `persist-task-v1`, tied to `ai.mobileqa.demo`. `QualificationRequest` has a case identifier, not a reusable case definition: its validator rejects another case/package/serial. The fake protocol selects a predetermined scenario; it is not a customer test either.

The real demo sends Minitap a goal to create exactly `qa-<attempt UUID>`, press Save and stop. The supervisor captures the created state, force-stops/relaunches the app, captures it again and evaluates the exact task row in the UI hierarchy. Minitap does not currently perform the restart. ADB-demo mode instead uses deterministic fixture navigation, without model calls. These modes must remain separately identified.

There are no persisted case/suite/plan versions, approvals, customer runs or worker HTTP leases in main. Product examples AUTH-001/002/003 and TASK-001/002 are requirements examples, not implemented records.

## Proposed test definition and action execution

### Definition, execution and evidence are separate

Author a bounded JSON import document; validate it with the same Rust service that phase 05 will call. JSON is the canonical first import format; no YAML dependency or custom programming language. Definitions contain semantic instructions and typed checks, not arbitrary shell/Python or fixed screen coordinates. The following is a readable example, not a generated wire fixture:

```text
Case: TASK-PERSIST / version 1 / Android
Requirement: a saved task survives an app process restart
Data: task_title = unique synthetic value derived from attempt ID
Preconditions: supported APK; qualified device; clean fixture; backend ready
Actions:
  A1 navigate: create task named ${task_title}, press Save, return to list
  A2 checkpoint: capture created screen, prove task exists
  A3 restart_app: stop and launch the manifest's app, retain app data
  A4 checkpoint: capture reopened screen, compare exact task text
Expected check C1: same task exists after restart
Evidence: created/reopened PNG + bounded UI hierarchy + action trace
Reset: qualified clean-device adapter + separately qualified fixture reset
Budgets: explicit case duration, navigation steps, capture bytes, model policy
Review: business expectation approved; executability approved; exact content hash
```

`A2` is evidence that setup for the persistence assertion succeeded. Failure to create the task is inconclusive for this persistence-only case, unless an additional approved creation assertion explicitly defines a functional failure. Do not accidentally turn a navigation failure into a persistence bug. A3 restarts the process; deleting app data belongs to cleanup, never this action.

### Minimal schema, defined once in Rust

Use shared leaf types in `crates/contracts/src/execution.rs`, deriving Serialize/Deserialize, JsonSchema and ToSchema where applicable. Explicit validation supplements derive constraints. Browser operations expose them through Utoipa; worker envelopes expose them through the existing WorkerContracts schema root. No manually copied TS/Python DTOs.

- CaseVersion: UUID identity, logical case key, positive version, app ID, title, inline source/criterion references with content hash, provenance (`operator_authored` initially), priority, Android capabilities, ordered preconditions, data references, actions, expected checks, evidence requirements, setup/reset adapter references and revisions, budgets, author and lifecycle. Source excerpts must be non-secret; phase 06 supplies source-document entities later.
- Action: stable action ID and ordered tagged union `navigate {instruction, checkpoint_id}`, `restart_app {checkpoint_id}`, `checkpoint {checkpoint_id}`. No branching, loops, arbitrary commands or customer-provided executable callbacks. A single navigate action may cause several SDK taps/types/swipes. Capture after each action; explicit checkpoint actions add checkpoints without mutations.
- Check: stable check ID, checkpoint ID, expected description, method/version, prerequisite checkpoint references, requiredness and evidence IDs. Initial executable method `ui_property_equals_v1`: package-scoped resource ID, property enum `text|content_description|checked|enabled`, typed expected value and bounded observation window. For absent/present elements, use `ui_element_presence_v1` with exact target, expected Boolean and a positive screen-ready marker. Missing ready marker, wrong package or ambiguous target is inconclusive, not evidence of absence.
- Equality defaults to exactly one matching element; multiple matches require a declared exact-text target filter, otherwise inconclusive. No implicit contains/fuzzy match. A stable screen and expired observation window with a contradictory value can establish failure. Raw capture pairs and timestamps remain available to audit that determination.
- Required checks reference successful prerequisite observations. A missing prerequisite observed at runtime is blocked; unknown/failed navigation is inconclusive. Missing required evidence prevents pass. JSON limits: import <=1 MiB, <=20 actions and <=20 checks per case, <=100 resolved cases per plan; nonempty unique IDs and all references resolve. Limits are initial configurable policy ceilings, not reliability claims.
- Data values: allow bounded literal synthetic strings and an explicit `attempt_unique` generator. Resolve only named placeholders with a small substitution function; no template evaluation. Store resolved non-secret values per attempt. Credential references are opaque IDs and can be resolved only by an allowlisted operator adapter; no secret values in case JSON, manifests, event bodies or reports.
- SuiteVersion: logical identity/version, title and ordered case-version references. PlanVersion: identity/version, objective, approved suite/direct-case references, requiredness, data variants, pinned device/profile, environment/reset policies, budgets/retry policy and exclusions. The build is selected at run submission, never stored as the plan's permanent build.
- Approval: exact definition revision/hash, actor, timestamp and purpose (`business` or `executability`). Add explicit app-scoped reviewer grants (`business`, `execution`) through trusted operator commands. An operator role alone cannot assert customer business approval; one authorized person may hold both grants. Import is draft-only; explicit approve commands call services with grants and compare reviewed hashes. All case/suite/plan dependencies must be approved at exact versions. Deprecated versions remain readable historically but unavailable for new selection.
- V1 permits one device and sequential execution, while preserving suite membership/dedup semantics. Same `(case_version, data_variant, device_configuration)` resolves once. Conflicting logical case versions or policies reject submission.

### Who performs an action

1. Rust resolves approved definitions and freezes the manifest; the browser cannot supply a replacement prompt or expected result in the run request.
2. Python installs the exact checked APK and applies the allowlisted fixture adapter.
3. Python compiles each navigate instruction with approved synthetic data and boundaries, then calls the pinned SDK's `Agent.run_task(request=TaskRequest(...))` on the reserved serial. Minitap observes the screen and selects low-level actions; runtime coordinates/selectors remain trace data. This follows the documented task execution model: [Minitap tasks](https://www.minitap.ai/docs/mobile-use-sdk/core-concepts/tasks).
4. Python owns deterministic lifecycle actions and checkpoint capture. It sequences semantic actions, not another model controlling every tap. Keep SDK import/model access inside the isolated child process.
5. Python emits checkpoint observations and hashed evidence; Rust verifies artifact identity, checks method/input agreement and independently evaluates typed evidence before committing outcomes. Re-evaluate the supported UI checks from retained bounded hierarchy bytes in Rust, so a worker-supplied verdict alone cannot establish pass. Add direct API dependency quick-xml = "=0.41.0", already present transitively in Cargo.lock; use quick_xml::{Reader, events::Event}. Reject DTD/custom entities, input above 2 MiB, depth above 128 and more than 10,000 nodes; keep end-name/attribute validation enabled. Built-in XML escaping must decode normally for exact-text comparisons. Validate PNG structure/dimensions against the profile before counting it as available evidence.
6. Record SDK task status separately from semantic action evidence and low-level trace availability. Do not claim a precise live tap count or one-to-one action trace correlation that the pinned SDK does not expose. Missing trace details have explicit reasons.

Unsupported visual/manual/backend checks are retained as non-executable definitions and prevent normal submission in this phase; do not silently omit them. A future registered read-only backend verifier requires its own endpoint policy and qualification. Arbitrary customer APK compatibility, external browser login and generic credential-entry automation are not promised: the first real acceptance uses the uploaded controlled demo and its registered adapter, then a separately qualified app adapter can reuse the same API/protocol. Definitions are reusable within those supported capabilities.

## UX design

Before: App → uploaded build/validation; Tests and Runs → placeholders; device demo → separate operator CLI/private report.

After:

```text
App / build → Release check preview → Run test → Run detail
                  |                             |
          approved case + checks          queued / running / finalizing
          prerequisites + gaps            cancel requested / recovery required
                                                |
                                      expected vs observed + evidence
                                      attempts + missing checks + reset state
```

| Touchpoint      | Before              | After                                                                        |
| --------------- | ------------------- | ---------------------------------------------------------------------------- |
| AppDetail build | APK validation only | Build-specific readiness and Run button, disabled with concrete blockers     |
| Tests           | Placeholder         | Read-only default approved plan/case details; operator-managed label         |
| Runs            | Placeholder         | App-filtered paginated list, durable detail URL and reload recovery          |
| Report          | Private CLI file    | Persisted checks, evidence, attempt history and operational cleanup state    |
| Cancel          | Absent              | Pending acknowledgement shown until termination or quarantine is established |

Use Mantine components and existing workspace routing. No editor or generation buttons. An empty plan says an operator must configure an approved test. Network errors provide Retry without duplicating submission. Poll every 2 seconds while visible, 15 seconds while hidden, stop when terminal; configurable values are UI defaults. Terminal report and cleanup status remain separately visible.

## Mandatory reading / unified discovery

All references use baseline `d4c4729` and are repository-relative. “All” means the full file.

| Priority/category | File:lines                                                                                            | Pattern / reason                                                              |
| ----------------- | ----------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| P0 authority      | docs/architect/README.md:all; product.md:sections 4–8; implementation/04-execution-and-reports.md:all | Domain, approval, manifest, outcome and recovery rules                        |
| P0 status         | docs/architect/status.md:all                                                                          | Distinguish local evidence from pending cloud/device qualification            |
| P0 types          | crates/contracts/src/browser.rs:68–334, 734–end                                                       | Browser DTOs, OpenAPI registration and OPERATIONS route agreement             |
| P0 types          | crates/contracts/src/worker.rs:all; worker/qualification.rs:8–44, 95–173                              | Existing fake and qualification contracts stay distinct                       |
| P0 entry/error    | apps/api/src/controllers/setup.rs:102–134, 219–246; errors.rs:1–79                                    | Thin Axum handlers, ApiResult, safe structured failures                       |
| P0 auth/state     | apps/api/src/services/apps.rs:18–44, 104–166, 236–254                                                 | Tenant authorization, transactions and currently hard-coded readiness         |
| P0 worker         | apps/mobile-worker/src/mobile_qa_worker/qualification/runner.py:48–121, 136–330                       | Supervised child, dirty marker, cleanup and existing failed-process usage gap |
| P0 SDK            | apps/mobile-worker/src/mobile_qa_worker/qualification/sdk_adapter.py:75–174; sdk_compat.py:all        | Pinned 4.0.0 task seam and guarded ToolRuntime compatibility                  |
| P0 device         | apps/mobile-worker/src/mobile_qa_worker/qualification/device.py:232–308; verifier.py:75–110           | Demo-only APK/markers and deterministic persistence oracle                    |
| P1 persistence    | apps/api/migration/src/lib.rs:all; m20260912_000001_app_setup.rs:all; models/_entities/builds.rs:all  | SeaORM schema registry and immutable APK storage identity                     |
| P1 storage        | apps/api/src/storage/mod.rs:111–205; services/uploads.rs:302–352                                      | Private publication/materialization and parent-scoped builds                  |
| P1 operator       | apps/api/src/tasks/operator.rs:19–31, 58–77, 247–end                                                  | Trusted task entry, actor membership and safe audit logging                   |
| P1 client         | apps/web/src/api/setup.ts:1–37; runtime.ts:all; pages/AppDetail.tsx:all                               | SDK + generated Zod, CSRF closure, centralized error handling                 |
| P1 routes         | apps/web/src/routes.tsx:all; hooks/use-workspace.ts:all                                               | Authenticated workspace screens and query scope                               |
| P1 Rust tests     | apps/api/tests/app_setup.rs:128–148, 185–207, 604–641                                                 | Real HTTP/database fixtures, status/error/security agreement                  |
| P1 UI tests       | apps/web/src/pages/AppDetail.test.tsx:1–80                                                            | Mantine/Query/router harness and fetch interception                           |
| P1 worker tests   | apps/mobile-worker/tests/test_sdk_adapter.py:1–90; test_verifier.py:all; test_qualification.py:all    | Subprocess isolation, SDK seam and fault fixtures                             |
| P1 configuration  | apps/api/src/config.rs:1–97; apps/mobile-worker/pyproject.toml:all; qualification/config.py:all       | Process-only settings, Python 3.12+, pinned SDK                               |
| P1 generation     | scripts/contracts.py:14–80; crates/contracts/src/bin/export.rs:all                                    | Existing schema roots and content-only synchronization                        |
| P1 checks/logging | justfile:1–50; scripts/runtime.py:140–end; middleware/mod.rs:all                                      | Explicit check window and request/reason-code logging                         |
| P2 operations     | docs/architect/environment.md; development.md; dependencies.md; device-qualification.md:all           | Doppler, no watchers, dependency and real-device constraints                  |

### Five traced paths

- Browser submission: protected workspace page → generated SDK/Zod wrapper → Session extractor → app authorization → run service transaction → manifest/jobs → durable detail query.
- Worker: explicit polling CLI → worker authentication → PostgreSQL claim → local host lock/dirty journal → SDK subprocess/checkpoints → ordered events/artifact upload → fenced completion.
- Evidence: local private bounded capture → reserved artifact identity → stream/hash validation → ArtifactStore publication → sealed DB metadata → Rust check evaluation → authenticated report content endpoint.
- Cancellation: authorized request → persisted cancel flag → heartbeat response/local watchdog → process-group termination → partial evidence + cleanup receipt → finished or quarantine. Browser disappearance changes nothing.
- Recovery: stale lease → fenced attempt and resource quarantine → old process stop + reset proof → explicit recovery reconciliation → new eligible capacity. No automatic replay of a partly executed case.

## Patterns to mirror

### Naming and thin controller

Source: `apps/api/src/controllers/setup.rs:119–125`:

```rust
async fn get_app(
    State(ctx): State<AppContext>,
    session: Session,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<AppResponse>> {
    let app = apps::authorized(&ctx, session.user.id, id).await?;
    Ok(Json(apps::detail(&ctx, &app).await?))
}
```

Rust/Python files and functions use snake_case; Rust DTOs and React pages use PascalCase. Services take AppContext and validated identities; controllers remain transport adapters.

### Error handling

Source: `apps/api/src/errors.rs:24–31`:

```rust
pub fn missing() -> Self {
    Self::new(404, "not_found", "Record not found")
}
pub fn unauthorized() -> Self {
    Self::new(401, "unauthenticated", "Sign in to continue")
}
pub fn invalid(message: impl Into<String>) -> Self {
    Self::new(422, "invalid_input", message)
}
```

Use 404 for foreign scoped objects. Specific conflicts use 409. Preserve request ID/no-store/error normalization; no raw SQL, URLs with credentials or SDK output in public errors.

### Repository/service transaction

Source: `apps/api/src/services/apps.rs:18–25, 117, 165`:

```rust
memberships::Entity::find()
    .filter(memberships::Column::UserId.eq(user))
    .filter(memberships::Column::OrganizationId.eq(org))
    .filter(memberships::Column::Active.eq(true))
    .one(&ctx.db)
    .await?
    .ok_or_else(ApiFailure::missing)
```

Existing create begins `let tx = ctx.db.begin().await?;` and commits `tx.commit().await?;`. Use `sea_orm::{TransactionTrait, EntityTrait, ColumnTrait, QueryFilter, ActiveModelTrait, Set}`; new claim SQL can use bound `Statement` parameters where ORM expression support is awkward. Never keep a transaction open during device or storage I/O.

### Logging

Source: `apps/api/src/services/uploads.rs:299`:

```rust
tracing::info!(upload_id=%id,app_id=%app.id,phase="uploaded","APK bytes sealed");
```

Use run/attempt/worker IDs, phase and reason code for execution logs. Raw prompts, credentials, access tokens and unrestricted SDK traces stay out of ordinary logs.

### Runtime client validation

Source: `apps/web/src/api/setup.ts:18–26`:

```ts
async function checked<T>(
  promise: Promise<{ data: unknown; response: Response }>,
  schema: z.ZodType<T>,
  statuses: readonly number[] = [200],
): Promise<T> {
  const result = await promise
  if (!statuses.includes(result.response.status))
    throw new Error(`Unexpected API response status (${result.response.status})`)
  return schema.parse(result.data)
}
```

Extract this helper and CSRF state to a shared session-transport module before adding runs.ts; do not create a second token cache. Preserve runtime.ts's generated `zApiError.safeParse` boundary.

### Tests

Source: `apps/api/tests/app_setup.rs:185–190`:

```rust
#[tokio::test]
async fn persisted_workflow_auth_scope_and_real_validation() {
    let _database = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, ctx| async move {
        let owner = login(&server, &ctx).await;
        let foreign = login(&server, &ctx).await;
```

Move reusable helpers into `apps/api/tests/support/mod.rs`; ordinary route tests serialize destructive DB setup. Concurrency tests deliberately use separate connections/transactions with synchronization barriers inside one isolated database test.

## External research (2026-09-12)

| Topic                 | Official source                                                      | Finding                                                                                                                                                                               |
| --------------------- | -------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| SDK goals/checkpoints | https://www.minitap.ai/docs/mobile-use-sdk/core-concepts/tasks       | Natural-language goals, tracing, app-lock and task step settings; use existing pinned source seam                                                                                     |
| Agent lifecycle       | https://www.minitap.ai/docs/mobile-use-sdk/sdk-reference/agent       | Agent task execution and stop facilities; process termination still needs our supervisor                                                                                              |
| Queue claims          | https://www.postgresql.org/docs/current/sql-select.html              | SKIP LOCKED supports competing queue consumers; it is not a general consistent-read mechanism                                                                                         |
| Transactions          | https://www.sea-ql.org/SeaORM/docs/advanced-query/transaction/       | Begin/commit/rollback supported; keep resource reservation atomic                                                                                                                     |
| XML evidence          | https://docs.rs/quick-xml/latest/quick_xml/reader/struct.Reader.html | General streaming reader reference (currently 0.42.0); exact 0.41.0 documentation could not be retrieved. Keep locked 0.41.0 and compile against that API; enforce application bounds |

KEY_INSIGHT: Live SDK documentation has changed since older indexed examples; do not copy PlatformTaskRequest or default step counts. APPLIES_TO: SDK adapter. GOTCHA: retain minitap-mobile-use 4.0.0 and its actual `TaskRequest` fields, compatibility guard and source seam tests.

KEY_INSIGHT: Row locking selects eligible jobs; unique active reservations protect physical resources. APPLIES_TO: scheduler. GOTCHA: SKIP LOCKED alone does not prevent two distinct jobs from selecting the same phone/account.

## Architecture and scope

### Durable schema

Add migration `m20260912_000004_execution.rs` (if that identifier was consumed on the integration base, use the next unused migration number and update this plan before authoring). Retain registry marker. Tables and corresponding SeaORM entities:

- `test_cases`, `case_versions`, `test_suites`, `suite_versions`, `test_plans`, `plan_versions`: stable logical roots plus immutable version content in validated JSONB; unique `(root_id, version)`, app/org ownership, lifecycle and content hash. References inside JSONB are validated transactionally; no deletion of referenced versions.
- `reviewer_grants`, `definition_approvals`: scoped authority and exact-hash approvals; unique purpose/version approval, immutable audit.
- `execution_profiles`: approved device/image/model/adapter revision and qualification evidence reference. Worker-reported capability cannot self-approve a profile.
- `device_workers`, `execution_resources`: worker identity, credential digest/revocation, profile, health; resources represent device, shared account and app concurrency slot. No-account cases explicitly omit an account reservation; never fake a credential to satisfy readiness.
- `runs`: creator, app/org, build, approved plan, frozen manifest JSON/hash, idempotency key and request fingerprint, lifecycle, coverage result, timestamps. Unique `(app_id, idempotency_key)`; fingerprint includes actor and normalized submission. Cross-actor replay conflicts without disclosing another request.
- `case_executions`, `attempts`, `execution_jobs`: dedup identity, requiredness, ordered attempts, generation, worker, lease expiry, lifecycle/cancel/recovery state and reason. One active attempt/job per execution; retries append attempts.
- `resource_reservations`: resource ID unique while unreleased, attempt ID, generation, cleanup state. Expiry does not release a reservation; stop/reset reconciliation does.
- `execution_events`: event UUID, attempt, sequence, payload hash/body; unique event ID and `(attempt, sequence)`. Exact retry is a no-op; identity/payload conflict is 409.
- `run_artifacts`: attempt/checkpoint linkage, server-issued storage key, MIME, byte budget, expected/actual hash, pending/sealed/unavailable state and reason. Never trust a caller storage path.
- `check_results`: frozen expected method/value, observed typed value, prerequisite/evidence links and result reason. Final records are immutable.

Use tenant-aware foreign keys where relational IDs exist, indexes for app list/queued selection/lease expiry, timestamptz and UUIDs consistent with setup. Approved version edits create a new draft; administrative audit and cleanup changes cannot rewrite the manifest or original outcome.

### HTTP contracts

All JSON routes return declared generated envelopes or ApiError. Browser reads require Session; writes retain Origin/CSRF protections. Worker-only routes use a separate bearer extractor, never a cookie-auth bypass.

| Method/path                                                           | Input → success                                                                                              | Purpose                                                       |
| --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------- |
| GET /api/apps/{app_id}/execution-plan                                 | selected build_id query → 200 PlanPreviewResponse                                                            | Read-only approved definition/coverage, profile and blockers  |
| POST /api/apps/{app_id}/runs                                          | build_id, plan_version_id, environment_revision; Idempotency-Key header → 201 RunResponse (200 exact replay) | Freeze and enqueue; reject stale preview                      |
| GET /api/apps/{app_id}/runs                                           | cursor/limit → 200 RunListResponse                                                                           | App-scoped history                                            |
| GET /api/runs/{run_id}                                                | none → 200 RunResponse                                                                                       | Manifest, progress, attempts/checks/artifact descriptors      |
| POST /api/runs/{run_id}/cancel                                        | none → 200 RunResponse                                                                                       | Idempotent cancellation request                               |
| GET /api/runs/{run_id}/artifacts/{artifact_id}/content                | none → 200 binary                                                                                            | Authenticated private evidence, safe content type/disposition |
| POST /api/worker/claims                                               | protocol_version, claim_id, profile_id → 200 ClaimResponse tagged leased/idle                                | Atomically claim and recover a lost response                  |
| POST /api/worker/attempts/{attempt_id}/heartbeat                      | generation, last_event_sequence → 200 LeaseStatusResponse                                                    | Renew deadline, return cancel/recovery instruction            |
| POST /api/worker/attempts/{attempt_id}/events                         | generation, bounded ordered event batch → 200 EventReceipt                                                   | Deduplicated progress/checkpoint observations                 |
| GET /api/worker/attempts/{attempt_id}/build                           | generation query → 200 APK bytes                                                                             | Download only this lease's immutable build                    |
| POST /api/worker/attempts/{attempt_id}/artifacts                      | generation + metadata → 201 ArtifactUploadResponse                                                           | Reserve upload ID and byte budget                             |
| PUT /api/worker/attempts/{attempt_id}/artifacts/{artifact_id}/content | binary body + generation header → 200 ArtifactReceipt                                                        | Bounded stream/hash publication; exact retry idempotent       |
| POST /api/worker/attempts/{attempt_id}/complete                       | generation, observation/evidence refs, usage, execution status → 200 AttemptReceipt                          | Fenced evaluation/finalization; cleanup may still be pending  |
| POST /api/worker/attempts/{attempt_id}/cleanup                        | generation, stopped/reset evidence and host boot identity → 200 CleanupReceipt                               | Release or quarantine owned resources                         |

Mutation worker routes require worker bearer plus per-attempt token (`X-Lease-Token`) after claim. Token is an opaque random capability scoped by DB identity/generation/expiry, hashed at rest. Claim response replay reissues a token for the same still-active claim and invalidates the old token; it never leases another job for the same claim ID. Worker stores one durable pending claim ID and request before sending and switches to its accepted lease atomically. No claim while its local journal is dirty. A recovery receipt for a stale generation can be recorded as diagnostics by trusted maintenance, but cannot finalize outcome or automatically release newer resources.

401 invalid/revoked identity; 404 foreign resource; 409 conflicting payload/stale revision/fence/protocol mismatch; 422 unsupported definition/readiness; 413 byte limit; 429 quota; 503 temporary storage/dependency outage. Expand route contract tests and OPENAPI operation registry; current setup test enumerates all browser operations and must substitute new path parameters and accept the new security scheme where relevant. Worker schema envelopes are generated to Python, and their actual HTTP mapping is covered by separate real route tests; don't generate a handwritten Python mirror of OpenAPI.

### Scheduling, budgets, cancellation

Lock app concurrency resource first, then device/account in deterministic ID order, then job; all reservation/claim paths use the same order. Use a short transaction with eligible job selection and `FOR UPDATE SKIP LOCKED`; unique unreleased resource reservations and conditional generation updates close races. DB time owns lease expiry. Default one active case per app and one per device/account. Do not use Loco's generic retries for device mutations.

Initial explicit policy: heartbeat 10s, lease 60s, local stop deadline 40s since last acknowledged renewal, HTTP timeout 5s with bounded jittered backoff. Record defaults in process settings and test them with injected clocks. On lost API connectivity, the worker must stop before lease expiry and quarantine if termination/cleanup is uncertain. Server expiry invalidates authority and keeps resources reserved; it does not schedule them onto another host. Readiness/claim paths lazily reconcile expiry, and an explicit maintenance task handles idle systems; neither depends on a browser staying open.

Case/plan budgets are operator-supplied and checked against host ceilings; process hard deadline bounds wall time, SDK max_steps bounds its configured loop but is not advertised as exact tap count, byte quota covers all captures and traces. Model usage is persisted including failures/unknown counts; observed budget exhaustion stops new work. Hard token/spend ceilings are only offered where enforceable provider limits exist; otherwise record unavailability and reject policies that demand that hard guarantee. Do not use a prompt as a budget enforcement mechanism.

Queued cancellation marks unstarted executions canceled and removes eligible jobs. Active cancellation requests stop; outcomes already established remain immutable. Completion racing cancel is resolved transactionally: an already-final attempt stays final; cancel seen before successful finalization preserves supported checks but leaves unfinished coverage canceled. No automatic retry by default; plan may permit one diagnostic retry, only after verified reset and a new attempt ID. Mixed failure/pass remains review-required. Required failed evidence has precedence over later uncertainty.

### Artifacts and secrets

Reuse ArtifactStore's private local/S3 publication; initial transfer proxies through API instead of adding multipart/presigned workflows. Stream with a cap and hash, publish outside transaction, then seal conditionally for the current generation. Quarantined/orphan files remain unavailable to reports; existing cleanup gains scoped pending-artifact cleanup. Revalidate authorization on content reads; render PNG/video only through safe MIME handling, download XML/raw logs with nosniff and attachment disposition. Never serve raw trace HTML inline.

Worker host config maps opaque fixture/profile/credential references to allowlisted adapters. Demo adapter contains demo tunnel/backend/reset specifics. General device utility accepts validated package/activity/profile and has no unconditional demo tunnel, `run-as`, or demo storage assertion. Align build size with the API's 250 MiB upload limit using the smaller advertised host capability (current demo limit is 100 MiB); reject incompatibility before claim, never silently truncate. Resolve activity from trusted APK metadata or qualified adapter config, not arbitrary command text.

Process config uses Doppler injection for secret consumers only. No .env files or new global configuration. Worker boot token is operator-provisioned from injected input, stored only as digest server-side; local fakes use isolated non-production credentials. Model child receives only its necessary model configuration, not worker bearer, lease token, database URL or Doppler token. Do not register raw SDK directories wholesale as customer-visible evidence: allowlist normalized redacted artifacts, preserve unknown usage, and retain raw diagnostics only in private operator custody.

## Alternatives and exclusions

Chosen: versioned semantic case plus a small typed action/check sequence; PostgreSQL leases; polling UI; reuse SDK and existing device lifecycle. Hard-coded free-text jobs would bypass approvals; a custom tap DSL/editor would duplicate SDK navigation and phase 05; generic background retries could replay physical effects; live streaming is unnecessary for the first report.

NOT building: full Tests editor, AI generation, exploratory discovery, general backend/visual verifier, arbitrary credential workflows, iOS, multiple simultaneous phones, auto-scaling, deployment/provider changes, paid qualification launches, report PDF export, billing or regression comparisons. No merging/pushing/provisioning is authorized by this planning request.

## Files to change

Paths below are the intended implementation map. Existing generated consumers are updated only through `just types`.

| Files                                                                                                                                                   | Action           | Purpose                                                                  |
| ------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | ------------------------------------------------------------------------ |
| crates/contracts/src/execution.rs; worker/execution.rs                                                                                                  | CREATE           | Shared definition/result leaves and worker HTTP envelopes                |
| crates/contracts/src/lib.rs; browser.rs; worker.rs                                                                                                      | UPDATE           | Export/derive/register schemas and operations, preserve old protocols    |
| crates/contracts/tests/execution.rs; contracts/fixtures/execution/*.json                                                                                | CREATE           | Valid/invalid definition, lease and event fixtures                       |
| apps/api/migration/src/m20260912_000004_execution.rs; lib.rs                                                                                            | CREATE/UPDATE    | Durable schema and registration                                          |
| apps/api/src/models/_entities/{table_name}.rs for each table above; mod.rs                                                                              | CREATE/UPDATE    | SeaORM entities with matching relations                                  |
| apps/api/src/services/test_definitions.rs; runs.rs; scheduler.rs; run_artifacts.rs; verification.rs; worker_auth.rs                                     | CREATE           | Validation/approval, manifests, leases, storage, verdicts and auth       |
| apps/api/src/services/apps.rs; mod.rs; config.rs                                                                                                        | UPDATE           | Build/plan-specific readiness, service exports and process settings      |
| apps/api/src/controllers/runs.rs; worker.rs; mod.rs; app.rs                                                                                             | CREATE/UPDATE    | Routes and lifecycle/task registration                                   |
| apps/api/src/tasks/execution.rs; mod.rs                                                                                                                 | CREATE/UPDATE    | Import/review/grants/profile/worker provisioning and reconciliation      |
| apps/api/src/storage/mod.rs; tasks/cleanup.rs                                                                                                           | UPDATE           | Streaming read and pending evidence cleanup reuse                        |
| apps/api/tests/support/mod.rs; app_setup.rs; execution.rs; worker_protocol.rs                                                                           | CREATE/UPDATE    | Shared auth setup, real routes, concurrency and recovery                 |
| apps/api/Cargo.toml; Cargo.lock                                                                                                                         | UPDATE if needed | Bounded XML parsing dependency through package manager only              |
| apps/mobile-worker/src/mobile_qa_worker/execution/{**init**,client,journal,runner,actions,sdk_adapter,observations,artifacts,config,fake}.py            | CREATE           | HTTP worker, durable spool, parameterized execution and fake transport   |
| apps/mobile-worker/src/mobile_qa_worker/device_runtime/{**init**,device,process,evidence}.py                                                            | CREATE           | Extract reusable device/supervisor primitives from qualification         |
| apps/mobile-worker/src/mobile_qa_worker/qualification/{device,process,evidence,runner,sdk_adapter}.py                                                   | UPDATE           | Retain demo wrappers, shared utilities and failure-path usage harvesting |
| apps/mobile-worker/src/mobile_qa_worker/cli.py                                                                                                          | UPDATE           | Explicit execution-worker command; fake and qualification stay separate  |
| apps/mobile-worker/tests/test_execution_{client,runner,actions,artifacts}.py                                                                            | CREATE           | Fault injection, contract and supervisor tests                           |
| apps/web/src/api/session-transport.ts; runs.ts; runs.test.ts; setup.ts                                                                                  | CREATE/UPDATE    | Shared CSRF helper and validated run calls                               |
| apps/web/src/pages/{Runs,RunDetail,Tests}.tsx; corresponding .test.tsx                                                                                  | CREATE           | List, report and read-only definitions                                   |
| apps/web/src/components/app/run-preview.tsx; pages/AppDetail.tsx; routes.tsx                                                                            | CREATE/UPDATE    | Build selection, preview and run action                                  |
| scripts/execution_smoke.py; justfile; .github/workflows/ci.yaml; scripts/test_ci_scope.py                                                               | CREATE/UPDATE    | Explicit fake HTTP smoke and affected CI coverage                        |
| docs/architect/{status,contracts,system,environment,development,dependencies}.md; implementation/04-execution-and-reports.md                            | UPDATE           | Implemented evidence vs pending gates, protocol and operations           |
| contracts/browser.openapi.json; contracts/worker.schema.json; apps/web/src/api/generated/*; apps/mobile-worker/src/mobile_qa_worker/generated/models.py | GENERATE         | Rust-authoritative consumer output                                       |

## Step-by-step tasks

All VALIDATE bullets describe the final verification window, not checks after each edit. Finish all agreed code/tests/config first, then generate/format/check/build once. Batch fixes and rerun only invalidated checks.

### Task 1 — establish integrated baseline and contract packet

- ACTION: Carry this plan onto the integrated 02+03 source and author definition/worker/browser contracts.
- IMPLEMENT: Shared types and semantic validation above, request/status/error mapping, fixtures, explicit protocol version 1 separate from qualification and fake versions. Keep one owner for contracts/migrations/generated outputs.
- MIRROR: browser.rs registration; worker.rs strict tagged variants and explicit validate; scripts/contracts.py schema roots.
- IMPORTS: serde, schemars, utoipa, uuid, chrono; export execution from lib.rs and worker.rs.
- GOTCHA: Do not implement against detached d137e35; no API/database imports in contracts.
- VALIDATE: Schema round-trip and invalid reference tests; real operations inventory agrees after route tasks finish.

### Task 2 — persist definitions and approvals

- ACTION: Add migration/entities and test_definitions service.
- IMPLEMENT: Tables above; immutable version content; approval hash comparison/grants; suite/plan resolution, duplicate conflict rejection and unsupported-method blockers. Make definition import transactional and idempotent by app/logical key/version/content hash.
- MIRROR: services/apps.rs transactions and scoped authorization; migration registry.
- IMPORTS: crate::errors::{ApiFailure,ApiResult}, crate::models::_entities, loco_rs::app::AppContext, sea_orm transaction/query traits, mobile_qa_contracts::execution.
- GOTCHA: Existing operator role is not customer expectation approval; no imported Boolean that simply sets approved=true.
- VALIDATE: Stale approval, foreign reference, conflicting versions, immutable historical definition and duplicate membership tests.

### Task 3 — operator import and readiness

- ACTION: Add Execution maintenance Task and read-only plan preview.
- IMPLEMENT: Commands `import`, `grant-reviewer`, `approve`, `register-profile`, `register-worker`, `reconcile` with actor/app scope. Import accepts non-secret JSON path; secrets only injected environment. Separate import from approval and qualification registration. Preview computes build validation/profile compatibility/plan/account/reset blockers against selected build; do not just flip execution_ready=true globally.
- MIRROR: tasks/operator.rs actor gate and services/apps.rs readiness.
- IMPORTS: loco_rs::task::{Task,TaskInfo,Vars}; services::{apps,test_definitions}; generated DTO re-exports.
- GOTCHA: Missing account reference can be valid only for an explicit no-account adapter; an unqualified profile stays blocked.
- VALIDATE: Draft cannot run, revoked reviewer rejected, stale environment preview conflicts, demo no-account readiness does not grant arbitrary customer readiness.

### Task 4 — run manifest and submission

- ACTION: Implement runs service and browser controllers.
- IMPLEMENT: Authorize every nested ID, lock relevant plan/environment state while resolving, freeze manifest/build hash/config, create executions/jobs atomically, fingerprint idempotency, paginated app list and report detail. Compare environment revision to preview; exact replay returns existing run.
- MIRROR: controllers/setup.rs and services/apps.rs; uploads::build for parent-scoped build.
- IMPORTS: axum::{extract::{State,Path,Query},Json}; crate::services::auth::Session; services::runs; mobile_qa_contracts::browser.
- GOTCHA: Failed submission leaves no partial job; retries never accept a new payload under the old key.
- VALIDATE: Concurrent double submit, stale environment, foreign build/plan and restart persistence via actual HTTP.

### Task 5 — worker identity and atomic claims

- ACTION: Implement worker_auth extractor and scheduler.
- IMPLEMENT: Hashed worker/token state, scoped profile admission, idempotent claim journal handshake, transactional reservation with unique active resources, expiry reconciliation and generation fencing. Return idle with poll delay when no work; bound claim body and protocol.
- MIRROR: existing auth digest/session patterns, ApiFailure, short SeaORM transactions.
- IMPORTS: sea_orm::{ConnectionTrait,Statement,DatabaseBackend,TransactionTrait}; uuid; sha2::{Digest,Sha256}, subtle::ConstantTimeEq; opaque entropy via the existing loco_rs::hash::random_string facility from services/auth.rs; worker execution contracts.
- GOTCHA: A lost claim response is not license to claim a second job; don't release resource on lease expiry alone.
- VALIDATE: Real PostgreSQL simultaneous claims and same-account/different-device conflicts; wrong/revoked token, stale fence and response-loss replay.

### Task 6 — events, heartbeat, cancellation and recovery

- ACTION: Complete protocol lifecycle and reconciliation.
- IMPLEMENT: Durable dedup/event ordering, heartbeat deadline, cancellation races, cleanup acknowledgement, quarantine, explicit recovery and retained diagnostic retries. A reconcile task runs independently of web polling; claim/read paths handle expired state safely too.
- MIRROR: scheduler state transaction and qualification process-group/dirty-marker rules.
- IMPORTS: chrono, scheduler/runs/worker_auth services; generated LeaseStatus/EventReceipt types.
- GOTCHA: Database fencing cannot stop an old process tapping a device. The local watchdog and reserved/quarantined resource enforce physical exclusion.
- VALIDATE: Worker/API outages, late completion, reordered/duplicate events, cancel vs complete and reset failure tests with injected clocks.

### Task 7 — private artifact transport

- ACTION: Implement reserve/upload/download/seal service and routes.
- IMPLEMENT: Scoped immutable storage keys, streaming caps/hash, allowed MIME, safe binary responses, incomplete upload recovery and cleanup. Inspect current generation both before transfer and before sealing; orphan cleanup is not customer-visible success.
- MIRROR: ArtifactStore::scratch/publish/materialize and uploads.rs streaming patterns.
- IMPORTS: crate::config::Setup, crate::storage::ArtifactStore, tokio I/O, hashing dependency already used by uploads; run_artifacts contracts.
- GOTCHA: Filesystem/S3 and PostgreSQL do not share a transaction; manifest available status requires sealed metadata plus verified object.
- VALIDATE: Partial transfer, wrong hash, oversized stream, symlink/path attempts, cross-tenant read and repeated identical upload.

### Task 8 — Python polling transport and durable journal

- ACTION: Implement execution client/config/journal and explicit CLI.
- IMPLEMENT: Standard-library HTTP client initially (urllib.request with redirects disabled, strict origin, timeouts and bounded response reads); generated Pydantic model_validate_json at every JSON boundary. Lease token in headers only. Dedicated supervisor heartbeat/watchdog remains responsive while child runs; durable pending claim/events/artifacts/receipt journal under private state. Retry transport records by ID, never replay physical actions on startup.
- MIRROR: qualification process.py and atomic_json; existing CLI/fake separation.
- IMPORTS: urllib.request, threading, time, json, pathlib; mobile_qa_worker.generated.models; device_runtime.process.
- GOTCHA: HTTPS required outside loopback; redirects must not leak bearer credentials; persistence logs omit secret values. Long blocking model calls cannot own heartbeat loop.
- VALIDATE: Lost responses, malformed envelopes, auth expiry, backoff, child still-running on restart and dirty host refusal with local fake HTTP server.

### Task 9 — parameterized device lifecycle and action executor

- ACTION: Extract shared lifecycle primitives and add general manifest execution.
- IMPLEMENT: Parameterized install/launch/capture; registered demo fixture/reset adapter; sequential navigate/restart/checkpoint actions; pinned SDK child and compatibility guard; checkpoint observations; private diagnostics allowlist. Record package/build identity and verify reset before reuse. Harvest partial child usage in finally even for nonzero exit/timeout; unknown remains unknown.
- MIRROR: qualification/runner.py, device.py and sdk_adapter.py exact 4.0.0 seam.
- IMPORTS: device_runtime modules, qualification.sdk_compat, existing UsageRecorder (extract common helper if needed), generated execution models; SDK imported only inside SDK child after environment isolation.
- GOTCHA: Qualification request still rejects arbitrary apps; don't loosen that contract to smuggle production jobs through it. Remove demo tunnel/run-as assumptions from shared utility, retain in demo adapter.
- VALIDATE: Existing qualification regression plus action ordering, restart-retains-data, fake no-SDK import, child environment, byte/deadline limit and failure usage tests.

### Task 10 — evidence verifier and reports

- ACTION: Implement deterministic server evaluation and immutable result projection.
- IMPLEMENT: Parse bounded retained UI hierarchies, validate PNG/checkpoint metadata and prerequisite chain, compare approved values, persist check results, final attempt and coverage summary. Separate cleanup state from functional outcome; store missing evidence reasons, duration, usage and mixed attempts.
- MIRROR: qualification/verifier.py interpretation and product result precedence; uploads validation failure categories.
- IMPORTS: generated execution enums, run_artifacts service; quick_xml::{Reader, events::Event} at the existing locked 0.41.0 version through Cargo; no model dependency for v1 verdicts.
- GOTCHA: Successful SDK task alone never passes; a demonstrable failed required check survives later transport uncertainty. Missing target without screen readiness is not a verified absence.
- VALIDATE: Pass/fail/blocked/inconclusive/canceled/optional/mixed matrix; malformed/contradictory evidence and worker-forged pass assertion rejected.

### Task 11 — UI preview, run progress and report

- ACTION: Replace Tests/Runs placeholders with the scoped read-only/runtime flow.
- IMPLEMENT: Shared session transport helper, generated SDK + Zod wrappers, app plan preview/build readiness, stable idempotency key retained through retry, run list/detail/cancel and safe evidence viewer. Pin workspace/app in query keys and invalidate on switch/logout; poll according to visibility/terminal rules.
- MIRROR: api/setup.ts, runtime.ts, pages/AppDetail.tsx, routes.tsx and Mantine patterns.
- IMPORTS: generated sdk/types/zod, @tanstack/react-query, react-router, @mantine/core, existing use-workspace.
- GOTCHA: Do not create a new CSRF store; do not advertise full editor or hide required blockers behind green job lifecycle status.
- VALIDATE: DOM flow, malformed success/error payload, duplicate click, reload during run, logout/foreign workspace, cancel-pending, missing artifacts and terminal polling tests.

### Task 12 — deterministic end-to-end fake and regression tests

- ACTION: Finish meaningful fault tests and fake HTTP worker integration.
- IMPLEMENT: Fake worker uses the exact claim/event/artifact/complete protocol with synthetic marked evidence; no path can advertise fake as a real device. Add execution_smoke.py using isolated local PostgreSQL/API, imported reviewed synthetic case and app/build fixtures, fake worker and persisted report after API restart.
- MIRROR: scripts/app_setup_smoke.py, API test harness and existing Python fixture tests.
- IMPORTS: existing scripts/runtime helpers; standard-library subprocess/HTTP; generated contracts in worker fixtures.
- GOTCHA: Fake result selection is test configuration only, never customer run input; shared destructive DB fixtures must not race.
- VALIDATE: One smoke proves the actual transport and saved report; fault tests cover all lifecycle and tenant boundaries above.

### Task 13 — documentation and explicit developer commands

- ACTION: Complete docs/CI/commands before checks.
- IMPLEMENT: New explicit `just dev-execution-fake` and `just smoke-execution` recipes, worker operation runbook within phase 04/environment/development docs, CI path ownership, retention of open qualification/hosted gates. Update status only with completed evidence after validation.
- MIRROR: justfile explicit commands, scripts/test_ci_scope.py and current workflow selection.
- IMPORTS: no new runtime imports for documentation.
- GOTCHA: No device/model launch in setup/check/build/smoke defaults; no .env examples, new global settings or automatic external actions.
- VALIDATE: Review command dependencies, path-selection fixtures and architecture links.

### Task 14 — one final verification window and acceptance evidence

- ACTION: Generate, format, check and build after the complete edit batch.
- IMPLEMENT: Execute the command matrix below once, fix failures together and rerun invalidated checks. Record counts, command results, actual device/profile and any pending acceptance gates.
- MIRROR: docs/architect/development.md and project AGENTS.md.
- IMPORTS: none.
- GOTCHA: Real-device/model/cloud execution requires its already-authorized operational scope and qualified profile; this plan does not authorize paid launches or policy bypasses.
- VALIDATE: All deterministic checks and actual routes pass; real acceptance is separately recorded, not inferred from fake tests.

## Testing strategy

| Input / fault                                                                   | Expected result                                                   |
| ------------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| Empty actions/checks, duplicate IDs, invalid reference/type or maximum exceeded | 422, no approved/executable case                                  |
| Update reviewed content, then approve old hash                                  | 409, original approval remains bound to original content          |
| Same case variant through two suites / conflicting policy                       | One execution / explicit conflict                                 |
| Double submission or response loss                                              | Same run for same key/body; conflicting body 409                  |
| Two workers claim different jobs sharing device/account                         | At most one active reservation and execution                      |
| Worker loses network or heartbeat                                               | Local stop deadline, server fence, no resource reuse until reset  |
| Duplicate/out-of-order event and completion                                     | Exact dedup, conflict rejection, no result corruption             |
| API restart during device run                                                   | Persisted lease/run recovered without starting a second execution |
| Cancel during navigation or cleanup                                             | Pending stop then canceled/recovery; partial evidence preserved   |
| SDK says complete, hierarchy contradicts expected result                        | Failed check, never automatic pass                                |
| Missing ready marker, malformed hierarchy, missing required capture             | Inconclusive; no claim of proven absence                          |
| Backend/account unavailable before assertion                                    | Blocked                                                           |
| Required failure followed by retry pass                                         | Mixed / review required; old failed evidence remains              |
| Partial artifact/wrong hash/byte cap/foreign artifact ID                        | Rejected/unavailable evidence; unauthorized access denied         |
| Nonzero SDK exit with partial model usage                                       | Usage retained; unobserved calls/tokens not coerced to zero       |
| Unsupported visual/backend check                                                | Preview/submission blocker; never silently skipped                |
| Fake worker run                                                                 | Explicit simulated label on device and report                     |

Cover permission denied, missing/expired credentials, empty input, boundary sizes, invalid types, concurrent access and network failure. Use real PostgreSQL for reservation/approval races; isolated subprocesses for kill/reset behavior; DOM tests for the frontend; no broad new test framework.

## Validation commands

Commands refer to the integrated implementation checkout, after all code/tests/config have been authored. Setup prerequisites (pnpm/uv locked dependencies, JDK/Android build tools and existing PostgreSQL) follow development.md; fake tests/checks do not need Doppler. Do not invoke setup implicitly during checks.

```bash
just types
cargo fmt --all
just check-contracts
just check-api
just check-web
just check-worker
python3 scripts/test_ci_scope.py
just build
just smoke
just smoke-execution
```

`just check-api` uses scripts/runtime.py's PostgreSQL test setup and Rust format/Clippy/workspace tests; migrations are exercised against the real test database. `just check-web` includes TypeScript/lint/tests; `just check-worker` includes Ruff/Pyright/pytest. `just smoke-execution` is a NEW recipe authored in Task 13 and must exist before use; it launches no device/model and proves persistence after API restart. Avoid immediately repeating `just check` because those component checks already covered it.

Before any later commit, run the repository's scoped dependency audits (pnpm frontend and the installed Python/Rust audit procedures documented in dependencies.md), preserving the already documented RSA advisory as an unresolved finding rather than calling it clean. If a lock changes, inspect the dependency delta and audit it. This plan creation itself requires only link/consistency/whitespace review, not application checks.

Manual/real acceptance, only with an allowed browser and explicit operational scope:

- Upload the controlled good/broken APK through normal app setup and approve the imported persistence test and default plan.
- Select qualified device/image/adapter; preview required checks and missing prerequisites.
- Run good, seeded defect and unavailable-backend scenarios through the same browser/API/worker path; confirm passed, failed and blocked reports with decisive evidence.
- Restart API during a run; reload browser; confirm durable progress/result.
- Interrupt worker and cancel during navigation; prove old descendants stopped and device/account cannot be reused until clean reset or explicit quarantine recovery.
- Confirm screenshot access is private, expected/observed checks are legible, keyboard navigation and Retry work, and fake reports cannot be confused with device evidence.
- Record full reliability campaign/cloud qualification separately if still pending. Do not bypass existing browser admin policy through another access method.

## Acceptance criteria and completion checklist

- [x] Approved case/suite/plan versions are persisted; content/hash/approval authority is enforced.
- [ ] Upload → preview → queued job → real worker → evidence-backed saved report works for a qualified adapter.
- [ ] Every action/check in the approved case is explained and linked to evidence or a reason it was not completed.
- [x] Browser cannot replace approved expectations or launch drafts.
- [x] Tenant-scoped routes, idempotency, competing claims, cancellation and stale fencing pass real route/database tests.
- [ ] Worker process is stopped/reset or quarantined before any resource reuse.
- [ ] Artifact and verification failures cannot yield an unsupported pass; retry history is immutable.
- [x] API/browser reload/restart preserves report and execution identity.
- [ ] Generated contracts have zero drift; relevant format/lint/type/test/build/smoke checks pass.
- [ ] Logs/errors match existing safe conventions; secrets never enter case/report files.
- [x] Owning architecture docs distinguish implemented behavior from open real/hosted acceptance.
- [x] No full editor/generation/deployment or unrelated configuration scope was added.

## Risks and confidence

| Risk                                                            | Likelihood / impact                    | Mitigation                                                                                                                               |
| --------------------------------------------------------------- | -------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| Device stream is not yet merged or fully qualified              | Present / blocks real acceptance       | Integrated baseline and explicit acceptance gate; fake work remains useful                                                               |
| Demo-only device/reset assumptions leak into customer execution | High / false readiness or unsafe reset | Registered adapters and parameterized utility; compatibility rejected before dispatch                                                    |
| Stale process survives an expired DB lease                      | Medium / simultaneous physical effects | Local watchdog, process-tree termination, held reservations and quarantine                                                               |
| Model navigation lacks decisive evidence                        | High / unsupported verdict             | Required checkpoint prerequisites, deterministic checks and inconclusive outcome                                                         |
| Approval authority inferred from operator privileges            | Medium / wrong business expectations   | Explicit app reviewer grants and exact-hash approval audit                                                                               |
| SDK docs drift or compatibility subclass breaks                 | Medium / navigation failure            | Pinned 4.0.0 seam, guarded compat and offline real tool test                                                                             |
| Evidence contains credentials/provider traces                   | Medium / private data exposure         | Synthetic initial case, subprocess isolation, allowlisted normalized evidence                                                            |
| Storage publication succeeds but DB seal fails                  | Medium / orphan objects                | Pending records, idempotent publication, fenced seal and scoped cleanup                                                                  |
| XL implementation exceeds one reliable pass                     | High / incomplete integration          | Four ordered checkpoints: definitions; durable fake run; real adapter; acceptance, while respecting the agreed single final check window |

Confidence: 7/10 for implementation from this packet; lower for real/cloud acceptance until qualification evidence exists. This is an XL subsystem, not a small Run-button patch. No implementation or external changes were performed by creating this plan.

Next implementation entry: `/prp-implement .claude/PRPs/plans/04-execution-and-reports.plan.md` on the integrated source baseline.

## Implementation reconciliation — 2026-09-12

Local source and deterministic HTTP acceptance are implemented on
`codex/04-execution-and-reports`. See
[the implementation report](../reports/04-execution-and-reports-report.md) for task
status, tests and deviations. This plan remains active because real browser/device
acceptance and hosted acceptance have not been executed; the foundation smoke is
also blocked by pre-existing listeners on 5150/5173. Historical planning statements
above describe the pre-implementation assessment.
