# Plan: Commercial access and usage authorization

## Summary

Add a truthful pricing/usage screen and server-enforced authorization for the proposed operated Android QA offer. A $500 two-week pilot includes four reviewed checks; after a manually agreed renewal, a qualified app has a $250 monthly base, $125 per authorized check, and an optional $250 second-suite allowance. This plan implements agreement and usage records plus manual settlement; it does **not** take card payments or send invoices. Prices remain hypotheses until a real customer pilot measures cost and payment intent.

## User story and scope

As a small Android team's release owner, I want to see my agreed coverage, the exact charge and remaining authorized checks before starting a release check, so that I can budget an irregular release cadence without surprise charges.

**Complexity:** XL; cross-cutting schema, contracts, API authorization, three run submission paths, resource-consuming phone/authoring paths and UI. **Source PRD:** N/A; commercial assumptions from `.private/research/pricing-research-2026-09-21.md` and proposed pilot in `docs/architect/product.md:407-417`. **Estimated files:** 22–28 including tests/docs/generated outputs. **Status:** plan only. The user-requested GAN artifact is a visual prototype, not a working screen.

### Commercial contract to encode

| Mode           | Terms and visible usage                                                                                                                                                                                                                                                            |
| -------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Uncontracted   | May sign in, set up app, inspect saved work and request a pilot; cannot start paid-resource execution.                                                                                                                                                                             |
| Founding pilot | Proposed $500 once for 14 days; one qualified Android app/profile, up to five journeys/ten cases, four included reviewed suite/plan checks. No $125 charge for the four included checks.                                                                                           |
| Recurring      | Proposed $250 USD per contracted period for one suite of at most ten cases and up to one hour minor maintenance; $125 per authorized reviewed check; optional second suite (at most ten cases) and one more maintenance hour for $250 per period; at most eight checks per period. |
| Paused/expired | No new execution or charge; existing reports and agreed retention remain accessible. A run accepted while active may finish after expiry.                                                                                                                                          |

One check is one approved suite or release plan of at most ten resolved case occurrences on one accepted build and one qualified profile. A two-build comparison uses two checks. A two-suite build uses two checks. Bound each full run to the product's proposed 30 minutes and one diagnostic retry per case; do not claim that this is already proven on customer apps. A standalone saved-case run, interactive phone task or AI generation is **outside the priced check**: make those unavailable to customer sessions in commercial mode until an explicitly priced operator workflow is agreed. Never let them bypass the commercial gate. Read-only editing/setup remains available.

The amount is denominated in integer USD cents. A customer authorizes an individual quote before submission; there are no automatic overages. The contract period is a stored, half-open UTC interval `[starts_at, ends_at)` chosen in the manually accepted agreement. No implicit renewal or proration. The $250 base and $250 add-on are manually reconciled once per agreed period, including zero-check periods. Taxes, payment timing and cancellation terms belong in the signed customer agreement, not inferred by code.

## UX transformation

**Before:** Apps/Tests can preview and submit runs immediately. Settings has access, upload and storage information but no price, agreement or check ledger. The current `RunPreview` button says “Run release check.”

**After:** A Settings-linked “Pricing & usage” page at `/settings/commercial?app=<app_id>` explains the proposed pilot and current agreement, with an app selector limited to the current workspace. For an active app it shows period, suites, remaining checks, accrued delivered charges, pending authorized checks and credits. Every billable run surface shows a server quote with app/build/suite/version/profile, `$125` or `$0 included pilot`, current period total and cap before an explicit “Authorize check and run” action. If no agreement, show “Request a pilot” with a scoped, non-payment contact/qualification flow. A changed build, suite, price, period or readiness invalidates the quote and requires review. Errors never turn into free runs.

