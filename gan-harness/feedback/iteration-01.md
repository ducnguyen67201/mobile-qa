# GAN design evaluation — iteration 01

Source-only review of the authored frontend. No browser, runtime, build, tests, formatting or exports were run. Scores are provisional assessments of implementation intent, not observations of rendered quality. Generated bindings are deliberately awaiting the root's consolidated generation window. Rendered acceptance remains open.

| Dimension      | Score | Weight | Evidence                                                                                                                                                                                                                   |
| -------------- | ----: | -----: | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Design quality |   8.0 |    .35 | Coherent forest/ivory theme, restrained accents, spacious empty state, clear main/aside composition and status/identity separation. Small secondary text needs refinement.                                                 |
| Originality    |   7.3 |    .30 | Appropriate Android workbench identity and evidence-first language; no invented metrics. Primarily conventional cards and sidebar, with purposeful rather than decorative customization.                                   |
| Craft          |   6.7 |    .25 | Real Radix primitives, labeled inputs, skip link, focusable errors, reduced-motion treatment and mobile sheet. Missing useful navigation/account labels, stale status coordination and misleading durations reduce polish. |
| Functionality  |   6.0 |    .10 | Broad generated SDK/Zod integration and upload reconciliation are present, but three concrete user-facing defects below block acceptance.                                                                                  |

**Weighted score: 7.27 / 10. Result: revise.** The numerical threshold cannot override functional blockers.

## Required fixes

1. **Backend origins are falsely labeled optional.** `apps/web/src/components/app/app-form.tsx` renders an optional label and permits blank input, while `apps/api/src/services/apps.rs` calls `origins(..., true, ...)` and rejects an empty list. An operator following the form cannot create the app as instructed. Remove the optional label, require at least one backend origin, and give matching guidance in the environment editor. Keep login origins optional. Add a focused form regression case when the authoring batch is ready for test additions.

2. **Upload expiry is displayed incorrectly.** `apps/web/src/pages/Settings.tsx` uses `Math.round(upload_ttl_seconds / 3600)`, converting the actual 1,800-second TTL to “1 hours”. Display exact human durations (30 minutes here, 1 hour for session TTL), including correct singular/plural. This affects whether users expect an interrupted upload to remain recoverable.

3. **Build history can contradict the selected build's actual status indefinitely.** `apps/web/src/api/setup.ts` polls `buildQuery` while validating, but `buildsQuery` never polls or gets invalidated when that query reaches a terminal state. `AppDetail.tsx` renders the history badge directly from the stale list. A user can see “Validating” in the row and “Validated” below. Coordinate the cache when a selected build changes and/or poll the list only while one of its builds is validating. Cover validating → terminal consistency, including an unselected validating row if list polling is chosen.

## Craft improvements for this cycle

- Add a meaningful accessible name to the account dropdown trigger in `apps/web/src/App.tsx`. At the small breakpoint the visible name disappears and only the user's initial names the control. “Open account menu” plus the user's name is clearer.
- Wrap workspace navigation in a labeled `nav` landmark; the current official sidebar composition consists of generic `div` and `ul` elements.
- Raise core metadata labels, build timestamps, upload guidance and validator/policy details from 10–11px to a consistent readable small-text scale where practical. Decorative eyebrows may remain smaller. Rendered contrast/zoom/overflow checks remain unverified.
- App cards should wrap long environment names; the footer currently lacks `min-w-0`/wrapping and the arrow lacks `shrink-0`. Also reset build pagination when `appId` changes, so browser history or direct route transitions cannot apply an old app's cursor to the new app.
- `PageHeading` applies `break-all` to ordinary descriptive prose as well as package names. Prefer normal word wrapping for prose and an explicit monospace/wrapping treatment for identifiers.

## Preserve

Keep the official shadcn provenance, quiet layout, separate validation/readiness language, stored upload recovery, real metadata/hash disclosure, and absent fake dashboard statistics. Functional completeness remains mandatory. No need for extra animation, new widgets or decorative analytics to raise the score.

## Remaining evidence gates

After root finishes all scoped edits: generation, typecheck/lint, authored DOM/transport tests and application checks. Rendered desktop/mobile, keyboard behavior, contrast, zoom and actual APK workflow acceptance must remain explicitly unverified until the existing browser policy permits inspection; do not bypass that restriction.
