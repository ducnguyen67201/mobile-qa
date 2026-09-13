# Plan: Direct execution and AI test authoring

## Summary

Revamp phase 06 around reusable tests: testers pick controls and run ordinary steps directly, use predefined templates, or ask AI to explore selected journeys and propose named tests. Preserve the existing phone workspace, test library, approvals, HTTP worker leases and reports. AI is explicit authoring assistance or an explicit Ask AI step; selecting Tap or Enter text never silently invokes a model.

**Status: planned, not implemented.** This is the next revision of phase 06, following the task-session implementation. The older task-session plan remains implementation history, not the specification for this revamp.

## User story

As a tester, I want to interact with my app, select a reusable template, or generate named scenarios from its screens, so that I can build useful tests with minimal setup and rerun them without paying for AI to rediscover every click.

## Problem → solution

Today, the step editor compiles even picked controls into one Minitap goal. Save as test creates a case containing one navigation prompt and no assertions. The new flow preserves structured steps all the way from the browser through Rust to Python, and through saved definitions and repeated regression runs. AI discovery produces reviewable proposals with source screens and explicit expected results.

## Metadata

- Complexity: **XL**, approximately 60–75 handwritten/generated files, 14 implementation tasks.
- Source: user direction on 2026-09-13; [canonical phase 06 spec](../../../docs/architect/implementation/06-test-generation.md).
- Baseline inspected: `codex/06-task-sessions` at `1b68baa`; PR #6 targets `main`.
- Dependencies: existing phases 04–06 source, qualified Android device, private artifacts and library mutation service. AI additionally requires an explicitly configured model capability.
- Delivery slices: **A — direct execution and recording**, **B — reusable templates and saved tests**, **C — AI discovery and proposals**. Complete each agreed slice's source/test/config before its validation batch; the entire revamp is complete only after A–C.
- Confidence: **8/10**. Contracts and application paths are understood; real device selector/text behavior and AI proposal quality need qualification, not assumptions.

## UX design

### Before

```text
Tests → new case → stable key → long form
Task steps → one prose goal → Minitap for every action
Save as test → one navigate action → manually add assertions
```

### After

```text
Tests / selected app
[Generate with AI]  [Use a template]  [Create manually]

Smoke — saved task survives restart                  [Save] [Run test]
┌──────────────────────────────────┬───────────────────────────────┐
│ 1 Enter text: Task name           │ App preview                   │
│   "Buy milk"   [Pick on phone]    │ [Pick target | Control phone] │
│ 2 Tap: Save                      │                               │
│ 3 Restart app                    │     actual Android screen     │
│ 4 Check: saved item = Buy milk    │                               │
│ [+ Add step] [Ask AI]             │ [Record interactions]         │
│                                  │                               │
│ Direct · no AI calls              │ Latest capture / connection   │
└──────────────────────────────────┴───────────────────────────────┘

Generate with AI
Coverage: [Smoke ▾]   Optional: "Focus on creating tasks"
[Discover & generate] → visited screens + bounded progress → proposals
  [✓] Smoke — app opens                 Ready to try
  [✓] Create and save a task            Ready to try
  [ ] Empty task name is rejected       Confirm expected behavior
[Save selected tests] → editable drafts → trial → existing review/run flow
```

The preview remains a refreshed device capture, not video streaming. Preserve the resizer, full-width workspace, compact shell, keyboard controls and narrow-screen stacking.

### Interaction changes

| Touchpoint        | Before                             | After                                                      | Rule                                                                    |
| ----------------- | ---------------------------------- | ---------------------------------------------------------- | ----------------------------------------------------------------------- |
| New test          | Mandatory technical key            | Editable title; generated internal key                     | Create without typing a key; advanced key visibility only               |
| Pick on phone     | Adds metadata to prose             | Binds a typed selector to that step                        | Picking never taps the device                                           |
| Control phone     | Not available                      | Explicit direct tap/type/back/swipe mode                   | Execution serialized through the same session queue                     |
| Recording         | No structured recording            | Toggle records successful direct interactions              | Failed/uncertain actions stay in history, not silently replayable steps |
| Run task/test     | Always Minitap                     | Direct commands, with optional labeled Ask AI steps        | No automatic model fallback on selector failure                         |
| Templates         | Specialized existing demo template | Small versioned catalog with visible binding slots         | Works without model credentials                                         |
| AI generation     | Planned                            | Coverage preset, optional journey, one explicit start      | Default to Smoke and at most five proposals                             |
| Generated results | No results                         | Suggested names, steps, expectations, source screens, gaps | AI suggestion is neither approved nor passed                            |
| Save              | Create then save from browser      | Atomic draft creation with all structured content          | Retried response cannot duplicate tests                                 |
| Run report        | Whole task status                  | Ordered step outcomes, checks, evidence and AI usage       | Action completion is separate from assertion success                    |

## Mandatory reading and discovery

All paths below are repository relative. Line references identify the inspected baseline; generated files are outputs, never editing targets.

