# Plan: 05 — Typed test authoring, review and UI-to-run workflow

## Execution status

Local implementation and deterministic/HTTP checks passed on 2026-09-12. See the
[implementation report](../reports/05-test-library-and-plans-report.md). Rendered
browser and UI-triggered real-device acceptance remain pending; retain this plan
until those gates are verified.

## Summary

Build the phase 05 case/suite/plan editor and review API on the working phase 04 execution system. Customers can save drafts, review exact versions, see approved coverage and run it against another uploaded build; operators retain executability and device configuration authority. Rust owns business rules and all transport shapes; browser SDK/Zod and Python/Pydantic are generated from Rust, with real route tests and a UI-to-HTTP-to-worker acceptance path.

## User story

As an authorized app user, I want to author understandable actions and expected checks, get the exact version approved and run the selected coverage on an uploaded build, so that I can repeat a regression check without editing JSON or asking an operator to recreate definitions.

## Problem → solution

Operator-only immutable imports plus read-only Tests → typed editable drafts, immutable review versions, pinned suite/plan membership and an explicit default release plan, connected to the existing Run/report interface.

## Metadata and baseline

- Complexity: **XL**, approximately 50 owned/new files plus generated files; 12 ordered tasks.
- Source PRD: `docs/architect/product.md`, especially sections 5, 8, 10 and 22.
- Phase spec: `docs/architect/implementation/05-test-library-and-plans.md` already exists. This document supplies the missing implementation packet.
- Planning branch: `codex/05-test-library-and-plans`.
- Verified integrated source: `b4970c7`, merge of PR #5 into `feat/02-cloud-phone-and-feasibility`; its source tree matches phase 04 commit `96c81cc`.
- At planning time `origin/main` is `e88e401` and **does not contain the phase 02/04 integrated tree**. Start implementation from the integrated tree, or a subsequently verified main containing it. Do not implement on the older main by accident.
- Read `AGENTS.md`, `docs/architect/README.md` and owning docs. `docs/CODEX-NAVIGATION-GUIDE.md` is referenced by the supplement but absent in this baseline; the architectural index and source inventory below provide navigation.
- Planning only: no application implementation, generated output, provider calls or device launches in this task.

## Existing implementation versus remaining work

| Area                    | Existing, verified source                                                                        | This phase adds                                                                      |
| ----------------------- | ------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------ |
| Definitions             | Immutable `execution_definitions`, tagged `TestDefinition`, exact-hash approvals, operator tasks | Mutable draft workspace, catalog identity, review decisions, archive, browser API    |
| Execution               | Persisted manifests/attempts, HTTP leases, long polling, independent verification                | Gate new submissions against library lifecycle; preserve the protocol                |
| Tests UI                | Build-dependent read-only approved plan                                                          | Build-independent Cases / Suites / Release plan tabs and version detail              |
| Run UI                  | Preview, idempotent Run button, polling report/cancel/evidence                                   | Explicit selected/default plan, library links and editor-to-run navigation           |
| Device                  | One real good demo API → Minitap → emulator → evidence run passed                                | Exercise the same path from the finished UI when browser/device operation is allowed |
| Remaining qualification | Live good-path evidence only                                                                     | Keep live faults, cancellation/reliability and hosted acceptance separate            |

## UX design

Before:

```text
Tests → choose app → choose build → operator-managed plan (read-only)
Operator terminal → import JSON → grant reviewers → approve hashes
App build → Run → existing report
```

After:

```text
Tests → choose app → Cases | Suites | Release plan
Case → edit draft → Save → Request review → exact version + expectations
                                       → business review + executability review
Suites → choose approved versions → review pinned membership
Release plan → choose approved cases/suites → review coverage → set default
App build / Release plan → preview selected build → Run → progress + evidence
Approved case → New draft version → edit (previous plans/reports stay pinned)
```

| Touchpoint      | Interaction and states                                                                                                                                                      |
| --------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Tests landing   | App selection independent of APK upload; tab/count/filter/empty state; no build required to author                                                                          |
| Case editor     | Title, requirement/criterion reference text, preconditions, ordered actions and expected checks; explicit Save, dirty indicator, validation issues and review status        |
| Action editor   | Human labels Navigate / Restart app / Capture checkpoint; stable IDs generated when adding; accessible up/down controls, no drag dependency                                 |
| Expected checks | Plain-language description/requiredness first; operator-facing evidence selector fields in an advanced section; unsupported methods show why execution is unavailable       |
| Review          | Exact version/hash, expectations and technical readiness, named business/executability approvals; changes requested/rejected reason; disabled actions explain permission    |
| Suites          | Ordered pinned case versions; display newer approved version as available, never auto-replace selected membership                                                           |
| Release plan    | One visible default selection; approved cases/suites/profile, requiredness and exclusions; resolved unique coverage and limits; unmeasured estimates say “Not yet measured” |
| Run preview     | Explicit build and reviewed plan version, missing prerequisites, fake/real label; existing Run submission and report navigation                                             |
| Concurrent edit | 409 preserves local form values; show reload-current action and reviewable differences; no automatic overwrite/merge or blind retry                                         |
| Archive         | Explain impact before user action; remove from new selection; retain version history and historical reports                                                                 |
| Navigation      | Workspace/app/version pinned in URL/query keys; warn before discarding unsaved changes; never keep draft bodies in localStorage/sessionStorage                              |

## Scope and architectural decisions

### 1. Keep frozen versions separate from editable work

Do **not** update JSON/hash in `execution_definitions`. That table remains the immutable execution record. Add a catalog and separate draft storage in migration `m20260912_000005_test_library.rs` (if the migration identifier has already been used when implementation starts, choose the next unused timestamp/name and record it).

