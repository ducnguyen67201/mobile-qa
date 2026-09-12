# 05 — Test cases, suites and plans

Status: planned. Depends on: 04. Owns: Rust test-definition/versioning services, migrations and DTOs; frontend Tests screens.

## Deliverable

Replace operator-only case fixtures with a compact editor/review experience. A customer can understand exactly what will run and rerun approved tests on a new build without regenerating them.

## Objects and rules

| Object | Meaning | Minimum fields |
|---|---|---|
| Test case | One behavior with explicit expected checks | Title, source/criterion references, prerequisites, setup/reset, steps, checks, evidence method, fixture references, budgets |
| Suite | Named grouping such as Login or Tasks | Title, ordered case-version membership |
| Test plan | Reusable selection for a purpose | Suite/direct-case selection, requiredness, data/device configuration, policy |
| Run | One execution against a particular build | Immutable resolved manifest, attempts and results |

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
