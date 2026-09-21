# Implementation Report: Shared Model Registry and Resolver

## Summary

Implemented one API-owned, immutable, nonsecret registry for model identity. Exact typed model
references now resolve to frozen assignments before runs and phone sessions are queued. Protocol-5
workers advertise their qualified reference and supported providers; incompatible workers remain
unleased before device, Doppler or provider side effects. Python provider construction and
credential allowlisting now live in one runtime module, and browser gates/audit views consume the
same generated capabilities and frozen identity without adding a customer model picker.

## Assessment vs Reality

| Metric                |                  Predicted (Plan) |                                                                                                                            Actual |
| --------------------- | --------------------------------: | --------------------------------------------------------------------------------------------------------------------------------: |
| Complexity            |                                XL |                                                                                                                                XL |
| Implementation slices |                                 5 |                                                                                                                                 5 |
| Files changed         | 66 source/test/docs + 7 generated | 102 dirty worktree entries total; this includes preserved pre-existing React Flow suite work and overlapping generated/docs files |

## Tasks Completed

|   # | Task                                       | Status           | Notes                                                                                                                                                                    |
| --: | ------------------------------------------ | ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
|   1 | Architecture and contract vocabulary       | Complete         | Added typed references, definitions, resolved snapshots, capabilities and legacy-read binding.                                                                           |
|   2 | Registry persistence and operator workflow | Complete         | Added immutable migration plus register/show/retire trusted task operations.                                                                                             |
|   3 | API resolution and frozen assignments      | Complete         | Profile registration, run/session creation and authoring gates use the registry service.                                                                                 |
|   4 | Protocol-5 compatibility fencing           | Complete         | Claims persist safe heartbeats and match exact frozen references before reservation; phone options require a recent compatible advertisement for model-enabled profiles. |
|   5 | Central Python model runtime               | Complete         | Provider translation, credentials, factories and usage identity are centralized in `model_runtime.py`.                                                                   |
|   6 | Scripts and fixtures                       | Complete         | Direct setup omits model sentinels; model-enabled local execution consumes a validated resolved-model document.                                                          |
|   7 | Capability-driven browser                  | Complete         | Explicit capabilities gate AI actions; profile labels and run audit identity add no selection click.                                                                     |
|   8 | Cross-boundary tests                       | Complete         | Added contract, registry, claim, worker factory, browser capability and audit coverage.                                                                                  |
|   9 | Contract generation                        | Complete         | Generated browser OpenAPI/SDK/Zod and worker schema/Pydantic once and checked drift.                                                                                     |
|  10 | Validation and rollout docs                | Partial evidence | All non-database gates pass. PostgreSQL integration and smoke execution are blocked by retained local migration history, not a source/build failure.                     |

## Validation Results

| Level                    | Status                    | Notes                                                                                                                                                                                                  |
| ------------------------ | ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Static analysis          | Pass                      | Browser TypeScript/ESLint, worker Ruff/Pyright, Rust formatting and contract drift pass.                                                                                                               |
| Contract tests           | Pass                      | 8 targeted tests pass.                                                                                                                                                                                 |
| Browser tests            | Pass                      | 25 files / 149 tests pass, including frozen and historical run audit states.                                                                                                                           |
| Worker tests             | Pass with targeted reruns | Full run collected 179 tests and reported two failures; both exact tests pass on rerun. One was the known local HTTP boundary flake; one stale fixture was corrected for the protocol-5 session shape. |
| API unit/compile         | Pass                      | 11 library tests pass and every API integration binary compiles with `--no-run`.                                                                                                                       |
| API database integration | Environment blocked       | The retained `mobile_qa_test` ledger contains `m20260918_000008_regression`, absent from this worktree. No database reset or destructive migration was performed.                                      |
| Model-free HTTP smokes   | Environment blocked       | The shared smoke harness reaches the same migration-history failure before serving requests.                                                                                                           |
| Build                    | Pass                      | Rust and production web builds pass; the existing Vite chunk advisory remains.                                                                                                                         |
| Diff/security review     | Pass                      | `git diff --check` passes; no private files, temporary GAN artifacts or secret-shaped values were found.                                                                                               |
| Real runtime acceptance  | Not run                   | No device, provider/model, Doppler or rendered-browser acceptance was authorized or performed.                                                                                                         |

## Files Changed

The table lists the principal model-registry changes. Generated contract files and existing suite
UI edits remain visible in the shared worktree and were preserved.