| Table                        | Columns and invariants                                                                                                                                                                                    |
| ---------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_library_entries`       | `id` UUID, `app_id`, `kind`, `logical_key`, `next_version` positive integer, `revision` positive integer, `archived_at`, actor/timestamps; unique `(app_id,kind,logical_key)`                             |
| `test_library_drafts`        | `entry_id` PK/FK (one current editable draft per entry), `version` allocated from entry, optional `source_version_id`, `payload` JSONB, editor/timestamps; size and typed decode checked before storage   |
| `test_library_versions`      | `definition_id` PK/FK to execution_definitions, `entry_id`, `review_state` (`in_review`, `needs_input`, `rejected`, `approved`), timestamps; unique entry/version via existing frozen definition identity |
| `test_library_review_events` | UUID, entry/version, actor, purpose, decision, exact content hash, reason, timestamp; append-only audit                                                                                                   |
| `test_library_defaults`      | `app_id` PK, approved `plan_version_id`, positive `revision`, actor/timestamp; one explicit default plan version per app                                                                                  |
| `test_library_mutations`     | app/actor/mutation UUID, operation/fingerprint, typed response JSON; unique `(app_id,actor_id,mutation_id)`; atomic receipt and mutation; never store secrets                                             |

`revision` on the catalog entry is the optimistic concurrency token for draft saves, submission, review and archive. Any change increments it; version-detail replies include the current entry revision. Default-plan changes use their own revision. Cap counters to signed 32-bit storage range and expose bounded JSON integers; do not introduce browser BigInt revisions.

All mutation requests carry a client UUID `mutation_id`. Retries reuse that ID and identical payload. Authorize on every replay, check the persisted fingerprint before optimistic revision checks, return the original response for exact replay, 409 for different payload. New edits use a new ID. Store receipt in the same transaction, never after commit; old replay does not overwrite new draft content. Use a tagged stored response enum owned by Rust, not untyped JSON consumers.

Use the current app-row lock first, then entry/default row, consistently across CLI imports, browser mutation and run submission. Reference resolution/approval and archive happen under this same app lock so version/policy changes cannot race a submitted manifest. Transactions contain no network/model/storage calls.

### 2. Lifecycle and publication

- New entry: server allocates entry/version identity, creates a full **shape-valid** editable `LibraryDraftDefinition` template. It may have empty required content; it is not executable.
- Save draft: enforce JSON types, bounded total size (1 MiB), field/list limits and immutable kind/key/version. Allow incomplete content and return typed validation issues. Draft saves must not call the full execution-readiness validator as a blanket rejection.
- Request review: check expected revision, validate all semantic content and references, insert the immutable `execution_definitions` version and `test_library_versions(in_review)`, remove editable draft, append audit, increment entry revision. No automatic approval.
- Review: decisions are accepted only while the frozen candidate is in_review; approved/needs_input/rejected candidates require a new draft for changes. A new mutation repeating an already-recorded purpose decision returns 409; exact mutation replay remains idempotent. Business and executability decisions bind the frozen content hash plus expected entry revision. Reuse the existing reviewer grants. Both approvals are needed for `approved`; record the final transition atomically with the second approval.
- Needs input / reject: require a bounded reason, move the frozen candidate to `needs_input`/`rejected`. It is not runnable and cannot acquire further approvals. Retain prior approval/audit rows; they are historical evidence, not active admission. Editing means explicitly creating a new draft/version from the candidate, with no inherited approvals.
- New draft from an approved version: copy content into the next server-allocated version; leave prior approved version, default-plan selection, suite memberships and run manifests unchanged. A new draft does not revoke an old approval.
- One editable draft per entry. A second attempt to create another draft returns a typed conflict with the current draft identity.
- Archive operates on catalog entry, is reversible via explicit unarchive with revision check, and changes no frozen payload. It blocks new selections, review and **new runs referencing that entry**, including through a suite. Existing queued/active manifests continue, and saved reports/history remain readable.
- Creating a later case version never silently updates a suite or default plan. Refreshing membership creates a new suite/plan draft for review.
- Default plan initially backfills the latest fully approved phase 04 plan using the existing `created_at DESC,id DESC` order. Afterwards only an explicit set-default operation changes it. New plan approval alone does not move the default. An archived/incompatible default shows blockers, not a silently substituted plan.

### 3. Backfill and compatibility

Backfill one catalog entry per existing `(app,kind,key)` and link every existing definition. Preserve definition IDs, payloads, hashes, version numbers and approval records byte-for-byte. Mark linked versions approved only when both existing exact-hash approvals match, otherwise in_review. Initialize `next_version=max(existing version)+1` and default selection deterministically. Old runs continue to read only their stored manifests/results.

Refactor `test_definitions::import/approve/resolve` to share library registration/readiness routines. An operator import is still an immutable published candidate, not a browser draft. It must allocate/check catalog identity and not collide with an allocated draft version. The old CLI must not become a lifecycle/archival bypass. Keep `grant-reviewer`, worker/profile registration, lease tasks and protocol intact. Add revision-aware CLI review parameters as needed; update smoke scripts accordingly rather than introducing hidden privileged approval bypasses.

### 4. Authority and available execution capabilities

| Action                                                   | Server policy                                                                                             |
| -------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| Read, create/edit draft, request review                  | Active approved session with existing `apps::authorized` access; drafts are collaborative within that app |
| Business review                                          | Same app access plus `execution_reviewer_grants.business`                                                 |
| Executability review                                     | Same app access plus `execution_reviewer_grants.executability`; preserve existing grant semantics         |
| Archive/unarchive, set default                           | App operator; surface this in capabilities and explain unavailable controls                               |
| Grant review permission, register/qualify worker/profile | Existing explicit operator maintenance; no browser self-grant/qualification toggle                        |

One person can have both review grants. Neither organization membership, operator label alone nor creating a draft automatically grants review authority. Existing operators can explicitly grant themselves through normal maintenance as before. Capability replies guide UI but every write recomputes authorization; foreign parent/child IDs return existing opaque 404 behavior.

The initial executable adapter is still `demo_persistence_v1` / `ai.mobileqa.demo`, with `default` data variant and one pinned profile per plan. Cases for other packages may be stored and reviewed for business meaning, but cannot gain executability approval or queue work until a real adapter supports them. Use Rust-owned template/options replies to describe supported action/check kinds, resource selectors, limits and sanitized profile choices. Do not imply arbitrary customer login/reset scripts now work.

Current shared validator admits provenance `operator_authored` only. Add `user_authored` for browser-created cases, set it on the server and do not let browser writes forge `operator_authored`. Preserve existing strings in historical versions; phase 06 generated provenance remains future work. Requirement text can contain criterion/source references in this slice; no separate source-upload catalog or invented source UUIDs. Setup/reset capability remains the registered adapter's responsibility, displayed read-only.

## End-to-end type-safety contract

```text
Rust library DTOs and semantic validators
    → Utoipa browser OpenAPI
    → local contracts/browser.openapi.json
    → Hey API SDK + TypeScript + Zod
    → typed Mantine form state → generated request validation
    → typed Axum handler → Rust service → PostgreSQL
    → immutable existing RunManifest
    → Schemars worker schema → generated Pydantic → emulator/Minitap
    → typed worker result → Rust evidence evaluation → generated report Zod → UI
