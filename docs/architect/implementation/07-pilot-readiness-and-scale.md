# 07 — Regression, pilot readiness and scaling

Status: planned. Depends on: 06; operational safeguards begin in earlier specs. Owns: comparison/triage services, operational settings, deployment/runbooks and qualification evidence.

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

One customer app completes onboarding, an approved run, a new-build rerun and a reviewed comparison report. Record at least one actionable finding or explicit coverage limitation, actual turnaround/cost and customer feedback. Provide operator instructions for install failure, stuck worker, leaked lease, missing evidence, secret rotation and deletion. This is the point to sell a repeatable supported service with measured scope.
