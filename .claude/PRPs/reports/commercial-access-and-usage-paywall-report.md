# Implementation Report: Commercial access and usage paywall

This report records the earlier fixed-check increment. The 2026-09-22 monthly
credit successor is documented in [product Section 28](../../../docs/architect/product.md#28-monthly-credit-plans--local-implementation)
and [current status](../../../docs/architect/status.md).

## Summary

The operated Android QA offer now has local, server-enforced commercial access. A
trusted CLI activates one app/profile/coverage agreement for a 14-day pilot or an
explicit recurring period. A customer requests an exact, expiring quote for one
saved release plan or suite on one build, reviews the price, and authorizes it
before the run is queued. The accepted run reserves one check in the same database
transaction; replay preserves that reservation, and concurrent submissions cannot
exceed the agreed cap. The customer page shows the agreement, base/add-on, pending
checks, delivered charges, credits and run IDs. Pilot requests and invoicing are
manual; no payment provider or renewal was added.

## Assessment versus plan

| Metric     | Planned              | Actual                                                                |
| ---------- | -------------------- | --------------------------------------------------------------------- |
| Complexity | XL                   | XL across migration, Rust contracts/API, React and tests              |
| Confidence | 7/10 internal design | Local route and UI behavior verified; live economics unmeasured       |
| Files      | 22–28 estimated      | 46 source/generated/document files plus this report and archived plan |

## Tasks completed

| #   | Task                           | Result                                                                                                            |
| --- | ------------------------------ | ----------------------------------------------------------------------------------------------------------------- |
| 1   | Commercial rules               | Proposed terms, check/credit policy, manual settlement and rollout gates documented                               |
| 2   | Durable schema and contracts   | Agreement, quote, usage, audit and pilot-request tables; generated OpenAPI/SDK/Zod                                |
| 3   | Activation, status and quote   | Process-only audited activation/status changes; tenant-scoped status, request and exact quote routes              |
| 4   | Atomic run authorization       | Release-plan and suite creates verify quote under app lock; replay and cap preserved                              |
| 5   | Resource and settlement policy | Saved-case/phone/authoring starts gated; queued pre-claim cancellation credits; reviewed delivery/credit commands |
| 6   | Customer UI                    | Pricing/usage page, pilot request, explicit run quote confirmation and report billing status                      |
| 7   | Reconciliation and rollout     | CLI exports base/add-on/check subtotal and ledger; hosted rollout remains gated by product acceptance             |

## Validation

| Check                                 | Result                                                                                             |
| ------------------------------------- | -------------------------------------------------------------------------------------------------- |
| `just types` / `just check-contracts` | Pass, zero generated drift                                                                         |
| `just format`                         | Pass after installing the worktree's locked worker dependencies                                    |
| `just check-web`                      | Pass: format, typecheck, lint, 164 tests across 26 files after 2026-09-22 page refresh             |
| `just check-api`                      | Pass: Rust fmt, Clippy and workspace tests on fresh PostgreSQL, including 3 commercial route tests |
| `just build`                          | Pass: web production bundle and Cargo workspace                                                    |
| Rendered browser/device acceptance    | Open under existing admin-policy denial and explicit real-device qualification gates               |

The commercial route tests cover pilot inclusion and four-check cap, two concurrent
last-slot submissions, one reservation on replay, quote reuse rejection, tenant
isolation, saved-case/phone bypass denial, cancellation credit, recurring `$125`
quote/`$250` base, and closing an expired agreement before a new period. Web tests
cover explicit consent, stable retry identity, pilot request and delivered-charge
ledger presentation. The route campaign used synthetic signed APK fixtures; no
device or model call was made.

The 2026-09-22 page refresh places agreement usage beside the offer, adds an
interactive recurring estimate and opens a paid-pilot request dialog. Component
tests verify that estimate changes do not create usage, the dialog submits one
request, and the active-agreement action links to saved suites. The dialog records
interest only; payment and agreement activation remain manual. The web production
bundle rebuilt successfully after this change.

## Deviations and limitations

- The isolated branch uses migration `000014` because `000013` is already in
  progress as unrelated uncommitted work in the original checkout.
- Existing integration fixtures without an agreement retain a test-environment-only
  execution path. Any contracted test app uses the real quote gate; hosted
  environments fail closed without an agreement. This keeps unrelated route tests
  meaningful while preserving production enforcement.
- The pilot contact path is an app-scoped request record readable by a trusted
  operator CLI, with no invented sales address or automatic message.
- A reserved check remains pending until a human reviews its report. Only pre-claim
  cancellation is credited automatically. An app failure does not itself earn a
  credit. No test here proves a real customer report is usable or that the proposed
  prices cover device, model and operator costs.
- The shared retained test database has an incompatible historical migration
  ledger. Validation used fresh temporary databases; test config was restored and
  those databases were removed afterward. The original dirty checkout was not
  changed.
- The paywall was translated from the GAN prototype to Mantine and tested through
  DOM behavior. Rendered visual acceptance was not attempted because of the
  documented browser admin-policy denial.

## Next gates

1. Review this branch and the signed customer scope before any hosted cutover.
2. Define a backfill/legacy policy for existing hosted apps and qualify the real
   Android profile plus usable report/review flow.
3. Run a customer-authorized pilot, measure device/model/operator cost and payment
   intent, then revisit the proposed prices and invoice terms.