```

1. Create `crates/contracts/src/test_library.rs` and `test_library_api.rs`; pure Rust only, no Loco/DB imports. Reuse `TestDefinition`, `TestAction`, `ExpectedCheck`, `CaseSelection`, `ExecutionBudget`, `ApprovalPurpose` and existing manifest types. Do not create frontend/Python equivalents.
2. New named DTOs: `LibraryEntryResponse`, `LibraryListResponse`, `LibraryDraftResponse`, `LibraryVersionResponse`, `LibraryVersionListResponse`, `LibraryOptionsResponse`, `LibraryProfileChoice`, `LibraryCapabilities`, `LibraryCoveragePreview`, `DefaultPlanResponse`; mutation requests listed below. All tagged unions/enums have Serde/Utoipa declarations; unknown request fields are denied.
3. Draft responses contain generated `LibraryDraftDefinition`, root revision, base/source ID, typed issues and capabilities. Draft/version replies also carry an optional generated `LibraryCoveragePreview` for suites/plans: ordered unique pinned selections, required/supporting counts, exclusions and issues. Compute it in Rust from approved dependencies without a build; the existing execution-plan endpoint adds build/environment readiness later. Forms use that generated union and discriminant narrowing. Local UI-only state (open panels, dirty state, NumberInput's empty string) is allowed, but submit adapters must parse generated request schemas. No `as GeneratedRequest`, `as unknown as`, `any`, or handwritten `z.object` transport clones.
4. Define Rust `LibraryIssue { code: LibraryIssueCode, field: String, item_id: Option<String>, message: String }`; paths and IDs locate controls, not scripts. Reuse semantic validation helpers to produce structured issues; browser does not duplicate approval/reference/adapter rules. At most 100 bounded issues per reply.
5. Define tagged `LibraryErrorDetails` (validation issues, stale revision/current identity, idempotency conflict). Keep the common `ApiError` envelope; extend `ApiFailure` with an optional details setter defaulting to None. Serialize only typed library details into it. `middleware/mod.rs` already parses/reissues ApiError and must preserve validated details/request ID.
6. Common browser interceptor still validates generated `zApiError`. The library wrapper then validates `body.details` with generated `zLibraryErrorDetails`; malformed/unknown details fall back to generic `ApiClientError`, never an unchecked field map. The generic JSON Value in existing ApiError is an untrusted boundary, not a license to cast.
7. Query/path DTOs live in contracts too. Register every method/path/request/status/error in Utoipa and actual handlers; include CSRF and idempotency/revision requirements in the API declarations. Explicitly bind success status keys using `satisfies keyof OperationResponses` where wrappers support multiple statuses.
8. Only `just types` updates generated OpenAPI, TypeScript/Zod and Pydantic. If shared execution validation changes provenance, ensure current manifests and local worker fixtures still round-trip. No worker lease protocol version bump is needed for browser authoring; do not add drafts or review grants to WorkerContracts.
9. Add compile-negative tests to `contracts.compile.ts`, real handler-versus-OpenAPI inventory tests, malformed input/output transport tests, and JSON Schema/Pydantic round-trip of a browser-published definition frozen into an execution manifest.
10. PostgreSQL JSON columns decode into Rust DTOs using the existing typed helpers. Constraints, typed decode, authorization and semantic checks complement each other; generated TypeScript alone does not guarantee DB data or network responses are valid.

## Browser API inventory to implement

Prefix `L = /api/apps/{app_id}/test-library`. All routes use `Session`; mutations inherit existing same-origin/CSRF protection. UUID cursors are parent/filter scoped, with fixed page size 50. List filtering uses generated kind/review/archive enums, not arbitrary SQL fragments. `revision` and `mutation_id` are required mutation fields except initial creation, where the client-chosen entry UUID also makes identity explicit.

| Method/path                                    | Operation ID             | Input                                                                                                                                  | Success                                                                            |
| ---------------------------------------------- | ------------------------ | -------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| GET L                                          | listTestLibrary          | `LibraryListQuery { kind?, status?, archived?, cursor? }`                                                                              | 200 LibraryListResponse                                                            |
| GET L/options                                  | getTestLibraryOptions    | app path                                                                                                                               | 200 LibraryOptionsResponse (capabilities, templates, sanitized profile choices)    |
| POST L                                         | createTestLibraryEntry   | `CreateLibraryEntryRequest { mutation_id, entry_id, kind, key, template_profile_id? }`                                                 | 201 LibraryDraftResponse; 200 exact replay                                         |
| GET L/{entry_id}                               | getTestLibraryEntry      | parent/entry path                                                                                                                      | 200 LibraryEntryResponse (summary/current draft pointer/current approved versions) |
| GET L/{entry_id}/draft                         | getTestLibraryDraft      | parent/entry path                                                                                                                      | 200 LibraryDraftResponse; 404 no draft                                             |
| POST L/{entry_id}/draft                        | forkTestLibraryDraft     | `ForkLibraryDraftRequest { mutation_id, expected_revision, source_version_id }`                                                        | 201 LibraryDraftResponse; 200 replay                                               |
| PUT L/{entry_id}/draft                         | saveTestLibraryDraft     | `SaveLibraryDraftRequest { mutation_id, expected_revision, definition: LibraryDraftDefinition }`                                       | 200 LibraryDraftResponse (includes semantic issues)                                |
| POST L/{entry_id}/submit                       | submitTestLibraryDraft   | `SubmitLibraryDraftRequest { mutation_id, expected_revision }`                                                                         | 201 LibraryVersionResponse; 200 replay                                             |
| GET L/{entry_id}/versions                      | listTestLibraryVersions  | cursor                                                                                                                                 | 200 LibraryVersionListResponse                                                     |
| GET L/{entry_id}/versions/{version_id}         | getTestLibraryVersion    | all parent IDs                                                                                                                         | 200 LibraryVersionResponse with content/hash/audit/capabilities                    |
| POST L/{entry_id}/versions/{version_id}/review | reviewTestLibraryVersion | `ReviewLibraryVersionRequest { mutation_id, expected_revision, content_hash, purpose, decision: approve/needs_input/reject, reason? }` | 200 LibraryVersionResponse                                                         |
| POST L/{entry_id}/archive                      | archiveTestLibraryEntry  | `ArchiveLibraryEntryRequest { mutation_id, expected_revision, archived: bool }`                                                        | 200 LibraryEntryResponse                                                           |
| GET /api/apps/{app_id}/default-test-plan       | getDefaultTestPlan       | app path                                                                                                                               | 200 DefaultPlanResponse (nullable plan version; revision 0 before first set)       |
| PUT /api/apps/{app_id}/default-test-plan       | setDefaultTestPlan       | `SetDefaultTestPlanRequest { mutation_id, expected_revision, plan_version_id }`                                                        | 200 DefaultPlanResponse                                                            |
| GET existing /api/apps/{app_id}/execution-plan | getExecutionPlan         | shared `ExecutionPlanQuery { build_id, plan_version_id? }`                                                                             | existing 200 PlanPreviewResponse; explicit selected version or configured default  |

Errors retain 400/401/403/404/409/413/422/429/500/503 plus default ApiError as applicable. Missing reviewable content is 422 with issues; valid draft saves with incomplete content return 200 plus issues. Stale revisions/hashes and conflicting duplicate membership return 409. No untyped 204 mutations. Existing createRun remains the sole browser execution submission; no alternate privileged “run draft” route.

New entry templates are constructed in Rust from existing fixture/domain types, with server key/version and app package. Define `LibraryDraftDefinition` as a Rust-tagged union of CaseDefinition, SuiteDefinition and **PlanDraftContent**. PlanDraftContent mirrors PlanDefinition's members and budget using the same Rust types, with `profile_id: Option<Uuid>` so an incomplete draft needs no fabricated/nil profile ID. Save requests and draft responses use LibraryDraftDefinition; submission explicitly converts it into existing TestDefinition and requires a selected profile. Published version responses remain TestDefinition. This is the only intentional draft-versus-executable shape difference; all consumer types are generated.

## Resolution and run integration

Keep current `test_definitions::resolve` ordered direct-cases-then-suites behavior and conflict policy. It deduplicates by logical case key and rejects conflicting version/requiredness/data variant; since this phase admits only `default` variant and one profile, this is the supported v1 identity. Do not silently broaden to multiple versions of one case. Tests should explicitly cover direct+suite and two-suite duplicates.

Split checks that are currently coupled: draft structural bounds; semantic validity; review authority; current lifecycle eligibility; build/profile readiness. Read-only plan preview must return meaningful blockers when a pinned case/suite is archived or needs input. Do not reject ordinary draft loads because execution is not ready. Current `runs::preview` chooses latest approved plan; replace that selection with the explicit default table and support the selected-version query above. Run creation checks that selected plan and every resolved dependency are approved and unarchived under the app lock before freezing. Report reads must not re-resolve current library content.

Extend `RunPreview` with optional planVersionId and library links; include it in `planQuery` keys/SDK input. Reuse its current user/workspace/app/build/plan/environment idempotency key. Show a refresh/conflict state on stale environment. Library edits/reviews invalidate the relevant library, default-plan and plan-preview queries, not historical run data. Building a new default draft never changes a queued manifest.

## Mandatory reading and discovery inventory

Line references refer to integrated baseline `b4970c7`; these are search anchors, not ranges to mechanically patch after edits.

| Priority/category      | File:lines                                                                                                         | Pattern/evidence                                                                    |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------- |
| P0 domain shapes       | crates/contracts/src/execution.rs:98–193                                                                           | Existing budget/action/check/case/suite/plan/approval types                         |
| P0 semantic validation | crates/contracts/src/execution.rs:406–559                                                                          | Bounds, provenance restriction, ordered prerequisite references, supported variants |
| P0 persistence         | apps/api/migration/src/m20260912_000004_execution.rs:1–35                                                          | Immutable versions/approval schema; never replace this applied migration            |
| P0 services            | apps/api/src/services/test_definitions.rs:1–367                                                                    | Import, grant, approval, profiles and pinned dedup resolution                       |
| P0 run boundary        | apps/api/src/services/runs.rs:1–219                                                                                | Latest-plan selection, readiness, immutable manifest transaction/idempotency        |
| P0 transport           | crates/contracts/src/execution_api.rs:1–110; browser.rs:76–81                                                      | Utoipa inventory and common untrusted error details                                 |
| P0 authorization       | apps/api/src/services/apps.rs:18–44; auth.rs:23–55                                                                 | App assignment/operator scope, Session CSRF                                         |
| P0 errors              | apps/api/src/errors.rs:1–66; middleware/mod.rs:1–80                                                                | Safe ApiError envelope, request-ID rewrite, no raw DB error disclosure              |
| P0 browser boundaries  | apps/web/src/api/session-transport.ts:1–22; runtime.ts:1–63                                                        | Generated status/schema guard; validated error interceptor                          |
| P1 UI state            | apps/web/src/pages/Tests.tsx:1–63; routes.tsx:1–45                                                                 | Current read-only Tests and nested authenticated app shell                          |
| P1 forms               | apps/web/src/components/app/app-form.tsx:1–110                                                                     | Mantine useForm/useMutation, mounted guard and workspace navigation                 |
| P1 run UX              | apps/web/src/components/app/run-preview.tsx:20–137; api/runs.ts:1–53                                               | Generated manifest input, stable idempotency, cache keys                            |
| P1 scope/cache         | apps/web/src/hooks/use-workspace.ts:1–35; components/app/session.tsx:1–80                                          | Workspace routing resets detail IDs; session loss clears query cache                |
| P1 tests               | apps/api/tests/execution.rs; tests/support/mod.rs:20–192                                                           | Real PostgreSQL/API fixtures, independent grants, DATABASE_BOOT guard               |
| P1 UI tests            | apps/web/src/pages/RunDetail.test.tsx; pages/Home.test.tsx; api/runs.test.ts                                       | DOM routes plus actual generated-client transport checks                            |
| P1 compile checks      | apps/web/src/test/contracts.compile.ts:1–30; test/setup.ts                                                         | Negative TypeScript fixtures, DOM-only setup limitations                            |
| P1 generation          | scripts/contracts.py:24–120; crates/contracts/src/worker/execution.rs                                              | Staged generation and zero-drift ownership; worker envelope roots                   |
| P1 HTTP smoke          | scripts/execution_smoke.py:28–88 and main; scripts/app_setup_smoke.py                                              | Isolated test API, actual HTTP, generated Python worker, no model calls             |
| P2 dependencies/config | apps/web/package.json; apps/api/Cargo.toml; apps/mobile-worker/pyproject.toml; justfile; .github/workflows/ci.yaml | Existing locked Mantine/Query/Zod/Rust/Pydantic, formatter and scoped CI            |
| P2 authority           | docs/architect/product.md:section 5; implementation/05-test-library-and-plans.md                                   | Lifecycle, review split and explicit version pinning                                |

Five traces to preserve:

1. Browser click → generated SDK → Session/typed handler → app authorization → service/transaction → typed response → generated Zod → cache.
2. Draft → immutable candidate → two exact-hash approvals → pinned suite/plan → run manifest → generated worker job → report.
3. Save/review/archive/default mutation → revision and audit/receipt together → targeted cache invalidation.
4. API failure → ApiFailure → middleware request ID → zApiError → optional zLibraryErrorDetails → accessible user feedback.
5. Logout/workspace switch → query cancellation/cache clearing or scoped keys → no previous app draft/report rendered under the new workspace.

## Patterns to mirror (actual source excerpts)

Naming and transport — `apps/web/src/api/runs.ts:7–16`:

```ts
export const planQuery = (workspace: string, appId: string, buildId: string) =>
  queryOptions({
    queryKey: ['execution-plan', workspace, appId, buildId],
    enabled: !!appId && !!buildId,
    queryFn: () =>
      checked(
        sdk.getExecutionPlan({ ...options, path: { app_id: appId }, query: { build_id: buildId } }),
        z.zPlanPreviewResponse,
      ),
  })
