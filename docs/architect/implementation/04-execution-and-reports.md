# 04 — Run one test and produce an evidence report

Status: locally implemented; simulated HTTP and one real API-to-emulator good-path run verified. Browser, live fault/reliability and hosted acceptance remain pending. Depends on: 02 and 03. Owns: Rust run/scheduler/report modules and migrations, worker protocol/adapter, Runs UI.

## Implementation scope

The local implementation covers versioned test definitions, device actions, independent
evidence checks, HTTP leases and the Runs interface. Validation and remaining device and
hosted-storage acceptance gates are tracked in [current status](../status.md).
The customer test editor belongs to spec 05.

## Model assignment

New model-enabled manifests freeze one validated `ResolvedModel` before queue insertion. The
registry may later be retired without changing that historical snapshot. Protocol-5 workers must
advertise the same reference and provider support before device reservation; mismatches remain
queued. Comparison context includes the resolved reference/revision, so different model revisions
are not comparable even when the test and device profile otherwise match. Historical manifests
without a snapshot remain readable and are reported as unavailable model context, never guessed.

## Deliverable

From the browser, run one manually authored saved case on an uploaded build. Show queued/running progress and a persisted report. The same path supports a real worker and a clearly identified development fake worker.

## Smallest durable domain

Introduce minimal case-version, suite-version and default-plan-version records now, even though the full editor arrives in spec 05. An operator fixture/import command creates these through normal validation. Do not create a separate throwaway free-text job model that bypasses validation.

Persist a run manifest binding project, build checksum, saved plan/case versions, requiredness, data variants, qualified device/image configuration, reset policy and execution/model settings. Include attempts, checks, jobs, device/account leases, events and artifacts. Store identifiers/references for secrets; never snapshot secret values into the manifest.

## APIs and flow

Customer routes: `POST /api/apps/:app_id/runs`, `POST /api/apps/:app_id/suite-runs`,
`GET /api/runs/:run_id`, `POST /api/runs/:run_id/cancel`, and authorized artifact access.
Release submission takes a build and saved plan. Direct suite submission takes a build,
qualified profile and exact saved suite version, then derives a bounded transient execution policy;
it never creates a hidden library plan. The server resolves either source into the same immutable
manifest and ordered attempt queue. Use an idempotency key to prevent a double-click producing
two runs.

Worker operations: claim, heartbeat, publish events, request artifact upload, complete attempt, acknowledge cancellation. These are our HTTP endpoints. The Python process needs no database credentials or public HTTP listener.

1. Validate build/readiness, membership, definition validity and budgets.
2. Atomically persist manifest and queued jobs.
3. Claim an eligible job in a short PostgreSQL transaction using row locking/`SKIP LOCKED`; reserve device and test account and issue a scoped lease token.
4. Worker downloads the checked APK, verifies reset/install/preconditions and starts the qualified adapter.
5. Worker publishes bounded ordered events and evidence; Rust evaluates persisted checks and final result rules.
6. Report survives application/worker restart and browser reload.

Implement the narrowly scoped device scheduler as application code. Loco's generic background-job retries must not automatically replay device mutations. Ordinary internal Rust jobs may use its framework facilities separately.

## State, cancellation and recovery

Keep job lifecycle distinct from test outcome. Jobs progress through queued → leased → running → finalizing → finished, with cancel-requested and recovery-required paths. Test outcomes are pass, fail, blocked, inconclusive, skipped or canceled.

Use lease generation/fencing tokens, expiry, heartbeats, event IDs and sequence numbers. Reject stale completion and deduplicate resubmissions. A worker protocol version is checked before dispatch. Begin with one active case per device, one active lease per test account and explicit per-project concurrency limits.

On lease expiry, invalidate completion authority and mark recovery required. Terminate/isolate the old execution and verify device/backend reset before reuse. Database fencing alone does not stop an old process tapping the phone. Cancellation remains pending until termination is acknowledged or resources are quarantined; it must not falsely promise immediate physical stop.

Run details expose durable cancellation intent separately from the attempt state. An attempt
with cancellation requested and recovery required remains quarantined and holds
reservations until trusted operator recovery records physical reset evidence; the UI
must show both facts.

The supervisor enforces the run deadline and stops its owned child process group. Before a real worker claims a job, it checks that the host Android tools can run with its selected JDK; the child inherits the same `JAVA_HOME`. If AVD setup fails before an emulator process has ever launched, the owned directory can be discarded and free ports verified as clean cleanup. Once an emulator has launched, an unverified stop or reset remains quarantined; elapsed queue time alone never releases its reservation. The queued run displays `device_recovery_required` until explicit operator recovery with physical evidence.

Set bounded run time, actions, artifact bytes and model usage where observable. If the SDK cannot enforce an internal action budget, use a process-level hard deadline and record that limitation. Preserve partial evidence on timeout or upload failure.

## Verification and report rules

- Check declared expectations against retained evidence. SDK completion is execution metadata, not a pass assertion.
- A required failed check produces failure; missing prerequisites produce blocked; insufficient evidence produces inconclusive.
- A required unexecuted/canceled/blocked/inconclusive case prevents a complete passing plan.
- Retries create additional attempts. Mixed failure/pass history is surfaced for review and never silently converted to a clean pass.
- Show build/device/config, plan coverage, expected/observed behavior, screenshots/logs, verification method, attempt history and actionable failure category.

Poll run status through TanStack Query initially, stopping when terminal and backing off while hidden. SSE and live phone streaming can wait. The report reads persisted results, not transient agent prose.

## Acceptance tests

Use the fake worker for deterministic tests: duplicate submit/completion, worker disappearance, late completion, cancellation, artifact failure, backend outage, mixed retries and cross-project access. Use a real PostgreSQL integration test for competing claims and lease uniqueness.

Repeat the three qualified device scenarios through the browser. Restart the Rust app during a run and recover persisted progress; interrupt the worker and prove it cannot cause double execution on the same account/device. Done means an actual useful report, not just a “job succeeded” badge.

## Local implementation scope

The phase 04 implementation uses one typed version store (`execution_definitions`)
for cases, suites and plans, with distinct Rust variants and immutable saved versions
and technical admission. Attempts serve as durable queue records; the manifest pins
resolved case selections. This reduces parallel table/entity scaffolding while keeping
case/suite/plan meaning and version identity separate. All SQL goes through SeaORM
with bound parameters; the migration owns constraints.

The first registered adapter is `demo_persistence_v1`. Real mode reuses the phase 02
controlled device lifecycle and executes imported semantic actions through the shared
pinned Minitap seam. Fake mode uses the same HTTP protocol and labels all reports as
simulated. Arbitrary customer package/reset/login adapters and the phase 05 editor
remain future work. SDK traces remain private; normalized checkpoint PNG/XML and
semantic action events are the customer-visible evidence in this slice.

Completion remains subject to the recorded validation report and open real-device /
hosted acceptance gates. No paid resources are launched by normal checks or smoke.

Worker claims now use bounded 30-second HTTP long polling with immediate local
commit notifications and a five-second database fallback across API processes.
See the backend-to-emulator runbook in development.md.
