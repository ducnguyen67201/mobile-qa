# 06 — Run tasks visually and save reusable tests

Status: planned. Depends on: 04–05 and a qualified execution path. Owns: Rust interactive session/task services and generated transport contracts, Python/Minitap session execution, React phone/task UI, and optional test generation/review.

## Deliverable

The primary experience is: open an uploaded app on an emulator, see its screen,
point to a control or describe a task, and press **Run task**. Minitap plans and
performs the requested task while the user sees progress and screen updates.
Creating cases, suites, release plans and review records is not a prerequisite
for this interactive task flow. This replaces the form-first generation proposal
following user feedback on 2026-09-13; none of the interactive flow is implemented yet.

Afterward, **Save as test** optionally turns the task and captured evidence into
a reusable draft through phase 05. Reusable regression checks retain explicit
expected outcomes and existing review semantics. Batch generation from pasted
stories remains a secondary workflow: it produces drafts, never automatically
approved regression definitions.

## First implementation: task session

- Show the current Android screen beside one task input and **Run task**. Select
  a validated build and configured device automatically when unambiguous; show
  a simple chooser when needed. Display preparation, unavailable device and
  connection errors explicitly. Never substitute a mock phone for a missing session.
- Let a user mark a control on the captured screen as context for the task.
  Bind the selection to its screenshot/frame and UI hierarchy; resolve it again
  against the current device state before acting. A selection does not silently
  send a tap. **Run task** authorizes the described task and its bounded execution.
- Pass the goal and selected context to Python's existing Minitap seam. Minitap
  owns planning and device interaction; do not add a competing Rust tap planner.
  Show available plan/action progress and refresh the phone from actual device
  captures. Do not claim streaming plan events until the pinned SDK seam supports them.
- Expose queued, preparing, planning, acting, completed, needs-input, failed and
  stopped states with screenshots and an observed outcome. **Stop** cancels the
  active task and confirms worker termination before releasing the device.
- Keep the original goal fixed for each attempt. Show task completion separately
  from a verified test pass. If a success criterion is absent, report observations;
  never invent an expectation or equate the agent's completion claim with a pass.
- Keep **Save as test** and advanced case fields secondary. Saving does not
  silently approve a case or change an existing release plan.

Rust owns authenticated app-scoped sessions, frozen build/environment binding,
idempotent task submission, task events, cancellation and private evidence access.
Python claims durable HTTP jobs and owns exclusive device leases, Minitap, capture
and cleanup. Reuse phase 04 lease infrastructure without weakening the existing
approved-regression run route: interactive tasks are an explicit job kind with
separate result semantics. Account/device isolation, step/time limits, restart
recovery and uncertain side effects remain enforced; lease expiry never blindly
replays a task that may already have changed the app.

Rust contracts generate browser SDK/types/Zod and worker Pydantic models through
the existing pipelines. Cover actual session/task routes, events, errors and
evidence responses at their runtime boundaries. No handwritten parallel wire types.

## Input

Persist a source revision containing the customer's text, app/environment context, declared critical journeys and constraints. Start with pasted text; repository crawling, ticket integrations and complex document ingestion are deferred.

Provide the generator with supported device capabilities, available fixture/reset adapters, permitted verification methods and existing cases. Do not send account secrets. Treat supplied text/app observations as data, not authority to expand tool permissions.

## Pipeline

1. **Normalize requirements:** extract criterion IDs, expected behavior, prerequisites and ambiguity. Preserve source anchors and contradictory statements for review.
2. **Propose scenarios:** generate happy paths and relevant negative cases grounded in those criteria. Bound count and scope to the pilot's critical journeys; avoid an unreviewable catalog.
3. **Validate structure:** parse typed output, apply business validation, reject unknown source IDs and unsupported checks. Repair malformed output only within a bounded retry budget.
4. **Check readiness:** map fixture/reset needs and execution limits. Separate “valid draft” from “executable on this configuration.” Missing knowledge becomes a question/needs-input state.
5. **Group and compare:** suggest suites/default plan, compare with existing cases and flag potential duplicates. Never merge or overwrite approved definitions automatically.
6. **Review:** show source → proposed expectation → verification method. Save edits and approval through spec 05. An approved selection becomes eligible for execution.

Current-screen capture is required for the initial task session. Broader device
discovery remains optional; a screen inventory is not a prerequisite for giving
Minitap a task. Observing the current app helps locate a control; it cannot
establish whether a buggy behavior is correct.

### Optional screen discovery and authoring

The phase 05 editor currently assumes the author knows the app; APK intake does
not discover screens. As an extension to task sessions, users may explore selected
journeys, review captured screens and ask AI to suggest additional drafts. This
is planned, not implemented; collecting all screens is not a required first step.

Capture screenshots, UI hierarchy and observed navigation transitions with build,
environment and session provenance. Show the selected screen beside the case
editor so authors can describe controls and attach relevant evidence. Both manual
and generated drafts use the same phase 05 validation and review services.

Discovery must have explicit journey and time/action limits, account/reset setup
and cancellation. Present visited screens and unexplored gaps, never a promise of
all screens: authentication, roles, data and network state can change the UI.
Treat captured content as untrusted app data and apply evidence access/redaction
rules before model use. Requirements or explicit author decisions establish the
expected result; screenshots alone cannot establish correctness. Missing intent
remains a review question rather than an automatically accepted expectation.

## Job and failure behavior

Generation runs as a durable, bounded Rust background job and returns a job ID immediately. Persist source revision, prompt/template version, model/provider configuration, usage and per-stage output status. Limit request size, scenario count, model time, repair attempts and spend.

