# Plan: Model catalog and qualified capacity

## Summary

Extend the existing immutable model registry with versioned operator assignments, qualified multi-model worker advertisements, compatible queue selection, and visible queue blockers. Preserve historical profile and protocol-5 reads, frozen run/session models, and physical-device recovery fencing. Live model/device qualification remains a separate operator action.

## User Story

As an operator, I want to add a newly qualified model and assign it to new work without changing device profiles or historical runs, so that capacity can scale safely and queued work has an actionable explanation.

## Problem → Solution

Model choice is embedded in profiles, workers advertise one reference, the oldest incompatible job blocks compatible work, and `queued` lacks a reason. Add app/profile/purpose assignment revisions, exact model sets, eligible SQL claim selection, and computed queue state.

## Metadata

- **Complexity**: XL
- **Source PRD**: `docs/architect/implementation/09-model-catalog-and-capacity.md`
- **PRD Phase**: implementation sequence 1–4; phase 5 operator qualification
- **Estimated Files**: 25–35
- **Confidence**: 7/10

## UX Design

### Before

Run detail: `queued`, even when no worker is online or recovery blocks the device.

### After

Run detail: `queued` plus a bounded reason and last compatible worker heartbeat. Historical snapshots remain unchanged.

### Interaction Changes

| Touchpoint          | Before                | After                           | Notes                      |
| ------------------- | --------------------- | ------------------------------- | -------------------------- |
| Operator CLI        | register/retire model | stage/activate/drain assignment | No customer picker         |
| Worker host profile | one `model_ref`       | bounded qualified reference set | Legacy single ref readable |
| Run detail          | generic queued        | computed reason                 | No private paths/secrets   |

## Mandatory Reading

| Priority | File                                                                                                                     | Why                                   |
| -------- | ------------------------------------------------------------------------------------------------------------------------ | ------------------------------------- |
| P0       | `docs/architect/README.md` and `implementation/09-model-catalog-and-capacity.md`                                         | Authority and acceptance              |
| P0       | `crates/contracts/src/model_registry.rs`, `execution.rs`, `task_sessions.rs`                                             | Wire shapes and frozen snapshots      |
| P0       | `apps/api/src/services/model_registry.rs`, `scheduler.rs`, `task_sessions.rs`, `runs.rs`                                 | Resolution, leasing and queue reads   |
| P0       | `apps/mobile-worker/src/mobile_qa_worker/qualification/config.py`, `model_runtime.py`                                    | Local qualification and advertisement |
| P1       | `apps/api/migration/src/m20260921_000010_model_registry.rs`, `apps/api/src/tasks/execution.rs`                           | Additive migration/CLI patterns       |
| P1       | `apps/api/tests/model_registry.rs`, `execution.rs`, `task_sessions.rs`, `apps/mobile-worker/tests/test_model_runtime.py` | Real DB and worker tests              |

## External Documentation

No external research needed; use existing SeaORM/PostgreSQL, generated contracts and worker protocol patterns.

## Patterns to Mirror

- **Naming**: Rust `snake_case` service functions and `#[serde(deny_unknown_fields)]` contracts in `crates/contracts/src/model_registry.rs`.
- **Errors**: `ApiFailure::invalid`, `conflict`, and stable reason codes in `apps/api/src/services/model_registry.rs`.
- **Logging**: `tracing::info!` for trusted operator action and bounded `reason_code` warnings in `apps/api/src/services/scheduler.rs`.
- **Data access**: `rows`, `one`, `exec`, and explicit `db.begin()` in `apps/api/src/services/model_registry.rs`.
- **Tests**: `DATABASE_BOOT` plus `request::<App,_,_>` in `apps/api/tests/model_registry.rs`.

## Files to Change

| File/group                                                                                                                         | Action               | Justification                                            |
| ---------------------------------------------------------------------------------------------------------------------------------- | -------------------- | -------------------------------------------------------- |
| `crates/contracts/src/{model_registry,execution,task_sessions}.rs`                                                                 | Update               | Assignment snapshot, bounded advertisement, queue reason |
| `apps/api/migration/src/*model_assignment*`, `lib.rs`                                                                              | Create/update        | Audit and versioned routing rows                         |
| `apps/api/src/services/{model_registry,runs,scheduler,task_sessions,test_definitions,test_library}.rs`                             | Update               | Assignment resolution and admission                      |
| `apps/api/src/tasks/execution.rs`                                                                                                  | Update               | Trusted assignment lifecycle                             |
| `apps/mobile-worker/src/mobile_qa_worker/{qualification/config,model_runtime,execution/runner,task_sessions,execution/actions}.py` | Update               | Qualified set and protocol 6                             |
| `apps/web/src/pages/RunDetail.tsx`, generated browser/worker outputs                                                               | Update via generator | Queue explanation and protocol shapes                    |
| relevant API/worker/web tests                                                                                                      | Update               | Routing, compatibility, blockers                         |
| `docs/architect/{implementation/09-model-catalog-and-capacity,status,contracts}.md`                                                | Update               | Observed implementation and limits                       |

## NOT Building

- Automatic provider fallback, customer model picker, autoscaling, a second registry, real device/provider calls in ordinary checks.
- Rewriting saved manifests or releasing recovery reservations automatically.

## Step-by-Step Tasks

### Task 1: Immutable registry audit and assignment store

