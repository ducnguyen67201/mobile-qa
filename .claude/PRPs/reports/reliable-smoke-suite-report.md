# Implementation Report: Reliable smoke suite

## Summary

The saved-suite path now asks for an explicit build and qualified device, shows
the selected checksum and suite version, offers an optional completed-run
baseline, and freezes that selection in the existing multi-case run. The API
shares baseline candidate lookup with saved cases. The existing scheduler
claims worker-eligible attempts in a deterministic run/case/attempt order.
Reports identify the suite and do not call missing clean-start proof or an
optional failure an all-case pass. A two-case simulated HTTP smoke and an
explicit real-device suite mode were added.

## Assessment vs reality

| Metric     | Predicted                                                            | Actual                                                                                       |
| ---------- | -------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| Complexity | Large, about 12–16 source/test files plus generated outputs and docs | Large, 30 feature paths before this report, including contracts, UI, tests, scripts and docs |
| Confidence | Route/browser/simulated proof; device and rendered browser separate  | Automated checks and simulated HTTP passed; device and rendered browser remain open          |
| Migration  | None                                                                 | None; reused nullable `execution_runs.baseline_run_id`                                       |

## Tasks completed

| Task                         | Status                      | Evidence / limitation                                                                                                                  |
| ---------------------------- | --------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| Suite contract compatibility | Done                        | Old omitted baseline round-trip test and generated OpenAPI/SDK/Zod.                                                                    |
| Shared baseline admission    | Done                        | Shared candidate scan, exact suite suggestion, completed same-app pin, route test.                                                     |
| Deterministic worker pickup  | Done                        | Stable ID tie-breaks; route claim test checks case zero and reservation blocking.                                                      |
| Honest report policy         | Done                        | Real proof and sealed required-check evidence; optional failures explicit.                                                             |
| Explicit suite UI            | Done                        | Build/device/baseline choices, checksum, frozen retry, readiness, DOM tests.                                                           |
| Suite report                 | Done                        | Saved-suite heading/version, per-case existing evidence, server result.                                                                |
| Simulated HTTP smoke         | Done                        | Two saved cases; three fake scenarios; idempotency and restart/pin assertions.                                                         |
| Real acceptance harness      | Implemented; live gate open | `--suite` mode is ready. Another worktree currently runs workers against the same physical host profile; no new device claim was made. |

## Validation results

| Check                                                | Result                                                                         |
| ---------------------------------------------------- | ------------------------------------------------------------------------------ |
| Generated contract drift and contract tests          | Pass; `just types`, `just check-contracts`.                                    |
| Scoped Rust/TS/Python format and changed-script Ruff | Pass.                                                                          |
| API Clippy and workspace tests                       | Pass with a disposable migrated PostgreSQL database and process-scoped JDK 17. |
| Web typecheck, lint, tests                           | Pass; 25 files, 158 tests.                                                     |
| Worker Ruff, Pyright, tests                          | Pass; 185 tests.                                                               |
| Production build                                     | Pass; Vite and Cargo. Vite retains its existing large-chunk warning.           |
| HTTP acceptance                                      | Pass; `smoke-test-library`, `smoke-execution`, `smoke-suite`, all simulated.   |
| Real Android and rendered browser                    | Open; no claim of completion or run IDs.                                       |

## Files changed

| Area          | Files                                                                                                               | Change                                                              |
| ------------- | ------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| Contracts     | `crates/contracts/src/regression.rs`, generated OpenAPI/browser SDK/Zod                                             | Add optional suite baseline and preview choices.                    |
| API           | `run_baselines.rs`, `case_runs.rs`, `suite_runs.rs`, `scheduler.rs`, `runs.rs`, `domain/regression.rs`, route tests | Share candidate lookup, pin baseline, order claims, verify summary. |
| Web           | `saved-suite-run.tsx`, `run-result.tsx`, `RunDetail.tsx`, coverage flow and tests                                   | Explicit choices, source labels and assertions.                     |
| Scripts       | `smoke_suite.py`, `execution_smoke.py`, `regression_acceptance.py`, `justfile`                                      | Two-case simulated and opt-in real acceptance.                      |
| Documentation | Architect spec 10, status, development, decisions, README and spec 05                                               | Record rules, implemented behavior and open gates.                  |

## Deviations from plan

- Kept the existing single-case 07B acceptance path and added `--suite` to its
  isolated harness instead of replacing it or duplicating its setup.
- Scoped formatting to feature files because unrelated untracked
  `docs/visuals/mvp-roadmap.html` is present. No changes were made to it.
- Used a disposable PostgreSQL container with a temporary, restored test
  configuration because the retained local test database names an absent
  historical migration. Set JDK 17 for tests because the shell default is 8.
- The real-device acceptance could not safely run while another worktree's
  execution worker was attached to the same physical host profile. The
  browser admin-policy gate also remains open.

## Issues encountered

The first API pass exposed a test fixture move and test-module placement;
both were corrected. The retained test database migration ledger and Java 8
default prevented local integration checks until the disposable database and
JDK 17 were used. A fresh disposable database needed normal forward migrations
before the HTTP smoke's operator provision command.

## Next steps

- When the physical host is exclusively available and recovery is verified,
  run `just smoke-suite-real` on the local good/broken APKs and record run IDs,
  hashes, checks, start/cleanup receipts and comparison in architect status.
- Complete rendered browser acceptance through authorized access.
