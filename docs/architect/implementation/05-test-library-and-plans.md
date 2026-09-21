# Spec 05 — Save and run test library

## Current product behavior

The 2026-09-20 Save simplification supersedes the former two-review lifecycle.
A member edits a case, suite or release plan and clicks **Save**. No reviewer grants,
submission, business approval or executability approval are required. Save persists
content; it never claims that a test passed or starts a run.

Cases have ordered actions and optional checks for interactive trials. Release
snapshots still require the existing semantic fields and required evidence checks.
Suites select exact case versions. Plans select exact suites/cases and an execution
profile, with requiredness, variants and budgets. Builds are selected when running.

Suite and release-plan editors also present their ordered membership as a non-connectable
coverage flow. Authors select saved cases or suites directly from a canvas toolbar; a selection
appends the next numbered step immediately. Arrowed step rails visualize the stored order, while
the adjacent keyboard-accessible detail list retains explicit move, remove and requiredness
controls. The flow is a live projection of the existing form values and saved-version options;
its layout coordinates are browser presentation state and are never persisted as test semantics.
Frozen versions preserve the same hierarchy, including visibly unavailable references. Run
reports use a linear execution flow derived only from the frozen manifest and actual attempts
because the manifest does not retain suite provenance. Flow lines communicate containment and
order, not dependencies: every case still begins from its required clean state. The adjacent
form/report remains the authority for detailed editing or evidence.
When a suite has an executable saved version, its canvas exposes one **Run suite** action.
The tester chooses a build, qualified device and optional baseline. The browser refreshes server
readiness, closes any idle interactive phone session, submits the exact suite version
idempotently and opens the persisted live run map. Unsaved changes use **Save & run suite**,
so the queued manifest always pins the version returned by Save. The run map
polls durable attempt state and changes each numbered step from queued to running to its recorded
outcome; selecting a started step jumps to its evidence. A suite run uses a transient bounded
execution policy and does not create or mutate a release plan.
The [reliable smoke-suite spec](10-reliable-smoke-suite.md) owns explicit
run setup, pinned baseline, picker order and acceptance limits.
At narrow widths the map becomes a full-size top-to-bottom sequence instead of scaling the
desktop columns down. Ordinary wheel scrolling continues to move the page; intentional canvas
panning is bounded so the sequence cannot be lost in empty space. Saved case and suite pickers
add a selection immediately, while explicit keyboard-accessible move and remove controls remain
the only way to reorder or remove membership. The map is the primary coverage summary, so the
draft editor does not repeat it in a second large coverage card. Advanced execution values may
collapse behind a summary, but missing-profile and other blocking issues remain visible.

## Save and versioning

The API owns a single save service inside the existing app transaction and lock.
Manual edits, templates and selected AI proposals use it. It preserves bounded
incomplete content and returns field issues. Complete content with valid references
creates an immutable executable snapshot; identical saves reuse that snapshot.
In the case editor, setup issues link to their action/check and field. Selecting an
issue expands the relevant section, scrolls to the field and focuses it; invalid
fields and collapsed action rows are marked. Error locations follow stable item IDs
when actions move. Editing an affected field clears its previous server error; Save
revalidates the new value. Other unresolved errors remain visible.
Check details expose an optional screen-readiness picker independently of the result
target. Choose a stable control when the result may disappear; a missing result on
that ready screen fails the assertion, while a missing readiness control or ambiguous
result remains blocked. Picking a new result preserves an explicitly selected readiness control.

The server allocates versions. AI provenance stays on editor content and the catalog;
the executable case uses the permitted user-authored representation.

The current entry always opens editable, including entries previously In review,
Rejected or Needs input. Historical version links remain read-only. Reads hydrate
from the last immutable version if no working content exists, without writing on GET.
The internal draft DTO/storage names and GET/PUT /draft routes are retained.

The response's saved_version_id refers only to a snapshot matching current saved
content. An incomplete edit returns null, even if an older usable snapshot exists.
Interactive Run test tries the current form; release selection uses explicit saved
versions. The UI distinguishes unsaved edits from saved content and run readiness.

Mutation IDs and optimistic entry revisions protect retries and concurrent editors.
A failed response retry reuses its mutation ID. New mutation fingerprints use the
save_lifecycle_v2 namespace. Pre-upgrade receipts remain historical records; retrying
an old mutation ID returns an idempotency conflict rather than decoding an obsolete
response or replaying an old submit as Save. Refresh the page and reload saved content
after upgrading before starting a new mutation.

## Admission and permissions

Saved versions must belong to the app, remain active, and satisfy definition/reference
validation. Runtime admission retains package/adapter compatibility, qualified
profile, APK readiness, device capabilities, evidence methods, limits and budgets.
Save does not require an online device or qualified profile. Missing runtime setup
blocks Run with a readiness reason; it does not prevent saving complete content.

Membership and CSRF still protect edits. Operators retain archive/restore and explicit
default-plan selection. Saving a new case, suite or plan version does not retarget
existing membership pins or the default plan. Coverage deduplicates exact case,
variant and device selections and reports conflicting requiredness.

Queued manifests, completed reports, content hashes and historical versions remain
immutable. Archive removes new selection/admission but preserves historical evidence.
Operator imports link saved versions without manufacturing approval records.

## Upgrade and legacy audit

Forward migration 000008 renames review_state to legacy_review_state and permits
null for new saves. Prior states, review events, reviewer grants and execution
approval metadata remain historical evidence; active admission does not consult them.
Submit, review and fork routes and grant-reviewer/approve CLI commands are removed.
Historical DefinitionResponse.approvals stays readable and is empty for new snapshots.

Run normal startup migrations; never reset a user's database to upgrade. Restart
the API and refresh clients to load the regenerated contract. Downgrade refuses if
new save-only versions exist because it cannot invent review decisions. Use a
forward fix rather than changing audit history to force a downgrade.

## Validation and acceptance

- A member with no reviewer grants saves cases, suites and plans and uses their exact
  versions. New snapshots have no approval records.
- Incomplete edits survive reload; no-op/retried saves do not duplicate snapshots.
- Concurrent saves yield one commit and one stale-revision response, preserving edits.
- Cross-app references, revoked access, unsupported execution and archives stay blocked.
- AI source identities are validated, batch saves are atomic, and AI tags survive editing.
- Existing review-state records open editable and migration preserves historical bytes.
- Simulated HTTP acceptance saves a case/suite/plan, selects a default, queues a run,
  edits the case, restarts the API and verifies the original manifest unchanged.
- Intentional Buy milk input with Buy eggs expectation saves successfully and fails
  during checking. Save and successful execution are separate outcomes.

Use status.md for observed validation evidence and remaining real-device acceptance.
