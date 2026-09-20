# 07 — Regression, pilot readiness and scaling

Status: implementation planned; detailed planning and design review complete (2026-09-15). Depends on: 06; operational safeguards begin in earlier specs. Owns: comparison/triage services, operational settings, deployment/runbooks and qualification evidence.

## Deliverable

An operated pilot for one Android app can repeat a release check, understand changed failures, recover from interruptions and measure the actual cost of service delivery.

## Regression and triage

Compare runs only with explicit compatibility checks: case/expectation version, data variant, device configuration and relevant environment changes. Show newly failing, still failing, recovered, unchanged and not comparable. A changed expected result cannot masquerade as a fixed defect.

Retain all attempts and findings. Let reviewers classify an app defect, infrastructure/prerequisite problem, test-definition issue or unresolved outcome. Review/triage labels do not rewrite recorded evidence. Reports show coverage gaps and required incomplete cases beside failures.

## Settings needed for the pilot

- Project access and reviewer/operator identity.
- Test environment, permitted origins and masked secret references.
- Per-run deadline, retries, concurrency and cost limits.
- Artifact retention/deletion, with private access and report export review.

Use proposed defaults from the product spec: 14-day raw build/artifact retention and reviewed reports/metadata expiring 30 days after pilot close unless renewed. Make actual policy visible. Retention jobs must handle objects, database references and backup expiry; immediately revoke access when deletion is requested. Do not promise immediate deletion from retained backups.

## Initial production shape

Build the React assets once and serve them with the Rust application at the same origin. Deploy one application instance initially, managed PostgreSQL, private object storage, and a separate qualified device host. Use immutable release artifacts and versioned configuration; apply reversible/backward-compatible migrations where feasible. Back up the database and rehearse restore before relying on stored customer reports.

Manually provision one pilot worker/device and account assignment. No auto-scaling controller, Redis, Kafka, Temporal or Kubernetes initially. Generation capacity is bounded separately from HTTP requests; device jobs always run outside the web process.

## Reliability gate

Use the product spec's controlled demo evaluation: at least ten fixed case/configuration scenarios, each repeated five times from verified clean state. Include known-good behavior, seeded defects, missing prerequisites, timeout, contamination and interruption. Maintain held-out scenarios for release evaluation.

Proposed acceptance: zero false passes on the defined broken/unmet-prerequisite scenarios; every planned case accounted for; at least 90% conclusive correct outcomes for known-good cases. Report raw counts and all failures. This small evaluation is a pilot gate, not a claim of universal accuracy. Rerun relevant qualification when the agent, model, emulator image or verifier changes.

Security/access gates: cross-project isolation at APIs and artifacts, narrow worker credentials, no secrets in evidence/logs, no public ADB, cancellation and uncertain-lease recovery exercised. Review raw evidence before sharing customer report exports.

## Metrics and economics

Record queue wait, boot/install/run/reset time, device minutes, retries, model usage/cost, artifact volume and operator/customer minutes. Track blocked/inconclusive reasons and false-pass findings, not just completion rate.

Calculate cost per delivered report from allocated host idle time + active execution + model calls + storage/transfer + human review. Measure successful and failed attempts. Price the pilot against observed costs and coverage; defer “unlimited” plans and automated billing.

## Scale only at observed bottlenecks

| Signal                                                       | Smallest next change                                                                    |
| ------------------------------------------------------------ | --------------------------------------------------------------------------------------- |
| Queue wait misses the agreed turnaround while device is busy | Add another qualified worker/device and matching test-account capacity                  |
| Slow cold boot dominates                                     | Maintain a small warm pool with verified reset and an idle-cost cap                     |
| Web latency rises independently of device load               | Add an API replica; shared leases/sessions/storage must already work across replicas    |
| Generation consumes web process resources                    | Move existing Rust generation jobs into a separate process                              |
| Job queries show measured contention                         | Tune indexes/claim batching; consider dedicated queue infrastructure only with evidence |
| First customer requires another platform/ABI                 | Qualify that device provider and adapter before selling coverage                        |

