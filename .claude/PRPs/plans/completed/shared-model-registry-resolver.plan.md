# Plan: Shared Model Registry and Resolver

## Summary

Replace the free-form model strings currently repeated across execution profiles, private host
profiles, manifests, browser capability checks, and Python provider constructors with one
versioned, nonsecret model registry owned by the Rust API. The API resolves a model once for a
specific purpose, freezes the complete resolved assignment into each run or phone session, and
workers execute that assignment through one provider factory after advertising a compatible
qualified model reference.

This is an architectural cleanup, but it deliberately does **not** introduce a new service
process or a generic AI framework. It follows the repository's existing contract-generation,
immutable-version, operator-import, PostgreSQL service, worker-fencing, and generated-consumer
patterns.

## User Story

As an operator of Mobile QA, I want every profile, run, browser view, and worker to use the same
registered model identity, so that a run cannot be leased to a host configured for a different
model and direct-only work never needs model credentials.

## Problem → Solution

Today `ExecutionProfile.model` is a raw string, a private worker TOML repeats another raw string,
the API only freezes that string, and three Python call sites independently assume OpenAI. That
allowed the registered `gpt-4.1` profile and the host's `no-model-adb-demo` profile to pass through
different setup paths until the worker failed with `host_profile_does_not_match_manifest` after a
run had already been leased.

→ Introduce a versioned `ModelReference`, immutable `ModelDefinition`, frozen `ResolvedModel`,
and explicit capabilities in `crates/contracts`. Persist nonsecret definitions in PostgreSQL;
resolve them through one API service; include the resolved snapshot in manifests/sessions; have
workers advertise the exact qualified reference before lease; and centralize provider-specific
construction and credential allowlisting in one Python runtime module.

## Metadata

- **Complexity**: XL
- **Source PRD**: N/A
- **PRD Phase**: N/A (standalone architectural feature)
- **Estimated Files**: 66 source/test/document files plus 7 generated artifacts
- **Implementation shape**: 5 ordered slices; do not deploy a partially migrated model-enabled
  worker/profile combination

The five slices are: (1) Tasks 1–2, contract/registry foundation; (2) Tasks 3–4, API freeze and
lease fencing; (3) Tasks 5–6, worker/provider and operator setup migration; (4) Tasks 7–8, browser
and end-to-end coverage; (5) Tasks 9–10, generation, validation, documentation, and rollout.

---

## Requirements and Invariants

1. The API is authoritative for model resolution. Browser and worker code consume generated
   model contracts; neither maintains its own catalog.
2. A model definition is nonsecret and immutable at `(key, revision)`. Retirement prevents new
   resolution but never changes a frozen run/session.
3. `ResolvedModel` contains the exact provider, provider model identifier, display label,
   capabilities, and registered reference used for execution. It contains no token, Doppler
   project/config, environment-variable value, or secret locator.
4. New model-enabled profiles reference an exact `ModelReference`; they never request "latest".
   Creating a new registry revision requires a new qualified execution profile before use.
5. The API infers required capabilities from the action being admitted:
   `minitap_navigation` for explicit Navigate/Ask AI actions and `structured_authoring` for AI
   proposal drafting. Direct actions require no model.
6. Run/session creation resolves and freezes the assignment before queueing. Workers do not query
   a mutable catalog during an attempt.
7. A worker advertises the exact model reference qualified on that host. A mismatch prevents a
   lease; it must not acquire the device, boot an emulator, call Doppler, call a provider, or force
   recovery.
8. Provider dispatch is explicit and closed. Initially `open_ai` is the only implemented provider;
   an unsupported provider fails with a safe reason code. There is no automatic provider/model
   fallback.
9. Direct and fake profiles use `None`, not `""`, `"none"`, or `"no-model-adb-demo"`. Direct-only
   browser flows, workers, tests, and smokes make zero model calls and fetch no model secret.
10. Historical manifests and sessions remain readable. The rollout uses dual-read/new-write model
    bindings; no historical run is silently rewritten and old/new model contexts are not comparable.
11. Existing completed and active work in the `codex/react-flow-suite-ui` worktree is preserved.
    Generated files currently have unrelated suite/run UI changes, so generation must occur once
    after source edits and the resulting diff must be reviewed instead of replacing them manually.

---

## UX Design

### Before

```text
Customer chooses execution profile
            |
            v
API freezes profile.model = "gpt-4.1"
            |
            v
Worker reads host model = "no-model-adb-demo"
            |
            v
Run is leased -> child setup fails -> phone requires recovery

Browser AI gating: truthy(profile.model)
Provider dispatch: provider="openai" repeated in multiple Python modules
```

### After

```text
Operator registers ModelDefinition(key, revision, provider, provider_model, capabilities)
                                  |
Operator qualifies ExecutionProfile -> ModelReference(key, revision)
                                  |
Customer chooses the same execution profile (no new click)
                                  |
API model_registry::resolve(purpose) -> frozen ResolvedModel
                                  |
                    +-------------+-------------+
                    |                           |
           RunManifest.resolved_model   PhoneSession.resolved_model
                    |                           |
                    +-------------+-------------+
                                  |
Worker advertises qualified ModelReference before lease
                                  |
                    exact match? -- no --> remain queued / visible setup issue
                                  |
                                 yes
                                  |
model_runtime provider factory -> isolated Doppler child -> provider
```

### Interaction Changes

| Touchpoint                     | Before                                                        | After                                                                   | Notes                                                     |
| ------------------------------ | ------------------------------------------------------------- | ----------------------------------------------------------------------- | --------------------------------------------------------- |
| Test/release profile selection | Customer chooses an operator profile                          | Same selection and defaulting behavior                                  | No added customer click or model picker                   |
| Phone connection               | Any registered worker row can make a profile appear available | Only a currently advertised, model-compatible profile is offered        | Direct profiles remain model-free                         |
| AI controls                    | Enabled with `!!profile.model`                                | Enabled only when the frozen assignment has the required capability     | Ask AI and Generate can be gated independently            |
| Run setup                      | Raw model string is copied with the profile                   | API resolves an active immutable revision and freezes the snapshot      | Resolution failure is a setup blocker, not a worker crash |
| Queued run                     | Mismatch is discovered after lease                            | Incompatible worker receives no lease                                   | No device recovery for configuration mismatch             |
| Run detail                     | Provider-returned string appears only in usage                | Shows registered model label/reference/revision and usage               | Historical runs show a legacy/unknown badge               |
| Operator setup                 | Repeats provider model strings in JSON/TOML/CLI               | Registers one model definition and uses its typed reference in profiles | Secrets remain in Doppler                                 |

---

## Unified Discovery Table