| Priority/category   | File:lines                                                                | Pattern / reason                                                                    |
| ------------------- | ------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| P0 authority        | `docs/architect/README.md:1`                                              | Canonical ownership; planned versus implemented evidence                            |
| P0 transport        | `crates/contracts/src/execution.rs:67`                                    | Existing action kinds; profile and immutable case contracts                         |
| P0 transport        | `crates/contracts/src/execution.rs:430`                                   | Semantic case/check validation, limits and placeholder rules                        |
| P0 transport        | `crates/contracts/src/task_sessions.rs:28`                                | Frame-local controls, selection, task and lease contracts                           |
| P0 library          | `crates/contracts/src/test_library.rs:69`                                 | Tagged draft content; revision and capability contracts                             |
| P0 state            | `apps/api/src/services/task_sessions.rs:164`                              | Locked submission, identity fingerprint and replay before readiness checks          |
| P0 fencing          | `apps/api/src/services/task_sessions.rs:345`                              | Worker identity, lease expiration, update and clean-release rules                   |
| P0 save             | `apps/api/src/services/test_library_mutations.rs:70`                      | App transaction, authorization even on replay, mutation receipts                    |
| P0 runtime          | `apps/mobile-worker/src/mobile_qa_worker/execution/actions.py:100`        | Approved runs currently require Minitap/model even before direct restart            |
| P0 runtime          | `apps/mobile-worker/src/mobile_qa_worker/task_sessions.py:227`            | Exclusive device lifecycle and task dispatch                                        |
| P0 UI               | `apps/web/src/components/task-session/phone-workspace.tsx:409`            | SaveTask currently loses step structure and creates no checks                       |
| P1 entry            | `apps/api/src/controllers/task_sessions.rs:18`                            | Session browser identity versus scoped Worker identity                              |
| P1 UI               | `apps/web/src/pages/Tests.tsx:143`                                        | Existing catalog tabs, new-entry affordance and capabilities                        |
| P1 UI               | `apps/web/src/components/task-session/task-steps.tsx:1`                   | UI-only steps and taskGoal compiler to replace                                      |
| P1 UI               | `apps/web/src/components/test-library/draft-editor.tsx:1`                 | Dirty draft and optimistic revision ownership                                       |
| P1 device           | `apps/mobile-worker/src/mobile_qa_worker/qualification/device.py:97`      | Owned serial/AVD, launch, snapshot and cleanup; never attach to an arbitrary device |
| P1 verification     | `apps/api/src/services/verification.rs:10`                                | Rust independently observes retained hierarchy; model prose is not a verdict        |
| P1 errors           | `apps/api/src/services/test_library_mutations.rs:45`                      | Typed stale-revision details and actionable errors                                  |
| P1 logs             | `apps/api/src/services/runs.rs:220`                                       | Structured IDs and phase; do not log task text, screenshots or secrets              |
| P1 API tests        | `apps/api/tests/task_sessions.rs:1`                                       | Real HTTP/Postgres, explicit synthetic worker boundary                              |
| P1 worker tests     | `apps/mobile-worker/tests/test_task_sessions.py:1`                        | Generated Pydantic fixtures and parameterized invalid hierarchy tests               |
| P1 web tests        | `apps/web/src/pages/TaskSession.test.tsx:1`                               | DOM interactions through generated SDK boundaries                                   |
| P1 dependencies     | `apps/mobile-worker/pyproject.toml:1`                                     | Optional SDK, strict Python checks; explicit dependencies for new imports           |
| P1 dependency locks | `apps/mobile-worker/uv.lock:1518`, `:3024`                                | Existing langchain-openai 1.6.2 and uiautomator2 3.5.0                              |
| P1 secrets          | `apps/mobile-worker/src/mobile_qa_worker/qualification/sdk_adapter.py:45` | Isolated model child and Doppler-only secret injection                              |
| P2 commands         | `justfile:1`, `scripts/contracts.py:1`                                    | Explicit generation/check batches, no watchers                                      |
| P2 migration        | `apps/api/migration/src/m20260913_000006_task_sessions.rs:1`              | Existing session/task persistence and shared reservation namespace                  |

Graphify provenance points to `72bd3fb`, before these execution/library/session changes; relationship discovery therefore used current source. The supplement's `docs/CODEX-NAVIGATION-GUIDE.md` is absent. Do not use parent sources or stale graph edges as implementation authority.

### Five traced paths

1. **Task entry:** React TaskSteps → taskGoal → generated runPhoneTask SDK → Session controller → locked phone_tasks row → worker heartbeat response → Minitap subprocess. Replace prose compilation for direct steps at every boundary.
2. **Saved definition:** SaveTask → separate create/save calls → versioned library → submit/review → execution definitions → immutable run manifest. Replace the two-call promotion with a single mutation; preserve all existing review rules.
3. **Regression execution:** run admission → resource reservation → worker HTTP lease → execution/actions.py → SDK or restart → retained checkpoint artifacts → Rust verification → report. The direct executor must integrate here, not just in previews.
4. **Device state:** owned AVD → frame/control capture → browser selection → current-frame validation → task state → cleanup or quarantine. Frame-local numeric control IDs are not persistent selectors.
5. **Generated transport:** Rust derives + route registry → Utoipa/OpenAPI → Hey API SDK/types/Zod; Schemars WorkerContracts → Pydantic. Add generation/model request types to the same pipeline, not handwritten Python/TypeScript equivalents.

## External documentation

Researched on 2026-09-13; pin existing installed versions during implementation.

