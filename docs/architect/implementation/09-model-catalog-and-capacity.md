# 09 — Model catalog and qualified capacity

Status: locally implemented and simulated validation passed; live qualification and rollout remain
operator gates. Depends on: 04 execution leases and recovery, 06 phone sessions and AI
authoring, and 07 cost/reliability limits. Owns: model rollout policy, qualified worker
capabilities, model-aware claim selection and visible queue blockers. It does not own
device verdicts or a customer-facing model picker.

## Existing foundation and scaling gap

The API already stores immutable model definitions by `key@revision`. The pure
[contract](../../../crates/contracts/src/model_registry.rs) owns the current
definition and resolved snapshot; the [registry service](../../../apps/api/src/services/model_registry.rs)
rejects a different payload for an existing reference. New model-enabled work freezes
the resolved snapshot. Protocol-5 workers advertise one qualified reference, and the
API checks an exact match before leasing. Direct work has no resolved model.

This foundation should remain the single catalog. Today a worker advertises only one
reference, and an execution profile binds device setup to model choice. The claim
query examines the oldest queued job for a profile before checking model compatibility;
an incompatible first job can keep later compatible work waiting. A run can also stay
`queued` when no worker is alive or recovery holds the device, without showing which
condition applies. These are capacity and operator-visibility gaps, not reasons to add
a second model registry or an automatic model fallback.

## Definition and lifecycle

- Keep the immutable definition small: stable reference, display name, provider,
  exact provider model identifier and supported product capabilities. A future
  provider model such as a hypothetical `openai.gpt-6` gets a new reference; a
  correction to an existing definition gets a new revision. Never edit a registered
  payload or rewrite a saved run/session snapshot.
- The trusted operator registers a reviewed nonsecret definition. Registration of
  identical content is idempotent; conflicting content at the same reference fails.
  Record actor, time and content digest as audit metadata outside the immutable
  payload. Validate provider support and capability names before activation.
- Keep credentials, Doppler scope, token price, rate limits, routing defaults and
  host qualification out of the definition. They have different lifecycles. Version
  operational limits and price assumptions separately; unknown usage/cost remains
  unknown rather than inferred from a model name.
- Use explicit staged, active, draining and retired _routing_ states. Activation
  permits new assignments only after a compatible qualified worker is live.
  If both phone purposes have active assignments, one live phone worker must
  advertise both exact references before either assignment can be activated.
  Draining stops new assignments but allows already frozen queued work to finish on
  matching capacity. Retirement preserves definitions and historical snapshots.
  A safety shutdown revokes workers/cancels affected work explicitly; retirement
  alone does not silently substitute another model.

## Assignment separated from device qualification

Introduce an operator-owned, versioned model assignment for each app, purpose
(navigation or structured authoring) and compatible execution profile. It points to
an active definition reference and carries bounded model-call limits. The physical
execution profile continues to describe package, adapter, image, device identity and
reset qualification. New profiles should not need a model field; historical profiles
and manifests remain readable without mutation.

At run or phone-session creation, the API validates the assignment and required
capability, then freezes the resolved definition and assignment version. It may
queue work while a qualified worker is offline and must show that wait explicitly.
Direct-only work freezes no model and remains runnable when no model is configured.
Changing the default affects only new work. A queued job
retains its frozen reference; moving it to another model requires an explicit new
run/session and leaves the old record intact. Customers do not choose provider names
in the initial product UI.

## Qualified worker inventory and claim admission

A worker may advertise a bounded set of exact references only after each has
qualification evidence for its pinned worker runtime, SDK revision and device/image
combination. The API stores separate execution and phone poll versions, capabilities
and heartbeat times with a short liveness window. A phone poll does not establish
execution claim eligibility. The set is operator configured on the worker host; registry presence
alone never qualifies a host. A worker with no model references can still claim
direct-only jobs. One physical device retains one active lease, regardless of how
many models its worker supports.

Claims use the frozen reference and required capability to select the oldest
_eligible_ queued attempt under the existing PostgreSQL lock, reservation and fencing
rules. A worker skips incompatible jobs and can claim later compatible work; it
never changes a job's model. The same rule applies to phone sessions. If no matching
worker is live, the job remains queued without reserving a device. Recovery-required
reservations continue to block claims until verified operator recovery. Keep worker
credentials app/profile scoped and provider secrets in the isolated SDK child.

## Queue explanation and rollout

Preserve the durable `queued` state, but expose a computed, bounded reason such as
`worker_offline`, `model_unavailable`, `worker_upgrade_required`, `capacity_busy` or `device_recovery_required`
with the last compatible worker heartbeat. A missing heartbeat is an observation,
not proof that a model or device is permanently unavailable. Show the reason on Runs
and in operator logs; do not expose credentials, private host paths or raw provider
errors. A queued job should never imply that execution has begun.

Roll out additively: register and review the new definition; qualify it against the
pinned worker/SDK and reliability scenarios; stage a worker advertisement; activate
an assignment for new work; watch queue wait, outcomes, usage and cleanup; then drain
or retain old capacity. Do not enable a new provider by database row alone: its SDK
adapter and secret boundary require code and qualification. Reuse the existing API,
PostgreSQL queue and Python worker; add devices only when measured wait or capacity
requires them.

## Implementation and acceptance sequence

1. Add audit metadata and versioned operator assignment while preserving the current
   single-reference protocol and legacy reads. Verify immutable registration,
   retirement behavior, exact capability matching and frozen new-work snapshots.
2. Add bounded multi-reference worker qualification and an additive protocol version.
   Keep protocol-5 single-reference workers compatible during rollout; revoke or
   drain them deliberately. Verify stale advertisements, revoked workers and provider
   mismatch cannot lease model work.
3. Select the oldest compatible job in one transaction. Cover incompatible older
   jobs, competing workers, direct-only jobs, one-device reservations, lease expiry,
   cancellation and recovery with real route/database tests.
4. Show computed queue reasons through generated browser contracts and runtime Zod
   validation. Verify offline, mismatched model, busy capacity and recovery-held
   cases in the UI and route tests. Record queue wait and model usage without making
   device/model calls in ordinary checks.
5. Qualify the selected model/runtime/device combination explicitly before live
   rollout. A real device campaign and provider calls are operator actions; passing
   simulated tests does not establish model quality or cost.

Success means adding a new qualified model without editing existing definitions or
saved reports, without stranding compatible jobs behind incompatible ones, and with
an actionable reason whenever a job waits. It does not mean automatic model choice,
automatic fallback or autoscaling.

## Current implementation boundary

The additive assignment table stores immutable revisions with audited state changes.
`execution` task actions stage, activate, drain and retire a revision. Activation
requires a recent compatible worker advertisement. New navigation work freezes the
active reference and revision; historical profile bindings remain readable. Protocol 6
workers advertise up to eight references from a host profile carrying nonsecret
qualification evidence, while protocol 5 still supports one reference. The API picks
the oldest compatible queued attempt or phone session. A phone session freezes a
separate structured-authoring model when assigned and requires the worker to support
both exact references on one live phone worker before activation. Run details compute
queue wait, reason and last compatible execution heartbeat using the worker's actual
claim protocol gates and current reservation state.

This source and simulated-test capability does not qualify a new provider model,
runtime, SDK build or physical device. Operators must verify the evidence fields and
perform the separate campaign before publishing a new assignment. Model call budget
enforcement and fleet metrics remain follow-up work; `max_calls` is recorded in the
assignment but is not yet a runtime stop condition.