| Category                  | File:Lines                                                                               | Pattern                                                                          | Key Snippet / Finding                                                                  |
| ------------------------- | ---------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| Contract ownership        | `docs/architect/contracts.md:9-24,36-46`                                                 | Rust types generate browser and worker consumers                                 | `crates/contracts` is the only wire-shape source                                       |
| Current model shape       | `crates/contracts/src/execution.rs:228-244`                                              | Immutable profile payload, but model is untyped                                  | `pub model: String`                                                                    |
| Frozen execution          | `crates/contracts/src/execution.rs:254-273`                                              | Manifest owns full execution profile                                             | `pub profile: ExecutionProfile`                                                        |
| Model admission           | `apps/api/src/services/runs.rs:75-95,161-179`                                            | API gathers blockers then freezes manifest                                       | Navigate is currently checked with `profile.model.is_empty()`                          |
| Immutable operator import | `apps/api/src/services/test_definitions.rs:185-215`                                      | Exact replay is idempotent; changed payload conflicts                            | `Profile revisions are immutable; use a new profile ID`                                |
| Data access               | `apps/api/src/services/execution_store.rs:10-58`                                         | Bound SeaORM raw SQL helpers and safe `ApiFailure` conversion                    | `rows`, `one`, `exec`, `json`, `decode`, `conflict`                                    |
| Persistence               | `apps/api/migration/src/m20260912_000004_execution.rs:8-25`                              | Migration owns tables, checks, indexes                                           | Profiles/runs are JSONB; workers bind a `profile_id`                                   |
| Worker fencing            | `apps/api/src/services/scheduler.rs:113-188,225-248`                                     | Validate protocol/profile before `SKIP LOCKED` lease                             | Current claim cannot prove host model compatibility                                    |
| Phone availability        | `apps/api/src/services/task_sessions.rs:68-95`                                           | Options join profiles to non-revoked workers                                     | Current query knows registration, not live host model                                  |
| Phone freeze              | `apps/api/src/services/task_sessions.rs:132-179`                                         | Profile is copied into durable session                                           | Add the resolved model snapshot at this boundary                                       |
| Worker mismatch           | `apps/mobile-worker/src/mobile_qa_worker/execution/actions.py:100-119`                   | Pre-side-effect host check                                                       | Raw `profile.model != manifest.profile.model` produced the current error               |
| Session mismatch          | `apps/mobile-worker/src/mobile_qa_worker/task_sessions.py:233-255`                       | Model/image check precedes host lock and device boot                             | Replace raw-string check with exact reference/capability check                         |
| Host config               | `apps/mobile-worker/src/mobile_qa_worker/qualification/config.py:21-86`                  | Frozen dataclass validates nonsecret private TOML                                | `model: str` plus Doppler scope are currently mixed together                           |
| Provider construction     | `apps/mobile-worker/src/mobile_qa_worker/qualification/sdk_adapter.py:113-157`           | OpenAI/Minitap nodes built inside adapter                                        | `provider="openai", model=profile.model` is duplicated                                 |
| Structured authoring      | `apps/mobile-worker/src/mobile_qa_worker/authoring/model_adapter.py:28-89`               | Isolated child, bounded typed output                                             | Direct `ChatOpenAI(model=profile.model, ...)`                                          |
| Discovery provider        | `apps/mobile-worker/src/mobile_qa_worker/authoring/minitap_discovery.py:138-148,217-227` | Model factory patched for bounded calls                                          | Another OpenAI/provider-model construction site                                        |
| Secret isolation          | `apps/mobile-worker/src/mobile_qa_worker/qualification/sdk_adapter.py:45-67`             | Child environment is cleared and allowlisted                                     | Provider-specific credential lookup belongs in the central runtime                     |
| Browser capability        | `apps/web/src/components/test-library/ai-authoring.tsx:76-85`                            | UI derives readiness from generated session                                      | Current `!!session?.profile.model` is string truthiness                                |
| Browser direct action     | `apps/web/src/components/task-session/phone-workspace.tsx:385-404`                       | Controls are disabled from typed session state                                   | Replace `!s?.profile.model` with capability lookup                                     |
| Browser audit             | `apps/web/src/pages/RunDetail.tsx:44-57,175-181`                                         | UI displays persisted backend facts                                              | Add frozen model identity; preserve legacy usage display                               |
| Error handling            | `apps/api/src/errors.rs:17-47,70-86`                                                     | Safe public errors; internal details logged by reason code                       | Never return provider responses or credential names/values                             |
| Worker errors             | `apps/mobile-worker/src/mobile_qa_worker/qualification/config.py:17-19`                  | Stable snake-case safe reason codes                                              | Continue `QualificationError("...")`                                                   |
| Logging                   | `apps/api/src/services/runs.rs:258-262`                                                  | Structured identifiers and phase                                                 | Log model key/revision/provider, never credentials                                     |
| Route agreement           | `apps/api/tests/execution.rs:445-...` and `apps/api/tests/test_library.rs:273-302`       | OpenAPI operations have real handler tests                                       | Required only if a browser route is added; this plan reuses existing option/run routes |
| API integration tests     | `apps/api/tests/execution.rs:34-117`                                                     | Real PostgreSQL + authenticated HTTP + synthetic worker                          | Extend prepared fixture through registry registration                                  |
| Worker unit tests         | `apps/mobile-worker/tests/test_task_sessions.py:90-142`                                  | Stop at first side-effect boundary with mocks                                    | Assert mismatch never reaches `host_lock` or client/device calls                       |
| Generation workflow       | `Justfile:38-65`                                                                         | One explicit type-generation phase, then batched checks                          | `just types`, `just check-*`, `just check`                                             |
| Architecture constraint   | `docs/architect/implementation/06-test-generation.md:47-78`                              | Direct work is model-free; legacy payloads preserved; protocols fence new fields | New model binding must be additive and version-gated                                   |

---

## Mandatory Reading

Files that MUST be read before implementing:

| Priority | File                                                                     | Lines                     | Why                                                               |
| -------- | ------------------------------------------------------------------------ | ------------------------- | ----------------------------------------------------------------- |
| P0       | `docs/architect/README.md`                                               | all                       | Documentation authority and update rules                          |
| P0       | `docs/architect/contracts.md`                                            | 1-59, 121-156             | Generated transport ownership and legacy behavior                 |
| P0       | `docs/architect/environment.md`                                          | 125-146                   | Doppler/worker isolation and direct-only guarantees               |
| P0       | `docs/architect/implementation/04-execution-and-reports.md`              | 15-65                     | Frozen manifest, leases, usage, and report invariants             |
| P0       | `docs/architect/implementation/06-test-generation.md`                    | 47-100                    | Direct/model boundary, worker protocols, historical compatibility |
| P0       | `crates/contracts/src/execution.rs`                                      | 228-273, 428-444, 595-650 | Profiles, manifests, usage, validation, worker job IPC            |
| P0       | `apps/api/src/services/runs.rs`                                          | 57-180                    | The authoritative run-resolution/freeze boundary                  |
| P0       | `apps/api/src/services/scheduler.rs`                                     | 113-248                   | Lease fencing and protocol gating                                 |
| P0       | `apps/api/src/services/task_sessions.rs`                                 | 68-179, 290-383           | Phone availability, freeze, and claim path                        |
| P0       | `apps/mobile-worker/src/mobile_qa_worker/qualification/config.py`        | 21-86                     | Private host profile validation                                   |
| P0       | `apps/mobile-worker/src/mobile_qa_worker/execution/actions.py`           | 28-119                    | Saved-run model child and current mismatch                        |
| P0       | `apps/mobile-worker/src/mobile_qa_worker/task_sessions.py`               | 160-255                   | Interactive model child and current mismatch                      |
| P1       | `apps/api/src/services/test_definitions.rs`                              | 169-215                   | Immutable execution profile registration                          |
| P1       | `apps/api/src/services/execution_store.rs`                               | all                       | Required DB access/error pattern                                  |
| P1       | `apps/api/src/domain/regression.rs`                                      | 1-89                      | Comparable-context policy must include resolved model             |
| P1       | `apps/mobile-worker/src/mobile_qa_worker/qualification/sdk_adapter.py`   | 14-67, 88-163             | Credential isolation and Minitap provider construction            |
| P1       | `apps/mobile-worker/src/mobile_qa_worker/authoring/model_adapter.py`     | all                       | Structured authoring provider construction                        |
| P1       | `apps/mobile-worker/src/mobile_qa_worker/authoring/minitap_discovery.py` | 110-232                   | Discovery provider factory injection                              |
| P1       | `apps/web/src/components/test-library/ai-authoring.tsx`                  | 55-115                    | Generated-session capability gating                               |
| P1       | `apps/web/src/components/task-session/phone-workspace.tsx`               | 385-404, 480-505          | Direct/AI run and profile selection UX                            |
| P1       | `apps/web/src/pages/RunDetail.tsx`                                       | 44-80, 175-181            | Frozen execution audit display                                    |
| P1       | `apps/api/tests/execution.rs`                                            | 1-125, 445 onward         | Real DB/route/worker integration pattern                          |
| P1       | `apps/mobile-worker/tests/test_task_sessions.py`                         | 90-142                    | No-side-effect mismatch test pattern                              |
| P2       | `scripts/local_device.py`                                                | 35-120                    | Current placeholder/hardcoded local model setup                   |
| P2       | `scripts/task_session_smoke.py`                                          | 101-140                   | Real local profile bootstrap shape                                |
| P2       | `Justfile`                                                               | 38-130                    | Generation, checks, and explicit real-device boundaries           |

`docs/CODEX-NAVIGATION-GUIDE.md` was named by the repository supplement but is not present in
this checkout or the saved-project checkout. The authoritative `docs/architect` sources above and
verified source traces are therefore the navigation basis for this plan.

## External Documentation

No external research needed — this feature uses established internal Rust/SeaORM, generated
OpenAPI/Schemars, React Query, Pydantic, Doppler-child, and Minitap adapter patterns. Provider model
availability is intentionally operator data and must not be inferred from changing vendor docs.

---

## Strategic Design

### Approach

Use a database-backed registry **inside the existing API service**, not a new deployable service:

```rust
// Conceptual source shape; exact derives follow existing contract modules.
pub struct ModelReference {
    pub key: String,
    pub revision: u32,
}

pub enum ModelProvider {
    OpenAi,
}

pub enum ModelCapability {
    MinitapNavigation,
    StructuredAuthoring,
}

pub struct ModelDefinition {
    pub reference: ModelReference,
    pub display_name: String,
    pub provider: ModelProvider,
    pub provider_model: String,
    pub capabilities: Vec<ModelCapability>,
}

pub struct ResolvedModel {
    pub reference: ModelReference,
    pub display_name: String,
    pub provider: ModelProvider,
    pub provider_model: String,
    pub capabilities: Vec<ModelCapability>,
}
```

The API's conceptual `modelToUse(...)` operation is implemented idiomatically as
`model_registry::resolve(db, binding, required_capabilities)`. It is the only place that turns a
profile's requested binding into a provider assignment. Browser code receives summaries and
frozen snapshots from existing endpoints; worker code receives the same `ResolvedModel` through
generated Pydantic models.

