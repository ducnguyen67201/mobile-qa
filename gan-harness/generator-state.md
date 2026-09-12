> Historical baseline below describes the original shadcn UI. Mantine now replaces it; previous scores/build sizes are not current verification.

# Generator state

Initial authoring cycle complete; evaluator review pending. Official shadcn/ui new-york-v4 source pinned at 3ba91b1cc83e1bbe4ab35a422ff2a694849c5048; source provenance recorded in apps/web/SHADCN-SOURCE.md. Design uses a forest sidebar, warm light canvas, deliberate type/spacing, large calm first-app empty state and evidence-led build detail. No fabricated metrics. Sidebar, sheet, inputs, table, menus, status primitives are imported from upstream. Responsive main/aside layout and keyboard navigation use those components.

Functional scope authored: session guard/sign-in/logout, apps/create, environment revisions/reference selection, settings, uploads and recovery, build history/deep links/status, honest readiness. Auth guard/empty-state/create-sheet/placeholder DOM tests, stored-build status/upload-recovery DOM tests, and real generated SDK fetch-boundary tests are authored. Source review corrected nested main landmarks and recovery of saved build links. No checks, builds, runtime, or browser inspection executed. Rendered acceptance remains open due to existing policy denial.


## Iteration 02 response
All iteration 01 findings addressed in source: backend origins required with matching create/edit guidance; exact duration formatting (30-minute expiry retained); terminal detail status invalidates history and validating rows poll including unselected rows; account menu and workspace navigation are labeled; core metadata uses readable small text; long environment labels wrap; changing app resets page state/cursors via keyed content; identifier wrapping is explicit and prose uses normal word wrapping. Added required-origin, exact-duration, selected-terminal consistency, and unselected polling regression test sources. Checks remain deferred until the root's final validation window.


Iteration 02 evaluator result: provisional source-only PASS, 7.87/10 (see feedback/iteration-02.md). Selected-detail regression additionally simulates validating → terminal on a subsequent polling request. No runtime/rendered quality claim; final validation and rendered acceptance remain pending.

## Final executable frontend validation

After the root opened the consolidated validation window: TypeScript and ESLint pass; all 51 Vitest tests in 5 files pass; production Vite build passes; pnpm audit reports no known vulnerabilities. Build emits a non-blocking 500 kB chunk-size warning (main JS 670.69 kB, gzip 203.60 kB). No browser was opened.

Validation found and resolved pinned Hey API 0.99 interoperability issues: root bounded byte schemas now generate JS numbers consistently; the schema-derived binary resolver emits z.instanceof(Blob); the application client passes the original SDK header record to generated request validation while preserving generated transport. Test fetch primitives were aligned to Node's Blob/File/FormData brands to accurately exercise multipart serialization under JSDOM. Generated files remained root-owned.

Rendered desktop/mobile design, actual browser keyboard behavior, zoom and contrast still require permitted manual acceptance. Source-only GAN score remains provisional 7.87/10.

## Mantine maintenance refactor

All product surfaces now use packaged Mantine 9.6.1 controls. Removed local shadcn
primitives, utility helper, mobile hook and Tailwind/plugin dependencies. The warm
canvas/forest navigation remains centralized in theme.ts and small product CSS.
Existing real API logic and validation/readiness messages are preserved. DOM tests
use MantineProvider in test mode plus a layout-only ResizeObserver stub. New behavior
coverage exercises drawer Escape/focus restoration and account-menu logout.
Fresh source review passed after named close controls, breakpoint-driven drawer
closure, scrollable navigation and long-text wrapping fixes. All 53 tests, TypeScript,
ESLint, Vite production build and pnpm audit pass. Main JS is 728.88 kB / 220.86 kB gzip,
with a nonblocking chunk-size warning. No browser inspection occurred. See
feedback/mantine-review.md; previous scores remain historical.
