# Implementation report: 04 — Execution and reporting

Date: 2026-09-12. Branch: `codex/04-execution-and-reports`.
Status: local implementation and simulated HTTP acceptance verified; phase acceptance remains open.

## Summary

Operators can import immutable versioned cases, suites and plans, grant scoped review
authority and approve exact content hashes. A browser run freezes the approved plan,
APK checksum, environment revision and qualified profile. PostgreSQL queues attempts
and reserves resources; authenticated Python workers claim leases, publish semantic
events and private artifacts, and acknowledge cleanup. Rust independently evaluates
retained evidence. Tests/Runs pages expose approved actions, expected/observed checks,
attempt history, cancellation, screenshots and simulation labels.

The runnable adapter is `demo_persistence_v1`, for `ai.mobileqa.demo`. A navigate action
passes its approved instruction to pinned Minitap; restart_app performs a deterministic
force-stop/launch; each checkpoint captures PNG/XML. Required checks inspect retained
hierarchies, readiness markers and prerequisite checks. SDK completion never proves pass.
The phase 05 authoring editor and arbitrary customer login/reset adapters remain outside
this slice.

## Assessment versus plan

| Area            | Planned                                      | Actual                                                                   |
| --------------- | -------------------------------------------- | ------------------------------------------------------------------------ |
| Complexity      | XL; confidence 7/10                          | XL across contracts/API/worker/web/docs                                  |
| Baseline        | Main plus phase 02 integration               | `d4c4729`, including main `9eb5dba`                                      |
| Local execution | Same HTTP protocol for fake and real adapter | Implemented; fake verified through real HTTP                             |
| Acceptance      | Browser/device scenarios, outage/recovery    | Deterministic tests and API restart smoke verified; real acceptance open |

## Tasks

| Task                      | Status                               | Evidence / deviation                                                      |
| ------------------------- | ------------------------------------ | ------------------------------------------------------------------------- |
| 1. Baseline/contracts     | Implemented                          | Integrated branch; generated Rust → OpenAPI/Zod and Pydantic              |
| 2. Definition persistence | Implemented                          | Typed immutable version store, approvals and grants                       |
| 3. Operator/readiness     | Implemented                          | Import/grant/approve/profile/worker/reconcile/recover commands            |
| 4. Submission/manifests   | Implemented                          | Scoped immutable manifest, idempotency and pagination                     |
| 5. Claims/identity        | Implemented                          | Worker bearer identity, atomic reservations, fenced leases                |
| 6. Lifecycle/recovery     | Implemented                          | Ordered dedup events, cancellation, quarantine, explicit recovery/retries |
| 7. Private evidence       | Implemented                          | Streaming caps/hash, immutable publication and fenced sealing             |
| 8. Python transport       | Implemented                          | Bounded same-origin HTTP, durable journal and heartbeat supervisor        |
| 9. Device actions         | Implemented; real validation pending | Reused qualified demo lifecycle and shared Minitap navigation seam        |
| 10. Verification/report   | Implemented                          | Independent PNG/XML check results and immutable attempts                  |
| 11. UI                    | Implemented                          | Approved preview, read-only Tests, run list/detail/cancel                 |
| 12. Fake/regression       | Verified locally                     | Real HTTP passed/failed/blocked and restart persistence                   |
| 13. Docs/commands/CI      | Implemented                          | Explicit fake/smoke commands; 17 CI selection cases                       |
| 14. Acceptance window     | Partially verified                   | Local checks pass; foundation smoke blocked; real/hosted gates open       |

## Validation

Checks ran after the initial coherent source/test/config batch. Failures were corrected
and only affected checks were repeated. Counts below include successful targeted reruns.