For wire compatibility, `ExecutionProfile` keeps JSON key `model` but changes its Rust type to an
optional, untagged `ModelBinding` that accepts either a new `ModelReference` object or a legacy
string. New registrations accept only `None` or `Registered(ModelReference)`. Legacy strings are
read-only compatibility input: the API may map a nonempty historical identifier to one unique
active OpenAI registry definition with the same `provider_model`; it never writes another raw
string. Empty/`none` legacy values are treated as model-free only for historical direct/fake
records.

`RunManifest.resolved_model` and `PhoneSession.resolved_model` are optional and omitted for
direct/fake work and historical records. New model-enabled work must contain a frozen value.
`ModelUsage` keeps its existing provider-reported `model: String` for historical storage and adds
`model_reference: Option<ModelReference>`; no selection logic may use the usage string.

### Clean-Code Boundaries

- **Single responsibility**: contracts describe values; `model_registry.rs` resolves/persists;
  run/session services freeze; `model_runtime.py` dispatches providers; UI only presents facts.
- **Dependency direction**: domain/application code depends on `ModelReference` and capabilities,
  not on `ChatOpenAI`, Minitap provider strings, Doppler, or environment-variable names.
- **Explicit policy**: purpose-to-capability mapping is named functions/enums, not truthy strings.
- **Closed provider switch**: one exhaustive provider factory; adding a provider requires a new
  enum variant, credential allowlist, adapter test, and qualification.
- **No speculative abstraction**: no generic plugin framework, remote registry daemon, pricing
  engine, fallback graph, or customer model marketplace.

### Alternatives Considered

| Alternative                                                  | Rejected Because                                                                                                               |
| ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------ |
| Put `gpt-4.1` in a shared constant                           | Removes spelling drift but cannot version capabilities, retire a model, freeze provider mapping, or audit a run                |
| Let every worker fetch the current registry at execution     | Mutable lookup can change an already queued run and requires workers to gain another network/auth boundary                     |
| Store the provider/model directly in every execution profile | Continues duplication and makes a model update indistinguishable from device qualification                                     |
| Add a standalone model-registry microservice                 | Violates the current small architecture and adds deployment/auth/failure modes without product value                           |
| Let the browser select provider/model                        | Adds customer clicks and leaks operator infrastructure choices; product docs say customers choose profiles, not agent plumbing |
| Auto-fallback to another provider/model                      | Breaks reproducibility, qualification, spend accounting, and the explicit no-hidden-fallback decision                          |
| Rewrite historical JSON to the new shape                     | Weakens auditability and violates the historical-manifest compatibility requirement                                            |

### Scope

- Shared model contract values and legacy/new binding parsing.
- Immutable PostgreSQL registry with active/retired state.
- Trusted operator task actions to register, inspect, and retire definitions.
- API resolution for profile registration, run preview/create, phone options/open, and AI authoring.
- Frozen assignments in runs/sessions and comparison/audit behavior.
- Protocol-5 worker model-capability advertisement before leases.
- One Python provider runtime/factory and conversion of all current model call sites.
- Browser capability gating and run audit display with no added customer selection.
- Synthetic, integration, generated-contract, web, and worker tests.
- Dev/smoke fixture cleanup and canonical architecture/runbook updates.

## NOT Building

- A new network service, Redis cache, or client-side registry.
- Customer-facing model/provider selection or credential entry.
- Provider fallback, automatic model upgrades, or "latest" aliases.
- Pricing, token-cost calculation, quotas, billing, or rate-limit routing.
- New model providers beyond the explicitly implemented OpenAI adapter.
- Storage of API keys, Doppler tokens, Doppler project/config, or secret values in registry rows,
  browser DTOs, manifests, journals, or logs.
- Live phone streaming, cold-start orchestration, general device-fleet scheduling, or fixing the
  separate artifact-root/worktree issue.
- A second job queue or an agent/domain framework.
- Real provider/device calls in ordinary checks or CI.

---

## Patterns to Mirror

### CONTRACT_SOURCE_OF_TRUTH

```rust
// SOURCE: crates/contracts/src/lib.rs:1-20
//! Transport source of truth shared by the API, browser and fixture worker.
//! Keep this crate independent of Loco, SeaORM and device SDKs...
pub mod browser;
pub mod worker;
pub mod execution;
pub mod execution_api;
```

Create `model_registry.rs` here and re-export it. Add reachable schemas to both Utoipa components
and `WorkerContracts`; do not handwrite TypeScript/Python equivalents.

### IMMUTABLE_OPERATOR_REGISTRATION

```rust
// SOURCE: apps/api/src/services/test_definitions.rs:193-215
let value = json(&p)?;
let r = rows(
    &ctx.db,
    "SELECT payload,app_id FROM execution_profiles WHERE id=$1",
    vec![p.id.into()],
).await?;
if let Some(r) = r.first() {
    if field::<Uuid>(r, "app_id")? != app
        || field::<serde_json::Value>(r, "payload")? != value
    {
        return Err(conflict(
            "Profile revisions are immutable; use a new profile ID",
        ));
    }
    return Ok(());
}
```

Registry registration must be idempotent for identical `(key, revision, payload)` and conflict
for different payload. Retirement is separate metadata and does not mutate payload.

### BOUND_DATABASE_ACCESS

```rust
// SOURCE: apps/api/src/services/execution_store.rs:22-54
pub async fn rows(
    db: &impl ConnectionTrait,
    sql: &str,
    args: Vec<Value>,
) -> ApiResult<Vec<QueryResult>> {
    Ok(db.query_all_raw(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        sql,
        args,
    )).await?)
}
```

Keep SQL in the service/migration, use bound values, and never introduce an unreviewed ORM/domain
layer solely for this feature.

### SAFE_ERROR_HANDLING

```rust
// SOURCE: apps/api/src/errors.rs:33-47
pub fn missing() -> Self {
    Self::new(404, "not_found", "Record not found")
}
pub fn invalid(message: impl Into<String>) -> Self {
    Self::new(422, "invalid_input", message)
}
pub fn internal() -> Self {
    Self::new(500, "internal_error", "The operation could not be completed. Try again.")
}
```

Return actionable setup messages to users, stable safe reason codes to workers, and log internal
registry/provider failures without provider payloads or credentials.

### STRUCTURED_LOGGING

```rust
// SOURCE: apps/api/src/services/runs.rs:258-262
tracing::info!(
    run_id=%id,
    app_id=%app,
    phase="queued",
    "Approved execution queued"
);
```

Registry logs use `model_key`, `model_revision`, `provider`, `phase`, and relevant run/session IDs.
Never log `OPENAI_API_KEY`, environment contents, Doppler output, prompt/image content, or provider
response bodies.

### FREEZE_BEFORE_QUEUE

```rust
// SOURCE: apps/api/src/services/runs.rs:161-179
out.manifest = Some(RunManifest {
    app_id: app,
    build_id: build,
    build_sha256: field(&b, "sha256")?,
    environment_revision: field(&env, "revision")?,
    profile,
    cases,
    budget: p.budget.clone(),
    diagnostic_retries: p.diagnostic_retries,
    exclusions: p.exclusions.clone(),
    // ...
});
```

Resolve inside the same application transaction used to create the run/session; copy the full
nonsecret `ResolvedModel` into the durable payload before attempts can be claimed.

### PROTOCOL_AND_LEASE_FENCING

```rust
// SOURCE: apps/api/src/services/scheduler.rs:113-136
if ![1, 2, 3, 4].contains(&input.version) || input.profile_id != worker.profile_id {
    return Err(conflict("Unsupported protocol or worker profile"));
}
// ...
if profile.execution_context.is_some() && input.version < 3 {
    return Ok(ClaimResponse { lease: None, poll_after_seconds: 5 });
}
```

Add protocol 5. Model-enabled assignments require v5 plus an advertised exact reference/provider;
incompatible workers receive no lease. Model-free direct work retains its current compatible
protocol floor unless another new field actually requires v5.

### NO_SIDE_EFFECT_MISMATCH_TEST

```python
# SOURCE: apps/mobile-worker/tests/test_task_sessions.py:128-142
lock = Mock(side_effect=AdmissionReached)
client = Mock()
monkeypatch.setattr(task_sessions, "host_lock", lock)
if accepted:
    with pytest.raises(AdmissionReached):
        task_sessions.run_session(client, lease, tmp_path, tmp_path / "host.toml")
else:
    with pytest.raises(QualificationError, match="^worker_profile_mismatch$"):
        task_sessions.run_session(client, lease, tmp_path, tmp_path / "host.toml")
    lock.assert_not_called()
assert client.mock_calls == []
```

Preserve this proof at both API lease admission and Python side-effect admission.

### ISOLATED_MODEL_CHILD