- **ACTION**: Add additive migration and operator assignment service.
- **IMPLEMENT**: Preserve definition payload immutability, record registration actor/time/digest separately, version app/profile/purpose assignments with staged/active/draining/retired routing state and bounded call budget. CLI actions verify operator scope and exact capability/provider before activation.
- **MIRROR**: Existing `model_registry::register`, `test_definitions::register_profile`, execution task CLI.
- **IMPORTS**: contracts model types, `ConnectionTrait`, `TransactionTrait`, `Uuid`.
- **GOTCHA**: Historical retired definitions/snapshots remain readable; never mutate prior assignment revisions.
- **VALIDATE**: DB tests for duplicate/conflicting registration, assignment lifecycle, capability checks and snapshot stability.

### Task 2: Freeze assignment for new work

- **ACTION**: Resolve operator assignment independently of physical execution profile.
- **IMPLEMENT**: Allow model-free new Minitap profiles; use assignment for new navigation runs and phone sessions; freeze assignment version and resolved model. Legacy profile binding is fallback for historical profiles only. Direct-only runs remain model-free.
- **MIRROR**: `runs::assemble` and `task_sessions::open`.
- **IMPORTS**: model registry service and model capability contract.
- **GOTCHA**: Do not require a live worker at queue creation; do not change existing snapshots.
- **VALIDATE**: Route tests for active/draining/retired assignment and direct work.

### Task 3: Qualified multi-reference worker protocol

- **ACTION**: Add protocol 6 while retaining protocol 5 single-ref reads.
- **IMPLEMENT**: Bounded exact model reference list and qualification evidence in local host config; API admission validates advertised shape, provider and liveness; worker child verifies frozen reference in set. Direct-only worker may advertise none.
- **MIRROR**: `WorkerModelCapabilities`, `Profile.load`, `worker_capabilities`, runner/phone poll.
- **IMPORTS**: generated Pydantic models after `just types` in final validation/generation phase.
- **GOTCHA**: A catalog row alone never qualifies a host; one device still one lease.
- **VALIDATE**: Worker unit tests and API real DB tests for v5/v6, stale/revoked/mismatch.

### Task 4: Eligible queue selection

- **ACTION**: Select oldest compatible queued item under existing transaction locks.
- **IMPLEMENT**: SQL predicate compares frozen model reference and provider against the authenticated advertisement before `ORDER BY ... FOR UPDATE SKIP LOCKED LIMIT 1`; use same rule for phone sessions. Preserve app/device reservations, fencing, cancellation and recovery.
- **MIRROR**: `scheduler::claim` and `task_sessions::claim`.
- **IMPORTS**: `serde_json`, execution store helpers.
- **GOTCHA**: An incompatible older item cannot block a compatible later one; no implicit model substitution.
- **VALIDATE**: Competing worker, direct, older incompatible and recovery DB tests.

### Task 5: Queue explanation and documentation

- **ACTION**: Compute bounded reason at read time and render it.
- **IMPLEMENT**: Add queue reason contract with heartbeat; distinguish offline, model unavailable, capacity busy, recovery required. Update Run detail and operator logs. Record queue wait; retain existing model usage reference.
- **MIRROR**: `runs::detail`, generated Zod boundary, `RunDetail.tsx`.
- **IMPORTS**: `chrono`, generated types and Zod.
- **GOTCHA**: A heartbeat is a recent observation; do not expose host paths or provider errors.
- **VALIDATE**: Route/UI tests per reason; update architecture status with simulated and live gates separately.

## Testing Strategy

| Test                 | Input                                        | Expected                                  |
| -------------------- | -------------------------------------------- | ----------------------------------------- |
| Immutable definition | same/conflicting payload                     | idempotent/conflict                       |
| Assignment           | new active revision                          | only new work changes; old snapshot stays |
| Worker set           | valid, duplicate, oversized, absent evidence | exact bounded admission                   |
| Claim order          | incompatible older and compatible newer      | newer leases; older remains queued        |
| Direct work          | no model assignment/advertisement            | claims normally                           |
| Recovery             | device reservation exists                    | queued reason; no lease                   |
| Offline/mismatch     | stale or wrong advertised set                | reason and no lease                       |

## Validation Commands

Finish all source, tests and config first. Then run `just types`, `just check-contracts`, `just check-api`, `just check-worker`, `just check-web`, and affected API/web builds once. Rerun only failed/invalidated commands. No live device/model calls. Use a disposable test database if the shared DB contains unrelated branch migrations.

## Acceptance Criteria

- [ ] Immutable definitions and historical snapshots preserved.
- [ ] New assignment revision changes only future work.
- [ ] Qualified v6 worker can advertise bounded exact references; v5 remains readable.
- [ ] Compatible jobs do not wait behind incompatible older jobs.
- [ ] Queued run shows a bounded reason and compatible heartbeat.
- [ ] Generated contracts, route/DB/worker/UI tests and scoped validation pass.
- [ ] Operator qualification gate remains explicit and unclaimed.

## Completion Checklist

- [ ] Update owning architecture and status documents.
- [ ] Archive this plan and write the implementation report.
- [ ] Review diff; do not push or modify private profile or live recovery state.

## Risks

| Risk                                               | Mitigation                                                |
| -------------------------------------------------- | --------------------------------------------------------- |
| Queue predicates and old protocol behavior diverge | Shared compatibility helper plus real DB route tests      |
| Misleading queue reason during races               | Compute read-time observation, bound heartbeat            |
| New model activated without usable capacity        | Require recent compatible advertisement before activation |
