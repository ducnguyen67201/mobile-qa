# Implementation report: Phase 06 visual task sessions

## Summary

Implemented the primary task-first experience on `codex/06-task-sessions`: open a
validated app build, see real device captures, describe a task or select a control,
run it through Minitap, inspect task history, stop the session, and optionally save
an incomplete reusable test draft. A case, suite, release plan or approval is not
required to try an interactive task. Existing saved regression routes retain their
approval rules and existing drafts remain available.

The app and Tests pages now prominently link to the simpler flow. A single compatible
worker and the newest validated build are selected automatically. A closed session
can be reopened with one button. Missing setup is shown without a simulated phone.

## Assessment versus plan

Complexity remained XL. The new implementation builds on local phase 05 source;
GitHub PR #5 is phase 04, so this worktree does not establish that phase 05 was merged.
No phase 05 files were discarded or reset. No commits or PR pushes were performed.

| Task                                   | State                 | Evidence                                                                                                    |
| -------------------------------------- | --------------------- | ----------------------------------------------------------------------------------------------------------- |
| Rust browser/worker contracts          | Complete              | Utoipa + Schemars, generated SDK/Zod/Pydantic, zero drift                                                   |
| Durable sessions and tasks             | Complete              | Migration 000006, immutable goals, request fingerprints                                                     |
| Browser and worker APIs                | Complete              | Session ownership, scoped worker identity, shared reservations, lease expiry quarantine                     |
| Python device worker                   | Complete              | Explicit task-worker command; Doppler-isolated pinned Minitap, screen capture, host locks and dirty markers |
| Minimal browser flow                   | Complete source       | Default selection, task input, screen controls, progress, Stop, optional Save as test                       |
| Automated and API-to-device acceptance | Passed within scope   | Results below                                                                                               |
| Rendered dashboard acceptance          | Pending               | Existing browser admin restriction respected                                                                |
| Persistent local worker connection     | Prepared, not applied | Needs explicit permission to add a Doppler worker credential                                                |

## Validation

| Check                              | Result                                                                                                                                                         |
| ---------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Rust format/Clippy/workspace tests | Passed; 43 tests including the new real route lifecycle test                                                                                                   |
| Browser format/typecheck/ESLint    | Passed; affected checks rerun after navigation followups                                                                                                       |
| Browser Vitest                     | 93 passed; affected library/session files passed after followups                                                                                               |
| Worker Ruff/Pyright                | Passed                                                                                                                                                         |
| Worker pytest                      | 120 initially passed, one existing local HTTP test had a transient connection failure; targeted retry passed, then all 20 affected execution/task tests passed |
| Contract regeneration/drift        | Passed; zero drift and exporter helper test passed                                                                                                             |
| Production Vite + Rust build       | Passed; Vite retains the existing large-chunk warning                                                                                                          |
| Real HTTP → Minitap → emulator     | Passed for two different goals and selected-control context                                                                                                    |
| Local API/Vite proxy after restart | Both health endpoints returned 200; both new protected phone routes returned 401 without authentication                                                        |

Real acceptance run: `a8101844-8f33-4292-b093-8dd02e719b1c`, session
`b62e0a4e-ca27-442a-b421-f03f6827e53a`. It used the existing gpt-4.1 model/profile,
a fresh owned Android emulator and the actual offline sample APK. The normal
application API accepted these tasks without any case or plan:

1. Enter Buy milk and save it. Minitap completed; captured UI hierarchy contained
   Buy milk in `task_row`.
2. Select the task input, replace its text with Walk the dog and save it. Minitap
   completed; captured hierarchy contained Walk the dog in `task_row`.
3. Stop session. The worker stopped/discarded its emulator, the API returned closed,
   and the worker exited successfully. Emulator port 5554 was no longer listening.

Private evidence lives under `.private/task-session-acceptance/<run-id>/` (initial
and final PNGs, structured result, model traces and local logs). No model/API tokens
were written into these artifacts. SDK trace labels are not used as QA verdicts.
The new smoke entry point is `scripts/task_session_smoke.py`, requiring explicit
profile/APK arguments; it is never a dependency of normal check/build commands.

## Implementation details and limits

- Sessions share phase 04's physical reservation namespace, with exactly one attempt
  or session owner. An expired connection retains its reservation and quarantines
  the phone; it cannot silently replay a task.
- Claims wait up to 30 seconds, checking durable work every three seconds. Heartbeats
  continue during boot and model work. Stop is acknowledged only after cleanup.
- The worker re-resolves a selected control by resource ID and label against a fresh
  device hierarchy. Missing/ambiguous controls fail with an actionable message.
- Private session JSON retains the latest bounded PNG/control list plus task history.
  The local SDK trace is retained for operator evidence; no complete video/timeline
  streaming interface is claimed.
- Session completion is distinct from an independently verified test pass. Saving
  copies the user's goal to an editable draft with no invented expected results.