| Check                                             | Result                                                                                  |
| ------------------------------------------------- | --------------------------------------------------------------------------------------- |
| Rust format / Clippy all targets, warnings denied | Passed                                                                                  |
| Rust tests                                        | 29 passed: 18 API unit/route tests and 11 contract tests                                |
| TypeScript / ESLint                               | Passed                                                                                  |
| Frontend tests                                    | 69 passed                                                                               |
| Worker Ruff / strict Pyright                      | Passed                                                                                  |
| Worker tests                                      | 115 passed, including 14 execution tests                                                |
| Contract generation/check + exporter test         | Passed; zero generated drift                                                            |
| CI path selection                                 | 17 cases passed                                                                         |
| Rust workspace and production web builds          | Passed; existing Vite chunk-size warning                                                |
| `just smoke-execution`                            | Passed; real HTTP/Python/PostgreSQL, simulated evidence only                            |
| `just smoke`                                      | Blocked before startup by existing listeners on 5150/5173; no unrelated process stopped |
| `pnpm --dir apps/web audit --prod`                | No known vulnerabilities                                                                |
| `cargo audit`                                     | Existing RUSTSEC-2023-0071 in rsa 0.9.10 remains; audit is not clean                    |
| Real browser/device/model/cloud/S3 acceptance     | Not run                                                                                 |

The smoke imports a signed **synthetic intake fixture**, not a runnable demo APK. It
queues a run, restarts the API, executes the Python HTTP fake for passed/failed/blocked,
checks private evidence metadata and cleanup, restarts again and compares saved reports.
It establishes transport and persistence, not device reliability or hosted storage.

## Test coverage added

- `crates/contracts/tests/execution.rs`: 5 tests covering definitions, invalid references,
  unsupported/manual checks, unknown imported fields and real profile APK capacity.
- `apps/api/tests/execution.rs`: 5 route/database tests covering authorization inventory,
  immutable approvals, competing claims, idempotency, fencing, cancellation/expiry,
  evidence upload/replay and passed/failed/blocked independent evaluation; malformed XML/PNG.
- `apps/mobile-worker/tests/test_execution.py`: 14 tests covering generated job data,
  fake evidence, bounded HTTP/redirects, private journals/no replay, SDK stop failure and
  approved checkpoint deadlines. Existing qualification/SDK tests remain passing.
- `apps/web/src/api/runs.test.ts` and `pages/RunDetail.test.tsx`: 4 tests covering generated
  transport and report behavior. Setup/Home expectations now match the implemented UI.
- Existing health route inventory now includes the five additional browser paths.

This is meaningful subsystem coverage, not proof of every fault permutation listed in
the planning packet. Live worker interruption/reset, concurrent cancel-versus-complete,
remote storage failures and rendered browser acceptance need further acceptance evidence.

## Deviations and limits

1. One typed `execution_definitions` table replaces separate case/suite/plan root/version
   tables. Tagged Rust variants retain domain distinctions without duplicate persistence code.
2. Attempts are the durable queue records; manifest case indices identify executions.
   Bound raw SQL through SeaORM replaces a large set of trivial generated ORM entities.
3. The real executor reuses phase 02 demo Device/Evidence/host locking, with a shared SDK
   navigation seam. A broad generic device runtime extraction would imply unsupported
   customer adapters; package/account/reset readiness remains deliberately restricted.
4. Customer-visible evidence is normalized checkpoint PNG/XML and semantic events.
   Raw SDK traces stay private. Video/log ingestion is not part of worker publication here.
5. A restarted worker refuses an active journal instead of automatically replaying actions
   or resuming transport. Operator recovery is required after ambiguous loss of ownership.
6. Plan duration admission covers declared attempt budgets and retries; queue waiting and
   cleanup are separate from active case time. This is not an end-to-end wall-clock SLA.
7. PNG validation checks framing, CRC, dimensions and required chunks, not decoded pixels;
   XML check methods establish outcomes. Visual assertions remain unsupported.
8. Real device/API/browser acceptance, the full Minitap reliability campaign and hosted
   storage qualification remain open. Nothing was pushed, merged or deployed.

## Issues fixed during validation

SeaORM 2 raw query method names and quick-xml's normalization API were corrected.
The private storage key now uses the store's required four UUID components. The HTTP
roundtrip test caught that mismatch. Obsolete placeholder/path-count tests were updated.
Unconfirmed SDK process termination now prevents clean reset/reuse. Malformed/truncated
hierarchies cannot establish a result. Real profile upload capacity matches the existing
100 MiB demo installer, and checkpoint waits honor approved observation limits.

A worker check accidentally launched from repository root used the wrong Pyright scope;
it was rerun from `apps/mobile-worker` with the configured strict source scope, passing.

## Remaining acceptance