| Topic                            | Source                                                                                    | Finding                                                                                          |
| -------------------------------- | ----------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| Direct Android operations        | [uiautomator2 3.5.0 README](https://github.com/openatx/uiautomator2/blob/3.5.0/README.md) | Selector-based click/set_text, explicit serial connection, hierarchy and bounded waits exist     |
| Existing agent structured output | [Minitap task requests](https://www.minitap.ai/docs/mobile-use-sdk/core-concepts/tasks)   | Task builder supports typed output; installed 4.0.0 builder has `with_output_format` at line 214 |
| Structured authoring calls       | [LangChain ChatOpenAI](https://docs.langchain.com/oss/python/integrations/chat/openai)    | Typed structured output and usage metadata; explicit timeout/retry settings are necessary        |

**KEY_INSIGHT:** use uiautomator2 for direct execution with a resolved selector and explicit owned serial. **APPLIES_TO:** tap/type/recording and repeat runs. **GOTCHA:** implicit waits default to 20 seconds; set bounded waits explicitly. Text entry must use RPC, not shell interpolation. Unicode, empty replacement and keyboard behavior require real-device validation.

**KEY_INSIGHT:** structured model output can use generated Pydantic models. **APPLIES_TO:** discovery decisions and scenario proposals. **GOTCHA:** schema-valid output does not prove a selector exists or an expectation is correct. The installed `langchain_openai/chat_models/base.py` contains `with_structured_output`; explicitly declare langchain-openai 1.6.2 in the AI extra rather than relying on a transitive import.

**KEY_INSIGHT:** Minitap remains useful for explicit Ask AI steps. **APPLIES_TO:** legacy navigation and opt-in complex tasks. **GOTCHA:** its final prose/structured response is not an authoritative replay trace. Discovery below uses a constrained decision loop that calls the shared direct executor, so captured actions are real executable data. No additional agent framework is needed.

## Patterns to mirror

### Naming and generated runtime boundaries

Source: `apps/web/src/api/test-library.ts:17`.

```typescript
export const libraryKey = (workspace: string, appId: string) =>
  ['test-library', workspace, appId] as const
```

Source: same file, line 26.

```typescript
checked(sdk.listTestLibrary({ ...options, path: { app_id: appId }, query }), z.zLibraryListResponse)
```

Use kebab-case web modules, snake_case Rust/Python modules and PascalCase generated contract types. Query keys include workspace, app and resource identity; mutations invalidate the owning catalog, not every workspace.

### Error handling

Source: `apps/api/src/services/test_library_mutations.rs:45`.

```rust
ApiFailure::new(
    409,
    "stale_revision",
    "This record changed. Compare your draft with the current version before saving again",
)
.with_library_details(LibraryErrorDetails::StaleRevision {
    entry_id: id,
    current_revision: revision,
})
```

### Data access and service boundary

Source: `apps/api/src/services/test_library_mutations.rs:76`.

```rust
library::authorize(ctx, actor, app).await?;
let tx = ctx.db.begin().await?;
one(
    &tx,
    "SELECT id FROM apps WHERE id=$1 FOR UPDATE",
    vec![app.into()],
)
.await?;
```

Controllers use `State<AppContext>`, `Session` or `Worker`, `Json<GeneratedRequest>` and `ApiResult<Json<GeneratedResponse>>`. Model/device work runs outside transactions. Preserve the existing app → resource locking order and mutation fingerprints.

### Logging

Source: `apps/api/src/services/runs.rs:220`.

```rust
tracing::info!(run_id=%id,app_id=%app,phase="queued","Approved execution queued");
```

Use IDs, phase, counts, duration and sanitized error codes. Worker CLI already configures `logging` (`apps/mobile-worker/src/mobile_qa_worker/cli.py:41`); capture structured usage in results instead of logging prompts, text inputs, tokens or entire HTTP payloads.

### Test structure

Source: `apps/api/tests/task_sessions.rs:7`.

```rust
#[tokio::test]
async fn task_session_requires_no_plan_and_fences_worker_and_task_identity() {
    let _guard = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, ctx| async move {
        let owner = login(&server, &ctx).await;
```

Reuse `apps/api/tests/support`, generated Python models, pytest parameterization and TaskSession DOM tests. Synthetic model/device tests must be labeled; they do not establish real Android behavior.

## Architecture and decisions

### One structured action format

Refactor `TestAction` into a Rust enum tagged by `kind`. Keep existing `navigate`, `restart_app` and `checkpoint` JSON byte-compatible in field values and serialization order. Add `direct` with `id`, `checkpoint_id` and a tagged `DirectCommand`. Existing non-navigation `instruction: ""` remains accepted/preserved for legacy records. Provide Rust accessors for common ID/checkpoint fields; update consumers to exhaustive matching.

Reuse this action type in phone tasks, saved cases, template instances, AI proposals and run manifests. Direct commands initially support tap, set_text, directional swipe, back, restart and wait_for. Expected checks remain separate assertions associated with step checkpoints, using existing supported methods. In the UI, a Check step edits an expected check and its capture checkpoint; it does not ask AI whether the test passed.

`DirectTarget` is a tagged selector: resource ID or accessibility description, qualified by the app package and optional class. Prefer resource IDs. Match exactly one enabled node; type requires an editable non-password node. Do not persist frame-local indexes or use current input text as the identity. Picking carries frame/control identity for author-time validation plus the reusable selector. Resource-ID assertions remain the initial supported automatic check surface; other selected targets may be actionable but require an explicit supported check binding before review.

No arbitrary XPath, shell commands, downloaded scripts or model-generated executable code. Direct text is literal bounded data, including Unicode and empty replacement; preserve the existing explicitly allowed `${task_title}` fixture substitution only where the contract declares it. No automatic fuzzy match, coordinate replay or Ask AI fallback.

### Compatibility and worker capabilities

Keep the old phone task route for existing clients; interpret legacy goal-only requests as explicitly legacy AI tasks. Add a versioned structured command route rather than ambiguously accepting goal and steps together. New UI exclusively uses the new route.

Add advertised operation capabilities to worker/profile readiness. Direct-only registered profiles must boot and run with no model configuration; add a `direct` driver while preserving `fake` and `minitap`. Require a model only if the manifest contains navigate/Ask AI or generation requests. Existing Minitap profiles may support both paths only after the new direct capability is qualified. Do not infer qualification from the installed package. Host config accepts an absent model for direct profiles; frozen legacy manifests retain their original model fields.

Introduce a protocol capability/version on worker registration and claim requests. Do not deliver new structured jobs to old workers. Unknown versions/unsupported operations produce a typed readiness error. Drain or retain old active sessions under the legacy executor during deployment; do not reinterpret an in-flight goal as direct steps.

Never rewrite published definition payloads, stored content hashes, historical manifests or approvals. Forking an old draft can display navigation as Ask AI; converting prose into direct steps is an explicit authoring action that needs validation and a new reviewed version. Golden fixtures must verify legacy serialization and hashes are unchanged by the Rust enum refactor; if the serializer changes bytes, preserve an explicit legacy serializer before shipping.

### Shared direct executor

Create `mobile_qa_worker/automation` with selectors, device commands and trace capture. Both task sessions and `execution/actions.py` call it. Use explicitly pinned uiautomator2 3.5.0 in a `device` extra; AI packages remain in optional `sdk`/`ai` extras. Construct the client against the already leased serial, never the first attached device.

For each command: validate current package and target → mark step started durably → resolve against fresh hierarchy → execute once → capture → report immutable action ID, outcome and evidence. A lost device response is uncertain, not safe to retry. Polling reads may retry within bounded time; taps/text/swipes may not be repeated automatically. Missing controls return needs-input/blocked; app assertion mismatch is failed; unknown outcome is inconclusive. Rust independently verifies regression checks from retained artifacts.

Retain host locks, dirty journal, reservations, heartbeat, deadline and quarantine behavior. Stop cancels the current child and blocks subsequent steps, but does not claim to undo a tap. Release only after device cleanup is confirmed. No manual and automatic commands execute concurrently on one lease.

### Phone control and recording

The right panel has **Pick target** and **Control phone** modes. Pick binds a step only. Control executes a single typed command through the same queue; text entry uses a compact text dialog after selecting an editable control. Back/swipe are explicit buttons/directions. Only ready sessions accept commands.

Record interactions is off by default. When enabled, append successful commands from authoritative receipts to the draft, preserving targets, literals and order. Show failed and uncertain commands in session history. Use stable client command IDs and expected session revision; two tabs cannot double-send an action from the same frame. Stale-frame requests return a refresh instruction. Re-resolve accepted targets immediately before execution; a screenshot selection is not a permanent coordinate.

Draft state has one owner. In a case editor the phone workspace reads/writes the existing draft actions/checks; remove the disconnected parallel task editor. Trials freeze the current unsaved sequence and do not implicitly save/approve it. Reordering a pending draft cannot change an already submitted sequence.

### Templates, naming and save

Serve a small code-versioned catalog from Rust. Initial templates:

- **Smoke — app opens:** launch and check a selected landing-screen marker.
- **Form — submit valid input:** input target, example value, submit target and success assertion.
- **Validation — required field:** empty input and an author-confirmed error expectation.
- **Persistence — value survives restart:** input, save, restart and a bound saved-value assertion.

Templates contain explicit slots, descriptions and supported check methods. Slots remain unresolved until picked or entered; never pretend generic templates already know an APK's resource IDs. Templates require no AI. A qualified demo mapping can prebind the sample APK only when package/build evidence matches.

Create drafts with human names and an internal key derived from a slug plus UUID suffix; the UI never requires a key. Persist template ID/version and generation provenance in additive draft metadata. Freeze its source/expectation-confirmation revision on submission and reference it from the existing case provenance field, so later catalog edits cannot change what was reviewed. Avoid changing immutable legacy case content for catalog-only category labels.

Add atomic library mutations for saving a recorded sequence and accepting a selected proposal batch. Validate all selected items and write entries, complete draft content, provenance and receipts in one app transaction. Same mutation ID plus same payload returns the original result; differing payload conflicts. Duplicate names are permitted; IDs/keys remain unique. Do not automatically create/default/approve a release plan. Testers can trial drafts immediately; approved regression runs retain phase 05 rules.

### AI discovery and scenario generation

**Generate with AI** opens a small drawer with Smoke selected, optional journey text, and the compatible build. Reuse the selected validated build/profile and active session when unambiguous. If no phone exists, the explicit Discover & generate action opens one. Show missing device/model capability honestly; templates/manual creation stay available.

Persist a generation job before any model call. The Rust-owned job is linked to an app/build/environment revision and phone session, executed through its existing leased worker command queue. Generation can consume a compatible saved discovery snapshot without driving the phone again; initial worker execution still uses the session transport. A separate general-purpose queue service is not needed.

For discovery, the Python parent captures a bounded hierarchy and redacted screenshot, sends observations to an isolated model child, and receives a generated `DiscoveryDecision` containing one allowed direct command or finish/needs-input. The parent validates the decision against current controls and executes it through the shared direct executor. Record actual commands and resulting snapshots. The model child has no worker/API token and no direct device tools. Retain Minitap only for explicit Ask AI tasks; do not use an opaque Minitap prose trace as a direct replay recording.

Initial bounded defaults: at most 12 actions, 8 distinct captured states, 180 seconds, 5 proposed cases and 20 steps per case. Maximum 16 model calls per job, including proposal drafting and one repair attempt; maximum 2,000 output tokens per discovery decision and 8,000 for the proposal batch. Bound each model request to 64 KiB sanitized textual context plus at most two existing size-bounded redacted images. Set provider retries to zero; the job owns all retries. Enforce total deadline and remaining budget before every call/action. These are conservative implementation defaults, not measured pricing promises. Emit measured tokens/calls, unknown usage explicitly, and stop on missing usage when further budget cannot be accounted for. Do not display invented dollar savings.

Discovery uses the declared test environment and allowed journey. Read-only exploration is the default; form submission/write actions require the selected journey to permit test-data changes. Enforce a generated allowed-action set and configured target restrictions in the executor, not just a prompt. Stop for external packages, login/reset requirements, unsupported controls or excluded actions. This is not a semantic guarantee that every permitted app button is harmless; keep discovery on qualified test fixtures and show the explored scope.

Before provider transmission, redact password-node text and corresponding screenshot regions using a standard local image library; exclude credentials, account references and unrelated screen content. Do not confuse hiding picker controls with redacting screenshots. Redaction failure makes the observation unavailable to AI. App text remains untrusted data and cannot expand action permissions or provider/tools configuration.

After discovery, send compact retained observations, actual trace, selected coverage type, optional source requirements and existing case names/signatures to a structured drafting call. Use generated Pydantic proposal types. Proposed cases contain editable title, category (smoke/happy_path/validation/persistence), structured steps, expected checks or explicit unbound questions, source snapshot/requirement references and duplicate suggestions. Resolve references server-side against that job's retained data; reject invented selectors/sources and unsupported operations.

An observed state does not establish desired behavior. Mark expectations from requirements separately from AI suggestions; suggested expectations must be explicitly accepted by the author before review. For example, observing that an empty task is saved cannot redefine a stated requirement that empty tasks must be rejected. Proposals lacking intent remain needs-input. Trial success never auto-approves a case.

Reuse discovery by app, build checksum, environment revision, device image, package, declared initial state/reset scope and discovery-policy version. Mark old context stale after any binding changes; prompt/template version changes invalidate generated proposal reuse. Reuse is tenant scoped and visible. Opening Tests or uploading an APK never triggers paid generation automatically. Repeated direct regression runs make **zero model calls**; explicit Ask AI steps retain separately recorded usage.

### Persistence and proposed routes

Add a new additive SeaORM migration, preserving all applied migrations. Extend phone task persistence with frozen structured payload, session revision and durable ordered step receipts. Add `test_generation_jobs`, `test_discovery_snapshots` and `test_generation_proposals`; store metadata/source links in PostgreSQL and bounded evidence in private artifact storage. Add indexes for app/creator/status and unique job/item identity. Extend lease-authorized `PhoneUpdate` with tagged command progress, generation-stage updates and artifact receipts; validate stage transitions, monotonic counters, command identity and stored source references on every update. Add `POST /api/worker/phones/{session_id}/generation-artifacts` for bounded artifact bytes, authorized by Worker plus the current lease token; return a generated opaque receipt. Generation save links and mutation receipts commit atomically.

| Route (proposed)                                                           | Purpose                                     | Key contract                                                      |
| -------------------------------------------------------------------------- | ------------------------------------------- | ----------------------------------------------------------------- |
| `POST /api/phones/{session_id}/commands`                                   | Structured sequence or single manual action | command_id, expected_revision, frame binding, typed steps/checks  |
| `GET /api/apps/{app_id}/test-templates`                                    | Versioned template catalog                  | Generated template slots/category/check types                     |
| `POST /api/apps/{app_id}/test-generations`                                 | Persist an explicit generation job          | mutation identity, build/session, source revision, scope, budgets |
| `GET /api/apps/{app_id}/test-generations/{job_id}`                         | Progress, proposals, usage and gaps         | Generated bounded job response                                    |
| `POST /api/apps/{app_id}/test-generations/{job_id}/cancel`                 | Stop future calls/actions                   | Idempotent cancellation acknowledgement                           |
| `POST /api/apps/{app_id}/test-generations/{job_id}/accept`                 | Atomically save selected proposals          | mutation_id, expected_job_revision, item IDs and edited content   |
| `POST /api/apps/{app_id}/test-library/from-recording`                      | Atomically save a typed trial/recording     | mutation_id, session/command provenance, title, actions/checks    |
| `GET /api/apps/{app_id}/test-generations/{job_id}/artifacts/{artifact_id}` | Private source/evidence view                | Existing membership/creator scope, opaque artifact ID             |

Document exact success/error codes in Utoipa and real route tests. Browser requests use Session/CSRF; worker claim/update extensions use Worker plus lease token. Worker artifact writes use bounded lease-authorized uploads following run_artifacts; never accept filesystem paths or remote URLs from model output. Creator owns interactive generation access; app membership and current library capabilities are rechecked on acceptance, including retries.

Job states: queued → discovering → drafting → ready/needs_input/failed/canceled. Persist stage, counters and partial proposals after each completed stage. Ready/needs_input proposals are editable drafts, not run outcomes. Replays after network loss return stored job state; worker crash after uncertain device effects quarantines the session and never auto-resumes discovery. Cancellation retains already generated proposals and never mutates approved cases. Retain snapshots while referenced by saved provenance; unreferenced expired jobs can be removed using the existing explicit cleanup workflow, not a new unsupervised daemon.

## Files to change

Brace groups below enumerate related owned modules; directory entries cover the explicitly named new modules. Generated files are listed to prevent hand edits.

| File / group                                                                                                                                                | Action        | Responsibility                                                                                   |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------- | ------------------------------------------------------------------------------------------------ |
| `crates/contracts/src/execution.rs`, `task_sessions.rs`, `test_library.rs`                                                                                  | UPDATE        | Typed actions, checks, profile capabilities, commands and atomic save DTOs                       |
| `crates/contracts/src/test_generation.rs`                                                                                                                   | CREATE        | Templates, discovery decisions, proposal/source/job/usage contracts                              |
| `crates/contracts/src/test_generation_api.rs`                                                                                                               | CREATE        | New browser operations                                                                           |
| `crates/contracts/src/{lib.rs,browser.rs,worker.rs,task_sessions_api.rs,test_library_api.rs,execution_api.rs}`                                              | UPDATE        | Exports, worker envelopes and exact operation registries                                         |
| `contracts/fixtures/execution/`                                                                                                                             | ADD           | Legacy hash fixtures and direct/mixed/invalid sequences                                          |
| `contracts/fixtures/generation/`                                                                                                                            | CREATE        | Grounded, ambiguous, contradictory, duplicate and malicious-output fixtures                      |
| `contracts/{browser.openapi.json,worker.schema.json}`                                                                                                       | GENERATE      | `just types` only                                                                                |
| `apps/web/src/api/generated/`, `apps/mobile-worker/src/mobile_qa_worker/generated/models.py`                                                                | GENERATE      | Existing exporter locations, confirmed by scripts/contracts.py and apps/web/openapi-ts.config.ts |
| `apps/api/migration/src/m20260913_000007_direct_test_authoring.rs`, `lib.rs`                                                                                | CREATE/UPDATE | Additive migration and registry                                                                  |
| `apps/api/src/services/{task_sessions.rs,test_library.rs,test_library_mutations.rs,test_definitions.rs,runs.rs,verification.rs,run_artifacts.rs}`           | UPDATE        | Queue, save, admission, selector/check evidence and private storage integration                  |
| `apps/api/src/services/{test_generation.rs,test_templates.rs}`                                                                                              | CREATE        | Durable generation orchestration and template catalog                                            |
| `apps/api/src/controllers/{task_sessions.rs,test_library.rs,test_generation.rs}`                                                                            | UPDATE/CREATE | Browser and worker endpoints                                                                     |
| `apps/api/src/{app.rs,services/mod.rs,controllers/mod.rs,errors.rs}`                                                                                        | UPDATE        | Registration and generated actionable error details                                              |
| `apps/api/src/services/worker_auth.rs`, `apps/api/src/tasks/execution.rs`                                                                                   | UPDATE        | Capability/version admission for old/new workers                                                 |
| `apps/mobile-worker/src/mobile_qa_worker/automation/{__init__.py,selectors.py,direct.py,trace.py}`                                                          | CREATE        | Shared direct execution and fresh target resolution                                              |
| `apps/mobile-worker/src/mobile_qa_worker/authoring/{__init__.py,discovery.py,generation.py,model_adapter.py}`                                               | CREATE        | Bounded authoring loop, schemas and isolated model seam                                          |
| `apps/mobile-worker/src/mobile_qa_worker/{task_sessions.py,cli.py}`                                                                                         | UPDATE        | Structured command dispatch and explicit model child                                             |
| `apps/mobile-worker/src/mobile_qa_worker/execution/{actions.py,runner.py,client.py}`                                                                        | UPDATE        | Direct and mixed approved runs plus generated lease protocol                                     |
| `apps/mobile-worker/src/mobile_qa_worker/qualification/{device.py,config.py,sdk_adapter.py}`                                                                | UPDATE        | Device-only readiness and shared ownership; preserve legacy SDK behavior                         |
| `apps/mobile-worker/{pyproject.toml,uv.lock}`                                                                                                               | UPDATE        | Explicit pinned device/AI extras, no incidental SDK upgrade                                      |
| `apps/web/src/api/{task-sessions.ts,test-library.ts,test-generation.ts}`                                                                                    | UPDATE/CREATE | Generated checked SDK queries and mutations                                                      |
| `apps/web/src/components/task-session/{phone-workspace.tsx,task-steps.tsx}`                                                                                 | UPDATE        | Structured editing, control/record modes, step feedback                                          |
| `apps/web/src/components/test-library/{draft-editor.tsx,definition-fields.tsx,library-presentation.tsx}`                                                    | UPDATE        | Shared draft ownership, legacy display, names/categories                                         |
| `apps/web/src/components/test-library/{generation-drawer.tsx,template-picker.tsx,proposal-review.tsx}`                                                      | CREATE        | Simple creation choices and proposal review                                                      |
| `apps/web/src/pages/{Tests.tsx,TestLibraryDetail.tsx}`                                                                                                      | UPDATE        | Entry points and advanced fields kept secondary                                                  |
| Existing Rust/web/worker execution and library tests; new direct/generation tests                                                                           | UPDATE/CREATE | Detailed test matrix below                                                                       |
| `scripts/test_library_smoke.py`, `scripts/direct_authoring_smoke.py`, `scripts/test_ci_scope.py`, `.github/workflows/ci.yaml`, `justfile`                   | UPDATE/CREATE | HTTP acceptance and appropriate explicit checks                                                  |
| `docs/architect/{implementation/06-test-generation.md,implementation/00-master-spec.md,status.md,decisions.md,contracts.md,dependencies.md,environment.md}` | UPDATE        | Source/status reconciliation and explicit runtime/dependency changes                             |

The browser output directory is `apps/web/src/api/generated` (confirmed by `apps/web/openapi-ts.config.ts:13`); no relocation is intended. Shared registration, migrations and generated outputs have one integration owner. No delegation is required by this plan.

## Not building

- Guaranteed discovery of every screen, arbitrary customer APK qualification, iOS or a device cloud.
- A second web UI framework, Redis/Celery, Appium server, or a general agent orchestration platform.
- Automatic paid discovery on upload, automatic approvals/default plans, or automatic AI recovery of broken selectors.
- Free-form scripts, arbitrary XPath, coordinates as reusable selectors, hidden login credentials, or production-environment exploration.
- Pixel-perfect video streaming, automatic self-healing, repository/ticket ingestion or an unlimited template marketplace.
- Migration of historical reports or approved case hashes to new definitions.

## Step-by-step tasks

Validation listed per task describes acceptance coverage to write, not permission to run checks before the complete slice's edit batch.

### Task 1: Define shared actions and compatibility

- **ACTION:** UPDATE execution/session/library contracts; CREATE generation contracts and legacy fixtures.
- **IMPLEMENT:** Tagged TestAction and DirectCommand/DirectTarget; shared bounded step IDs/checkpoints; generation/proposal/template/source types; capability/version and session-revision fields. Retain legacy wire payloads and pure Rust validation. Define required-check and expectation-confirmation rules separately from permissive draft storage.
- **MIRROR:** execution.rs derives and semantic validation; test_library.rs tagged draft variants.
- **IMPORTS:** serde, schemars, utoipa, uuid and existing execution/library types only.
- **GOTCHA:** Serde defaults do not enforce business constraints; test actual deserialize-plus-validate and generated runtime consumers. Do not add optional fields that alter legacy serialized hashes.
- **VALIDATE:** Golden legacy JSON/hash fixtures, invalid unions, bounded values, unsupported selector/checks, direct-only and mixed capability calculations.

### Task 2: Persist command and generation state

- **ACTION:** CREATE migration 000007; UPDATE registry and data services.
- **IMPLEMENT:** Revisioned command admission, ordered step receipts, generation jobs/snapshots/proposals and atomic promotion links. Enforce app/creator/session/build binding, uniqueness and bounded payloads; follow existing reservation locks.
- **MIRROR:** task_sessions migration; test_library mutation receipts and execution_store helpers.
- **IMPORTS:** sea_orm_migration prelude; execution_store, ConnectionTrait, TransactionTrait.
- **GOTCHA:** Never edit applied migrations or keep a transaction open during a model/device call; no blanket cleanup of current local sessions.
- **VALIDATE:** Upgrade a populated schema; replay/concurrent-submit/rollback tests; existing sessions, definitions and approvals survive unchanged.

### Task 3: Add structured command routes and fencing

- **ACTION:** UPDATE task session controllers/services and worker envelopes.
- **IMPLEMENT:** New commands endpoint, generated step outcomes, frozen request fingerprints, monotonic session revision, duplicate ack receipts, bounded private captures and capability-aware claims. Legacy task route remains AI-only. Manual, discovery and sequence commands share one exclusive execution lane.
- **MIRROR:** phones::task, lease/update and Worker/Session controller extractors.
- **IMPORTS:** generated contracts via mobile_qa_contracts; existing worker_auth and ApiFailure.
- **GOTCHA:** Changed payload with reused identity must conflict even when the session is now busy; terminal receipts cannot be rewritten by later updates.
- **VALIDATE:** Real HTTP tests for cross-tenant access, CSRF, stale frame/revision, old worker denial, concurrent tabs, out-of-order updates, cancellation and lease loss.

### Task 4: Implement shared direct device executor

- **ACTION:** CREATE automation modules; UPDATE explicit dependencies and device config.
- **IMPLEMENT:** Owned-serial uiautomator2 adapter, exact selector resolution, literal set_text, bounded waits/swipes, capture receipts and unknown-outcome handling. Device-only initialization must not import SDK/model packages or require provider credentials.
- **MIRROR:** Device.adb argument arrays, device ownership and existing dirty journal.
- **IMPORTS:** generated models, qualification.device.Device, QualificationError; uiautomator2 3.5.0 inside the device seam.
- **GOTCHA:** Changing layout invalidates coordinates; duplicate selectors never choose the first match. Generic Unicode must not be passed through a shell.
- **VALIDATE:** Fake device tests for Unicode/quotes/newlines/empty input, ambiguous/missing/disabled/password targets, bounds, wrong package, timeout-after-side-effect and zero model imports/calls.

### Task 5: Use direct actions in previews and regression runs

- **ACTION:** UPDATE task_sessions.py, execution/actions.py, runner.py, CLI, profile admission and verification integration.
- **IMPLEMENT:** Dispatch each typed action to the shared executor; use Minitap only for explicit navigate/Ask AI. Capture per-step evidence and existing checks; enforce model readiness only for AI actions. Add direct driver support throughout CLI/generated profiles and run admission. Preserve reset, stop, journal and quarantine behavior.
- **MIRROR:** Existing checkpoint evidence/result upload and Rust verification.
- **IMPORTS:** automation.direct, generated TestAction/DirectCommand/results, existing SDK adapter only in AI branch.
- **GOTCHA:** Fix the model requirement in approved execution too; changing only the phone UI does not reduce rerun cost. Do not infer a pass from command completion or filter an ambiguous target solely by the expected value for new direct checks.
- **VALIDATE:** Same direct sequence through phone and approved run; mixed action order; known-good/defective/missing prerequisite reports; no model call without Ask AI; no replay after uncertain interruption.

### Task 6: Simplify step authoring and add control/recording

- **ACTION:** UPDATE PhoneWorkspace, TaskSteps and draft editor integration.
- **IMPLEMENT:** Replace taskGoal transport with generated action union, single draft state owner, Pick/Control mode, recording toggle, type dialog, per-step status and Check affordance. Preserve resizer/accessibility and legacy draft editing. Show Direct/Uses AI labels and missing capabilities only where relevant.
- **MIRROR:** Existing Mantine controls, TanStack mutations, generated checked SDK and workspace-scoped keys.
- **IMPORTS:** generated types/Zod; existing useWorkspace, ErrorNotice and resizable-workspace.
- **GOTCHA:** Picking must never execute; recording appends receipts once; async results must not append into a different app/draft after navigation.
- **VALIDATE:** DOM tests for pick versus tap, recording/reorder, immutable submitted sequence, stale frames, two drafts/tabs, keyboard interaction, reconnect and unsaved edits.

### Task 7: Add atomic save and predefined templates

- **ACTION:** UPDATE library mutations; CREATE template service/catalog and template picker.
- **IMPLEMENT:** Generated internal keys, editable human title, versioned template slot binding and recording promotion in one transaction. Save typed steps/checks/provenance, not a prose navigate action. Templates are usable with AI disabled.
- **MIRROR:** Mutation enum, expected_revision and receipt service; current library creation capabilities.
- **IMPORTS:** test_library/test_generation contracts; uuid, serde, library::capabilities.
- **GOTCHA:** Template defaults must not assert app-specific behavior without binding/confirmation. Replay cannot duplicate a case or overwrite an existing approved one.
- **VALIDATE:** Four catalog templates; unbound-slot readiness; unique keys and editable names; lost-response retry; complete rollback on malformed content; zero model use.

### Task 8: Implement durable generation orchestration

- **ACTION:** CREATE generation service/controller and extend leased worker queue.
- **IMPLEMENT:** Start/status/cancel/artifact routes, idempotent job creation, capability checks, frozen source/context scope, limits and result persistence. Reuse existing session and resource ownership; resume UI polling after reload, not device actions after a crash.
- **MIRROR:** Existing phone session authorization and execution task receipts; run_artifacts private reads.
- **IMPORTS:** test_generation contracts, task_sessions, execution_store and existing artifact storage abstraction.
- **GOTCHA:** Snapshot/model data is tenant private. Reusing a stale build or environment must require a new discovery, not silently bind old selectors.
- **VALIDATE:** Route/OpenAPI agreement, cross-app snapshot denial, active-session conflicts, cancellation races, lease expiry, budget caps and stale reuse tests.

### Task 9: Add bounded discovery and structured AI drafting

- **ACTION:** CREATE authoring worker modules and isolated model CLI seam.
- **IMPLEMENT:** Capture/redact → typed one-action decision → validate → shared direct executor → retain trace loop, then typed scenario drafting. Use generated Pydantic schemas with langchain-openai 1.6.2, explicit configured model, zero SDK retries and bounded repair/usage. Ask AI remains a separate Minitap execution action.
- **MIRROR:** SDK child environment sanitization and usage recorder; generated Pydantic parsing at all worker boundaries.
- **IMPORTS:** generated discovery/proposal types; automation executor; langchain_openai.ChatOpenAI in model child only; Pillow 12.3.0 declared explicitly for image redaction, matching the existing lock.
- **GOTCHA:** Provider structured output may reject some schema constraints: use the generated schema through supported JSON/tool output plus local strict validation, never replace it with a handwritten shape or unconstrained model text. No model-generated code or provider-directed URL fetching.
- **VALIDATE:** Fixed provider replies for invalid union/source/selector, prompt injection, contradiction, unknown usage, timeout, exhausted budgets, redaction failure and cancellation; all ordinary tests deny external model connections.

### Task 10: Add proposal review and atomic acceptance

- **ACTION:** CREATE proposal review UI and generation accept mutation.
- **IMPLEMENT:** Editable names, category, steps, source previews, expected-versus-observed explanation, unbound questions and duplicate suggestions. Batch save selected proposals atomically into drafts; retain generation/item identity and source revisions. Require explicit confirmation of suggested expected behavior before review submission.
- **MIRROR:** Current revision-conflict UI, library error details and transactional receipts.
- **IMPORTS:** generated generation/library DTOs, checked SDK, Mantine form and query helpers.
- **GOTCHA:** Existing approved content cannot be auto-merged; deselecting an item must not save it. Partial model results are reviewable only when their own structure/source references validate.
- **VALIDATE:** Partial output, source mismatch, stale job revision, selected-only acceptance, duplicate names, atomic retry and preserved approval boundaries.

### Task 11: Connect Tests creation and readiness

- **ACTION:** UPDATE Tests and TestLibraryDetail entry points.
- **IMPLEMENT:** Generate with AI / Use a template / Create manually; Smoke default and optional goal. Reuse build/profile when unambiguous, show chooser only if needed. Display explored screens/gaps, progress/cancel, usage and available reuse. Keep suites/plans and advanced review controls secondary.
- **MIRROR:** Existing workspace/app selection, empty states and query remount behavior.
- **IMPORTS:** generation-drawer, template-picker, proposal-review and existing generated options APIs.
- **GOTCHA:** Mounting a page, choosing a preset or uploading an APK must not create paid jobs; only the explicit start action does.
- **VALIDATE:** Empty app/no build/no worker/no model states, manual/template availability without AI, reload recovery, no duplicate starts, accessible compact layout and full keyboard path.

### Task 12: Complete contract exports and regression coverage

- **ACTION:** Finish all fixtures/tests/config, then GENERATE consumers and run the validation batch.
- **IMPLEMENT:** Register exact routes/DTOs in Utoipa and WorkerContracts; update every old field-based TestAction consumer; cover direct-only and mixed fixtures, migration and invalid boundaries. Add CI path selection for new modules and helpers.
- **MIRROR:** scripts/contracts.py staging/change-only writes and actual route operation assertions.
- **IMPORTS:** Existing export registry and generated schema entry points; no consumer model copies.
- **GOTCHA:** Generation occurs after handwritten source/test/config is complete. Resolve generated-union compile errors as one correction batch, rerun only invalidated checks.
- **VALIDATE:** Generated drift zero; full affected Rust, web and worker checks; build and CI scope suite; API error Zod coverage as well as success shapes.

### Task 13: Prove direct execution, templates and AI quality

- **ACTION:** CREATE direct_authoring_smoke.py and execute the acceptance matrix after checks.
- **IMPLEMENT:** HTTP template → structured draft → trial → reviews → plan → direct worker → report with fake device/provider in standard smoke. Separate explicit real-device direct acceptance and bounded real-model discovery evaluation; preserve existing developer processes.
- **MIRROR:** scripts/test_library_smoke.py and documented owned emulator lifecycle.
- **IMPORTS:** Existing smoke helpers and generated protocol consumers; approved local profile supplied as input.
- **GOTCHA:** A synthetic pass is not real-device proof. Do not use alternate browser access around the existing inspection restriction; record rendered acceptance as pending when unavailable.
- **VALIDATE:** Sample text entry/save/restart, Unicode, seeded broken persistence, cancel while acting, missing target and repeated direct run with zero provider calls; AI proposals from good/broken/ambiguous fixtures never redefine a defect as expected.

### Task 14: Reconcile documentation and rollout evidence

- **ACTION:** UPDATE owning specs, dependencies, environment instructions, status and implementation report.
- **IMPLEMENT:** Record actual capabilities, usage, tests and unresolved qualification gates. Explain direct versus Ask AI, template binding and how to start a capable worker. Stage worker/API/web deployment with protocol fencing and additive migrations; keep PR base main. Remove temporary GAN Markdown if a design evaluation is used.
- **MIRROR:** Canonical README authority/status rules and current implementation reports.
- **IMPORTS:** None.
- **GOTCHA:** Do not claim arbitrary customer apps or full-screen coverage from the demo. This plan does not authorize new cloud purchases or global configuration changes.
- **VALIDATE:** Links/status match source and measured evidence; no secrets or private captures in Git; review final diff and dependency audit results before commit/push.

## Testing strategy

| Layer        | Input / scenario                                             | Required result                                                               |
| ------------ | ------------------------------------------------------------ | ----------------------------------------------------------------------------- |
| Contracts    | Legacy persisted case/profile fixtures                       | Identical serialization/content hashes, unchanged historical approvals        |
| Contracts    | Valid direct, mixed and malformed actions                    | Exhaustive generated unions; invalid operation combinations rejected          |
| Rust HTTP/DB | Repeated command, same ID with changed content, two tabs     | One frozen command or explicit conflict; no double device execution           |
| Rust HTTP/DB | Wrong app/creator/worker/token/revocation                    | No task, snapshot or proposal access                                          |
| Rust HTTP/DB | Worker mutation of step content or terminal receipt          | Rejected; stored submitted content wins                                       |
| Worker       | Missing/duplicate/moved/stale targets                        | Fresh unique resolution or blocked; no fuzzy first-match behavior             |
| Worker       | Unicode, quotes, newlines and empty set_text                 | Literal RPC input; no shell interpretation                                    |
| Worker       | Device side effect then timeout/lease loss                   | Uncertain/quarantined; never automatic replay                                 |
| Execution    | Direct-only profile without model key/SDK package            | Real device commands work; model usage exactly zero                           |
| Execution    | Explicit Ask AI between direct steps                         | Only that step reaches Minitap; ordered trace and recorded usage              |
| Checks       | Assertion mismatch, ambiguous evidence, missing prerequisite | Failed / inconclusive or blocked as appropriate; never false green            |
| Templates    | No bindings, partial bindings, complete bindings             | Clear needs-input; ready only after supported expectations are bound          |
| Generation   | Contradictory requirement and observed defective app         | Preserve requirement; flag uncertainty/defect instead of blessing observation |
| Generation   | Invented source/control, malicious app text                  | Rejected references/action escalation; no extra tools or calls                |
| Generation   | Budget exhaustion, no usage, cancel, malformed output        | Bounded exit, retained partial evidence, explicit state                       |
| Generation   | Same context versus changed build/environment                | Tenant-scoped reuse only for identical frozen scope                           |
| Library      | Batch save interrupted or retried                            | All selected drafts once, or full rollback; no automatic reviews/default plan |
| Web DOM      | Pick versus control, recording, reorder and app switch       | Correct step target/order and no accidental execution or cross-draft edits    |
| Web DOM      | Missing AI capability                                        | Manual/template/direct flows remain usable                                    |
| End to end   | Saved generated direct test run twice                        | Second run uses frozen structured case; no discovery/model invocation         |

Include empty/max-size inputs, invalid enums/types, unauthorized requests, network failure and concurrent access at their actual boundaries. Test behavior and side-effect counts, not component implementation details.

## Validation commands

Run only after completing the agreed slice's code/test/config batch. Existing commands below are verified from justfile/package scripts; new smoke command is a planned deliverable.

```bash
just types
just format
just format-check
just check-contracts
just check-api
just check-worker
just check-web
python3 -m unittest scripts.test_ci_scope
just build
just smoke-test-library
python3 scripts/direct_authoring_smoke.py
```

Expected: generated outputs agree, strict checks pass, real HTTP/Postgres smoke retains mutations across restart, simulated boundaries are labeled and no model/device is started by ordinary checks. `just check-api` uses the owned test environment and real migration/route tests; include populated-schema upgrade coverage in that suite rather than resetting the development database.

Before committing, run `pnpm --dir apps/web audit --prod`. Audit the worker environment with `uvx pip-audit --path apps/mobile-worker/.venv/lib/python3.12/site-packages` after installing the agreed extras; exclude the unpublished local project from advisory lookup rather than treating it as a PyPI distribution. Confirm the actual virtualenv Python directory if the interpreter has changed. Record known pre-existing advisories separately; do not silently upgrade unrelated packages. Review `git diff --check` and the actual diff.

Real-device acceptance is explicit and separate: supply a qualified local direct profile, run the sample save/restart sequence through the new HTTP smoke's real-device mode, repeat it with model credentials absent, inspect retained checks and verify clean shutdown. Add documented CLI arguments for that mode in Task 13. Real AI evaluation is an explicitly opted-in mode with the frozen call/time budget; synthetic model outputs are the default. No routine check launches either mode.

### Manual acceptance

- [ ] Tests offers the three creation choices without a stable-key question.
- [ ] A tester selects an input on the phone, types a value, taps Save and records reusable steps.
- [ ] Pick mode selects only; Control mode executes; mode distinction is visible and keyboard usable.
- [ ] A template explains its remaining bindings and produces an editable named draft without AI.
- [ ] Generate with AI starts bounded discovery only on explicit action, then shows named proposals and source screens.
- [ ] Expected behavior can be corrected independently of observed behavior; uncertainty stays visible.
- [ ] Saved tests run through the same structured worker path and reports; direct reruns show zero AI calls.
- [ ] A failed selector prompts repair or explicit Ask AI, with no hidden model invocation.
- [ ] Resize and compact navigation still leave both task and phone visible.
- [ ] Browser inspection follows the existing admin-policy restriction; unavailable rendered acceptance remains an open gate.

## Acceptance and completion checklist

- [ ] All A–C tasks delivered with generated end-to-end contracts; no handwritten wire equivalents.
- [ ] Direct control and recording work on the qualified sample, including literal Unicode text.
- [ ] Typed steps survive save/review/manifest/retry unchanged; historical content hashes survive the upgrade.
- [ ] Direct-only device operation and saved runs require no model key and make zero model calls.
- [ ] All four templates are reusable and show required binding/expectation inputs.
- [ ] AI discovery produces at most the configured proposals with names, categories, actual source references, executable steps or explicit missing bindings.
- [ ] AI-generated expectations are reviewed; no automatic approval or false pass.
- [ ] State transitions, cancellation, tenant access, leases and uncertain recovery are tested.
- [ ] Formatting, type checks, lint, affected tests, builds and generated drift checks pass.
- [ ] Real device and model evaluations are reported separately from synthetic validation.
- [ ] Owning docs and rollout instructions match implementation; secrets/artifacts remain private.
- [ ] No unrelated UI stack, hosting or broad app qualification work is added.

## Risks and mitigations

| Risk                                                  | Likelihood / impact | Mitigation                                                                                            |
| ----------------------------------------------------- | ------------------- | ----------------------------------------------------------------------------------------------------- |
| Existing UI looks direct but saved runs still call AI | High / high         | One shared typed action contract and executor; test both paths and model-call count                   |
| Enum/profile refactor changes historical hashes       | Medium / high       | Golden stored fixtures, legacy serializer compatibility, no data rewrite                              |
| Stale or duplicate controls cause wrong actions       | High / high         | Fresh unique selector resolution; fail visibly; no coordinate/fuzzy fallback                          |
| Discovery assumes observed defects are correct        | High / high         | Requirement provenance, expectation confirmation and seeded-defect evaluation                         |
| Model calls exceed intended cost                      | Medium / medium     | Frozen call/output/time limits, zero implicit retries, usage accounting and snapshot reuse            |
| Preview and run compete for the same emulator         | Medium / high       | Shared reservation namespace, single command lane and capability/lease fencing                        |
| Generic templates appear ready for unsupported apps   | Medium / high       | Explicit slots and qualification checks; sample binding only for matched fixtures                     |
| Long-running discovery loses lease or evidence        | Medium / high       | Per-stage persistence, stop/quarantine on uncertainty, bounded private artifact retention             |
| Large scope slips into cosmetic-only delivery         | High / high         | Three independently verifiable slices; completion requires reusable zero-model reruns plus generation |

## Rollout notes

Deploy compatible workers first without advertising new capability; migrate/API next, then register the new qualified capabilities and deploy UI. Old workers continue only legacy jobs. Before rollback, stop new admissions, drain new jobs and retain the additive schema/data; an old binary must not receive new action variants. Do not drop generation records or rewrite approved cases to make a rollback look compatible.

The currently connected sample app and developer servers are not touched during planning. Implementation must preserve active sessions or explicitly drain them before replacing the worker. This plan changes the product direction of phase 06; it does not claim the revamp is already available.
