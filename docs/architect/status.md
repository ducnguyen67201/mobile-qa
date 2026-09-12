# Implementation status and evidence

Last reconciled: 2026-09-12. Runtime source baseline: `fe3cf30` on
[PR #1](https://github.com/ducnguyen67201/mobile-qa/pull/1). This documentation consolidation
changes no runtime source or dependencies. This is a milestone record, not a live CI badge.

## Implemented and observed

- Separate API/web/worker apps; root Cargo workspace and pure contract crate.
- Loco health API, generated browser SDK/types/Zod validation, structured API errors,
  and actual route/OpenAPI agreement coverage.
- SeaORM migration wiring and real local PostgreSQL integration; no product tables yet.
- React navigation and health loading/error/Retry UI; other product routes are placeholders.
- Rust worker JSON Schema, generated Pydantic and deterministic passed/failed/blocked fixtures.
- Minitap 4.0.0 installed as an optional SDK extra; import tested with connections blocked.
- Explicit setup/dev/generation/check/build/smoke commands; no check watchers. CI selects
  affected apps/contracts and cancels superseded PR runs.
- Doppler project `mobile-qa`, scoped locally to `dev`; live CLI process injection verified
  from apps/api. No secret values printed. Vite does not load env files; only the API child
  receives Doppler-fetched values during normal development.

## Validation evidence

The apps/contracts refactor completed 53 unique tests: 21 web, 5 Rust, 26 Python and
1 exporter synchronization test. Strict TypeScript, ESLint, Rust format/Clippy, Ruff,
Pyright, generated drift, builds and runtime smoke passed. Frontend and installed-Python
advisory checks were clean after scoped dependency fixes; the unpublished local worker
is not covered by PyPI advisory lookup.

The affected-CI follow-up verified 23 path cases with picomatch, workflow YAML/command
assertions, API-only integration/build checks and contract-only checks. The Doppler
follow-up passed the 21 affected web tests, typecheck/lint/build, recipe dry run and a
synthetic process isolation/cleanup probe. A subsequent live Doppler metadata assertion
verified the configured project/config without dumping secrets.

[Hosted workflow run for fe3cf30](https://github.com/ducnguyen67201/mobile-qa/actions/runs/34682742656)
completed successfully for scope, API, web, worker and contracts. CodeRabbit status was
also successful when inspected; this is not a claim of a separate human review.

## Open gates and planned work

Graphify refresh workflow and agent navigation guidance are authored locally. Validation
with graphifyy 0.9.58 produced 305 nodes and 399 edges across 53 source files, including
Rust, TypeScript and Python; source paths were portable and generated consumers excluded.
A health-route query returned the route and Rust contract. Repeated generation was
byte-identical for unchanged input. Actionlint passed. Local bare-Git scenarios verified
no-change behavior, output-only publication, stale-result discard and non-fast-forward
rejection. The local preview is not committed because its working files include edits
newer than the embedded HEAD provenance.

Hosted generation and bot publication remain unverified until this change is merged to
main and the first `Refresh codebase graph` run succeeds. These checks do not establish
hosted token permissions or repeat the unrelated application suites.

| Item | Status / next evidence |
|---|---|
| Spec 01 rendered browser/keyboard/Retry/HMR acceptance | Pending: browser tool could not verify admin policy; no bypass attempted |
| Spec 02 real Android/cloud qualification | Planned; no cloud host, emulator execution, model calls or device evidence yet |
| Spec 03 auth/app creation/APK upload | Planned; current scaffold is unauthenticated |
| Spec 04 worker HTTP leases/runs/evidence reports | Planned; local fake protocol is not a scheduler |
| Spec 05 versioned case/suite/plan editor and approvals | Planned |
| Spec 06 requirements-to-tests generation | Planned |
| Spec 07 regression, retention, production deployment and pilot reliability | Planned |
| Paid pilot/customer validation | No accepted customer app, signed pilot or demonstrated willingness to pay recorded |

Doppler project creation does not provision database/model credentials, deploy a service,
or implement customer-secret resolution. Model/device performance, reset reliability,
HMR timing and production readiness must not be inferred from foundation tests.

The source foundation supports independent device feasibility (02) and app setup (03)
work with agreed ownership. Preserve spec 01's open browser gate during that work.
Historical planning reports remain outside the repository; this portable record replaces
them as the canonical status entry point. Update this file with new evidence rather than
copying old pass counts into every specification.