- Run the foundation smoke when existing development ports are available.
- In an allowed browser and explicitly scoped real device/model session, execute good,
  broken and unavailable-backend cases through the dashboard; verify cancellation,
  interruption, private evidence and physical cleanup/quarantine.
- Exercise hosted storage and retain the separate full reliability campaign evidence.
- Review the local diff before any commit/PR action. Recheck scoped dependency audits
  before committing, including Python; no Python dependency versions changed here.

The plan remains in `plans/` rather than `completed/`: open acceptance is not represented
as completion. Architecture status and the phase spec point to this report.

## File inventory

77 modified/new files (including plan/report and generated outputs).

| File                                                                   | Action  | Current lines |
| ---------------------------------------------------------------------- | ------- | ------------- |
| `.claude/PRPs/plans/04-execution-and-reports.plan.md`                  | Created | 544           |
| `.claude/PRPs/reports/04-execution-and-reports-report.md`              | Created | 139           |
| `.github/workflows/ci.yaml`                                            | Updated | 237           |
| `Cargo.lock`                                                           | Updated | 6539          |
| `apps/api/Cargo.toml`                                                  | Updated | 42            |
| `apps/api/migration/src/lib.rs`                                        | Updated | 21            |
| `apps/api/migration/src/m20260912_000004_execution.rs`                 | Created | 33            |
| `apps/api/src/app.rs`                                                  | Updated | 81            |
| `apps/api/src/controllers/mod.rs`                                      | Updated | 8             |
| `apps/api/src/controllers/runs.rs`                                     | Created | 127           |
| `apps/api/src/controllers/worker.rs`                                   | Created | 161           |
| `apps/api/src/services/execution_store.rs`                             | Created | 59            |
| `apps/api/src/services/mod.rs`                                         | Updated | 23            |
| `apps/api/src/services/run_artifacts.rs`                               | Created | 234           |
| `apps/api/src/services/runs.rs`                                        | Created | 355           |
| `apps/api/src/services/scheduler.rs`                                   | Created | 397           |
| `apps/api/src/services/test_definitions.rs`                            | Created | 302           |
| `apps/api/src/services/verification.rs`                                | Created | 265           |
| `apps/api/src/services/worker_auth.rs`                                 | Created | 67            |
| `apps/api/src/tasks/execution.rs`                                      | Created | 104           |
| `apps/api/src/tasks/mod.rs`                                            | Updated | 6             |
| `apps/api/tests/app_setup.rs`                                          | Updated | 890           |
| `apps/api/tests/execution.rs`                                          | Created | 464           |
| `apps/api/tests/fixtures/execution/synthetic.png`                      | Created | binary        |
| `apps/api/tests/health.rs`                                             | Updated | 48            |
| `apps/api/tests/support/mod.rs`                                        | Created | 194           |
| `apps/mobile-worker/src/mobile_qa_worker/cli.py`                       | Updated | 132           |
| `apps/mobile-worker/src/mobile_qa_worker/execution/__init__.py`        | Created | 1             |
| `apps/mobile-worker/src/mobile_qa_worker/execution/actions.py`         | Created | 249           |
| `apps/mobile-worker/src/mobile_qa_worker/execution/client.py`          | Created | 88            |
| `apps/mobile-worker/src/mobile_qa_worker/execution/fake.py`            | Created | 46            |
| `apps/mobile-worker/src/mobile_qa_worker/execution/journal.py`         | Created | 28            |
| `apps/mobile-worker/src/mobile_qa_worker/execution/runner.py`          | Created | 306           |
| `apps/mobile-worker/src/mobile_qa_worker/execution/sdk_adapter.py`     | Created | 24            |
| `apps/mobile-worker/src/mobile_qa_worker/generated/models.py`          | Updated | 568           |
| `apps/mobile-worker/src/mobile_qa_worker/qualification/device.py`      | Updated | 378           |
| `apps/mobile-worker/src/mobile_qa_worker/qualification/sdk_adapter.py` | Updated | 186           |
| `apps/mobile-worker/tests/test_execution.py`                           | Created | 220           |
| `apps/web/src/api/generated/index.ts`                                  | Updated | 4             |
| `apps/web/src/api/generated/sdk.gen.ts`                                | Updated | 391           |
| `apps/web/src/api/generated/types.gen.ts`                              | Updated | 2310          |
| `apps/web/src/api/generated/zod.gen.ts`                                | Updated | 840           |
| `apps/web/src/api/runs.test.ts`                                        | Created | 25            |
| `apps/web/src/api/runs.ts`                                             | Created | 26            |
| `apps/web/src/api/session-transport.ts`                                | Created | 13            |
| `apps/web/src/api/setup.ts`                                            | Updated | 188           |
| `apps/web/src/components/app/run-preview.tsx`                          | Created | 40            |
| `apps/web/src/pages/AppDetail.test.tsx`                                | Updated | 237           |
| `apps/web/src/pages/AppDetail.tsx`                                     | Updated | 288           |
| `apps/web/src/pages/Home.test.tsx`                                     | Updated | 206           |
| `apps/web/src/pages/RunDetail.test.tsx`                                | Created | 51            |
| `apps/web/src/pages/RunDetail.tsx`                                     | Created | 37            |
| `apps/web/src/pages/Runs.tsx`                                          | Created | 27            |
| `apps/web/src/pages/Tests.tsx`                                         | Created | 24            |
| `apps/web/src/routes.tsx`                                              | Updated | 53            |
| `apps/web/tsconfig.tsbuildinfo`                                        | Created | 1             |
| `contracts/browser.openapi.json`                                       | Updated | 6219          |
| `contracts/fixtures/execution/persistence-case.json`                   | Created | 18            |
| `contracts/worker.schema.json`                                         | Updated | 1540          |
| `crates/contracts/src/browser.rs`                                      | Updated | 828           |
| `crates/contracts/src/execution.rs`                                    | Created | 584           |
| `crates/contracts/src/execution_api.rs`                                | Created | 104           |
| `crates/contracts/src/lib.rs`                                          | Updated | 9             |
| `crates/contracts/src/worker.rs`                                       | Updated | 134           |
| `crates/contracts/src/worker/execution.rs`                             | Created | 20            |
| `crates/contracts/tests/execution.rs`                                  | Created | 66            |
| `docs/architect/contracts.md`                                          | Updated | 119           |
| `docs/architect/dependencies.md`                                       | Updated | 141           |
| `docs/architect/development.md`                                        | Updated | 235           |
| `docs/architect/environment.md`                                        | Updated | 133           |
| `docs/architect/implementation/04-execution-and-reports.md`            | Updated | 84            |
| `docs/architect/status.md`                                             | Updated | 290           |
| `docs/architect/system.md`                                             | Updated | 103           |
| `justfile`                                                             | Updated | 93            |
| `scripts/apk_fixtures.py`                                              | Updated | 55            |
| `scripts/execution_smoke.py`                                           | Created | 112           |
| `scripts/test_ci_scope.py`                                             | Updated | 42            |