```python
# SOURCE: apps/mobile-worker/src/mobile_qa_worker/qualification/sdk_adapter.py:45-67
def prepare_environment(directory: Path) -> None:
    if directory != directory.resolve() or (directory / ".env").exists():
        raise QualificationError("unsafe_sdk_directory")
    key = os.environ.get("OPENAI_API_KEY")
    if not key:
        raise QualificationError("model_credentials_unavailable")
    allowed = {k: v for k, v in os.environ.items()
               if k in ("PATH", "LANG", "LC_ALL", "VIRTUAL_ENV", "HOME")}
    # ... clear and replace environment ...
```

Move provider-specific credential selection into `model_runtime.py`, preserving environment
clearing, private cwd, dotenv disablement, telemetry disablement, and process cleanup.

### GENERATED_BROWSER_BOUNDARY

```ts
// SOURCE: apps/web/src/api/test-library.ts:30-39
export const libraryOptionsQuery = (workspace: string, appId: string) =>
  queryOptions({
    queryKey: [...libraryKey(workspace, appId), 'options'],
    enabled: !!appId,
    queryFn: () =>
      checked(
        sdk.getTestLibraryOptions({ ...options, path: { app_id: appId } }),
        z.zLibraryOptionsResponse,
      ),
  })
```

Reuse current endpoints and generated validators. Do not add a handwritten catalog fetch or
duplicate model interfaces in React.

### TEST_STRUCTURE

```rust
// SOURCE: apps/api/tests/execution.rs:34-71
async fn prepared(
    server: &TestServer,
    ctx: &AppContext,
    owner: &Login,
) -> (Uuid, Uuid, Uuid, worker_auth::Worker, String) {
    // Real app/build/database setup, then an explicit immutable profile.
    let profile = ExecutionProfile { /* ... */ };
    defs::register_profile(ctx, owner.user, app.id, profile.clone())
        .await
        .unwrap();
    // ...
}
```

Register a synthetic model definition before a model-enabled profile and keep real provider/device
I/O out of integration tests.

---

## Files to Change

| File                                                                     | Action      | Justification                                                                                                 |
| ------------------------------------------------------------------------ | ----------- | ------------------------------------------------------------------------------------------------------------- |
| `crates/contracts/src/model_registry.rs`                                 | CREATE      | Shared model reference/provider/capability/definition/resolution/worker-capability types and validation       |
| `crates/contracts/src/lib.rs`                                            | UPDATE      | Export the new contract module                                                                                |
| `crates/contracts/src/execution.rs`                                      | UPDATE      | Typed profile binding, frozen manifest assignment, worker claims, usage identity, navigation child assignment |
| `crates/contracts/src/task_sessions.rs`                                  | UPDATE      | Frozen session assignment and protocol-5 host capability advertisement                                        |
| `crates/contracts/src/automation.rs`                                     | UPDATE      | Wrap isolated authoring model request with `ResolvedModel`                                                    |
| `crates/contracts/src/worker/qualification.rs`                           | UPDATE      | Qualification request/result carries registered reference/resolved assignment without secrets                 |
| `crates/contracts/src/worker.rs`                                         | UPDATE      | Make all new worker schemas reachable                                                                         |
| `crates/contracts/src/execution_api.rs`                                  | UPDATE      | Include new schemas in OpenAPI components                                                                     |
| `crates/contracts/tests/model_registry.rs`                               | CREATE      | Pure validation/serde/legacy compatibility tests                                                              |
| `crates/contracts/tests/execution.rs`                                    | UPDATE      | Profile validation and manifest serialization cases                                                           |
| `apps/api/migration/src/m20260921_000010_model_registry.rs`              | CREATE      | Immutable registry plus worker advertised-capability/last-seen columns and indexes                            |
| `apps/api/migration/src/lib.rs`                                          | UPDATE      | Register migration above scaffold marker                                                                      |
| `apps/api/src/services/model_registry.rs`                                | CREATE      | Registration, retirement, lookup, legacy resolution, capability checking, summaries                           |
| `apps/api/src/services/mod.rs`                                           | UPDATE      | Export model registry service                                                                                 |
| `apps/api/src/services/test_definitions.rs`                              | UPDATE      | Resolve/validate model reference during new execution-profile registration                                    |
| `apps/api/src/services/runs.rs`                                          | UPDATE      | Infer required run capability and freeze `ResolvedModel`                                                      |
| `apps/api/src/services/case_runs.rs`                                     | UPDATE      | Ensure transient saved-case runs use common resolved path                                                     |
| `apps/api/src/services/suite_runs.rs`                                    | UPDATE      | Ensure transient suite runs use common resolved path                                                          |
| `apps/api/src/services/task_sessions.rs`                                 | UPDATE      | Resolve session model, persist worker capability heartbeat, filter incompatible phone choices                 |
| `apps/api/src/services/test_authoring.rs`                                | UPDATE      | Gate Ask AI/generation by explicit frozen capabilities                                                        |
| `apps/api/src/services/scheduler.rs`                                     | UPDATE      | Protocol-5 exact model compatibility before attempt reservation/lease                                         |
| `apps/api/src/services/test_library.rs`                                  | UPDATE      | Return model capability/availability summary with profile choices                                             |
| `apps/api/src/domain/regression.rs`                                      | UPDATE      | Include resolved model identity in comparability policy                                                       |
| `apps/api/src/tasks/execution.rs`                                        | UPDATE      | Add trusted register/show/retire model actions; keep files nonsecret                                          |
| `apps/api/tests/model_registry.rs`                                       | CREATE      | Real PostgreSQL registry immutability/resolution/retirement tests                                             |
| `apps/api/tests/execution.rs`                                            | UPDATE      | Frozen assignment, protocol fencing, direct-zero-model, comparison integration                                |
| `apps/api/tests/task_sessions.rs`                                        | UPDATE      | Resolved session and worker capability admission                                                              |
| `apps/api/tests/test_library.rs`                                         | UPDATE      | Generated profile choice capability summary                                                                   |
| `apps/mobile-worker/src/mobile_qa_worker/model_runtime.py`               | CREATE      | Provider switch, exact model construction, credential allowlist, host-assignment checks, usage identity       |
| `apps/mobile-worker/src/mobile_qa_worker/qualification/config.py`        | UPDATE      | Replace raw model string with typed optional qualified reference; retain Doppler scope separately             |
| `apps/mobile-worker/src/mobile_qa_worker/qualification/sdk_adapter.py`   | UPDATE      | Consume resolved assignment and central provider factory                                                      |
| `apps/mobile-worker/src/mobile_qa_worker/qualification/runner.py`        | UPDATE      | Carry resolved assignment through qualification child                                                         |
| `apps/mobile-worker/src/mobile_qa_worker/qualification/local.py`         | UPDATE      | Remove `no-model-adb-demo`; explicit agent uses a resolved registry document                                  |
| `apps/mobile-worker/src/mobile_qa_worker/execution/actions.py`           | UPDATE      | Remove raw model comparison; pass frozen model to child only for AI actions                                   |
| `apps/mobile-worker/src/mobile_qa_worker/execution/sdk_adapter.py`       | UPDATE      | Execute exact resolved provider model instead of reading host string                                          |
| `apps/mobile-worker/src/mobile_qa_worker/execution/runner.py`            | UPDATE      | Advertise host model capability in protocol 5 and preserve pre-side-effect failure                            |
| `apps/mobile-worker/src/mobile_qa_worker/task_sessions.py`               | UPDATE      | Advertise/validate exact model ref and pass frozen session assignment to AI children                          |
| `apps/mobile-worker/src/mobile_qa_worker/automation/session.py`          | UPDATE      | Gate Navigate by capability, not host string truthiness                                                       |
| `apps/mobile-worker/src/mobile_qa_worker/authoring/generation.py`        | UPDATE      | Put resolved assignment in isolated request and invoke only when capability exists                            |
| `apps/mobile-worker/src/mobile_qa_worker/authoring/model_adapter.py`     | UPDATE      | Use central factory; no direct `ChatOpenAI(profile.model)`                                                    |
| `apps/mobile-worker/src/mobile_qa_worker/authoring/minitap_discovery.py` | UPDATE      | Use central Minitap model construction; remove repeated provider literals                                     |
| `apps/mobile-worker/src/mobile_qa_worker/cli.py`                         | UPDATE      | Replace raw `--model` with typed nonsecret resolved-model input for explicit qualification only               |
| `apps/mobile-worker/tests/test_model_runtime.py`                         | CREATE      | Provider/capability/credential isolation and unsupported-provider tests                                       |
| `apps/mobile-worker/tests/test_task_sessions.py`                         | UPDATE      | Exact-ref mismatch before side effects and model-free direct cases                                            |
| `apps/mobile-worker/tests/test_execution.py`                             | UPDATE      | Frozen child assignment and no-provider direct behavior                                                       |
| `apps/mobile-worker/tests/test_qualification.py`                         | UPDATE      | New TOML/reference validation and qualification assignment                                                    |
| `apps/mobile-worker/tests/test_sdk_adapter.py`                           | UPDATE      | Central provider factory/usage reference tests                                                                |
| `apps/web/src/components/test-library/ai-authoring.tsx`                  | UPDATE      | Structured-authoring capability gating                                                                        |
| `apps/web/src/components/task-session/phone-workspace.tsx`               | UPDATE      | Navigate capability gating and compatible profile labels                                                      |
| `apps/web/src/components/test-library/membership-fields.tsx`             | UPDATE      | Show profile AI/direct capability summary without a model picker                                              |
| `apps/web/src/pages/RunDetail.tsx`                                       | UPDATE      | Display frozen model label/reference/revision and legacy state                                                |
| `apps/web/src/pages/TaskSession.test.tsx`                                | UPDATE      | Generated resolved-model fixture/capability behavior                                                          |
| `apps/web/src/components/test-library/ai-authoring.test.tsx`             | UPDATE      | Capability-specific generation states                                                                         |
| `apps/web/src/pages/RunDetail.test.tsx`                                  | UPDATE      | Audit display and direct/historical fallback                                                                  |
| `scripts/local_device.py`                                                | UPDATE      | Stop generating placeholder model strings; accept registry export for explicit agent run                      |
| `scripts/task_session_smoke.py`                                          | UPDATE      | Register definition/reference and use protocol 5 for model smoke                                              |
| `scripts/execution_smoke.py`                                             | UPDATE      | Use `None` for fake model and assert zero model requirements                                                  |
| `scripts/regression_acceptance.py`                                       | UPDATE      | Use `None` for direct profile                                                                                 |
| `contracts/fixtures/model-registry/synthetic-openai.json`                | CREATE      | Nonsecret synthetic registry fixture for checks; not a production default                                     |
| `docs/architect/decisions.md`                                            | UPDATE      | Record authoritative resolution/freeze/no-fallback decision                                                   |
| `docs/architect/contracts.md`                                            | UPDATE      | Document model binding, frozen assignment, legacy-read/new-write policy, protocol 5                           |
| `docs/architect/environment.md`                                          | UPDATE      | Separate registry data, host qualification reference, and Doppler credential scope                            |
| `docs/architect/implementation/04-execution-and-reports.md`              | UPDATE      | Require resolved assignment in new model-enabled manifests and comparison/report behavior                     |
| `docs/architect/implementation/06-test-generation.md`                    | UPDATE      | Provider factory, worker capability advertisement, AI capability gates                                        |
| `docs/architect/development.md`                                          | UPDATE      | Operator registration/export/profile rollout and recovery-free mismatch troubleshooting                       |
| `docs/architect/status.md`                                               | UPDATE LAST | Record only checks/acceptance actually observed after implementation                                          |
| `contracts/browser.openapi.json`                                         | REGENERATE  | Derived browser contract                                                                                      |
| `contracts/worker.schema.json`                                           | REGENERATE  | Derived worker contract                                                                                       |
| `apps/web/src/api/generated/{index,sdk.gen,types.gen,zod.gen}.ts`        | REGENERATE  | Derived browser SDK/types/validators                                                                          |
| `apps/mobile-worker/src/mobile_qa_worker/generated/models.py`            | REGENERATE  | Derived Pydantic models                                                                                       |