No per-customer database or service is needed by default. Preserve tenant scope and isolated device/account resources; do not add concurrency before reset and backend test-data isolation are reliable.

## Pilot handoff acceptance

One customer app completes onboarding, a validated run, a new-build rerun and a reviewed comparison report. Record at least one actionable finding or explicit coverage limitation, actual turnaround/cost and customer feedback. Provide operator instructions for install failure, stuck worker, leaked lease, missing evidence, secret rotation and deletion. This is the point to sell a repeatable supported service with measured scope.

## Implementation increments agreed on 2026-09-15

The next implementation starts with reliable replay and build comparison. Phase08 AI
flow discovery is already in merged source; it does not prove clean-reset replay on
arbitrary APKs. The current execution, profile admission, app launch and reset paths
still contain demo-specific rules. Isolate those rules before qualifying a real app.

Pending implementation plan:
[07 regression and pilot readiness](../../../.claude/PRPs/plans/07-regression-pilot-readiness.plan.md).
This working plan remains available for implementation; remove the link and completed
working file after implementation details and evidence are incorporated here and in status.

| Increment | Scope                                                                                                            | Acceptance boundary                                                                                           |
| --------- | ---------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| 07A       | Shared Android device operations, dedicated demo adapter, qualified direct adapter, durable start/reset evidence | One explicitly qualified local-state app; missing start proof or uncertain cleanup cannot become a clean pass |
| 07B       | Frozen baseline, deterministic comparison, compact report and audited triage                                     | A/B build runs retain all attempts; changed expectations/context never appear recovered                       |
| 07C       | Versioned limits, measured timing/cost, private export and retention/deletion                                    | Unknown costs remain unknown; access revocation precedes safe object deletion                                 |
| 07D       | Reliability campaign, deployment/runbook preparation and actual pilot handoff                                    | Defined 50-run gate plus an authorized non-demo app's repeated release report; hosted gates remain separate   |

Do not treat an increment's completion as completion of the operated pilot. No customer
APK or reset procedure is implied by the sample fixture. Until supplied and qualified,
real-app acceptance remains pending. First direct support is local-state-only; account
or remote backend mutation/reset requires a separately qualified app adapter.

### Clean code boundaries

Keep the existing Rust/Loco/SeaORM application, React/Mantine UI and Python worker.
No new domain framework, generic repository hierarchy, job queue or dependency-injection
container is required.

- Pure regression policy matches cases, checks compatibility and classifies deltas.
  It has no database, network, clock or framework dependency.
- Application services authorize both runs, load immutable snapshots, call the policy
  and persist baseline/triage records. A focused store owns parameterized SeaORM queries.
- Existing scheduling, leases, evidence verification and artifact storage retain their
  responsibilities. No AI call determines a replay verdict or comparison result.
- A narrow shared Android device boundary owns host/AVD/install/launch/capture I/O.
  Demo-specific backend, activity and preferences checks remain in a dedicated adapter.
  The new direct adapter uses qualified package/start assertions and a fresh owned AVD.
- React consumes generated comparison DTOs and owns only selection, filtering, pane
  width and disclosure state. It never reimplements compatibility or pass/fail policy.

New execution context is versioned and optional for historical records. Missing proof
is unknown, never implicitly verified. New adapter jobs require compatible worker
protocol support before leasing. Start receipts are sealed and acknowledged before
actions; stop/discard receipts govern device reuse. Recovery records preserve the
original failure. Lost mutation responses never authorize a blind retry.

### Comparison rules

The first 07B delivery is specified in
[Regression labels and durable test history](07b-regression-label-and-history.md).
It closes the editor/Runs history split before adding the Regression badge. The
specification now includes local implementation and acceptance evidence; the remaining
audited-triage scope stays here.

Match a case by app, stable case key and data variant. Require exact case version/hash,
environment revision and qualified execution signature (including reset/verifier/runtime)
for a comparable result. Build checksum may differ; that is the comparison axis.
Added/removed cases and changed expectations remain visible, with explicit reasons.