| File or group                                                                                          | Action    | Purpose                                                                                 |
| ------------------------------------------------------------------------------------------------------ | --------- | --------------------------------------------------------------------------------------- |
| `crates/contracts/src/model_registry.rs`                                                               | Created   | Canonical model identity, definitions, capabilities, bindings and worker advertisement. |
| `crates/contracts/src/{execution,task_sessions,worker,test_library}.rs`                                | Updated   | Frozen assignment, protocol-5 and usage transport shapes.                               |
| `apps/api/migration/src/m20260921_000010_model_registry.rs`                                            | Created   | Immutable definitions and worker capability heartbeat columns.                          |
| `apps/api/src/services/model_registry.rs`                                                              | Created   | Registration, retirement, legacy lookup, capability resolution and worker matching.     |
| `apps/api/src/services/{runs,scheduler,task_sessions,test_authoring,test_definitions,test_library}.rs` | Updated   | Resolution/freeze and pre-side-effect admission.                                        |
| `apps/api/src/tasks/execution.rs`                                                                      | Updated   | Trusted register/show/retire model workflow.                                            |
| `apps/mobile-worker/src/mobile_qa_worker/model_runtime.py`                                             | Created   | Sole provider/credential/factory boundary.                                              |
| `apps/mobile-worker/src/mobile_qa_worker/{execution,authoring,qualification,task_sessions}.py`         | Updated   | Consume frozen assignments and typed host references.                                   |
| `apps/web/src/lib/model-capabilities.ts`                                                               | Created   | Pure capability lookup over generated types.                                            |
| `apps/web/src/pages/RunDetail.tsx` and AI/session components                                           | Updated   | Frozen audit display and capability-driven controls.                                    |
| `contracts/browser.openapi.json`, `contracts/worker.schema.json`, generated TS/Pydantic                | Generated | Synchronized consumer contracts.                                                        |
| `docs/architect/*`                                                                                     | Updated   | Registry authority, environment boundary, rollout and observed validation.              |

## Deviations from Plan

- No customer-facing model registry mutation route was added; the trusted execution task remains
  the operator boundary, as intended by the architecture.
- Phone choices exclude unresolved, stale or incompatible model-enabled profiles rather than
  adding a new compatibility DTO or model picker. This keeps the minimum-click interaction while
  ensuring only recent matching advertisements appear connected.
- Database-backed validation could not be completed in this worktree because the existing shared
  test database contains a migration name that no longer exists here. The source was compiled and
  the database was deliberately left unchanged.
- Provider/device acceptance was not run because the plan makes it an explicit operator action.

## Issues Encountered

- The retained PostgreSQL test database has divergent migration history. This prevents Loco from
  booting any database-backed test or smoke in this worktree. A separate clean test database or
  reconciliation of that historical migration is required; resetting the user's database was out
  of scope.
- One worker test fixture omitted the now-required frozen session model. It was updated to mirror
  the real protocol payload and the exact failed test passes.
- The known local HTTP redirect-boundary worker test timed out during a broad run and passed on its
  targeted rerun.

## Tests Written or Extended

| Test file                                        | Coverage                                                                               |
| ------------------------------------------------ | -------------------------------------------------------------------------------------- |
| `crates/contracts/tests/model_registry.rs`       | Reference/definition validation and legacy/new serialization.                          |
| `crates/contracts/tests/execution.rs`            | Optional typed profile binding and historical serialization.                           |
| `apps/api/tests/model_registry.rs`               | Immutable replay/conflict, resolution and retirement (compiled; database run blocked). |
| `apps/api/tests/execution.rs`                    | Frozen run assignment, protocol fencing and comparison context.                        |
| `apps/api/tests/task_sessions.rs`                | Frozen phone assignment and protocol-5 claim matching.                                 |
| `apps/mobile-worker/tests/test_model_runtime.py` | Factory identity, capabilities, host matching and credential failure.                  |
| `apps/mobile-worker/tests/test_task_sessions.py` | Exact reference admission before side effects.                                         |
| `apps/web/src/lib/model-capabilities.test.ts`    | Navigation, authoring and model-free capability states.                                |
| `apps/web/src/pages/RunDetail.test.tsx`          | Exact frozen audit identity and historical fallback label.                             |

## Next Steps

- Run database-backed API tests and the three model-free smokes against a clean isolated test
  database whose migration ledger matches this worktree.
- Review the combined registry and preserved React Flow changes before committing.
- Register an approved nonsecret definition, manually migrate the private host TOML to exact
  `model_ref`, restart API/workers and confirm the protocol-5 heartbeat before any real model run.
- Run real device/model acceptance only when explicitly authorized.