## PR preparation follow-up

Repository formatting now uses `just format` / `just format-check`: pinned Prettier
for supported handwritten web/config/docs files, rustfmt for Rust, and Ruff for
Python source and scripts. A dedicated CI job checks the same Git-visible file set.
Generated contracts, dependency locks and private/build outputs are excluded.
This expands the original file inventory with repository-wide formatting changes.

Pre-PR verification repeated the affected format/static/test checks and confirmed
zero contract drift. Frontend audit found no known vulnerabilities. The installed
Python dependency audit found none; the local mobile-qa-worker package is not on
PyPI and was skipped. The documented Rust rsa advisory remains open. The PR is
stacked on phase 02's branch while PR #2 remains open. Real/browser/hosted gates
remain unchanged.

## Long-poll follow-up

Claims now wait up to 30 seconds with a 35-second worker HTTP timeout. A watch
notification wakes local waiting requests after run creation or resource-release
commit; a five-second database fallback handles other processes. No transaction
is held while waiting, and worker revocation is rechecked before another claim.
Idle replies request immediate reconnect. Heartbeat timeouts remain five seconds.

Validation: Clippy, Ruff/Pyright, format checks, seven execution route/database
tests, health route test, 15 worker execution tests, Rust build and simulated
HTTP/restart smoke passed. New coverage proves wake-on-commit before the fallback,
idle timeout without reservation, revocation after wake and claim-only timeout
extension. The runbook and `just dev-execution-real` explain real host launch;
no emulator/model acceptance was performed.