| Touchpoint                                      | Change                                                                                                                                       |
| ----------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| App detail / saved-suite / release-plan run     | Show quote after normal readiness; show clear blocker and pricing link if uncontracted; confirm then submit with quote identity.             |
| Saved-case and interactive phone / AI authoring | Keep reading/editing where safe; explain that execution requires operator-scoped work and block resource-consuming create paths server-side. |
| Pricing & usage                                 | Show contract state, USD itemization, 0–8 check estimator, per-run ledger and credit reason; mark sales prices as proposed before agreement. |
| Report                                          | Show check authorization/settlement status separately from pass/fail; never imply an app defect earns a credit automatically.                |

## Mandatory reading and codebase discovery

The Graphify index exists but was built at `aced159b42b32ac65f26c62a9551ef1e6d102076`; this checkout is `00521cdb6ebf95b0f527809b1f5cbeed029a917e` with substantial local edits. A focused Graphify query found the run path; the source references below supersede it. `docs/CODEX-NAVIGATION-GUIDE.md` referenced by the user-provided supplement is absent in this checkout.

| Priority | File:lines                                                                                                                                                  | Pattern / why                                                                                                                    |
| -------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| P0       | `docs/architect/README.md:1-40`, `product.md:300-322,365-390,407-417,445-451`, `implementation/07-pilot-readiness-and-scale.md:32-42`, `status.md:1-46`     | Authority, pilot bounds, billing automation deferred, real-device gates.                                                         |
| P0       | `apps/api/src/services/runs.rs:212-296`, `suite_runs.rs:102-175`, `case_runs.rs:84-168`                                                                     | Three independent create transactions, app lock, idempotency replay, manifest insertion. New metering must be atomic with these. |
| P0       | `apps/api/src/controllers/runs.rs:22-52,115-210`                                                                                                            | All three browser create routes; session and idempotency header.                                                                 |
| P0       | `apps/api/src/controllers/task_sessions.rs:1-45,117-130`, `test_authoring.rs:1-81`                                                                          | Phone open/task/command and generation can consume resources outside `execution_runs`.                                           |
| P0       | `apps/api/src/services/apps.rs:18-44`, `tasks/operator.rs:31-76,132-162`                                                                                    | Tenant authorization versus trusted process-only operator actions. Customer workspace `operator` is not a billing administrator. |
| P0       | `crates/contracts/src/browser.rs:74-80,758-765`, `execution_api.rs:5-35,42-53,115-138`, `lib.rs:1-28`                                                       | Rust-owned DTO/OpenAPI and aggregate merge; browser shapes are generated.                                                        |
| P1       | `apps/api/src/services/execution_store.rs:1-66`, `errors.rs:9-48,50-88`, `app.rs:57-77`, `migration/src/lib.rs:1-37`                                        | Bound SQL, safe errors/logging, route/task and migration registration; preserve scaffold markers.                                |
| P1       | `apps/web/src/components/app/run-preview.tsx:33-64,65-143`, `components/runs/saved-suite-run.tsx:35-125,133-223`, `saved-case-run.tsx:25-101,105-188`       | Existing preview, retry identity and buttons. The latter two are being edited on this branch: coordinate before implementation.  |
| P1       | `apps/web/src/api/runs.ts:1-66`, `api/regression.ts:1-58`, `api/session-transport.ts:1-23`, `api/runtime.ts:10-63`                                          | Generated SDK, generated Zod at boundaries, CSRF, error validation and stable create idempotency.                                |
| P1       | `apps/web/src/routes.tsx:1-57`, `components/app/navigation.tsx:27-35,130-181`, `pages/Settings.tsx:16-110`, `theme.ts:11-81`, `hooks/use-workspace.ts:4-32` | Protected workspace routes, navigation and forest/Mantine design.                                                                |
| P1       | `apps/api/tests/execution.rs:124-184`, `apps/web/src/pages/AppDetail.test.tsx:1-124`, `apps/web/package.json:7-15`, `justfile:41-70`                        | Real route/database contract tests and browser component test setup; final validation commands.                                  |

### Source patterns to mirror

Create under app lock and replay before new work (`apps/api/src/services/suite_runs.rs:119-139`):

