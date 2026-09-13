# Implementation report: Direct execution and AI test authoring

Branch: `codex/06-task-sessions`. Date: 2026-09-13.

## Outcome

Tests now offers Generate with AI, four templates and manual creation. Phone picking binds typed
targets; Control phone executes direct commands and optionally records completed steps. Case drafts
and preview trials share one structured editor. Direct tap/type/swipe/back/restart/wait commands
run through the same Python executor in trials and approved regressions. Only explicit Ask AI
steps use Minitap. AI generation explores bounded states and proposes named drafts with source
screens, questions and measured usage; selected proposals save atomically without approvals.

## Change size

The plan spans contracts, Rust, Python and React. Actual diff: 60 existing files
modified and 21 new files, including generated consumers, tests and canonical documentation.
The significant implementation owners are `test_authoring`/`authoring_validation` in Rust,
`automation`/`authoring` in Python, and the shared `phone-workspace`, `task-steps`,
`template-picker` and `ai-authoring` components in React.

## Plan assessment

| Area                                      | Result                                                                                                                      |
| ----------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| Shared contracts and compatibility (1)    | Strict command/target unions; semantic action validation; legacy serialized bytes retained                                  |
| Persistence and fencing (2–3)             | Existing session/task payloads, protocol 2, revisions, frozen content and ordered receipts                                  |
| Direct executor and integration (4–5)     | Shared exact-target RPC seam; literal Unicode; no automatic mutation replay; explicit AI only                               |
| Editor and recording (6)                  | One state owner, Pick/Control modes, recorded receipts, checks, existing resize/compact shell                               |
| Templates and atomic saves (7)            | Four versioned templates; generated keys; all-or-nothing draft creation and replay                                          |
| Generation orchestration/model seam (8–9) | Durable bounded jobs; isolated typed model calls; password redaction; source validation                                     |
| Proposal review and simple entry (10–11)  | Editable names/steps/requirements; expectation confirmation; name hints; manual/template paths without AI                   |
| Contracts and regression coverage (12)    | Generated browser/Python outputs, actual HTTP route coverage, CI scope and new tests                                        |
| Acceptance (13)                           | Synthetic HTTP flow and real direct executor pass; real-model quality and rendered full UI-to-device acceptance remain open |
| Documentation/cleanup (14)                | Canonical source/status updated; obsolete editor/prose compiler removed; no GAN working Markdown retained                   |

Implementation is delivered. The plan remains in the active plans directory because its separate
real-model quality and full rendered acceptance gates have not been completed; it is not marked
fully qualified or silently archived as entirely fulfilled.

## Validation

| Check             | Evidence                                                                                                                      |
| ----------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| Rust              | Clippy and 46 tests pass; new command tests and expanded real HTTP lifecycle tests                                            |
| Browser           | TypeScript/ESLint pass; prior 99-test suite plus compact-editor regression; 19 affected tests pass                            |
| Worker            | Ruff/Pyright pass; 136 tests pass across full run plus targeted retry                                                         |
| Contracts         | Generated outputs agree; exporter helper passes; no handwritten consumer transport types                                      |
| Formatting        | Prettier, rustfmt, Ruff pass; generator-owned graph output excluded from formatter                                            |
| Builds            | Rust workspace and Vite production builds pass; existing large JS chunk warning remains                                       |
| HTTP smoke        | Typed direct draft → review → plan → synthetic worker, API restarts, immutable manifest and passed/failed/blocked reports     |
| Device            | Actual sample APK, direct text/save/restart twice (plain text and Vietnamese), no model; owned emulator stopped and discarded |
| CI scope          | 24 cases pass including new smoke path and deletion/combined changes                                                          |
| Dependency audits | Production web and installed Python packages: no known vulnerabilities; local worker package is not a PyPI audit target       |

Device evidence is private at `.private/direct-acceptance/d14dea5c-964f-4c00-8e82-6a36d2d33b21`.
This is execution-seam acceptance, not a claim of rendered browser or arbitrary customer APK
qualification. No paid model request was made by the validation suite.

The existing `test_client_actual_http_response_and_redirect_boundaries` intermittently times out
when the worker suite runs together on this host; it passes individually. The initial Rust API
run picked up Java 8; rerunning with the installed JDK 17 passed. No global Java settings changed.

## Deviations and limits

- Kept the legacy TestAction envelope with an omitted optional typed command, rather than rewriting
  historical JSON. Strict nested unions plus boundary validation enforce kind/command agreement.
- Reused existing session JSON and library mutation receipts instead of introducing a duplicate queue,
  migration and artifact subsystem. Generation frames remain bounded private payloads, capped at eight.
- Invalid AI output fails closed; no automatic repair spend. Explicit capture reuse is restricted to the
  same session and journey/write policy, with environment revision fencing.
- Canceling generation stops/cleans its phone session. It does not leave a separate idle device job.
- Template version and proposal identity are retained in provenance. A separate prompt-version catalog,
  cross-library semantic deduplication, artifact retention and hosted qualification remain later work.
- New Rust API modules are `test_authoring`/`authoring_validation`; Python uses `automation`/`authoring`;
  UI generation/review share `ai-authoring.tsx`. These replace larger planned module subdivisions.

## Compact editor follow-up

Actions now use two compact rows, with an icon/type selector and controls above inline target/value inputs. Checks are optional for trials and expand inside their owning action. Picking, recording, reordering, check bindings and unsaved drafts are covered by the 19 affected browser tests. Full target identifiers remain in transport and are available on hover.

## Cleanup

Removed the superseded case-fields implementation and client wrapper that flattened steps into one
Minitap task. Historical API compatibility and approved versions remain intentionally supported.
No GAN review was run for this revision and no GAN working Markdown remains. Private device evidence
is ignored by Git. Generator-owned graph files were restored after incidental formatting; the formatter
now excludes them. Temporary acceptance scripts outside the repository are removed after verification.

## Remaining acceptance

Run a bounded real-model quality evaluation against good/broken/ambiguous fixtures, then verify the
rendered browser → live worker journey when the existing browser inspection restriction permits it.
PR #6 targets main and includes the earlier execution/library/session foundation plus this direct-execution revamp. Hosted scaling and real-model quality remain unqualified.