Canceling or retrying generation does not mutate already saved approved cases. Expose partial drafts with stage errors and allow a targeted retry. Key draft creation to generation/item IDs to avoid duplicate records on delivery retries. Do not hold a database transaction across model calls.

Use direct model API calls from Rust for structured drafting. Minitap's internal device reasoning remains in Python. OpenAI Agents SDK is unnecessary for the first pipeline; add orchestration only when measured workflows require it.

## UI

Primary: **App → phone preview + task → Run task → watch actions and outcome →
optionally Save as test**. Example: “Type Buy milk, save it, restart the app, and
check that Buy milk is still there.” The user does not manually author the
intermediate navigation steps. Keep the existing detailed case editor available
for advanced editing and existing drafts; do not discard their content.

Secondary: Tests → Generate from requirements → paste criteria → review proposed
cases. Show missing inputs, duplicates and unsupported checks before approval.
Make “edit existing draft” and “generate proposed revision” explicit. Uploading
a new APK does not automatically start a task or regenerate the library.

## Acceptance

The first vertical slice uses the offline input/save sample APK: open a real
session, show its captured screen in the UI, submit the example task, observe
Minitap interaction and retained text, and inspect the evidence. Verify both
plain-text goals and selected-control context. This must not use a hardcoded
sample tap script in place of Minitap. A second goal must exercise a different
interaction to establish that task input is actually honored.

Route and worker tests cover cross-app access denial, generated schema agreement,
duplicate submission, stale screen selection, exclusive device ownership,
missing device, model failure, cancellation and uncertain lease recovery.
DOM tests cover the task flow and accessible screen selection without advanced
case fields. Use synthetic SDK/device responses in ordinary checks; explicitly
label them. Real UI-to-Minitap acceptance remains an independent gate, not a claim
inferred from DOM or fake-worker checks.

Use fixed source fixtures: clear criterion, ambiguous behavior, contradictory source, missing reset, duplicate case and unsupported action. Check schema handling, valid source references, preserved uncertainty, partial errors and bounded retries. Stub provider responses in normal tests; run a small explicit real-model evaluation before pilot use.

Manually approved generated cases execute through the same manifest/worker/report path as authored cases. Seeded defects cannot be redefined as expected behavior by the generator. A schema-valid output alone does not satisfy this gate.

## Local implementation in progress

The first task-session implementation is on `codex/06-task-sessions`, layered on
local phase 05 source (GitHub PR #5 refers to phase 04). It adds dedicated phone
sessions/tasks sharing the existing physical reservation namespace. Browser entry
is App → Open app & try a task; it opens the newest validated build automatically
when one qualified real worker profile is configured. User input goes directly to
Minitap; completed tasks may be saved as incomplete case drafts for later review.

The first device path retains the qualified `ai.mobileqa.demo` adapter and Android
35 profile. Arbitrary app/package qualification and customer login/reset are not
silently enabled. A worker still needs explicit operator registration for each app;
no browser self-provisioning of worker tokens or qualification grants is added.
The task worker captures actual screenshots while the SDK acts, without claiming
per-step planner streaming. Captures are held privately in the session record;
this initial view retains the latest frame and task history, not a full video.

Run the explicit worker with injected `MOBILE_QA_WORKER_TOKEN` using
`uv run --no-sync --project apps/mobile-worker --frozen mobile-qa-worker task-worker
--origin http://localhost:5150 --state /absolute/private/task-worker
--profile /absolute/private/profile.toml`. The host profile and model must match
the registered execution profile. The SDK child gets model credentials through
its configured Doppler injection; API credentials never enter that child.
A missing device/model configuration produces an unavailable state, not a fake
successful phone. Ordinary tests use synthetic boundaries and do not launch it.

### Task and test workspace layout

The standalone task page and case editor share a task-left, phone-right workspace.
Both columns share the full available workspace width with compact outer padding and a draggable divider (30–70% preview width).
The divider also supports arrow keys, Home/End, and double-click to reset.
The phone panel fills the right half and stays beside the scrolling editor on
desktop, then stacks below it on narrow screens. The shell uses a 40px header and
a default 52px icon sidebar, which can still expand to show navigation labels. In the case editor, **Open phone preview** opens the default
build/profile without navigating away or discarding unsaved test fields. An existing
active session reconnects automatically. Viewing a saved case alone does not start
a new device session. The standalone Try page retains automatic opening.

Users can compose a task before the phone is ready, select a control on its current
capture, then run it while keeping their draft visible. Task trials execute the
written goal; editing draft fields alone neither executes nor verifies that draft.
Preview frames refresh through the existing session polling, not a video stream.
This layout change has DOM interaction coverage; rendered browser acceptance remains
pending under the existing inspection restriction.

Task composition uses numbered, reorderable steps: Ask AI, Tap, Enter text, Swipe,
Go back or Restart app. These are editor instructions compiled into one ordered
goal for the existing Minitap task endpoint, with a combined 4,000-character limit
and at most 20 steps. Action choices are still executed by the AI agent; the UI
does not claim deterministic device commands or individual step verification.
A selected live control anchors the start of the task; later targets are described
in each step. Completed task status covers the whole sequence. Steps stay visible
after submission and can be edited or run again with a new request identity.

Each Tap or Enter text step offers **Pick on phone**. The user arms that step and
selects a detected control from the current capture; its label fills the target
field, and the generated goal includes its resource ID beside the label. The
selection follows the step through reordering. Editing the target or changing
its action clears the captured-control metadata. Picking does not execute an
Android action and is available only when the phone is ready. Only controls on
the currently captured screen can be picked; targets on later screens can still
be described. Minitap locates the target during execution; a picked target is not
a guarantee that a future screen or control will be unchanged.
