# PR Review: #16 — Add monthly credit paywall and scheduled plan changes

**Reviewed:** 2026-09-22

**Author:** ducnguyen67201

**Branch:** `codex/commercial-access-paywall` → `main`
**Decision:** COMMENT — author self-review; fixes pushed for independent review

## Summary

The first review pass found billing retry, lock ordering, schedule ownership, and webhook parsing risks. They were fixed on the PR branch. The over-engineering pass found one unused browser helper and removed it. The actual Stripe renewal and signed-in plan-change click path remain acceptance gates.

## Findings

### CRITICAL

None.

### HIGH — fixed

- [credit_billing.rs](../../../apps/api/src/services/credit_billing.rs): A lost Checkout response expired the local intent, so a retry could create another payable session with a new Stripe idempotency key. Retry now reuses the intent and key; an old session must be confirmed expired by Stripe before a new checkout opens. An unknown session stays blocked for billing review.
- [credit_billing.rs](../../../apps/api/src/services/credit_billing.rs): Renewal webhook processing locked the intent before the app, while plan changes locked the app before the intent. Both now lock app then intent.
- [credit_billing.rs](../../../apps/api/src/services/credit_billing.rs): A pre-existing one-phase Stripe schedule could be adopted and rewritten without proof that this app created it. The endpoint now leaves it for billing review; only a fully tagged two-phase change can be recovered.
- [credit_billing.rs](../../../apps/api/src/services/credit_billing.rs): An extreme untrusted Stripe signature timestamp could overflow signed 64-bit subtraction. Validation now uses 128-bit arithmetic and has a regression assertion.

### MEDIUM — fixed

- [commercial.rs](../../../apps/api/src/services/commercial.rs): Legacy agreement counts and delivered cents were derived from the most recent 100 usage rows. Aggregates now count every row; the activity list remains capped.
- [saved-suite-run.tsx](../../../apps/web/src/components/runs/saved-suite-run.tsx): Saving an edited suite changes its version and dirty props. That transition could clear the quote for the just-saved version. The matching prepared version remains authorized; a component test exercises the prop transition.

### LOW — fixed

- [Settings.tsx](../../../apps/web/src/pages/Settings.tsx): Pricing navigation described the superseded pilot offer. It now describes monthly plans and credit usage.

## Ponytail review

`apps/web/src/api/commercial.ts:L36: delete: unused requestCommercialPilot browser wrapper. Nothing replaces it; the legacy server route stays for existing agreements.`

`net: -14 lines possible.` Applied.

## Validation results

| Check                                                           | Result                                                    |
| --------------------------------------------------------------- | --------------------------------------------------------- |
| Generated contract drift                                        | Pass: zero changes                                        |
| Web format, typecheck, lint                                     | Pass                                                      |
| Web tests                                                       | Pass: 167/167                                             |
| Web production build                                            | Pass                                                      |
| Rust format, workspace Clippy                                   | Pass                                                      |
| Commercial integration tests                                    | Pass: 6/6 on disposable PostgreSQL with configured JDK 17 |
| Webhook timestamp unit test                                     | Pass                                                      |
| API binary build                                                | Pass                                                      |
| Actual Stripe paid renewal and signed-in plan-change click path | Not yet verified                                          |

The first disposable-database test attempt selected the wrong JDK for APK fixture validation; the rerun with the configured JDK 17 passed all six tests. The shared retained test database still contains an unrelated historical migration that prevents the local full API command. The original PR commit passed all applicable CI jobs.

## Files in PR

The review concentrated on application source, migrations, transport definitions, tests, and architecture changes. Generated OpenAPI/SDK files were checked through the contract drift gate and dependency audit. This is a large PR; independent reviewer attention should focus on Stripe scheduling and the paid-invoice transition.

- Added: `.claude/PRPs/plans/completed/commercial-access-and-usage-paywall.plan.md`
- Added: `.claude/PRPs/reports/commercial-access-and-usage-paywall-report.md`
- Modified: `Cargo.lock`
- Modified: `apps/api/Cargo.toml`
- Modified: `apps/api/migration/src/lib.rs`
- Added: `apps/api/migration/src/m20260921_000014_commercial_access.rs`
- Added: `apps/api/migration/src/m20260922_000015_credit_billing.rs`
- Added: `apps/api/migration/src/m20260922_000016_credit_plan_changes.rs`
- Modified: `apps/api/src/app.rs`
- Added: `apps/api/src/controllers/commercial.rs`
- Modified: `apps/api/src/controllers/mod.rs`
- Modified: `apps/api/src/controllers/runs.rs`
- Modified: `apps/api/src/services/case_runs.rs`
- Added: `apps/api/src/services/commercial.rs`
- Added: `apps/api/src/services/credit_billing.rs`
- Modified: `apps/api/src/services/mod.rs`
- Modified: `apps/api/src/services/runs.rs`
- Modified: `apps/api/src/services/scheduler.rs`
- Modified: `apps/api/src/services/suite_runs.rs`
- Modified: `apps/api/src/services/task_sessions.rs`
- Modified: `apps/api/src/services/test_authoring.rs`
- Modified: `apps/api/src/services/test_library.rs`
- Modified: `apps/api/src/tasks/operator.rs`
- Added: `apps/api/tests/commercial.rs`
- Modified: `apps/api/tests/health.rs`
- Modified: `apps/api/tests/test_library.rs`
- Added: `apps/web/src/api/commercial.ts`
- Modified: `apps/web/src/api/generated/index.ts`
- Modified: `apps/web/src/api/generated/sdk.gen.ts`
- Modified: `apps/web/src/api/generated/types.gen.ts`
- Modified: `apps/web/src/api/generated/zod.gen.ts`
- Modified: `apps/web/src/api/regression.ts`
- Modified: `apps/web/src/api/runs.ts`
- Modified: `apps/web/src/components/app/run-preview.tsx`
- Added: `apps/web/src/components/commercial/RunAuthorization.tsx`
- Modified: `apps/web/src/components/runs/saved-case-run.tsx`
- Modified: `apps/web/src/components/runs/saved-suite-run.test.tsx`
- Modified: `apps/web/src/components/runs/saved-suite-run.tsx`
- Modified: `apps/web/src/pages/AppDetail.test.tsx`
- Added: `apps/web/src/pages/CommercialAccess.test.tsx`
- Added: `apps/web/src/pages/CommercialAccess.tsx`
- Modified: `apps/web/src/pages/RunDetail.tsx`
- Modified: `apps/web/src/pages/Settings.tsx`
- Modified: `apps/web/src/pages/TestLibrary.test.tsx`
- Modified: `apps/web/src/pages/TestLibraryDetail.tsx`
- Modified: `apps/web/src/pages/Tests.tsx`
- Modified: `apps/web/src/routes.tsx`
- Modified: `contracts/browser.openapi.json`
- Modified: `crates/contracts/src/browser.rs`
- Added: `crates/contracts/src/commercial.rs`
- Added: `crates/contracts/src/commercial_api.rs`
- Modified: `crates/contracts/src/execution_api.rs`
- Modified: `crates/contracts/src/lib.rs`
- Modified: `crates/contracts/src/regression_api.rs`
- Modified: `docs/architect/decisions.md`
- Modified: `docs/architect/environment.md`
- Modified: `docs/architect/implementation/05-test-library-and-plans.md`
- Modified: `docs/architect/implementation/07-pilot-readiness-and-scale.md`
- Modified: `docs/architect/product.md`
- Modified: `docs/architect/status.md`
- Added: `.claude/PRPs/reviews/pr-16-review.md`