Passed→failed is newly failing; failed→failed is still failing; failed→passed is
recovered; passed→passed is unchanged. Unknown, incomplete or incompatible context is
not comparable. A failed attempt remains visible even if a diagnostic retry passes.
Required/optional coverage is a separate dimension from these five delta categories.
Comparison remains provisional until both runs finish with verified start and cleanup.

Suggest the latest eligible earlier completed run, ordered by timestamp with a stable
ID tie-breaker. Show its build/date before submission and freeze the chosen ID in the
new run. First runs need no baseline. A suggested baseline is not an accepted release
or automatic reviewer approval. Triage records app defect, setup/device issue,
test-definition issue or unresolved as a separate audited annotation; it never edits
machine outcomes.

### Accepted compact report design

The gan-design generator/evaluator loop accepted iteration 2 at **8.335/10** against a
7.5 threshold. This is source/wireframe design acceptance, not rendered browser proof.
Temporary GAN Markdown was removed after the decisions were applied here and in the plan.

Use the established forest/sage palette and typography, narrow navigation and full
workspace width. A roughly 48px header and 44px context strip precede a two-pane report.
The left pane starts around 38% width, minimum 320px, and contains readable two-line
result rows around 56–64px high. The right displays only the selected test's paired
baseline/current evidence and expected/observed result. Keep the run action visible.

```text
Build 1.4  vs Build 1.3 · suggested baseline        [Run tests]
Smoke · 6 required tests · Android 35 · Clean start
┌──────────────────────────┬────────────────────────────────────┐
│ New 1 Recovered 1 Gaps 1     │ Save task — Newly failing          │
│                          │ Expected: task appears after save   │
│ Save task       New      │ Observed: task absent               │
│ Restart         Recovered│ [Baseline capture] [Current capture]│
│ Open app        Unchanged│                                    │
│ Login           Incomplete│ Triage: Unresolved                 │
│                          │ ▸ Actions & attempts ▸ Run details  │
└──────────────────────────┴────────────────────────────────────┘
```

Counts are illustrative. Actual totals reconcile to the union of compared cases and
show required incomplete coverage explicitly. Use status words/symbols as well as color.
Select the first actionable result initially, then preserve user selection during updates.
Evidence is tied to matching action/checkpoint identity; absent/expired/redacted evidence
has explicit text. Never show unrelated screenshots as a matching pair.

Before submission, Change opens a compact baseline picker. A completed report offers
View baseline only; changing the baseline requires another run. Fully incompatible
candidates explain why they cannot be selected; otherwise compatible runs can include
changed or missing cases with explicit not-comparable rows.

A failed cleanup keeps raw assertion evidence visible but prevents Recovered and pauses
new work on that device. Tester copy explains the interruption; operator recovery details
stay outside the main tester flow. Triage uses App defect / Setup or device issue /
Test needs editing / Unresolved, with one explicit annotation save action.

Collapse actions, all attempts, raw usage, hashes and extra artifacts. No large warning
stack or repeated form sections. On narrow screens show the list first and an inspector
with Back to tests; within a narrow inspector use Baseline/Current tabs rather than
unreadable paired phone thumbnails. Desktop divider supports keyboard resizing and
focus; icon controls have accessible names. No artificial live cursor in a historical
report. Active progress comes from actual worker receipts and evidence timestamps.

### Explicit remaining gates

Keep the 10-scenario × 5-repeat oracle outside the worker; include good behavior,
seeded defects, blocked prerequisites, timeout, contamination and interruption.
The existing smaller fixture campaign is not this acceptance gate. No ordinary
check invokes an emulator or model. Real qualification, rendered review and hosted
backup/restore evidence must each be reported separately.

Retention must inventory APKs, execution evidence, phone-session JSON screenshots,
exports and worker scratch, not only one artifact table. Revoke access before deletion;
hold dirty/active resources until safely stopped. Preserve deletion tombstones through
restore and disclose backup expiry. Initial scaling stays manual: one qualified worker
and measured queue/cost/reliability signals before adding capacity.

### 07A implementation notes — 2026-09-15

