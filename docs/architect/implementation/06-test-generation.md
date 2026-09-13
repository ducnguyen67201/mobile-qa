# 06 — Direct execution and AI test authoring

Status: **revamp implemented and locally validated**; real-model quality and broader qualification remain separate.
Depends on: 04–05, existing phase 06 sessions and a qualified Android execution path.
Owns: Rust command/generation services and contracts, Python device execution and bounded
AI authoring, React test creation and the shared phone workspace.

## Current behavior and next deliverable

Ordinary actions now use a shared Python direct executor. Minitap runs only explicit
Ask AI/legacy navigation steps. Tests offers templates, manual editing and an explicit
AI discovery/generation action. Case editing and phone trials share structured draft state;
steps are no longer flattened into one model prompt. Rust owns durable orchestration and
validation; Python owns device interaction and isolated model calls.

Detailed tasks, contracts, migration and acceptance are in the
[implementation plan](../../../.claude/PRPs/plans/06-direct-execution-and-ai-test-authoring.plan.md).
The prior [task-session plan](../../../.claude/PRPs/plans/06-task-sessions.plan.md) records
the original implementation rather than the new target behavior.

## Tester experience

Tests offers **Generate with AI**, **Use a template**, and **Create manually**.
Test names are readable and editable; internal stable keys are generated automatically.
The task editor remains on the left, actual Android captures on the right, with the
existing resizer and compact navigation.

- **Direct authoring:** select Tap, Enter text, Swipe, Back, Restart or a supported
  check. Pick a target on the phone and provide only the required value/expectation.
  The left panel uses compact two-row actions: an icon/type selector and controls
  above the target/value inputs, with accessible labels and multiline text support.
  Add action appends another. Checks expand only when added or explicitly opened. Optional checks appear inside their owning action;
  no check form is created until requested. Actions can be tried without checks.
  Reviewed release tests still require evidence under the existing approval rules.
  Removed-action checks remain recoverable for reassignment rather than being silently deleted.
  Run the typed sequence directly; show per-step progress, evidence and checks.
- **Control and record:** distinguish Pick target from Control phone. Picking only
  binds a step; controlling executes an explicit command. An optional recording
  toggle saves successful direct commands into the current draft in order.
- **Predefined templates:** begin with app-open smoke, valid form submission, required
  field validation and restart persistence. Templates contain visible binding slots;
  they do not pretend to know an arbitrary APK's controls or intended behavior.
- **Generate with AI:** select Smoke by default, optionally describe a journey, and
  start discovery/generation explicitly. Reuse a compatible build/profile/session
  when available; ask for a choice only when needed. Show visited screens and gaps,
  then editable proposals such as “Smoke — app opens” and “Saved task survives restart.”
- **Save and rerun:** save selected proposals or recorded steps atomically as drafts.
  A tester can trial drafts without first assembling a release plan. Existing review
  and immutable-definition rules still govern approved regression runs.

Templates, manual authoring and direct execution remain usable without model credentials.
AI is never invoked by uploading a build, visiting a page, or failing to find a selector.
Explicit Ask AI steps are labeled and account for their own usage. Direct-only repeated
runs must make zero model calls; emulator/infrastructure cost still exists.

## Planned execution and transport

Use a shared Rust-authored tagged action format across browser commands, recorded steps,
case definitions, generation proposals, worker requests and frozen manifests. Generate
browser SDK/types/Zod and worker Pydantic through the existing pipelines. No handwritten
consumer shapes or free-form executable scripts are introduced.

Python resolves each selected target against fresh hierarchy and performs direct commands
through an explicitly pinned uiautomator2 adapter on the already leased serial. Prefer
resource IDs and require unique matches. Frame-local control IDs and old coordinates are
not reusable selectors. Literal text goes through a device RPC, not shell interpolation.
Missing/ambiguous controls stop with a useful error; they do not trigger hidden AI recovery.

Both interactive sessions and approved regression runs use this executor. Per-step capture
feeds the existing independent Rust evidence checks. Agent completion and successful taps
alone are never a test pass. Keep supported assertion methods and require explicit expected
behavior; unsupported checks remain needs-input rather than being silently approximated.

Manual commands, automated sequences and discovery share one exclusive session command
lane. Preserve app/creator authorization, CSRF, scoped worker leases, idempotent request
fingerprints, revision fencing, reservations, heartbeat, cancellation, cleanup and quarantine.
A response lost after a possible device side effect is uncertain; never automatically replay it.

Add explicit direct/model capabilities and protocol versions. Direct profiles need no model;
old workers must not receive new action variants. Preserve legacy navigation JSON and stored
hashes, historical manifests, approvals and existing active-session semantics. Converting an
old prose test requires an explicit new draft/version; no silent historical data migration.

## Discovery and proposals

Rust persists a generation job before execution and dispatches it through the phone worker's
leased command queue. Python captures bounded observations, asks an isolated model child
for a typed next action, validates that action against the allowed journey and current controls,
and calls the shared direct executor. Persist actual action traces and source snapshots.
The model child receives no worker token and has no independent device-control tools.

Start with bounded discovery: 12 actions, 8 captured states, 180 seconds and 5 proposed
cases, each at most 20 steps. Include drafting in a maximum of 16 model calls;
freeze per-call output and context limits as specified in the plan. Count failed calls and unknown
usage, stop when limits prevent continuation, and show partial results honestly. These are
initial product defaults, not a promise of measured dollar cost.