No dependency or lockfile change is expected. If implementation appears to require a new package,
stop and reassess the design before adding it.

---

## Step-by-Step Tasks

### Task 1: Record the architectural decision and contract vocabulary

- **ACTION**: Define the canonical model concepts and their invariants before changing consumers.
- **IMPLEMENT**:
  - Add `crates/contracts/src/model_registry.rs` with `ModelReference`, `ModelProvider`,
    `ModelCapability`, `ModelDefinition`, `ResolvedModel`, `ModelBinding`, and
    `WorkerModelCapabilities`.
  - Use snake-case serde enums, `deny_unknown_fields`, bounded validation, nonzero revision,
    canonical lower-case key syntax (for example `^[a-z0-9][a-z0-9._-]{0,99}$`), bounded unique
    capabilities, and bounded provider/display identifiers.
  - Use `#[serde(untagged)]` only for `ModelBinding::{Registered(ModelReference), Legacy(String)}`
    to read historical `model: "..."` values. Keep the compatibility variant out of all new
    writes through API validation.
  - Change `ExecutionProfile.model` to `Option<ModelBinding>` while preserving JSON key `model`;
    add helpers `registered_model()`, `uses_legacy_model()`, and `requires_model()` so no caller
    pattern-matches raw strings.
  - Add optional `resolved_model` to `RunManifest` and `PhoneSession`; add the resolved model to
    navigation/authoring child envelopes; add optional `model_reference` to usage records.
  - Add optional protocol-5 worker capability data to both claim requests with serde defaults so
    old payloads still parse.
  - Write canonical docs/decision text in the same change, marking the feature planned until
    validation is complete.
- **MIRROR**: `CONTRACT_SOURCE_OF_TRUTH`; additive optional compatibility in
  `crates/contracts/src/execution_lifecycle.rs:9-31` and `task_sessions.rs:91-105`.
- **IMPORTS**: `serde::{Serialize, Deserialize}`, `schemars::JsonSchema`, `utoipa::ToSchema`,
  existing `bounded`, and standard collections only.
- **GOTCHA**:
  - Do not make `provider_model` or display label the stable identity; only `(key, revision)` is.
  - Do not put timeout/token limits in the registry; those remain purpose-specific bounded policy.
  - Utoipa and Schemars must emit the same optional/union semantics. Add the types to both export
    roots before generating.
  - Historical `model: ""` must round-trip; new direct/fake profiles serialize `None`.
- **VALIDATE**: `cargo test -p mobile-qa-contracts --test model_registry --test execution`; inspect
  schema output after Task 9, not by hand in this task.

### Task 2: Persist an immutable nonsecret registry and operator workflow

- **ACTION**: Add one authoritative registry store and trusted import/retire operations.
- **IMPLEMENT**:
  - Migration creates `model_definitions(key TEXT, revision INTEGER, payload JSONB,
created_at TIMESTAMPTZ, retired_at TIMESTAMPTZ NULL, PRIMARY KEY(key,revision))` with revision,
    key, and timestamp checks/indexes.
  - Add nullable `model_capabilities JSONB` and `model_last_seen_at TIMESTAMPTZ` to
    `execution_workers`; these store only nonsecret advertised facts.
  - `model_registry::register` validates the definition, inserts an immutable revision,
    accepts exact replay, and conflicts on different payload.
  - `retire` only sets `retired_at` idempotently; `resolve` can read retired definitions for audit
    but `resolve_for_new_work` rejects them.
  - `resolve_for_new_work` accepts a `ModelBinding` plus a set of required capabilities, returns
    one `ResolvedModel`, and distinguishes unknown, retired, ambiguous legacy, and
    missing-capability errors.
  - Legacy lookup exists in this module only: a nonempty historical string maps to exactly one
    active `open_ai` definition whose `provider_model` is equal. An ambiguous/missing mapping is a
    blocker. Empty/`none` compatibility values are model-free only for historical direct/fake data.
  - Extend the trusted `execution` task with `register-model`, `show-model`, and `retire-model`.
    Imports are bounded JSON files with no secrets. `show-model` prints safe JSON for qualification
    setup; never print environment/credentials.
- **MIRROR**: `IMMUTABLE_OPERATOR_REGISTRATION`, `BOUND_DATABASE_ACCESS`, migration registry marker
  in `apps/api/migration/src/lib.rs:17-29`, task parser in `apps/api/src/tasks/execution.rs:31-67`.
- **IMPORTS**: `mobile_qa_contracts::model_registry::*`, `sea_orm::ConnectionTrait`, helpers from
  `execution_store`, `chrono::Utc` only if needed for response projections.
- **GOTCHA**:
  - Registry retirement must not cascade to profiles/runs or alter payload JSON.
  - Do not auto-seed a production model in the migration. Tests register a synthetic fixture;
    operators explicitly register approved deployment models.
  - A registry row is global deployment configuration. The trusted local/server task is the
    administrative boundary; do not expose a customer mutation route in this slice.
  - Never accept credential env names or Doppler scope in `ModelDefinition`.
- **VALIDATE**: New real-PostgreSQL tests prove identical replay, conflict on mutation, exact
  revision lookup, retired rejection for new work, historical lookup, and no secret-shaped fields
  in serialized definitions.

### Task 3: Resolve profiles and freeze assignments at API boundaries

