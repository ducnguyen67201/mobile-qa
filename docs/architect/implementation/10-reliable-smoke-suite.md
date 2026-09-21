# 10 — Reliable smoke suite

Status: **locally implemented; simulated HTTP validated, real device and rendered
browser acceptance open**. Depends on the saved library (05), durable execution (04),
qualified clean-start replay (07A), and case comparison (07B). This focused
delivery makes an existing saved suite a repeatable release check; it does not
introduce a new definition kind or execution engine. [Status](../status.md)
remains the source for observed acceptance.

## User outcome

A tester saves a small group of independently verifiable cases, selects a build
and qualified device, runs the exact suite version, and reads a durable report
for every case. On another build, the tester can pin an earlier suite run as a
baseline and see which comparable checks changed. A queued, blocked, incomplete,
or recovery-held run never appears as a passing smoke check.

## Two-build workflow

Each submission creates one run for one build. Run A selects saved suite v3 and
build 1.3 with no baseline, executes each case from its declared clean start,
and retains the report. After Run A completes, Run B selects the same suite v3
and build 1.4, and the tester explicitly chooses Run A as its baseline before
submitting. The baseline run ID is frozen in Run B; the same qualified device
profile may be reused after verified cleanup. The runs have separate immutable
manifests and never install two APKs in one emulator session.

When Run B finishes, verified Passed→Failed is Regression, Failed→Passed is
Recovered, Failed→Failed is Still failing, and Passed→Passed is Unchanged.
Changed test versions or execution context and missing clean proof are Not
comparable. This compares release builds, not randomized user groups. A
historical run cannot gain or change its baseline afterward; selecting any two
completed runs later for an ad-hoc comparison is outside this delivery.

## Business rules

1. A **case** owns one scenario: ordered actions, explicit expected checks,
   prerequisites, and its own clean start and cleanup. An action sequence saved
   from the phone becomes a case when the tester confirms its expectations. A
   **suite** is a set of exact saved case-version references, displayed in an
   author-chosen order, with each reference's data variant and requiredness. It
   cannot contain an unsaved or check-free phone sequence as a second member
   type. A **plan** remains the reusable release
   policy over suites/cases and devices.
2. The suite map shows independent members connected to the suite, without
   case-to-case execution arrows. Display order does not set claim priority.
   Each case starts from its declared clean state. A failed case does not make
   another case pass or skip it implicitly. If cleanup holds the phone, other
   cases remain queued behind recovery rather than acquiring
   a fabricated result. Exact duplicate selections resolve once; conflicting
   versions or requiredness of one logical
   case are rejected by the existing resolver.
3. Saving a complete suite creates or reuses an immutable version. Editing cases
   or suite membership never retargets a saved version or an existing run. An
   incomplete suite can remain a draft but cannot be queued. The regular library
   permits 1–100 resolved cases with at least one required case; the smoke
   acceptance fixture uses two or three concise cases, not a new product limit.
4. Before submission, show the exact suite version, selected build and checksum,
   qualified device, readiness blockers, and baseline choice. The tester chooses
   the build and device explicitly. No baseline is selected by default; an
   eligible prior run may be shown as a suggestion, but the tester must choose
   it before submission. The chosen ID or None is frozen with the run.
   Refresh, retry, a newer upload, or a newer result cannot silently change that
   choice. A suite can run without a baseline to establish its first result.
5. A suite run uses the existing immutable manifest, attempts, lease fencing,
   verifier, artifact store, cancellation, and recovery rules. One submission
   identity queues at most one run. Preview is advisory; creation rechecks
   authorization, versions, environment revision, build, profile, and admission
   inside the existing transaction. No background execution follows Save alone.
6. The overall result is derived from recorded attempts. **Required checks
   passed** means every required case has conclusive passing checks, verified
   clean start and cleanup, and no disqualifying mixed/recovered attempt. Any
   required failed check is a failure. Any required blocked, canceled,
   inconclusive, unexecuted, or recovery-held case is incomplete. Optional
   failures remain visible and must be named in the summary; the UI must not say
   that every case passed. A queued run has no test verdict.
