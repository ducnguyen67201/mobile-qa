# Phase 05 evaluator — iteration 1

Independent source review, 2026-09-12. No browser, screenshot, runtime, formatter,
typecheck or test was used. The existing browser policy remains binding. These
scores assess source intent provisionally; rendered visual acceptance is unverified.

| Criterion      | Weight | Provisional score |
| -------------- | ------ | ----------------- |
| Design quality | 0.35   | 8.0               |
| Originality    | 0.30   | 7.6               |
| Craft          | 0.25   | 6.8               |
| Functionality  | 0.10   | 6.5               |
| Weighted total |        | **7.43 / 10**     |

**Disposition: revise.** Functional gaps below block acceptance independently of
the numeric threshold. Generated transport outputs and automated checks are still
pending the complete-source validation gate; this is not a functional test result.

## What works in source

- The case → suite → release plan tabs and build-independent library make the
  workflow approachable. Introductory copy explains why these objects exist.
- Case authoring gives requirements, numbered actions and expected results primary
  space; advanced evidence settings stay in an accordion. Native buttons allow
  keyboard ordering without drag-only interactions.
- Frozen versions, content hashes and dual-purpose review are clearly introduced.
  Simulation warnings remain visible near profile selection and run preview.
- Drafts use explicit saving, before-unload protection, route blocking and
  workspace-keyed remounts. Conflict responses retain local values. Browser API
  adapters consume generated requests and runtime response/error validators.
- Existing Mantine surfaces, forest accents and quiet summaries continue the
  product's visual language without invented dashboards or metrics.

## Required revisions

1. **Recover from a stale save without losing all local changes.** In
   `draft-editor.tsx`, choosing “Keep my edits” after comparison retains the old
   expected revision and stale error, while Save remains disabled. Offer an
   explicit reviewed rebase onto the fetched revision that preserves local fields
   and requires another Save; any subsequent concurrent update must still conflict.
   Keep a separate cancel/continue-comparison choice, and do not silently overwrite
   remote fields.
2. **Move archive/restore to the shared entry surface.** `TestLibraryDetail.tsx`
   currently exposes lifecycle controls only in `FrozenVersion`. Operators cannot
   archive a draft-only entry or restore an archived draft-only entry. Both paths
   need the same confirmation, expected revision, error and permission behavior.
3. **Let reviewers inspect every frozen execution field.** `DefinitionSummary`
   omits check property, text filter, readiness selector and prerequisite references.
   Plans do not show their direct case/suite pins, selected profile or diagnostic
   retries before a build exists. Present these with meaningful labels and version
   links, using expandable technical detail where appropriate. Flattened coverage
   is useful but cannot replace the submitted membership and profile definition.

## Further clarity and craft

- “Show archived” sends a filter that means archived-only. Label that behavior
  explicitly, or provide Active/Archived selection.
- Apply long-text wrapping to stable keys, unavailable version IDs and comparison
  cells. Heading and content-hash styles already wrap, but ordinary identifier
  text does not inherit those safeguards.
- Distinguish historical approvals on closed candidates from active completion;
  avoid “Awaiting review” for a purpose once the candidate is rejected or needs input.

Feedback was sent to the generator and parent. A source re-review should follow
the fixes. Actual responsive layout, contrast, keyboard focus and rendered visual
quality remain pending permitted browser acceptance.