- **ACTION**: Route all new model decisions through `model_registry::resolve_for_new_work`.
- **IMPLEMENT**:
  - `register_profile` accepts only `None` or `Registered(ModelReference)` for new profiles,
    verifies the exact active registry entry, and requires `minitap_navigation` for a Minitap
    profile. `android_direct_v1` and fake profiles require `None`.
  - Keep `ExecutionProfile::validate` pure: validate syntax/driver invariants there; perform DB
    existence/status/capability validation in the service.
  - In `runs::assemble`, infer required capabilities from the resolved cases. Direct-only cases
    produce `resolved_model=None`; any Navigate action requires `minitap_navigation`. Resolve before
    setting `out.manifest` and add an actionable blocker instead of queueing if resolution fails.
  - Case/suite/release routes continue using the common `runs::assemble` path; do not duplicate
    resolution in the new suite service.
  - In `task_sessions::open`, resolve again inside the durable creation transaction and freeze the
    assignment. Do not trust an earlier browser options response.
  - In AI command/generation services, require the correct capability from the already frozen
    session model: Navigate needs `minitap_navigation`; Generate needs both navigation and
    `structured_authoring`.
  - `test_library::options` and `task_sessions::options` project safe model label/capabilities and
    compatibility state from the registry. They do not expose credentials or ask the customer to
    select a model.
  - Preserve legacy completed payload reading. A new model-enabled run/session using a legacy
    profile succeeds only after the matching definition is registered; the new manifest/session
    still writes a full resolved snapshot.
- **MIRROR**: `FREEZE_BEFORE_QUEUE`, current common assembly in `runs.rs:57-180`, current phone
  durable payload in `task_sessions.rs:164-179`.
- **IMPORTS**: `super::model_registry`, generated model enums/types; no provider SDK imports in API.
- **GOTCHA**:
  - Preview can be stale; create/open must resolve in their own transaction before persistence.
  - Retiring a definition after a run/session is created cannot invalidate its frozen assignment.
  - Do not use `ModelUsage.model` as resolution input.
  - Direct-only runs must not be blocked merely because the registry is empty.
- **VALIDATE**: API integration tests cover direct/no-registry, known model, unknown revision,
  retired model, missing capability, legacy unique mapping, frozen snapshot after retirement, and
  no new attempt/session row when setup is blocked.

### Task 4: Advertise host compatibility and fence protocol 5 before leases

- **ACTION**: Make the worker tell the API which qualified model reference it can execute before
  the API reserves or leases a device.
- **IMPLEMENT**:
  - Replace `Profile.model: str` with `model_ref: ModelReference | None`, parsed with the generated
    Pydantic type from a TOML table. Keep `doppler_project`/`doppler_config` as separate host secret
    scope metadata.
  - Execution and phone workers build `WorkerModelCapabilities` from the validated host profile
    and compiled provider adapter support, then include it in protocol-5 claim requests.
  - API claim services validate exact reference equality and provider support against the frozen
    `ResolvedModel` before advisory lock, reservation insert, attempt/session state mutation, or
    lease-token issuance.
  - Persist the safe capability snapshot and `last_seen_at` on each claim poll. Phone options only
    present a model-enabled worker as connected when its recent advertisement matches the profile's
    resolvable reference. A model-free direct profile remains eligible without model capabilities.
  - For incompatible model workers, return no lease with a stable logged reason (or a safe 409 for
    malformed claims); leave the job queued. Do not convert it to `recovery_required`.
  - Protocol 5 is mandatory only for model-enabled frozen assignments. Historical/model-free paths
    retain the lowest currently valid protocol that understands their other fields.
- **MIRROR**: `PROTOCOL_AND_LEASE_FENCING`, resource reservation order in
  `scheduler.rs:164-224`, phone reservation order in `task_sessions.rs:306-382`.
- **IMPORTS**: Shared `WorkerModelCapabilities`, `ModelReference`, `ModelProvider`; existing
  `Profile`, `ClaimRequest`, and `PhoneClaimRequest`.
- **GOTCHA**:
  - Never trust provider support advertised by a worker as proof of model qualification; require
    both compiled-provider support and exact host `model_ref` equality.
  - Update last-seen/capabilities independently of leasing so readiness can recover when a correct
    worker connects.
  - Avoid log spam on every 30-second incompatible poll; log on changed capability state or use a
    bounded structured warning.
  - The API still owns assignment; the worker advertisement is an admission constraint, not a
    selection request.
- **VALIDATE**: Competing/mismatch claim tests prove no reservation, lease, state change, host lock,
  Doppler subprocess, or recovery flag; compatible v5 receives the exact frozen assignment;
  direct protocol compatibility remains green.

### Task 5: Centralize Python provider runtime and remove scattered model hardcoding

- **ACTION**: Add one provider adapter/factory and convert every current model call site.
- **IMPLEMENT**:
  - Create `model_runtime.py` with small cohesive functions:
    - `require_capability(resolved, capability)`;
    - `require_host_assignment(resolved, host.model_ref)`;
    - `prepare_provider_environment(resolved, directory)`;
    - `structured_model(resolved, policy, callbacks=...)`;
    - `minitap_model(resolved, callbacks=...)` / Minitap `LLMWithFallback` construction;
    - usage metadata that includes `resolved.reference`.
  - Keep one exhaustive `ModelProvider.open_ai` branch. This is the only owned location that knows
    `OPENAI_API_KEY`, `ChatOpenAI`, and Minitap's `provider="openai"` spelling.
  - The parent worker passes the frozen resolved assignment in `NavigationRequest` or the authoring
    envelope. Children must not reload provider/model identity from host TOML.
  - Convert saved-run Navigate, interactive Ask AI, Minitap discovery, structured proposal drafting,
    and phase-02 qualification to use the factory.
  - Preserve purpose-specific call policies at their current owners (timeouts, retry=0, completion
    limits, callbacks); pass those explicitly to the factory rather than hiding them in the registry.
  - Direct action paths do not import provider libraries and do not invoke the Doppler wrapper.
  - Replace generic raw mismatch errors with stable `worker_model_reference_mismatch`,
    `model_capability_unavailable`, `model_provider_unsupported`, and
    `model_credentials_unavailable`, keeping provider details out of public messages.
- **MIRROR**: `ISOLATED_MODEL_CHILD`, `NO_SIDE_EFFECT_MISMATCH_TEST`, current supervised subprocess
  cleanup in `execution/actions.py:51-98` and `authoring/generation.py:64-106`.
- **IMPORTS**: Generated `ResolvedModel`, `ModelCapability`, `ModelProvider`, `ModelReference`;
  provider packages only inside lazy factory branches.
- **GOTCHA**:
  - Do not import `langchain_openai`/Minitap on direct-only startup or ordinary tests.
  - The Minitap SDK requires provider string `openai`; keep that translation inside the factory.
  - The child environment must be cleared before provider import. Preserve telemetry false,
    dotenv disablement, private cwd, file-size bound, and no worker/lease/API token inheritance.
  - `LLMWithFallback` currently points twice at the same model; preserve behavior but do not market
    it as a fallback. No registry fallback is introduced.
- **VALIDATE**: Unit tests use fake constructors and blocked sockets to assert exact provider model,
  missing credential, unsupported provider, capability mismatch, usage reference, and direct-path
  zero imports/calls. Existing synthetic SDK graph tests stay green.

### Task 6: Remove placeholder/raw-model setup from scripts and fixtures

- **ACTION**: Port operator/dev flows to registry references without touching secrets or private
  config automatically.
- **IMPLEMENT**:
  - `scripts/local_device.py setup` creates a direct/model-free profile by omitting `model_ref`; it
    never writes `no-model-adb-demo`.
  - Replace `device-local-agent --model <provider-id>` with an explicit path to a safe
    `ResolvedModel` document produced by `execution show-model`; validate it with generated
    Pydantic before any device/model operation.
  - Model-enabled private TOML uses a typed `[model_ref] key=... revision=...` table. Do not rewrite
    a user's existing private profile automatically; fail with a migration message.
  - Update model-enabled smoke setup to register a model definition first and reference it from the
    execution profile. Update fake/direct fixtures to use `None`.
  - Keep the tracked model fixture synthetic/nonsecret. Real `gpt-4.1` approval remains operator
    input and current qualification evidence in private state/docs, not an auto-seeded code value.
  - Add a rollout check to dev startup that validates host TOML syntax and reports a missing/legacy
    `model_ref` before launching workers; it must not fetch a secret or modify Doppler config.
- **MIRROR**: Existing `Profile.load` fail-before-side-effect validation and explicit file bounds in
  `apps/api/src/tasks/execution.rs:23-29`.
- **IMPORTS**: Python stdlib plus generated Pydantic models; no new dependency.
- **GOTCHA**:
  - Runtime `.private` files are user/operator state. Document exact manual migration but do not
    edit, delete, or commit them.
  - Do not create `.env`/examples or print a Doppler value.
  - `just dev` remains allowed to start model-capable workers, but AI provider calls stay explicit
    UI actions.
- **VALIDATE**: Script unit tests cover new model-free output, rejected legacy private profile,
  resolved-document validation, and no Doppler invocation for setup/direct runs.

### Task 7: Port the browser to resolved capabilities with zero extra customer clicks