Capture only qualified test-environment journeys. Read-only exploration is the default;
write/form actions require a journey that permits test-data changes. Enforce operation and
target restrictions outside prompts. Login, unsupported controls, external packages and
missing reset prerequisites stop discovery with a visible explanation.

Redact password content and corresponding screenshot regions before provider transmission;
filtering picker controls alone is insufficient. Treat all app text as untrusted data. Persist
private source references through the session’s build checksum, environment revision, device
image and observations, plus versioned template/proposal identity. Prompt-version cataloging
remains part of the later qualification/retention work. Cache/reuse only within the same
app and compatible scope; changed builds/environments invalidate old discovery context.

AI proposals contain suggested title/category, structured steps, source references, expected
checks or explicit unanswered questions, and potential duplicates. Screens show what occurred;
requirements or the author's explicit decision establish what should occur. Suggested expected
behavior needs confirmation before review. Do not redefine a seeded defect as correct behavior.
Generated output never automatically approves a test or updates the default plan.

Generation state and selected-proposal acceptance are durable/idempotent. Canceling stops
future model calls/actions, retains completed proposals, and leaves approved cases unchanged.
A worker crash with uncertain side effects quarantines the session rather than resuming it.
Source images are bounded private session payloads read through authorized routes. Separate
artifact retention/garbage collection remains part of phase 07; referenced source tasks must
not be removed while saved drafts depend on them.

## Delivery order and acceptance

1. **Direct execution and recording:** prove picked tap/type/restart/check steps on the
   qualified sample through both the phone API and approved run pipeline, with no model
   credentials/calls. Verify Unicode, ambiguity, failures, cancellation and cleanup.
2. **Templates and reusable tests:** bind template slots, save complete typed drafts
   atomically, review them through existing semantics and rerun the frozen definitions.
3. **AI discovery and proposals:** run bounded discovery, generate named grounded scenarios,
   resolve missing expectations, save selected proposals and execute their direct steps.

Use real HTTP/PostgreSQL tests and generated boundary checks alongside synthetic worker,
model and DOM tests. Add legacy serialization/hash fixtures, old-worker capability fencing,
concurrent-command/retry tests and atomic batch-save rollback coverage. Real-device and
small explicit real-model evaluation are separate acceptance gates. Broken/ambiguous source
fixtures must prove the generator preserves intent and uncertainty.

The qualified sample/device scope remains in force; this revision does not establish arbitrary
customer APK support, all-screen coverage, automatic login/reset or hosted reliability.
Respect the existing browser inspection restriction and record rendered acceptance separately.

## Connecting the local worker

The existing `task-worker` owns a disposable Android emulator and claims sessions from Rust:

```sh
uv run --no-sync --project apps/mobile-worker --frozen mobile-qa-worker task-worker \
  --origin http://localhost:5150 --state /absolute/private/task-worker \
  --profile /absolute/private/profile.toml
```

API identity is supplied through the existing operator registration and Doppler process injection.
Direct-only host profiles omit `model` and use a registered direct execution profile. Install the
explicit `device` extra; add `ai` and `sdk` for generation and explicit Minitap steps. No model
secret is needed for direct steps. Restart the API and worker after updating; only Vite has HMR.
Ordinary tests never start a device or model. The supported adapter remains `ai.mobileqa.demo`
on the existing qualified Android image; this does not establish arbitrary APK support.

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
current structured steps and checks; editing fields alone does not control the phone.
Preview frames refresh through the existing session polling, not a video stream.
This layout change has DOM interaction coverage; rendered browser acceptance remains
pending under the existing inspection restriction.

Task composition preserves generated structured actions through picking, editing, saving,
trial and regression execution. Pick target binds a resource ID or accessibility description;
Control phone executes it after fresh unique-target resolution. Recording appends completed
interactions once. Later screens must be reached before binding controls on them. Failed
selectors block; they never silently invoke AI. Expected checks remain separate from actions.

### Persistence and rollout

This revision reuses existing phone session/task JSON payloads and library mutation receipts;
no duplicate generation queue or new migration is needed. Commands increment a session revision,
freeze their request fingerprint and require worker protocol 2. Legacy worker claims remain
valid for legacy work only. Started step receipts are durable before action execution; terminal
receipts cannot change. Environment revision changes require a new session. Snapshots are bounded
private session data (eight frames, sixteen model calls maximum), delivered through the existing
authorized phone/job route rather than a second artifact store. Canceling discovery stops and
cleans up its owning session. A lost worker is quarantined, never resumed mid-action.

Discovery uses a generated structured-response model child with at most eight states, twelve
actions, three minutes and five proposals. Read-only discovery permits wait/back/swipe only;
form submissions need the explicit test-data switch and a described journey. Missing usage stops
further calls. Invalid output fails closed without repair calls. Password fields are masked before
provider requests. Proposals retain source IDs and versioned template/generation provenance;
accepting them creates drafts and never grants approvals. Existing-name hints cover the loaded
catalog page, not semantic deduplication across the whole library. Same-session captures can be
reused explicitly, while changed environment/build/session scope requires discovery again.

### Verification scope

`just smoke-direct-authoring` exercises real HTTP draft/review/run persistence with synthetic
worker evidence; it is not real-emulator acceptance. Worker unit tests exercise the same direct
RPC seam, literal Unicode input, ambiguity, uncertainty, explicit AI dispatch and generation
source rejection. The direct executor passed local sample input/save/restart with plain text and Vietnamese text
on 2026-09-13, with no model configured or invoked. Real-model proposal quality and the complete
rendered browser-to-device journey still require separate acceptance.
The supported device adapter remains the qualified Android demo; arbitrary APKs are not qualified.