```rust
    let tx = ctx.db.begin().await?;
    one(
        &tx,
        "SELECT id FROM apps WHERE id=$1 FOR UPDATE",
        vec![app.into()],
    )
    .await?;
    if let Some(row) = rows(
        &tx,
        "SELECT id,fingerprint FROM execution_runs WHERE app_id=$1 AND idempotency_key=$2",
        vec![app.into(), key.into()],
    )
    .await?
    .first()
    {
        if field::<String>(row, "fingerprint")? != fingerprint {
            return Err(conflict("Idempotency key belongs to another submission"));
        }
        let run = runs::detail(&tx, field(row, "id")?).await?;
        tx.commit().await?;
        return Ok((run, false));
    }
```

Typed, safe API failure (`apps/api/src/errors.rs:39-41`) and structured execution log (`apps/api/src/services/case_runs.rs:166`):

```rust
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new(422, "invalid_input", message)
    }
    tracing::info!(run_id=%id,app_id=%app,phase="queued",source="saved_case_v1",reason_code=%reason_code,"Saved case queued");
```

Generated request and Zod on both sides (`apps/web/src/api/runs.ts:49-60`):

```ts
export function createRun(appId: string, body: CreateRunRequest, key: string) {
  return checked(
    sdk.createRun({
      ...options,
      path: { app_id: appId },
      headers: { ...headers(), 'Idempotency-Key': key },
      body: z.zCreateRunRequest.parse(body),
    }),
    z.zRunResponse,
    [200, 201],
  )
}
```

`apps/api/src/services/execution_store.rs:22-54` binds SQL values via SeaORM; migrations own constraints. `apps/api/tests/execution.rs:124-184` exercises actual routes/database, 201/200 idempotency and foreign access. `apps/web/src/pages/AppDetail.test.tsx:19-47,100-123` uses a memory router, mocked fetch, generated Zod and user-event clicks. No custom domain framework or handwritten consumer transport types.

## Architecture and invariants

1. **Commercial state:** add an app-scoped agreement table with status, offer (`pilot`/`recurring`), immutable price revision, period, suite/version allowlist, included/purchased cap, optional second suite, agreed manual invoice reference and audit timestamps. A history table records every trusted operator activation/pause/revision/credit with actor and reason. Do not put card data or secrets in it. No price is read from the browser.
2. **Quote:** create a short-lived, server-persisted quote for the exact already-admitted run intent/manifest fingerprint, actor, app, build, suite/plan version, profile, environment revision, price revision, period and amount. `GET /api/apps/{app_id}/commercial-access` returns app-scoped current state and ledger summary; `POST /api/apps/{app_id}/commercial-check-quotes` takes a tagged existing run intent and returns a quote/expiration/line items. Validate 10-case and one-suite bounds on the resolved manifest, not on user-supplied counts.
3. **Authorization:** submission uses an `X-Commercial-Quote-Id` header plus the existing `Idempotency-Key`. Show price/cap and have the user explicitly click authorize; no prechecked consent. Under the existing app lock and in the same transaction as run/attempt insertion, re-resolve readiness, verify quote actor/app/request fingerprint/price/period/expiry, count existing reservations, then insert exactly one ledger reservation keyed uniquely by `run_id`. Include quote identity in the new idempotency fingerprint. On replay, return the same run and ledger effect even if the quote later expires. Reject stale/mismatched quote or ninth check with a typed 409; no run is queued.
4. **Settlement:** reserve one check when accepted so concurrent tabs cannot exceed cap. An operator marks the reviewed report `delivered` (billable `$125` or consumes pilot inclusion), `credited` (our infrastructure prevented a usable report), or `unresolved` (no invoice until reviewed), with reason and audit identity. A passing verdict is not required; an app defect can yield a delivered, billable check. Preflight failure before run insertion creates no reservation. A queued run canceled before device claim is credited. On other cancellation/blocked cases, the reviewer follows the agreed policy and preserves original attempt evidence. Never silently auto-bill all `finished` runs.
5. **Other resource paths:** centrally gate paid app phone open/task/commands and AI generation before worker/model work; also gate saved-case create. Keep stop/cancel/read/artifact routes available for safety and customer access. An internal diagnostic path needs a separate trusted process-only grant and cost ledger, not a workspace-role bypass. Inventory every new resource-consuming endpoint during implementation.
6. **Manual settlement:** present base/add-on, delivered checks and credits for the stored period; export a reconciliation report for a human to issue an invoice externally. Do not store payment method or mark invoices paid automatically. A customer may view usage but cannot activate/reprice/credit an agreement. Commercial activation uses a trusted CLI task with an audited internal actor; the customer workspace `operator` role is insufficient.
7. **Rollout:** default hosted app commercial status is uncontracted and fail-closed for new resource use. Existing local/fake tests receive explicit test fixtures/grants; do not install a broad production bypass flag. Preserve historical run manifests and their idempotency behavior when no commercial quote existed. Keep read/export access on expired plans subject to normal retention. Implement a backfill/legacy policy before enabling enforcement for existing hosted apps; no hosted cutover until the technical acceptance gates in product/07 are met.