- **ACTION**: Stop reading raw profile model strings and present backend-resolved facts.
- **IMPLEMENT**:
  - Extend generated profile choices/session/run shapes; do not create local interfaces.
  - Add a small pure helper (in the nearest existing component/lib module, not a framework) that
    checks whether `ResolvedModel.capabilities` contains a generated enum value.
  - `ai-authoring.tsx` requires `structured_authoring`; Ask AI/Navigate controls require
    `minitap_navigation`; direct controls remain available with no model.
  - Profile selectors keep the same defaulting and number of clicks. Labels may add `AI enabled`,
    `Direct only`, or `Setup needs attention`, never a provider/model picker.
  - Run detail shows `display_name`, `key@revision`, and provider model from the frozen manifest;
    historical `resolved_model=None` with legacy profile data displays `Legacy model context
unavailable` rather than guessing.
  - Usage shows `model_reference` when present and falls back to existing provider-reported string
    only for historical records.
- **MIRROR**: `GENERATED_BROWSER_BOUNDARY`, backend-fact display comment in
  `apps/web/src/components/runs/run-result.tsx:18-21`, existing Mantine badges/alerts.
- **IMPORTS**: Generated types only, Mantine existing components, no package additions.
- **GOTCHA**:
  - Never derive authorization, provider support, or worker readiness in the browser.
  - Do not expose Doppler scope or credential names.
  - The current React Flow suite work is unrelated; preserve its uncommitted component/style/test
    changes while regenerating shared API artifacts.
- **VALIDATE**: Vitest covers direct-only, navigation-only, authoring-capable, incompatible worker,
  historical run, and exact audit label states; ensure no new model-selection interaction appears.

### Task 8: Complete end-to-end compatibility and regression tests

- **ACTION**: Prove the registry is authoritative across all boundaries and that failure occurs
  before side effects.
- **IMPLEMENT**:
  - Contract tests: validation bounds, serde new/legacy/null, generated schema reachability,
    duplicate capabilities, provider enum exhaustiveness.
  - Migration/service tests: immutable replay/conflict, retirement, exact revision, legacy unique/
    ambiguous mapping, capability failure, safe serialization.
  - API tests: profile registration, run/session snapshot, protocol-5 claims, worker last-seen,
    no lease on mismatch, direct with empty registry, comparison model revision difference.
  - Worker tests: exact reference admission, central provider construction, child request assignment,
    credential isolation, direct no provider import/call, usage reference.
  - Web tests: capability gating and frozen audit display.
  - Update test builders/fixtures everywhere to use a shared synthetic registry helper instead of
    scattering provider model strings.
- **MIRROR**: `TEST_STRUCTURE` and `NO_SIDE_EFFECT_MISMATCH_TEST`.
- **IMPORTS**: Existing test support modules, `unittest.mock`/pytest monkeypatch, React Testing
  Library, no live provider/device clients.
- **GOTCHA**:
  - A test asserting only the final error is insufficient; assert DB/device/subprocess side effects
    did not occur.
  - Fake and direct tests must work with no registry row and no Doppler environment.
  - Historical and new runs must be explicitly not comparable when model context differs.
- **VALIDATE**: Run targeted suites in Task 10, then the full repository checks once.

### Task 9: Regenerate contracts once and review every derived change

- **ACTION**: Generate TS/Zod/OpenAPI/JSON Schema/Pydantic only after all source shapes are final.
- **IMPLEMENT**:
  - Run `just types` once.
  - Confirm `ModelReference`, `ResolvedModel`, capabilities, optional legacy binding, claim
    advertisement, session/run assignment, and authoring/navigation envelopes appear in both
    browser and worker outputs.
  - Confirm generated Zod validators are used at existing API boundaries.
  - Review generated diffs alongside the already modified sequence/suite files; do not hand-edit
    generated output or discard unrelated changes.
- **MIRROR**: `docs/architect/contracts.md:11-16,38-46,55-59` and `Justfile:38-43`.
- **IMPORTS**: N/A.
- **GOTCHA**: Generated content changes only are expected; an unexpected dependency or unrelated
  schema deletion indicates an export-root mistake.
- **VALIDATE**: `just check-contracts` and `git diff --check`.

### Task 10: Validate, document observed status, and prepare rollout

- **ACTION**: Perform batched verification and update status only with observed evidence.
- **IMPLEMENT**:
  - Finish all code/test/config/docs edits before checks, per repository instruction.
  - Run targeted contract/API/worker/web tests; batch fixes; rerun only invalidated checks; then run
    full `just check` and `just build` once.
  - Run model-free smokes (`smoke-execution`, `smoke-test-library`, `smoke-direct-authoring`) and
    verify no Doppler/model/device call.
  - Do not run real model/device acceptance automatically. If explicitly performed, use a private
    registered model file and record exact evidence without credential/provider payloads.
  - Add rollout runbook:
    1. inspect counts of active legacy sessions/runs and drain/cancel safely;
    2. apply migration;
    3. register approved model definition;
    4. update private host TOML to exact model reference;
    5. restart API/workers (only Vite has HMR);
    6. verify protocol-5 capability heartbeat before starting model work;
    7. verify mismatch remains queued and does not enter recovery;
    8. rollback by retiring the new definition and restoring old binaries only after active v5 work
       is terminal—never replay a possibly completed device action.
  - Update `status.md` last with exact test counts and whether rendered/real-device/real-model
    acceptance did or did not run.
- **MIRROR**: Validation ordering in `AGENTS.md`, status evidence language in
  `docs/architect/status.md`, and restart guidance in `docs/architect/implementation/06-test-generation.md`.
- **IMPORTS**: N/A.
- **GOTCHA**:
  - Do not run formatting/type generation mid-edit.
  - Do not mutate user Doppler configuration or private profile automatically.
  - Do not claim the current `gpt-4.1` host is requalified merely because a registry row exists.
- **VALIDATE**: All commands and manual checks below; inspect final `git diff` for secrets,
  generated drift, unrelated suite UI loss, and temporary GAN artifacts.

---

## Testing Strategy

### Unit and Integration Tests

| Test                             | Input                                                  | Expected Output                                                   | Edge Case? |
| -------------------------------- | ------------------------------------------------------ | ----------------------------------------------------------------- | ---------- |
| Model reference validation       | canonical/noncanonical keys, revision 0/max            | Only canonical nonzero references accepted                        | Yes        |
| Definition validation            | duplicate/empty capabilities, long labels/provider IDs | Stable validation error                                           | Yes        |
| Registry idempotency             | same key/revision + same/different payload             | replay succeeds; mutation conflicts                               | Yes        |
| Registry retirement              | active then retired reference                          | historical read succeeds; new resolution blocked                  | Yes        |
| Legacy mapping                   | raw `gpt-4.1` with 0/1/2 matching definitions          | missing/unique/ambiguous result                                   | Yes        |
| Direct without registry          | direct/fake profile with `None`                        | profile/run/session accepted; zero resolved model                 | Core       |
| Model profile registration       | exact ref with/without navigation capability           | accepted / rejected                                               | Core       |
| Run freeze                       | registered ref, then retire registry row               | persisted run retains exact resolved snapshot                     | Core       |
| Phone freeze                     | compatible profile and worker                          | session contains exact snapshot                                   | Core       |
| Capability-specific UI           | navigation only vs structured authoring                | Ask AI and Generate gated independently                           | Core       |
| Protocol-5 claim                 | exact/missing/different host ref                       | lease only for exact compatible ref                               | Core       |
| Mismatch side effects            | wrong host ref                                         | no reservation, host lock, boot, Doppler, provider call, recovery | Critical   |
| Provider factory                 | OpenAI assignment + fake constructor                   | exact `provider_model` passed once                                | Core       |
| Unsupported provider             | future/invalid provider fixture                        | stable fail-closed error, no network                              | Yes        |
| Credential isolation             | missing key / unrelated injected vars                  | stable unavailable; unrelated vars removed                        | Critical   |
| Direct runtime                   | direct action under model-free host                    | no provider module import or Doppler child                        | Critical   |
| Usage identity                   | provider response with/without token metadata          | registered ref retained; unknown counters honest                  | Yes        |
| Comparison                       | same test/context but different model revision         | `not_comparable`                                                  | Core       |
| Historical report                | old manifest without resolved model                    | loads and shows legacy context, no guess                          | Yes        |
| Concurrent resolution/retirement | transaction resolves while retirement occurs           | persisted snapshot is internally complete; later new work blocked | Yes        |
| Cross-tenant/browser safety      | ordinary member reads profile/run                      | safe nonsecret summary only                                       | Security   |

### Edge Cases Checklist

- [ ] No registry rows exist
- [ ] Direct/fake profile has no model binding
- [ ] Minitap profile has no model binding
- [ ] Reference key is malformed or revision is zero
- [ ] Referenced revision is unknown
- [ ] Referenced revision is retired
- [ ] Definition lacks required capability
- [ ] Legacy raw string has no match or multiple matches
- [ ] Worker advertises no model, wrong key, wrong revision, or unsupported provider
- [ ] Correct worker connects after an incompatible worker; queued work remains claimable
- [ ] Worker heartbeat is stale
- [ ] Definition retires between preview and create/open
- [ ] Session/run is already frozen when registry status changes
- [ ] Historical manifest/session lacks `resolved_model`
- [ ] Historical usage lacks `model_reference`
- [ ] Direct action never invokes model even on selector failure
- [ ] Provider credential is absent
- [ ] Doppler injects unrelated server credentials; child strips them
- [ ] Provider returns no usage metadata
- [ ] Provider factory rejects unsupported enum before network
- [ ] Existing suite/run React Flow generated-file changes remain intact