```

Runtime validation — `apps/web/src/api/session-transport.ts:18–21`:

```ts
const result = await promise
if (!statuses.includes(result.response.status))
  throw new Error(`Unexpected API response status (${result.response.status})`)
return schema.parse(result.data)
```

Safe error — `apps/api/src/errors.rs:30–39`:

```rust
pub fn invalid(message: impl Into<String>) -> Self {
    Self::new(422, "invalid_input", message)
}
pub fn internal() -> Self {
    Self::new(
        500,
        "internal_error",
        "The operation could not be completed. Try again.",
    )
}
```

Repository/transaction — `apps/api/src/services/runs.rs:152–161` (matching block):

```rust
let tx = ctx.db.begin().await?;
one(
    &tx,
    "SELECT id FROM apps WHERE id=$1 FOR UPDATE",
    vec![app.into()],
)
.await?;
```

Service/approval — `apps/api/src/services/test_definitions.rs:218–221`:

```rust
let d = get(&tx, app, id).await?;
if d.content_hash != digest {
    return Err(conflict("Reviewed content hash changed"));
}
```

Logging — `apps/api/src/services/runs.rs:218`:

```rust
tracing::info!(run_id=%id,app_id=%app,phase="queued","Approved execution queued");
```

New library logs use actor/app/entry/version/revision/decision identifiers; never log draft bodies, requirements, artifact bytes, credentials or full review text.

Test structure — `apps/api/tests/execution.rs:112–118`:

```rust
#[tokio::test]
async fn route_manifest_idempotency_and_worker_fencing() {
    let _guard = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, ctx| async move {
        let owner = login(&server, &ctx).await;
```

## External documentation / research

No external research needed — this feature uses established internal patterns and installed pinned dependencies. No package upgrades or new runtime frameworks are proposed. Use the actual repository examples above for Mantine, Query, generated Zod, SeaORM transactions and Axum sessions. If implementation introduces an unfamiliar library API, verify official documentation before using it; this plan does not authorize changing the stack to solve editor state.

## Files to change

| Files                                                                                                                                                   | Action                | Purpose                                                                         |
| ------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------- | ------------------------------------------------------------------------------- |
| crates/contracts/src/test_library.rs; test_library_api.rs                                                                                               | CREATE                | Browser library DTOs, typed issues/lifecycle/mutations, OpenAPI inventory       |
| crates/contracts/src/lib.rs; browser.rs; execution.rs; execution_api.rs                                                                                 | UPDATE                | Merge library schemas/routes, shared preview query, user provenance validation  |
| crates/contracts/tests/test_library.rs                                                                                                                  | CREATE                | Structural versus semantic drafts, conversion, bounds and tagged-union fixtures |
| apps/api/migration/src/m20260912_000005_test_library.rs; migration/src/lib.rs                                                                           | CREATE/UPDATE         | Add catalog/drafts/review/default/receipt storage and backfill                  |
| apps/api/src/services/test_library.rs; test_library_review.rs                                                                                           | CREATE                | Draft/catalog lifecycle, shared approval/admission and concurrency              |
| apps/api/src/controllers/test_library.rs                                                                                                                | CREATE                | Typed authenticated browser endpoints                                           |
| apps/api/src/services/test_definitions.rs; runs.rs; tasks/execution.rs                                                                                  | UPDATE                | CLI compatibility, version allocation, archive/default resolution               |
| apps/api/src/controllers/runs.rs; controllers/mod.rs; services/mod.rs; app.rs                                                                           | UPDATE                | Shared query DTO and registration; preserve scaffold markers                    |
| apps/api/src/errors.rs; middleware/mod.rs                                                                                                               | UPDATE only as needed | Typed feature details serialization; preserve existing envelope behavior        |
| apps/api/tests/test_library.rs; tests/execution.rs; tests/health.rs                                                                                     | CREATE/UPDATE         | Real route/concurrency/history/route-inventory tests                            |
| apps/web/src/api/test-library.ts; test-library.test.ts                                                                                                  | CREATE                | Generated SDK/Zod wrappers, scoped queries and feature error parsing            |
| apps/web/src/pages/Tests.tsx; routes.tsx                                                                                                                | UPDATE                | App-scoped library tabs and detail/version routes                               |
| apps/web/src/pages/TestEditor.tsx; TestVersion.tsx                                                                                                      | CREATE                | Editable workspace and frozen version review                                    |
| apps/web/src/components/tests/case-editor.tsx; suite-editor.tsx; plan-editor.tsx; review-panel.tsx                                                      | CREATE                | Typed composable fields and review controls                                     |
| apps/web/src/components/tests/library-list.tsx; version-history.tsx                                                                                     | CREATE                | Status/filter/version/archive selection                                         |
| apps/web/src/api/runs.ts; components/app/run-preview.tsx; pages/AppDetail.tsx                                                                           | UPDATE                | Explicit selected/default plan and library links                                |
| apps/web/src/pages/TestEditor.test.tsx; TestVersion.test.tsx; Tests.test.tsx                                                                            | CREATE                | DOM edit/review/stale/permissions/plan/run flow                                 |
| apps/web/src/test/contracts.compile.ts; test/fixtures.ts; pages/Home.test.tsx; pages/AppDetail.test.tsx                                                 | UPDATE                | Generated type guards and existing UI fixtures                                  |
| apps/mobile-worker/tests/test_execution.py                                                                                                              | UPDATE narrowly       | Published browser case → existing generated worker manifest compatibility       |
| contracts/fixtures/test-library/*.json                                                                                                                  | CREATE                | Generated-shape examples and stable invalid/reference fixtures                  |
| scripts/test_library_smoke.py; justfile                                                                                                                 | CREATE/UPDATE         | Actual browser-API authoring/review/run HTTP flow with fake worker              |
| scripts/execution_smoke.py; .github/workflows/ci.yaml; scripts/test_ci_scope.py                                                                         | UPDATE                | Revision-aware maintenance and library/contract/consumer CI selection           |
| contracts/browser.openapi.json; contracts/worker.schema.json; apps/web/src/api/generated/*; apps/mobile-worker/src/mobile_qa_worker/generated/models.py | GENERATE only         | Rust-authoritative outputs; worker output may remain byte-identical             |
| docs/architect/implementation/05-test-library-and-plans.md; contracts.md; development.md; status.md                                                     | UPDATE                | Final implemented behavior, operation runbook and actual acceptance evidence    |

## Not building

- AI generation/discovery, source upload catalog, script export, arbitrary shell/Python test steps or a workflow canvas.
- General customer reset/login/account adapters, a device fleet editor or qualification self-service.
- Multiple device/data variants per plan, named multi-plan management UI, baseline comparisons or flaky-test analytics.
- A replacement queue/worker, Redis/Celery, SSE/WebSockets or changes to current long polling.
- Global config or secrets changes, automatic device/model launches, hidden browser-policy workarounds.

## Step-by-step tasks

Finish all code/test/config across these tasks before the one consolidated generation/format/validation window. VALIDATE bullets describe that window, not checks after each edit.

### Task 1 — contract packet and lifecycle fixtures

- ACTION: Author library DTOs, request/query types, unions and API inventory.
- IMPLEMENT: Lock `LibraryDraftDefinition` with optional-profile PlanDraftContent, lifecycle/permissions/issues/errors, revision/mutation bounds and provenance behavior. Published content uses existing TestDefinition. Add static fixtures and explicit draft-to-published conversion/validation.
- MIRROR: execution.rs derives/validation; execution_api.rs declaration/inventory.
- IMPORTS: serde, schemars only for worker-reachable types, utoipa::{ToSchema,OpenApi,IntoParams}, uuid::Uuid, chrono; no API dependencies in contracts.
- GOTCHA: A draft's semantic incompleteness must not make its typed load/save impossible. Avoid flattening tagged variants into optional-everything objects.
- VALIDATE: Round-trip each variant, unknown fields, incomplete versus publishable content, maximums and provenance compatibility.

### Task 2 — migration/backfill and mutation transactions

- ACTION: Add six narrow library tables/constraints and integrated backfill.
- IMPLEMENT: Catalog/version links, current draft, review audit, explicit default and typed receipts. App-first locking and counter allocation. Preserve historical definitions/approvals/run JSON; add DB indices for app/kind/archive and entry/version queries.
- MIRROR: phase 04 migration and execution_store bound Statement helpers.
- IMPORTS: sea_orm_migration::prelude, sea_orm::{ConnectionTrait,TransactionTrait}, execution_store::{one,rows,exec,decode,json,field,hash}.
- GOTCHA: Never modify applied migration 000004 or rewrite old content hashes. Detect invalid legacy references and report migration failure rather than manufacture approval.
- VALIDATE: Upgrade populated DB, uniqueness/concurrent allocation, rollback, replay and preservation of existing run/detail/worker contracts.

### Task 3 — drafts/catalog/options APIs

- ACTION: Implement typed list/detail/template/create/fork/save/history services and endpoints.
- IMPLEMENT: App scope, bounded drafts, immutable identity, mutation replay, expected revisions, structured issues, capabilities and sanitized profile choices; keep version pagination stable across updates.
- MIRROR: apps::authorized, controller::runs, app-form save flow, test_definitions import patterns.
- IMPORTS: Session, AppContext, ApiFailure/ApiResult; mobile_qa_contracts::test_library::*; axum::{Json,extract::{State,Path,Query}}.
- GOTCHA: Structural validation before storage; full readiness only at review/submission. Read access never implies approve/archive access.
- VALIDATE: Empty drafts without APK, foreign parent IDs, list cursors, duplicate creation, simultaneous saves and response-loss replay.

### Task 4 — submission and review authority

- ACTION: Publish exact candidates and implement approve/needs-input/reject.
- IMPLEMENT: Freeze draft transactionally, purpose grants, hash+revision validation, two-approval transition, append-only review audit, explicit fork after rejection, no approval inheritance. Reuse a common transaction-aware review service from browser and CLI.
- MIRROR: test_definitions::approve and app-row serialization.
- IMPORTS: ApprovalPurpose/TestDefinition, users entity for disabled check, library review DTOs and existing hash helpers.
- GOTCHA: Existing historical approvals do not override needs_input/rejected/archive lifecycle. Concurrent second review must reload after a revision conflict.
- VALIDATE: Business-only/executability-only/separate reviewers, self-author without grant, disabled/revoked memberships, stale save-versus-submit/approval, immutable history.

### Task 5 — archive/default and run admission

- ACTION: Wire library eligibility into preview/resolve/create and explicit default selection.
- IMPLEMENT: Archive/unarchive, default revision, shared ExecutionPlanQuery, exact selected plan, transitive archived dependency checks, consistent duplicate membership rules. Keep queued/finished manifests unchanged.
- MIRROR: runs::preview/create, test_definitions::resolve, current execution defaults.
- IMPORTS: existing execution DTOs and test_library service helpers; no worker changes.
- GOTCHA: Never choose latest approved after initial migration or fall back to a different plan when the default is blocked. No reference update on “newer version available.”
- VALIDATE: Two suites one case → one attempt; conflicting requiredness/version →409; archived referenced case blocks new run; old report and queued job retain frozen content.

### Task 6 — error transport and generated browser adapters

- ACTION: Extend safe error details and create all generated SDK wrappers/query factories.
- IMPLEMENT: Rust typed feature details inside ApiError; middleware preservation; generated request parsing and status-bound success parsing; safe feature-detail parsing; app/workspace/entry/version/filter query keys.
- MIRROR: runtime.ts interceptor, session-transport.ts checked, api/runs.ts.
- IMPORTS: generated sdk/types/zod, queryOptions/useMutation/queryClient; ApiClientError and shared headers/options.
- GOTCHA: No handwritten fetch transport or duplicate schemas. Preserve session/CSRF cache and the pinned Hey API header workaround.
- VALIDATE: Malformed success, non-JSON errors, unexpected success status, typed conflict details, malformed details fallback and TypeScript negative fixtures.

### Task 7 — library shell and case editor

- ACTION: Build app-scoped tabs, list/detail navigation and typed case editing.
- IMPLEMENT: Build-independent loading/empty/error/permission states; form fields and stable item IDs; add/reorder/remove actions/checks without breaking references silently; explicit Save, discard protection and stale revision UX; advanced evidence fields without raw JSON editing.
- MIRROR: Tests.tsx, AppForm, existing Mantine feedback and workspaceHref.
- IMPORTS: @mantine/core/form, @tanstack/react-query, react-router existing useNavigate/Link, generated LibraryDraftDefinition/TestAction/ExpectedCheck.
- GOTCHA: Keep required checks separate from required case selections. Do not auto-save on change or persist draft text in browser storage. Use typed conversion for numeric empty values and enum selects.
- VALIDATE: Missing expectations visible, add/reorder references, invalid numbers, Save retry, 409 preserves input, workspace/logout isolation, accessible labels/keyboard order.

### Task 8 — frozen review and history

- ACTION: Build version view, review controls and history/archive actions.
- IMPLEMENT: Exact frozen content/readiness, review actors/times, purpose buttons, reason input, clear “New draft version,” approval-state refresh; no mutable fields on approved detail.
- MIRROR: RunDetail evidence/status presentation and feedback components; generated review requests.
- IMPORTS: Mantine Card/Alert/Badge/Tabs/Button/Textarea, generated capabilities/review enums, library query/mutation wrappers.
- GOTCHA: Only server capabilities show actions; a disabled control is not the authorization mechanism. Archived/needs-input history remains readable.
- VALIDATE: Dual-review workflow, unauthorized actions, stale hash/revision, reject/fork retains prior approvals as history, late response cannot leak into another workspace.

### Task 9 — suites and default regression plan

- ACTION: Add pinned membership editor and release-plan review/default selection.
- IMPLEMENT: Ordered approved-case selector with version/requiredness, suite selection, qualified profile choice, budget/exclusions, resolved unique preview and explicit update-to-new-version. Create a default release-check draft through the ordinary API; never approve it automatically.
- MIRROR: case editor form state and existing RunPreview layout.
- IMPORTS: generated SuiteDefinition/PlanDraftContent/CaseSelection, Mantine Select/Checkbox/NumberInput and existing Query hooks.
- GOTCHA: No silent dedup of conflicting policies in browser; show server issues. No estimates derived by inventing model cost/duration.
- VALIDATE: Many-to-many membership, available newer version, missing profile, duplicate conflicts, default set/replay/revision and blocked archived default.

### Task 10 — complete UI → Run flow

- ACTION: Connect library review/plan screen and AppDetail to existing preview/run/report.
- IMPLEMENT: Selected plan-version query, build at runtime, stable run idempotency, invalidation after library changes, navigation links and required/excluded scope before Run. Retain fake banners, evidence privacy and terminal polling behavior.
- MIRROR: RunPreview mutation and planQuery; existing RunDetail tests.
- IMPORTS: existing generated createRun/getExecutionPlan/getRun and generated library responses only.
- GOTCHA: Preview is not authorization; createRun rechecks version lifecycle/build/environment before freezing. No direct emulator call from browser.
- VALIDATE: Edit → review → plan → build → run → report; repeat on new build; retry/double-click; original report unchanged after editing source case.

### Task 11 — real HTTP acceptance harness, CI and docs

- ACTION: Add `scripts/test_library_smoke.py`, `just smoke-test-library`, and owning docs/CI fixtures.
- IMPLEMENT: Reuse synthetic Google/app/build setup, provision review grants as test setup only, then author/save/submit/review/default/preview/run exclusively through new browser HTTP APIs. Start existing fake Python worker and assert saved report plus old-version immutability after a later edit. API restart/re-login confirms persistence. Extend CI filters for new Rust schemas/API and worker consumers.
- MIRROR: scripts/execution_smoke.py and test_ci_scope; documented formatter/generator ownership.
- IMPORTS: existing smoke helpers and standard-library HTTP/subprocess; no new test framework or secrets provider.
- GOTCHA: The fake HTTP harness proves protocol persistence, not rendered browser behavior. No operator import shortcut for the customer workflow under test; grants/profile registration may be fixture setup.
- VALIDATE: Fake HTTP acceptance, old manifest equality, generated worker parsing, correctly selected CI jobs on contracts/API/UI/script changes.

### Task 12 — consolidated validation and acceptance record

- ACTION: Generate/format/check/build after the full code/test/config batch, fix failures together and rerun only invalidated checks.
- IMPLEMENT: Run the matrix below and record counts, generated drift, actual manual/browser/device scope and remaining limitations in the implementation report/status.
- MIRROR: phase 04 report, AGENTS.md and development.md.
- IMPORTS: none.
- GOTCHA: Existing browser admin-policy denial remains binding. Do not work around it using an alternate browser or access method; report an unperformed browser acceptance as pending.
- VALIDATE: All deterministic checks plus allowed browser acceptance. Real demo/model launch requires the user-authorized operational scope at implementation time; the existing good-path result alone does not prove the new UI.

## Testing strategy

| Scenario                                                    | Expected assertion                                                             |
| ----------------------------------------------------------- | ------------------------------------------------------------------------------ |
| Empty title/actions/checks or absent draft plan profile     | Draft saves/loads as typed data with issues; submission blocked                |
| Oversized fields/body/arrays, invalid union/enum/UUID       | 400/413/422 safe errors; no partial writes                                     |
| Unknown request fields / forged provenance/version/approval | Rejected or server-assigned identity; never silently trusted                   |
| Concurrent draft save                                       | One success, one 409; no lost update                                           |
| Save versus submit/review/archive                           | App/entry locking and revision check; stable frozen hash                       |
| Lost response to save/submit/review/default                 | Same mutation returns original result, no duplicate version/audit              |
| Foreign app/entry/version/cursor/profile/member reference   | Opaque 404 or typed invalid reference without cross-tenant data                |
| User has app access without review grant                    | Can author, cannot approve; no self-grant                                      |
| Business approved, executability missing                    | Still not runnable; explicit missing review                                    |
| Reject/needs-input after one approval                       | Candidate blocked; history retained; edit allocates new version                |
| Approved A → plan → run → new draft A                       | Frozen A/plan/report byte-for-byte stable                                      |
| Same case via two suites/direct+suite                       | One resolved case for supported configuration                                  |
| Conflicting versions/requiredness                           | 409; no silent precedence                                                      |
| Archive transitive dependency                               | New run blocked; queued/historical manifest unchanged                          |
| Approve newer plan                                          | Explicit default remains previous version                                      |
| Change selected build                                       | Plan version stays selected; build readiness reevaluated                       |
| Malformed success/error details                             | Generated Zod rejects or generic safe failure, no unchecked rendering          |
| Logout/switch workspace during save                         | No cross-workspace cache update/navigation or leaked draft content             |
| Fake execution selected                                     | Simulation shown in preview and saved report                                   |
| Browser case → worker manifest                              | Generated Rust/Schemars/Pydantic round-trip, no handwritten worker test shapes |

Use unit tests for semantic conversion, real PostgreSQL route tests for persistence/races/approval, Vitest DOM tests for interactions and generated transport tests for boundary failures. Do not claim a coverage percentage or every race covered without measurement.

## Validation commands

After complete code/test/config authoring, deduplicate checks instead of running narrow and full variants back-to-back:

```bash
just types
just format
just format-check
just check-contracts
just check-api
just check-web
just check-worker
python3 scripts/test_ci_scope.py
just build
just smoke-test-library
```

- `just check-api` owns actual migrations/Clippy/Rust workspace tests in the existing isolated Compose test DB; no production DATABASE_URL.
- `just check-web` owns generated compile fixtures, lint and DOM/transport tests. `just check-worker` protects published-manifest compatibility and existing device behavior.
- `just smoke-test-library` must be authored in Task 11 before invocation; run sequentially with other API smoke scripts because they share the test port/database conventions.
- Prepare locked dependencies/JDK/Android intake fixtures explicitly, following development.md; checks/fake smoke do not fetch Doppler secrets.
- Review generated diffs and run scoped dependency audits before a later commit; retain the existing Rust rsa advisory, never call that audit clean.
- This planning task itself needs only plan/doc link consistency, formatting and diff review, not application tests.

Manual acceptance, only through an allowed browser:

1. Sign in as app author, create a draft before uploading a build, save incomplete content and see missing expectations.
2. Finish the controlled persistence case and request review; use independently authorized reviewer identities to complete both review purposes.
3. Group/pin the case, review the release plan and explicitly set default.
4. Upload/select good demo build, preview scope and Run; follow progress to an evidence-backed report.
5. Edit approved case into a new draft; reopen old report and confirm pinned content/hash unchanged.
6. Open two editors, save one and verify stale-save/review behavior in the other.
7. Exercise fake mode through normal UI for repeatable acceptance, then one scoped real emulator run using the new UI. Record which modes actually ran.
8. Check keyboard-only controls, mobile-width layout, private screenshot access, archived blockers and logout/workspace transitions.

## Acceptance and completion criteria

- [x] Case/suite/plan drafts are editable without CLI JSON and without an uploaded build.
- [x] Rust enforces draft bounds, semantic submission, approvals, app scope, archive and concurrency.
- [x] Every browser request/success/feature-error detail uses generated contracts and runtime validation.
- [x] Published content reaches the existing generated Python protocol without independent consumer shapes.
- [x] Review binds exact hash/revision/purpose; all history and old manifests remain immutable.
- [x] Default plan and membership versions change only through explicit reviewed choices.
- [x] No draft, needs-input, rejected or archived dependency can enter a new release run.
- [x] Existing Run/report/long-poll/device functionality remains intact.
- [x] Actual route inventory and schema/status agreement, compile-negative tests and fake HTTP acceptance pass.
- [ ] Allowed UI acceptance proves author → review → plan → build → run → report; otherwise mark that gate pending.
- [x] Formatting/type/lint/test/build/drift checks pass and owning docs distinguish implemented versus unverified behavior.
- [x] No unrelated device/framework/dependency/auth/settings expansion.
- [x] Implementation report lists deviations/actual checks and keeps unfinished browser/device acceptance explicit.

## Risks and mitigations

| Risk                                               | Likelihood / impact                | Mitigation                                                                             |
| -------------------------------------------------- | ---------------------------------- | -------------------------------------------------------------------------------------- |
| Older main used after stacked merge                | High / wrong baseline              | Integrated commit/tree recorded; verify ancestry before coding                         |
| UI duplicates contracts or semantic rules          | High / unsafe apparent type safety | Generated DTOs/Zod, shared Rust conversion/issues, route tests                         |
| Editable content invalidates historical hashes     | High / destroys auditability       | Separate drafts, immutable existing version store, backfill preservation tests         |
| Rejection/archive ignored by existing CLI/resolver | High / unintended run              | Shared lifecycle admission and transaction-aware CLI integration                       |
| Automatic latest plan/version changes scope        | Medium / incorrect coverage        | Explicit default pointer and pinned membership UI                                      |
| Editor implies unsupported customer execution      | High / misleading readiness        | Capability-driven UI, demo adapter constraint and technical review                     |
| Browser policy blocks manual acceptance            | Known / incomplete evidence        | DOM + real HTTP checks, retain explicit browser acceptance gate; no bypass             |
| XL scope grows into source generation/fleet UI     | Medium / incomplete core flow      | Twelve ordered tasks; source catalog, multiple device variants and generation excluded |

Confidence: **8/10** for deterministic implementation using the integrated baseline; final UI/device acceptance remains dependent on the allowed operational environment. This plan deliberately reuses the working dispatch/report pipeline and adds the authoring/version/review layer around it.
