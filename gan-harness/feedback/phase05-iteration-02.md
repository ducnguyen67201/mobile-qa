# Phase 05 evaluator — iteration 2

Independent source re-review, 2026-09-12. No browser, screenshot, runtime, test,
build or formatting command was used. Scores remain provisional code assessment;
rendered appearance and actual browser acceptance remain unverified.

| Criterion      | Weight | Provisional score |
| -------------- | ------ | ----------------- |
| Design quality | 0.35   | 8.2               |
| Originality    | 0.30   | 7.6               |
| Craft          | 0.25   | 7.8               |
| Functionality  | 0.10   | 7.5               |
| Weighted total |        | **7.85 / 10**     |

**Disposition: revise despite exceeding the numeric threshold.** A remaining
unsaved-data-loss path blocks source acceptance. No automated checks have been run
for this review; the parent owns the consolidated validation gate.

## Confirmed revisions

- The comparison modal now offers an explicit choice to retain local content
  against the displayed saved revision. It updates the revision baseline, clears
  the stale error and requires a separate Save using compare-and-swap semantics.
- Shared `EntryLifecycle` exposes archive and restore for draft-only entries as
  well as frozen entries. Permissions, expected revisions and confirmation remain
  visible. Draft editability follows refreshed server capabilities.
- Frozen detail includes action and checkpoint identities, check property and
  selectors, prerequisites, exact case/suite membership and profile/retry choices.
  These details are independent of build availability. Eligible references link to
  their frozen version. Content hash and simulation disclosure stay visible.
- The archive filter is accurately labeled “Archived only”. The existing global
  Mantine Text style supplies long-word wrapping, including comparison values.
- Authored DOM tests cover explicit saves, stale-revision recovery, immutable
  dual-review hash binding, action deletion, unsaved navigation and draft-only
  restoration. Their presence is source evidence, not a claimed passing result.

## Remaining blocker

**Preserve mounted draft content during background refresh failures and concurrent
submission.** `DraftWorkspace` currently replaces the editor with `ErrorNotice`
when a refetch errors, even if cached draft data exists. Separately,
`EntryWorkspace` replaces the editor with a frozen version when an entry refresh
reports no current draft after another actor submits it. Both unmount the local
unsaved state without route navigation, so the route blocker cannot protect it.
Keep an opened editor mounted, show a non-destructive warning and require an
explicit exit/discard decision. Add regression coverage for both triggers.

## Small corrections

- A closed review candidate should say “Closed”, not “Awaiting review”, for a
  purpose with no approval.
- Empty expected strings in property-equality checks should be labeled “Empty
  string”; the fallback “presence” describes a different verification method.

Feedback was sent to the generator and parent. Another focused source pass should
confirm these changes; rendered responsive layout, focus behavior, contrast and
visual quality remain pending permitted browser verification.
