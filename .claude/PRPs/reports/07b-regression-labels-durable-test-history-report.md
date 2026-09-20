# Implementation Report: 07B Regression Labels and Durable Test History

## Summary

Implemented the first 07B delivery: saved editor cases now create durable execution runs
through the existing scheduler, freeze an explicit build/profile/environment/baseline, appear
in source-aware history, and expose deterministic comparison labels without changing the
underlying execution outcome. Editor trials remain creator-scoped phone activity.

## Assessment vs Reality

| Metric        | Predicted (Plan)                     | Actual                                                                              |
| ------------- | ------------------------------------ | ----------------------------------------------------------------------------------- |
| Complexity    | Cross-stack delivery                 | Cross-stack delivery across Rust, React, Python protocol and generated contracts    |
| Confidence    | High with strict compatibility tests | High after generated drift, static, unit, integration, build and smoke verification |
| Files Changed | Not enumerated in attached plan      | 44 files: 6 created and 38 updated, including generated consumers                   |

## Tasks Completed

| #   | Task                                                 | Status   | Notes                                                                                             |
| --- | ---------------------------------------------------- | -------- | ------------------------------------------------------------------------------------------------- |
| 1   | Version manifest sources and preserve legacy JSON    | Complete | Strict tagged source enum; legacy plan fields remain optional and round-trip unchanged            |
| 2   | Admit saved cases to the existing execution pipeline | Complete | Shared manifest assembly; no hidden suite/plan or second queue                                    |
| 3   | Freeze and compare eligible baselines                | Complete | Exact source/case/data/environment/profile compatibility and clean evidence policy                |
| 4   | Add source-aware durable history                     | Complete | Cursor projection over execution runs and creator-scoped phone trials; legacy filter is explicit  |
| 5   | Update editor and Runs UI                            | Complete | Explicit build/profile/baseline, Save & run, cleanup wait, durable progress and comparison detail |
| 6   | Fence worker compatibility                           | Complete | Versioned manifests require execution protocol4                                                   |
| 7   | Update canonical architecture documentation          | Complete | Contracts, system, status and phase 07 ownership updated                                          |

## Validation Results

| Level                  | Status | Notes                                                                                                                                       |
| ---------------------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------- |
| Static analysis        | Pass   | Contract drift, Rustfmt, Clippy, TypeScript, ESLint, Ruff and Pyright                                                                       |
| Unit/integration tests | Pass   | 54 Rust, 122 web and 175 worker tests covered; one known intermittent loopback test passed alone                                            |
| Build                  | Pass   | Vite production build and Cargo workspace build; existing Vite chunk-size advisory remains                                                  |
| Integration            | Pass   | Test-library, execution and direct-authoring HTTP smokes, all simulated and restart-persistent                                              |
| Edge cases             | Pass   | Legacy round-trip, protocol fencing, failed baseline suggestion, same/cross-build labels, frozen baseline, legacy history and authorization |

## Files Changed

| Area                            | Action            | Scope                                                                          |
| ------------------------------- | ----------------- | ------------------------------------------------------------------------------ |
| `crates/contracts`, `contracts` | Updated/generated | Versioned sources, comparison/history DTOs, phone purpose and API declarations |
| `apps/api/migration`            | Created/updated   | Nullable plan ownership, source kind, frozen baseline and reversible cleanup   |
| `apps/api/src/services`         | Created/updated   | Saved-case admission, comparison, history, scheduler and task purpose          |
| `apps/api/tests`                | Updated           | Route, protocol, comparison, history, authorization and task fixtures          |
| `apps/web/src`                  | Created/updated   | Editor run controls, Runs projection, comparison report and DOM/API tests      |
| `apps/mobile-worker`            | Updated/generated | Protocol4 claim and generated Pydantic models                                  |
| `scripts/execution_smoke.py`    | Updated           | Versioned release-plan request shape                                           |
| `docs/architect`                | Updated           | Canonical contract, system, status and phase ownership                         |

## Deviations from Plan

- The attached plan was external to the repository, so it was not moved into
  `.claude/PRPs/plans/completed`; this report records the implementation while the supplied
  attachment remains untouched.
- Audited triage and multi-case coverage-delta rows remain outside this first delivery, as
  the plan explicitly states.
- A strict tagged `ManifestSource` enum replaced the initially attempted flattened wrapper;
  Serde's unknown-field handling made the flattened form unsafe. The wire shape is unchanged.

## Issues Encountered

- The shared test database contained a migration from another branch. Validation used and
  removed a disposable PostgreSQL container; the shared database was not changed.
- A fresh worktree lacked generated APK fixtures and worker tools. Pinned dependencies and
  synthetic fixtures were prepared locally without device/model calls or retained keys.
- The existing worker loopback HTTP test failed intermittently during the parallel suite and
  passed on an isolated retry. No production transport behavior was changed.

## Tests Written

| Test file                                 | Coverage                                                                                   |
| ----------------------------------------- | ------------------------------------------------------------------------------------------ |
| `crates/contracts/tests/execution.rs`     | Legacy/versioned manifest serialization compatibility                                      |
| `apps/api/tests/execution.rs`             | Saved-case admission, worker fencing, baseline selection/labels, history and authorization |
| `apps/api/tests/task_sessions.rs`         | Explicit phone-task purposes and protocol boundaries                                       |
| `apps/web/src/pages/TestLibrary.test.tsx` | Explicit saved-case build/profile submission                                               |
| `apps/web/src/pages/Runs.test.tsx`        | Saved-test/trial labels and explicit legacy filtering                                      |
| `apps/web/src/api/runs.test.ts`           | Compatibility query and history filter transport                                           |

## Next Steps

- Review this branch and create a pull request when desired.
- Implement audited triage and multi-case coverage deltas as the next 07B increment.
- Keep 07C retention/cost work and the 07D reliability/pilot campaign separate.
