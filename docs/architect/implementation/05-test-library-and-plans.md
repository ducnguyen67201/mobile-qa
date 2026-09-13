# 05 — Test cases, suites and plans

Status: locally implemented; deterministic validation and simulated HTTP acceptance passed. Rendered browser and UI-triggered real-device acceptance pending. Depends on: 04. Owns: Rust test-definition/versioning services, migrations and DTOs; frontend Tests screens.

## Deliverable

Replace operator-only case fixtures with a compact editor/review experience. A customer can understand exactly what will run and rerun approved tests on a new build without regenerating them.

## Objects and rules

| Object    | Meaning                                    | Minimum fields                                                                                                              |
| --------- | ------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------- |
| Test case | One behavior with explicit expected checks | Title, source/criterion references, prerequisites, setup/reset, steps, checks, evidence method, fixture references, budgets |
| Suite     | Named grouping such as Login or Tasks      | Title, ordered case-version membership                                                                                      |
| Test plan | Reusable selection for a purpose           | Suite/direct-case selection, requiredness, data/device configuration, policy                                                |
| Run       | One execution against a particular build   | Immutable resolved manifest, attempts and results                                                                           |

A plan does not permanently select the build. Choose the build at run time. Cases can belong to several suites; resolve duplicates using case version + data variant + device configuration. If duplicate selections disagree on policy/requiredness, reject the ambiguity for review rather than choosing silently.

Draft content can be edited. Approval records actor, time and exact version. Editing approved content creates a new draft; existing suites/plans/runs continue referencing their pinned versions until explicitly updated. Archiving removes items from new selection without deleting historical evidence.

## Implementation order

1. Case list and detail editor, backed by the versioned records introduced in spec 04.
2. Validation and readiness: missing expectations, unmet prerequisites, absent reset path and unsupported verification stay explicit.
3. Review actions: approve, reject/needs-input, archive; record audit identity. Customer reviewer approves business expectations; operator checks executability. One person can hold both permissions.
4. Suite grouping and membership editing; preserve many-to-many relationships.
5. One visible default regression plan with preview: unique cases, required checks, excluded gaps, device, fixtures and estimated budget where measured. Add named additional plans only when the default workflow works.
6. Run an approved plan on another uploaded build through spec 04.

Keep routes under Tests with tabs/detail panels. No drag-and-drop workflow canvas, test programming language or script-export subsystem is needed for the pilot.

## API behavior

Expose create-draft/update-draft/list/detail/version-history and approve operations for cases, suites and plans. Enforce project scope and use optimistic concurrency for edits/approval. Approval must fail if the reviewed revision changed. Normal runs reject unresolved draft references and incompatible configurations before queuing.

Business constraints are Rust service validation, not merely browser form schemas. Generated types describe shape, while server validation enforces meaning. Avoid allowing arbitrary Python/shell snippets as test steps or reset definitions; fixtures use qualified operator-owned adapters.

## Acceptance

- Approve case A, run it, edit A: the old report and approved plan remain unchanged.
- Select one case through two suites: preview and manifest contain one execution for the same variant/device.
- Stale approval, conflicting membership policies and unsupported check methods return useful errors.
- Requiredness is visible before submission and preserved in results.
- Tests cover version pinning, approval concurrency, manifest resolution and authorization. One browser flow proves edit → approve → plan preview → run.

This completes the user-authored test workflow. AI generation in spec 06 must produce drafts through these same services instead of bypassing review.

## Implementation packet

The implementation separates
editable drafts from immutable published versions, with exact-revision/hash review,
archive/default-plan rules, typed browser APIs and the complete UI-to-run flow.
Rust owns all DTOs and semantic validation; generated SDK/Zod and Pydantic remain
the only browser/worker contract pipeline. Real route agreement, stale-edit tests
and authoring-to-worker HTTP acceptance are explicit gates.

PR #5 merged into the phase 02 branch, not main; use the integrated tree or verify
that a later main includes it. The existing real API-to-emulator good-path test
does not replace browser acceptance of the new editor/review workflow. The implementation now uses that integrated baseline; new rendered acceptance remains pending.

## Implemented source

Migration 000005 adds the catalog, editable draft, frozen version lifecycle, review
activity, explicit default and atomic mutation receipt tables. Existing execution
versions/hashes and run manifests stay immutable. Upgrade validates historical
references and copies the old latest approved plan choice once; later approvals
never change that pointer.

The authenticated `/api/apps/{app_id}/test-library` routes support listing, create,
save, fork, submit, exact-hash review, archive and version history. App members can
author; purpose-specific grants govern both reviews. Operators alone archive or
select the default through `/api/apps/{app_id}/default-test-plan`. CLI imports and
approvals share the catalog/lifecycle rules. An optional controlled-demo template
uses the existing persistence case; ordinary creation starts an incomplete draft.
Browser-authored cases use server-assigned `user_authored` provenance.
The browser automatically assigns a UUID-based stable key for new cases, suites
and plans. Authors enter a readable title in the draft instead of inventing a key;
the saved key remains unchanged across versions. Creation retries preserve the
original mutation and entry identity.

Entry revisions and mutation UUIDs prevent lost updates and make identical retries
safe. Draft plans explicitly permit a missing profile. Save returns typed issues;
submit requires valid content and approved, active references. Requiredness/version
conflicts block publication. Archive blocks new runs transitively while previously
queued and completed runs use their frozen manifests.

The Mantine Tests UI exposes Cases, Suites and Release plan tabs, explicit saves,
reviewable stale-edit recovery, action/check ordering, advanced evidence settings,
version history and the existing run/report screen. No draft content is persisted
in browser storage. The Rust-owned DTOs generate browser SDK/types/Zod; safe feature
error details are also validated at runtime. Published definitions use the existing
Rust/Schemars/Pydantic worker protocol.

`just smoke-test-library` authors and reviews case/suite/plan versions through real
HTTP routes, explicitly selects the default, runs simulated Python evidence after
API restart and checks the manifest remains unchanged after a new draft edit.
It does not prove rendered UI or real emulator acceptance. Validation evidence and
remaining gates belong in the [current status](../status.md).