- Existing qualified demo package/Android profile limitations remain. Arbitrary APK
  qualification, customer account/reset support, broader discovery, and batch
  requirement-to-test generation remain planned. Per-step SDK planner event streaming
  is not exposed; the UI shows task-level progress and actual screen refreshes.
- The real smoke did not exercise an app restart goal or rendered browser interactions.
  Those gates are not inferred from two successful device tasks.

## Files

New files: contracts/task-session DTO and OpenAPI modules; migration 000006;
API task-session controller/service/route test; Python task worker and tests;
browser transport/page/DOM tests; real acceptance script; phase 06 plan/report.
Existing files updated: contract export roots, migration/service/controller registries,
API route inventory, worker CLI/HTTP transport, SDK seam comments, React routes and
app/test entry points, and owning architecture/status documents.

## Remaining setup / review

Nonsecret local worker configuration is prepared under `.private/phase06-local-worker/`
for app `25208e8d-5a5a-4f24-ad9a-a5a67952beae`, profile
`c00ec7f1-e05a-4403-a3fb-4a496a18a6d6`, Doppler `mobile-qa/dev_personal`.
Neither `dev` nor `dev_personal` currently provides `MOBILE_QA_WORKER_TOKEN`.
Adding that credential, registering its worker and starting a persistent local
worker have not been performed. The development API was restarted on port 5150
with migration 000006; the existing Vite server on 5173 remains running.

The plan remains active rather than archived because rendered acceptance and the
persistent user-facing worker connection are outstanding. Phase 06 is in progress;
this report does not mark the broader generation/discovery spec complete.

## PR preparation

The PR includes the phase 05 dependency and phase 06 primary task flow and targets
`feat/02-cloud-phone-and-feasibility`, whose tip is the merged phase 04 PR #5.
The parent branch remains open as PR #2 against main. This keeps the new diff
focused on the local phase 05/06 work rather than reintroducing the parent changes.

Pre-commit package audits: pnpm audit reported zero advisories; pip-audit of the
installed worker environment reported no known vulnerabilities. No dependency
versions were changed. These checks do not claim a fresh Rust dependency audit.
CI scope selection was extended for both task-session contract modules and the
explicit acceptance script; all 23 scope cases passed. Real device/model scripts
remain explicit and are not executed by ordinary CI.

## Task-left / phone-right editor update

The case editor now embeds the shared task workspace instead of linking away to
another page. Task composition and draft fields stay on the left; the current
phone capture stays in a sticky right panel on desktop and stacks on narrow
screens. Open phone preview starts a session explicitly from the editor, while an
existing active session reconnects. The standalone Try page retains automatic
opening. Task composition is available while the phone is opening.

Validation: TypeScript, ESLint, all 94 browser tests, production Vite build,
Prettier and diff checks passed. The added integration test opens the embedded
phone, selects a control, sends a generated-contract task request, observes the
returned frame update and confirms unsaved draft edits remain intact. The existing
bundle-size warning remains. Rendered browser acceptance remains pending; these
DOM checks do not claim visual or real-device acceptance for the new layout.

### Full-width workspace follow-up

Task and case routes now use the full content width with equal task/preview
columns and 16px desktop outer padding. The right panel fills the available
viewport height, and captures scale within it. The sidebar defaults to a 64px icon
rail with an expand control; the header is 48px. Case titles use a compact toolbar,
and task routes omit the decorative footer.

TypeScript, ESLint and production build passed. The browser suite initially passed
91/94 tests; three workspace tests assumed an expanded sidebar. Those interactions
were updated for the compact default, including switching via its menu, and all
eight workspace tests passed on rerun. TypeScript and affected lint passed again.
No runtime logic or transport contracts changed. Visual acceptance is still pending.

### Resizable preview and numbered task steps

Added a draggable divider with pointer capture, bounded pane sizes, keyboard
arrows, Home/End and double-click reset. The sidebar is now 52px and the header
40px. Task composition offers numbered Ask AI, Tap, Enter text, Swipe, Go back and
Restart app steps, with add/remove/reorder controls. The editor compiles them into
one ordered Minitap goal using the existing generated request contract. Empty
steps and combined goals above 4,000 characters cannot run. Steps remain visible
after submission; a deliberate subsequent run receives a fresh request identity,
while a failed request can retry with its original identity.

These action choices guide the AI agent; they are not a new deterministic command
API or a per-step pass/fail report. The selected capture control anchors the start
of the task. Later targets are described in the step fields.

Validation: all 98 browser tests, TypeScript, ESLint, Prettier, production build and
diff checks passed. New tests cover pointer/keyboard resizing, retained editor
values, ordered step submission, removal, and combined-length rejection. Browser
visual acceptance remains pending. The existing production bundle-size warning
remains; no worker or backend behavior was changed by this update.