7. Comparison is a separate fact from the current result. Suggest the latest
   completed, eligible saved-suite run for the same suite version and execution
   context, ordered deterministically; include an earlier failed run. Allow an
   explicit different completed same-app baseline, but label each case using the
   existing compatibility policy. Only identical case version/content, data
   variant, environment and execution signature with verified evidence on both
   sides can become Regression, Still failing, Recovered, or Unchanged. Changed
   coverage appears Added/Removed; missing proof or changed expectations is Not
   comparable. An incompatible baseline is explained, not replaced.
8. Queue blockers are distinct from app failures. A recovery-required device or
   dirty worker journal stays held until an operator verifies physical cleanup
   and uses the documented recovery path. Do not delete journals/reservations or
   mark cleanup verified to make a smoke run proceed. Direct-only cases need no
   model calls; AI navigation still requires its frozen qualified assignment.
9. Names in the Tests UI identify the object: **Run suite** / **Save & run
   suite**. Keep “sequence” for a case's ordered actions; label the suite map
   as independent test cases.
   The run page identifies a saved suite, build, device, per-case status,
   expected/observed evidence, baseline and coverage gaps. Browser display does
   not calculate the verdict.
10. The **task picker** is the API claim transaction, not a choice made by the
    Python worker or the suite canvas. Queue one attempt per resolved case when
    the run is created. A compatible worker may claim only an eligible attempt
    for its registered app and qualified profile. Within that eligible set,
    prefer the oldest run; for a saved suite, choose any queued eligible case
    using diagnostic attempt number and stable attempt ID as deterministic
    tie-breaks, without consulting its displayed case order. Release plans
    retain their existing case-index order. Use stable run IDs to break equal
    timestamps.
    A worker that cannot run an older attempt must not block a compatible newer
    one. There is no user-set priority or cross-app FIFO guarantee in this slice.

## Worker task pickup

The current execution path already stores suite cases as numbered
`execution_attempts`. The Python execution worker sends a claim ID and its
registered profile, protocol version, and model capabilities to
`POST /api/worker/claims`. Rust authenticates and rechecks that worker, reconciles
expired leases, and searches queued, uncanceled attempts for the same app and
profile. Direct actions, source-manifest version, frozen model assignment, and
qualified profile must match worker capabilities. A phone authoring session can
occupy the same app/device reservation; execution and authoring have no global
priority over one another.

For eligible attempts, the total order is `(run.created_at, run.id,
claim_order, attempt.number, attempt.id)`, where `claim_order` is zero for a
saved suite and `case_index` for other run sources. Thus suite membership order
does not select the next task; attempt IDs make the choice deterministic.
Selection is work-conserving
within a worker's eligible set: an older run for an offline/incompatible profile
does not prevent a different worker from taking its own eligible work. The suite
does not promise global FIFO across apps, profiles, or device hosts.

The claim transaction locks the app and chosen attempt, skips already locked
attempt rows, and reserves both `app:<id>` and `device:<physical identity>`.
Therefore only one attempt for that app and physical device can execute at a
time. It issues a short fenced lease; the worker runs exactly the manifest case
at `case_index`, heartbeats, publishes evidence, and reports completion.
Only verified physical cleanup releases the reservations and wakes the next
claim. A failed but clean case still lets another case run. An authorized
diagnostic retry, when configured, is inserted as attempt 2 for the same case
after cleanup; saved-suite runs currently configure no diagnostic retries.
Unverified cleanup or an expired lease retains the reservation and requires
recovery; the picker does not skip ahead within that app to make the suite look
complete.

For example, a suite displaying A, B, C creates attempts `(0,1)`, `(1,1)`,
`(2,1)`. The first worker claim may get B if B's attempt ID sorts first. While
B holds the app reservation, another claim gets no work for that app. After
verified cleanup, the next claim gets one of the remaining eligible cases.
These cases never consume one another's results as prerequisites. Queue
reasons distinguish capacity busy, worker offline/incompatible, model
unavailable, and device recovery required from an app test failure.