The first increment now separates generic Android operations (`device/android.py`) from
controlled demo qualification (`qualification/device.py`). `execution/adapters.py`
selects the two explicit policies; `execution/lifecycle.py` owns the new direct flow.
`execution_readiness.rs` admits saved cases against immutable qualification context.
`execution_preflight.rs` verifies sealed start artifacts under lease fencing before
accepting action events/evidence. The supervisor is the only process with worker
credentials; the child waits for an instance-bound private acknowledgement file.

Migration000007 adds immutable start receipts and append-only recovery history. Legacy
manifests omit absent context and preserve their serialization; protocol1/2 execution
workers cannot claim new profiles, and phone workers need protocol4 for those profiles.
Legacy worker completion/cleanup responses omit the newly added history fields; browser
reports include them. New direct runs require start proof even for automatic reservation
release; a failed start with local disposal still needs recovery acknowledgement.

Cleanup uses disposable-AVD qualification rather than a second boot on every generic
attempt. The demo's stronger fixture reset remains intact. Blank credential references
do not qualify a remote app; only explicitly declared `local_only` state is supported.
A real customer APK campaign is still required. 07B–D remain planned.

UI decisions: status words for clean start and cleanup remain independent of the raw
assertion verdict; canceled or historical missing receipts never imply a verified start.
Recovery does not hide initial cleanup failure. Start evidence appears only in the
collapsed lifecycle disclosure, avoiding duplicate full-height screenshots.

### 07A validation and implementation report

Tasks 1–5 are implemented in source. The work remains XL across the full four-increment
plan; this delivery covers the first increment only. The generic Android adapter,
qualification admission, receipt persistence and recovery projection are separate modules;
HTTP controllers remain transport boundaries. No new domain framework or model dependency
was introduced. Generated schemas remain the single transport source.

Validation on 2026-09-15: contract drift and exporter helper, Clippy, Rust 49 tests,
TypeScript/ESLint, browser 116 tests, Ruff/Pyright, worker 159 tests, CI-scope tests,
format checks, API/web builds, execution HTTP smoke and direct-authoring HTTP smoke.
The worker total includes a successful isolated retry of the existing intermittent
loopback HTTP test. No production workaround was added for that intermittent failure.
All other suites passed after the consolidated pass's import/type/route-inventory fixes.

New coverage includes legacy context omission, exact profile qualification, old/new
protocol admission, real route authentication and generation fencing, immutable start
receipt retries, sealed PNG/XML starting assertions, refusal to accept actions before
proof, cleanup/recovery history, conservative recovered-run summaries, two-package fake
ADB installation, password redaction, independent lifecycle stage failures and an
instance-bound supervisor acknowledgement. The five readiness DOM tests cover verified,
missing, canceled, simulated and recovery presentation. The HTTP smokes use a simulated
worker; neither those smokes nor fake ADB tests establish a qualified customer APK.

GAN final source/DOM assessment: Design 8.2, Originality 7.6, Craft 8.0, Functionality 8.4;
weighted 7.98 against 7.5. The accepted revision distinguishes an absent clean-start receipt
from a verified start, preserves original cleanup after recovery, and keeps preflight
screenshots in one collapsed disclosure. Rendered visual acceptance remains open under
the browser restriction. Temporary GAN Markdown was removed after applying these decisions.

Scope decisions: disposable-AVD cleanup replaces an extra reset boot for this explicitly
qualified local-only policy; the demo fixture reset remains separate. Attempt membership
continues to use stored case_index when reading the manifest; a public comparison index
belongs to 07B. Reports remain in this canonical document, rather than adding completed
plan/report files. The full plan remains pending for 07B–D. Real non-demo APK qualification,
the reliability campaign and hosted acceptance are not claimed by this delivery.


### 07B first delivery — 2026-09-20

Saved-case admission, durable history and conservative persisted regression comparisons
are implemented locally. See the [owning specification and implementation report](07b-regression-label-and-history.md#implementation-report--2026-09-20)
for contracts, compatibility decisions, validation and device acceptance. This delivery
adds no review gate and does not complete audited triage or the later operational phases.