---

## Validation Commands

Run from `/Users/ducng/.codex/worktrees/sequence-demo/mobile-qa`. Complete implementation edits
first; then generate and validate in this order.

### Targeted Contract and Static Validation

```bash
cargo test -p mobile-qa-contracts --test model_registry --test execution
just types
just check-contracts
```

EXPECT: Contract tests pass; generated files contain no drift on `--check`; no handwritten
consumer shapes exist.

### Targeted API Tests

```bash
cargo test --locked -p mobile-qa --test model_registry --test execution --test task_sessions --test test_library
```

EXPECT: Registry, freeze, protocol fencing, historical compatibility, and existing execution/
library/session integration tests pass against real PostgreSQL test setup.

### Targeted Worker Tests

```bash
cd apps/mobile-worker
uv run --no-sync --frozen pytest \
  tests/test_model_runtime.py \
  tests/test_task_sessions.py \
  tests/test_execution.py \
  tests/test_qualification.py \
  tests/test_sdk_adapter.py
uv run --no-sync --frozen ruff check .
uv run --no-sync --frozen pyright
```

EXPECT: All tests pass with no device, Doppler, or provider network access; strict typing and lint
are clean.

### Targeted Web Tests

```bash
pnpm --dir apps/web test -- \
  src/components/test-library/ai-authoring.test.tsx \
  src/pages/TaskSession.test.tsx \
  src/pages/RunDetail.test.tsx
pnpm --dir apps/web typecheck
pnpm --dir apps/web lint
```

EXPECT: Capability gating and audit display pass; zero type/lint errors.

### Model-Free Smokes

```bash
just smoke-execution
just smoke-test-library
just smoke-direct-authoring
```

EXPECT: Saved execution/library/direct-authoring flows pass with synthetic evidence and no model,
device, or Doppler access.

### Full Repository Validation

```bash
just format
just check
just build
git diff --check
```

EXPECT: No regressions, type errors, lint errors, formatting drift, or build failures. Vite's known
large-bundle advisory may remain unchanged and must not be reported as a new failure.

### Database Validation

```bash
cd apps/api
cargo test --locked --test model_registry
```

EXPECT: Up/down migration behavior, constraints, immutable replay, retirement, and worker
capability columns are verified by the test harness. Do not run destructive migration commands
against a user database merely for validation.

### Optional Explicit Runtime Validation

Only after the user/operator supplies an approved private definition and asks to run real
acceptance:

```bash
# Register nonsecret definition through the trusted task, update private host model_ref manually,
# then restart the full stack so API and both workers use protocol 5.
just dev-restart
just dev-logs
```

EXPECT: The worker advertises the exact key/revision; an AI action uses the frozen provider model;
a deliberately mismatched reference remains unleased and does not boot/quarantine the phone.
Never print Doppler values.

### Manual Validation

- [ ] Customer still selects only an execution profile; there is no model/provider picker.
- [ ] A direct-only profile connects, runs, and reports with no model badge and no Doppler/model call.
- [ ] An AI-capable profile shows the correct capability labels.
- [ ] Ask AI is disabled when navigation capability is absent.
- [ ] Generate tests is disabled when structured-authoring capability is absent.
- [ ] A new run detail shows model display name and exact `key@revision` from the frozen manifest.
- [ ] A historical run without a resolved snapshot is readable and visibly labeled legacy/unknown.
- [ ] A wrong host reference does not lease, boot, call Doppler/provider, or enter recovery.
- [ ] A correct worker can later claim the same still-queued work.
- [ ] Retiring a model blocks new work but leaves existing run/session detail unchanged.
- [ ] No secret, Doppler token, environment dump, prompt/image payload, or provider response body is
      present in Git diff, database model payload, browser DTO, journal, or logs.

---

## Acceptance Criteria

- [ ] One immutable registry is the only source that maps a registered reference to provider and
      provider model identity.
- [ ] New execution profiles use exact typed references; new direct/fake profiles use `None`.
- [ ] New model-enabled runs and phone sessions freeze a complete `ResolvedModel` before queueing.
- [ ] Browser and worker models are generated from Rust contracts; no duplicated handwritten wire
      shapes exist.
- [ ] All Python provider construction and provider credential allowlisting live in one runtime
      module.
- [ ] Model-enabled work requires protocol 5 and an exact qualified worker reference before lease.
- [ ] A mismatch leaves work queued and causes zero device/model side effects and no recovery state.
- [ ] Direct-only flows work with an empty registry and make zero model/Doppler calls.
- [ ] AI UI gates use explicit capabilities and add no customer selection click.
- [ ] Run reports expose exact frozen reference/revision and retain historical fallback display.
- [ ] Registry retirement affects only new resolution; frozen work/history remains stable.
- [ ] Historical raw-string profiles/manifests remain readable through isolated compatibility code;
      no new raw-string binding is written.
- [ ] No registry or manifest contains credentials or Doppler scope.
- [ ] Targeted and full validation commands pass; real device/model acceptance status is stated
      accurately.
- [ ] Canonical architecture/environment/development/status docs are updated with observed facts.

## Completion Checklist

- [ ] Code follows discovered contract/service/migration/worker patterns
- [ ] Error handling matches `ApiFailure` and `QualificationError` conventions
- [ ] Structured logs include safe reference metadata only
- [ ] Tests prove absence of side effects, not only returned errors
- [ ] Raw provider/model hardcoding is removed from business logic and call sites
- [ ] Direct/fake sentinels (`""`, `"none"`, `"no-model-adb-demo"`) are not written by new code
- [ ] Generated files came from `just types` once after source edits
- [ ] No new dependencies or domain framework were added
- [ ] Existing sequence/suite UI edits were preserved
- [ ] Documentation updated in the same change
- [ ] No temporary GAN harness artifacts are included
- [ ] No secrets/private runtime files are staged
- [ ] Self-contained — implementation requires no further codebase search or design decision

## Risks

| Risk                                                       | Likelihood       | Impact   | Mitigation                                                                                                       |
| ---------------------------------------------------------- | ---------------- | -------- | ---------------------------------------------------------------------------------------------------------------- |
| Partial rollout sends new model assignments to old workers | Medium           | High     | Protocol 5 gate, additive optional fields, ordered API/worker/profile rollout                                    |
| Legacy raw model has ambiguous registry match              | Medium           | Medium   | Fail closed; require exact operator registration/reference; never guess                                          |
| Mismatch still occurs after lease and quarantines device   | Low after change | High     | API compares advertised exact reference before reservation; Python repeats pre-side-effect check                 |
| Registry retirement races run/session creation             | Low              | Medium   | Resolve/freeze in creation transaction; immutable payload; retirement affects subsequent work only               |
| Provider credentials leak into registry/log/browser        | Low              | Critical | Contract excludes secret fields; central environment allowlist; serialization/log tests                          |
| UI enables wrong AI feature                                | Medium           | Medium   | Separate navigation vs structured-authoring capabilities and component tests                                     |
| Historical payload becomes unreadable                      | Medium           | High     | Untagged dual-read/new-write binding, optional snapshots, explicit legacy fixtures                               |
| Existing generated suite UI work is overwritten            | Medium           | High     | Generate once in dirty worktree, review content-sync diff, never hand-replace generated files                    |
| Registry becomes an unbounded generic framework            | Medium           | Medium   | Closed enums, one provider adapter, explicit NOT Building list, no plugin system                                 |
| Operator mistakes registration for qualification           | Medium           | High     | Registry and execution profile remain separate; docs/UI state registry mapping is not device/model qualification |

## Notes

- Graphify was present but built at commit `9c2a1fec34e645c64ea585a943fd8c42a484217f`, while this
  worktree is at `10fec1182a52150b45062109690fcd1a2c0b58f8` with additional local changes. Its broad
  query identified the expected run/session/worker/model seams, and every finding used here was
  verified in current source.
- The current runtime error is not an APK mismatch. It is the expected result of two unrelated
  raw model strings being compared at the worker boundary. This plan removes that ambiguity while
  preserving image/APK/profile checks as separate concerns.
- Registration does not prove provider quality or device qualification. A model revision remains
  eligible only through an explicitly qualified execution profile, and any provider/model change
  requires requalification under the existing product rules.
- The conceptual cross-system API is `modelToUse(request) -> ResolvedModel`, but the implementation
  should use language-idiomatic names: Rust `model_registry::resolve_for_new_work`, generated value
  objects at transport boundaries, and Python `model_runtime` factories. Do not attempt to share
  executable function code across Rust/TypeScript/Python.