## Ownership and design boundaries

| Owner                | Responsibility                                                                                                                                                                                                   |
| -------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `crates/contracts`   | Additive suite-run request/preview fields, endpoint declaration; Rust remains the transport source.                                                                                                              |
| `apps/api`           | Membership/admission, baseline selection, immutable run creation, pure result/comparison policy, persisted report. Reuse `suite_runs`, `case_runs`, `runs`, `run_comparisons`, and the existing execution store. |
| `apps/web`           | Explicit choices, Save & run flow, accurate suite labels and evidence display using generated SDK/types/Zod.                                                                                                     |
| `apps/mobile-worker` | Existing per-attempt device actions and cleanup. Change only to fix a reproduced defect; retain recovery fencing.                                                                                                |
| `scripts` / operator | Simulated HTTP smoke and explicit real-device acceptance in an isolated fixture workspace.                                                                                                                       |

No new queue, scheduler, case/suite table, approval state, runner framework, or
browser-owned comparison algorithm is needed. The existing nullable
`execution_runs.baseline_run_id` holds the selected baseline; changing the suite
request is additive so older browsers' omitted field still means None. Omit a
None baseline again when serializing the request for the idempotency fingerprint,
so an old no-baseline retry keeps its identity across the upgrade. Historical
run manifests and report bytes remain unchanged.

## Delivery order

1. Extend the saved-suite preview/request contract and API baseline admission by
   reusing the saved-case comparison policy. Keep the candidate query and
   compatibility rule in one shared service rather than copying a second scan.
2. Make the suite run control show explicit build/device/baseline choices and
   exact version. Preserve the current idempotent Save & run and phone-session
   cleanup behavior. Update suite/report wording and optional-failure summary.
   Add stable tie-breaks to the existing claim query and focused real-database
   tests for independent suite pickup, diagnostic retry, incompatible worker bypass, and
   reservation/recovery blocking. Do not add a second scheduler.
3. Extend route, pure policy, UI and secret-free HTTP smoke coverage for two
   cases, duplicate submission, cross-app/revoked access, immutable pins,
   restart persistence, pass/fail/blocked/incomplete, incompatible baseline,
   and no false green. Finish all edits before running the normal checks once.
4. On an explicitly qualified local demo profile, inspect the retained dirty
   state and complete operator recovery with evidence before any new claim.
   Run two independent saved cases as one suite: “saved task appears” and
   “saved task survives restart.” The known broken demo flavor displays the
   task immediately but loses it after restart, so only the second case should
   regress. Run the good build first, then the broken build with the first run
   pinned. Record every case's start/cleanup and check evidence, run IDs,
   comparison, model usage, and
   unresolved blockers. Use authorized rendered browser acceptance when access
   is available; keep that gate open if admin policy still denies it.

## Acceptance

- Saving and queuing a suite freezes its exact member versions, order, build,
  environment, profile, model assignment if used, and optional baseline. Later
  edits/uploads cannot change that run; a lost response returns the same run.
- The real good-build suite reaches a terminal report with all required cases
  conclusive and clean. The broken-build run shows the seeded failure only where
  verified, without losing the other case's result. A missing prerequisite or
  uncertain cleanup is visibly incomplete and never turns the suite green.
- The second run's pinned baseline survives API restart. Comparable case deltas
  use the existing 07B policy; changed test/context is visibly Not comparable.
- Every queued/started/finished case is accounted for in the run map and detail.
  The report says Saved suite, shows build/device and evidence, and does not use
  the first case title as the suite title.
- Automated contract/route/policy/browser/worker checks and simulated HTTP smoke
  pass. Real device and rendered browser acceptance are recorded separately;
  synthetic checks are never reported as live proof.

## Non-goals

Customer APK qualification, remote account reset, audited triage, retention,
hosted deployment, a reliability campaign, new test generation, suite-to-suite
dependencies, and automatic baseline approval remain in their owning phases.
