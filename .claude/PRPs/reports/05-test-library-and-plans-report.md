# Implementation report: 05 — Test library and plans

Date: 2026-09-12. Branch: `codex/05-test-library-and-plans`.
Status: local implementation and deterministic/HTTP verification passed. Rendered
browser and UI-triggered real-device acceptance remain open.

## Implementation

Customers can author cases, suites and release plans, save incomplete drafts,
request exact-version review and run approved coverage against an uploaded build.
Separate editable drafts protect immutable published definitions and historical
manifests. Entry revisions and atomic mutation receipts prevent lost updates and
make identical request retries safe. Review requires purpose-specific grants;
operator status alone does not grant approval authority. Browser and CLI publication
share the catalog and lifecycle rules.

The app default is explicitly selected, and all suite/plan membership is pinned.
Approving a newer version does not move those choices. Archive blocks new runs
through transitive references while previously queued jobs and reports retain their
frozen content. Migration 000005 backfills legacy identity, hash, approval and default
records without rewriting execution data.

The Mantine UI provides Cases / Suites / Release plan tabs, human-readable actions
and expected checks, advanced evidence fields, version history, archive/restore and
selected-plan Run/report navigation. Unsaved values survive stale saves, background
request errors and another author submitting the draft. A comparison requires an
explicit choice of current revision and a separate Save; nothing auto-merges.

Rust owns draft/response/request/error DTOs. The existing Utoipa→OpenAPI→Hey API
SDK/types/Zod pipeline handles every browser boundary; published definitions enter
the existing Rust→Schemars→Pydantic worker protocol. Five generated artifacts changed;
the worker protocol shape did not change.

## Assessment and tasks

The planned XL scope remains XL: 26 created files and 40 modified files
at report time, including generated artifacts, plan/report and preserved design
history. No new product framework, runtime dependency, device adapter or auth model
was introduced.

| Task                                 | Status                                             | Evidence                                                                           |
| ------------------------------------ | -------------------------------------------------- | ---------------------------------------------------------------------------------- |
| 1. Shared draft/library contracts    | Complete                                           | Rust types, semantic issues, generated SDK/Zod, negative compile fixtures          |
| 2. Migration/backfill and receipts   | Complete                                           | Six tables, app lock, scoped mutation receipt; isolated upgrade test               |
| 3. Catalog/drafts/options APIs       | Complete                                           | Fourteen authenticated operations, scoped reads/writes, incomplete-save coverage   |
| 4. Submission/review                 | Complete                                           | Exact hash/revision, independent grants, review history, blocked closed candidates |
| 5. Archive/default/run admission     | Complete                                           | Transitive archive blocking, explicit default, frozen queued manifest test         |
| 6. Browser adapters/error transport  | Complete                                           | Generated success/request/detail validation and malformed-boundary tests           |
| 7. Library/case editor               | Complete                                           | Actions/checks, advanced evidence, dirty/conflict recovery, no build prerequisite  |
| 8. Review/history                    | Complete                                           | Frozen semantics, review attribution, draft-only archive/restore                   |
| 9. Suites/release plan               | Complete                                           | Explicit version changes, unique coverage, profiles/limits/default                 |
| 10. Selected plan → Run/report       | Complete                                           | DOM flow and HTTP submission through the existing worker protocol                  |
| 11. HTTP harness/CI/docs             | Complete                                           | HTTP-only customer mutations, API restart, simulated Python evidence               |
| 12. Validation and acceptance record | Deterministic checks complete; manual gate pending | Results below; browser/device scope explicitly unverified                          |

## Validation results

| Check                              | Result                                                                                                                                         |
| ---------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| `just types`                       | Passed; five generated files changed                                                                                                           |
| `just check-contracts`             | Passed; zero drift and one exporter helper test                                                                                                |
| Rust formatting and Clippy         | Passed, warnings denied                                                                                                                        |
| Rust workspace tests               | 42 passed, including six new real PostgreSQL/route lifecycle tests and five draft-contract tests                                               |
| Web TypeScript / ESLint / Prettier | Passed, including negative generated-contract compile fixtures                                                                                 |
| Web Vitest                         | 90 passed across ten files; only the failing library file was rerun after fixture corrections                                                  |
| Worker Ruff / Pyright / pytest     | Passed; 117 tests, including published user-authored case compatibility                                                                        |
| CI scope selection                 | 20 cases passed                                                                                                                                |
| Rust and production web builds     | Passed using the two commands owned by `just build`                                                                                            |
| `just smoke-test-library`          | Passed: HTTP draft/save/replay, case/suite/plan review, explicit default, deduplicated coverage, API restart and frozen manifest after editing |
| Report persistence/evidence        | Simulated passed/failed/blocked reports survive restart, with verified fake cleanup and private artifacts                                      |
| Final repository formatting/diff   | Passed after formatting the final report and GAN feedback                                                                                      |

The initial API test run found the inherited shell was not selecting JDK 17. The
Rust test stage passed with the already-installed Homebrew JDK 17 selected for that
process. No global Java or Android configuration changed. Web test corrections
covered fixture completeness, route setup, ambiguous selectors and asynchronous
capability rendering; only failed/invalidated checks were rerun. The production
web bundle retains Vite's size warning (942.05 kB JS / 279.63 kB gzip).

## Design loop

Independent source evaluations: 7.43 → 7.85 → 8.00 / 10 (threshold 7.5).
Required fixes covered stale-edit recovery, draft-only archive/restore, complete
frozen semantics and unsaved work surviving background refresh/concurrent submission.
Temporary GAN feedback files were removed after the design changes were applied.
Scores are provisional source assessments, not verified rendered appearance.

## Deviations and acceptance limits

- Browser admin-policy restrictions were respected. Rendered layout, keyboard/focus
  and a real emulator run initiated from this new UI remain unverified. The plan
  stays in the active plans directory so those acceptance gates are not erased.
- The HTTP harness reuses the phase 04 setup/worker/restart driver through callbacks;
  all customer definition mutations use the new HTTP API. Operator commands only
  provision fixture profiles, grants and worker identity.
- The root build recipe was executed as its separate Rust and web commands to avoid
  waiting for frontend fixture corrections before building the API. Neither build
  was duplicated.
- Browser code uses local form state derived from generated types; no equivalent
  consumer schema or handwritten transport was introduced.
- Only the existing controlled demo adapter is executable. Arbitrary customer
  login/reset, additional adapters and AI generation remain later work.
- No live secret-consuming server was restarted, browser/device launched, PR pushed
  or global configuration changed. Dependency versions are unchanged; the previously
  recorded RSA advisory remains a historical open item, not a new clean audit claim.

## Main files

- Contracts: `crates/contracts/src/test_library.rs`, `test_library_api.rs`, generated browser artifacts.
- Storage/services: migration `m20260912_000005_test_library.rs`, `services/test_library.rs`, `test_library_mutations.rs`, existing definition/run admission.
- Routes: `apps/api/src/controllers/test_library.rs`, shared execution-plan query.
- UI: `pages/Tests.tsx`, `TestLibraryDetail.tsx`, `components/test-library/`, `api/test-library.ts`, selected-plan RunPreview.
- Tests: Rust contract and real route/upgrade suites, browser transport and twelve DOM workflows, worker compatibility test, `scripts/test_library_smoke.py`.
- Documentation: architecture status/contracts/development/spec 05 and the design review summary above.

Next acceptance work is the allowed rendered browser → backend → real emulator flow.
The implementation is ready for source review and PR preparation.
