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

## Save and versioning

The API owns a single save service inside the existing app transaction and lock.
Manual edits, templates and selected AI proposals use it. It preserves bounded
incomplete content and returns field issues. Complete content with valid references
creates an immutable executable snapshot; identical saves reuse that snapshot.
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