The visual design in `gan-harness/paywall-prototype.html` is exploratory. Its first generator/evaluator pass scored 8.31/10 against the design rubric; subsequent small accessibility and zero-use billing clarifications were applied. The evaluation was code-only because the repo records a browser admin-policy denial. Translate the concept into Mantine components and the existing theme rather than embedding the static HTML in production. Do not claim rendered visual acceptance.

## Files to change (proposed)

| File(s)                                                                                                                                                                 | Work                                                                                                                                                                                                                          |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/architect/product.md`, `implementation/07-pilot-readiness-and-scale.md`, `decisions.md`                                                                           | Mark commercial terms proposed; document commercial access, charge/credit rules and acceptance; retain automated billing as future work.                                                                                      |
| `apps/api/migration/src/m20260921_000014_commercial_access.rs`, `migration/src/lib.rs`                                                                                  | Add constrained agreement, quote, ledger and audit tables and indexes. `000013` is currently an unregistered in-progress hierarchical-suite migration: choose the next free version at implementation time and coordinate it. |
| `crates/contracts/src/commercial.rs`, `commercial_api.rs`, `browser.rs`, `lib.rs`, `execution_api.rs`, `regression_api.rs`                                              | Pure Rust request/response/error contracts, Utoipa paths, OpenAPI merge/operation list and quote header on existing create operations. No Loco/DB types.                                                                      |
| `apps/api/src/controllers/commercial.rs`, `controllers/mod.rs`, `app.rs`                                                                                                | Authenticated status/quote/ledger routes; preserve route markers.                                                                                                                                                             |
| `apps/api/src/services/commercial.rs`, `services/mod.rs`, `tasks/operator.rs`                                                                                           | App-scoped quote, atomic reserve, audited review/credit and trusted agreement activation.                                                                                                                                     |
| `apps/api/src/services/runs.rs`, `suite_runs.rs`, `case_runs.rs`, `controllers/runs.rs`                                                                                 | Quote validation and reservation in all run creates, preserving replay semantics.                                                                                                                                             |
| `apps/api/src/services/task_sessions.rs`, `test_authoring.rs`                                                                                                           | Gate phone/model work; leave stop/cancel/read paths intact.                                                                                                                                                                   |
| `apps/api/tests/commercial.rs`, affected existing route tests                                                                                                           | Real PostgreSQL HTTP tests for price, access, race, replay and credit; no device/model call.                                                                                                                                  |
| `apps/web/src/api/commercial.ts`, `pages/CommercialAccess.tsx`, `components/commercial/RunAuthorization.tsx`, existing run controls, `routes.tsx`, `pages/Settings.tsx` | Generated transport, Settings-linked app-scoped pricing/usage page and explicit pre-run quote.                                                                                                                                |
| `apps/web/src/pages/CommercialAccess.test.tsx`, affected run-page tests                                                                                                 | Quote display, no-agreement, stale/race/error and explicit consent behavior.                                                                                                                                                  |
| `contracts/browser.openapi.json`, `apps/web/src/api/generated/*`                                                                                                        | Generated only by `just types`; do not hand-edit.                                                                                                                                                                             |

## Tasks in dependency order

1. **Finalize proposed commercial rules.** ACTION: update owning architecture documents with the table/invariants above. IMPLEMENT: scope and offer version, who may activate, period semantics, exact billable/credit policy, customer support/contact CTA and pilot acceptance dependencies. MIRROR: status labels in `docs/architect/README.md:20-29`; product section 20. IMPORTS: none. GOTCHA: no claim of deployed checkout or validated unit economics. VALIDATE: doc link/consistency review.
2. **Add durable schema and pure contracts.** ACTION: migrate tables and add DTO/OpenAPI declarations. IMPLEMENT: integer cents, USD currency, per-app foreign keys, unique quote and `ledger.run_id`, nonoverlapping active periods, CHECK constraints, append-only audit, price revision. MIRROR: execution migration `m20260912_000004_execution.rs:8-25`, contract `execution_api.rs:5-35,42-53`. IMPORTS: SeaORM migration prelude; `serde`, `utoipa`, `uuid`, `chrono` in contracts. GOTCHA: local `000013` is in progress; settle migration order before code. VALIDATE: fresh DB migration/up-down and OpenAPI operation/route agreement test.
3. **Implement trusted activation and status/quote.** ACTION: add internal CLI activation/pause/credit and app-scoped read/quote routes. IMPLEMENT: exact run intent fingerprint, short expiration, status/price/period checks, resolved suite bounds, quote only when ordinary execution preview has no blockers; safe typed errors. MIRROR: process-only `tasks/operator.rs:31-76`, app auth `services/apps.rs:18-44`, SQL helpers `execution_store.rs:22-54`. IMPORTS: `AppContext`, `Session`, `ApiFailure`, generated Rust contracts, SeaORM transaction traits. GOTCHA: customer workspace operator cannot activate itself. VALIDATE: real HTTP foreign access, stale quote, wrong app, wrong actor, changed build/version/profile/environment and blocked readiness tests.
4. **Reserve atomically at every run create.** ACTION: wire three create services and controller quote header. IMPLEMENT: app lock → replay lookup → current manifest/quote verification → cap count → insert run/attempts plus unique reservation → commit → wake worker. MIRROR: `runs.rs:219-296`, `suite_runs.rs:109-175`, `case_runs.rs:91-168`. IMPORTS: `commercial` service, `HeaderMap`, `TransactionTrait`. GOTCHA: no second reservation on retry; a different quote/price under same idempotency key conflicts; a failed transaction queues nothing. VALIDATE: concurrent two-tab cap test, idempotent retry, replay after quote expiry, rollback on readiness change, all three endpoints fail closed.
5. **Close resource bypasses and settle reports.** ACTION: enforce commercial policy on phone open/task/command, AI generation and saved-case execution; add reviewed delivery/credit action and ledger read. IMPLEMENT: continue allowing stop/cancel, report reads, evidence reads and safe editing. Preserve attempt evidence and reason-coded credit audit. MIRROR: controller inventory above and `errors.rs:18-48`, `tracing::info!` pattern. IMPORTS: `commercial` service. GOTCHA: never infer a free check from a failed app verdict; do not charge a run with no usable report caused by our infrastructure. VALIDATE: real route tests for each bypass and settlement state, foreign credit/access, period boundary, blocked/canceled outcome.
6. **Implement Mantine paywall/usage UI.** ACTION: translate GAN prototype into a workspace-linked page and shared authorization component. IMPLEMENT: proposed pricing before agreement; active agreement/period/remaining checks/ledger after activation; exact quote at each supported run path; explicit action; concise error and refresh path; mobile and keyboard support. MIRROR: `theme.ts:11-81`, `Settings.tsx:16-110`, `run-preview.tsx:33-143`, `api/runs.ts:49-59`, `api/session-transport.ts:13-23`. IMPORTS: Mantine, React Query, generated SDK/types/Zod, `useWorkspace`, shared feedback. GOTCHA: no UI-calculated authority or fake “Pay now”; keep quote stable with existing pending idempotency key on network retry. VALIDATE: component tests at 360px logic-level, keyboard semantics, explicit consent and no-account/expired/credit states; browser visual inspection only after admin policy permits it.
7. **Reconcile and roll out.** ACTION: add operator report export, fixture setup, documentation and feature acceptance. IMPLEMENT: compare ledger to run IDs and reviewed reports; add metrics for reserved/delivered/credited, device/model/people costs; document manual invoice handoff and legacy policy. MIRROR: `07-pilot-readiness-and-scale.md:38-42`. IMPORTS: none new. GOTCHA: no live charge or deployment before customer contract and technical gates. VALIDATE: one fresh DB end-to-end fake-worker route campaign plus existing checks, then customer-authorized pilot separately.

## Testing and validation

| Test                                                   | Expected result                                                                                                            |
| ------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------- |
| New app, no agreement; expired/paused agreement        | All resource-consuming starts reject; read/setup and stop/cancel remain available.                                         |
| Pilot check 1–4 and fifth attempt                      | Four `$0` included reservations; fifth denied; no accidental `$125` charge.                                                |
| Recurring 0/1/4/8 checks, optional second suite        | $250/$375/$750 base examples and $1,500 for eight with two suites; ninth denied; ten-case cap per suite.                   |
| Concurrent last-slot requests and idempotent retry     | At most one new reservation; replay returns same run/charge.                                                               |
| Quote changes/expiry/cross-tenant/reused authorization | 409/404 as appropriate; no run or reservation; no quote leaks.                                                             |
| Good report, app defect, infra failure, canceled queue | Reviewed good/app-defect reports billable; infra failure credited; queued cancellation credited; all reasons audited.      |
| Month/period edge and credits                          | Boundary assigned from persisted agreement; pending previous period remains there; credit restores cap in original period. |
| Browser consent and network error                      | No run before explicit authorization; no duplicate after retry; estimated and accrued amounts never conflated.             |

Finish all code/test/config/docs edits, then run one consolidated validation pass per `AGENTS.md`:

```bash
just types
just check-contracts
just check-web
just check-api
just build
```

The API check uses the project's configured test database; use a fresh disposable PostgreSQL test database if the retained local migration ledger is stale, as `docs/architect/status.md:17-25` notes. Run the focused real-route commercial tests within `check-api` (or explicitly with the same test DB) and review generated diff. Do not run device/model acceptance in ordinary checks. Browser rendering remains unverified until the existing admin-policy denial is resolved; do not use another browser path to work around it.

## Acceptance and risks

- [ ] Proposed offer and contract policy are internally consistent and clearly labeled; no instant purchase claim.
- [ ] Server quotes exact qualified run intent and price in integer cents; customer explicitly authorizes each check.
- [ ] All execution/model entry points obey the access policy; no UI-only paywall.
- [ ] Atomic ledger produces at most one reservation per run and no ninth recurring check.
- [ ] Customer can see base, add-on, delivered checks, pending authorization and credits; manual invoice reconciliation matches run/report IDs.
- [ ] Real route tests prove tenant isolation, replay, concurrency, failure and cancellation rules; generated transport remains the only browser source.
- [ ] Product/decision docs updated with proposed status; status only advances on observed evidence.

**Main risks:** (1) pricing and cost are not yet validated on a real customer app; (2) technical hosted/reliability gates remain open; (3) report review is a human decision and must not be automated from verdict; (4) the active branch has extensive unrelated work and an in-progress migration; (5) browser admin-policy denial prevents visual acceptance. Mitigate by treating this as a proposed implementation plan, keeping manual settlement, coordinating touched files, and shipping the pilot only after documented technical and customer agreement gates.

**External research:** none needed for this increment because no payment provider is integrated; vendor selection, tax handling and payment terms need their own research before automated checkout. **Implementation confidence:** 7/10 for the internal paywall/metering design, lower for commercial rollout until a real pilot measures cost and buyers accept the offer.
